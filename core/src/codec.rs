//! File-format side: base64, CRC32, gzip (inflate + a stored-block writer), and
//! the .gdlvl patcher. Pure std so it also builds for wasm32.
//!
//! Layers handled by the patcher:
//!   outer file  : gzip | base64(gzip) | plain XML      (auto-detected)
//!   inside, XML : <k>k4</k><s>LEVEL STRING</s>
//!   level string: "H4sI..." = urlsafe-base64(gzip(raw)) or already raw
//!   raw         : "header;obj;obj;obj;"

// ---------------------------------------------------------------- base64

pub fn b64_encode(data: &[u8], urlsafe: bool) -> String {
    let alpha: &[u8; 64] = if urlsafe {
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"
    } else {
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
    };
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for ch in data.chunks(3) {
        let b = [ch[0], *ch.get(1).unwrap_or(&0), *ch.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(alpha[(n >> 18 & 63) as usize] as char);
        out.push(alpha[(n >> 12 & 63) as usize] as char);
        out.push(if ch.len() > 1 { alpha[(n >> 6 & 63) as usize] as char } else { '=' });
        out.push(if ch.len() > 2 { alpha[(n & 63) as usize] as char } else { '=' });
    }
    out
}

/// Accepts both the standard and url-safe alphabets; ignores padding and whitespace.
pub fn b64_decode(s: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let mut acc: u32 = 0;
    let mut bits = 0u32;
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' | b' ' | b'\n' | b'\r' | b'\t' => continue,
            _ => return Err(format!("invalid base64 character {:?}", c as char)),
        };
        acc = (acc << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1u32 << bits) - 1;
        }
    }
    Ok(out)
}

fn is_b64_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'-' | b'_' | b'=' | b' ' | b'\n' | b'\r' | b'\t')
}

// ---------------------------------------------------------------- crc32

pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xEDB8_8320 & 0u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}

// ---------------------------------------------------------------- inflate (RFC 1951)

const MAX_OUT: usize = 64 * 1024 * 1024;

struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
    buf: u32,
    cnt: u32,
}

impl<'a> Bits<'a> {
    fn bits(&mut self, need: u32) -> Result<u32, String> {
        let mut val = self.buf;
        while self.cnt < need {
            if self.pos >= self.data.len() {
                return Err("unexpected end of compressed data".into());
            }
            val |= (self.data[self.pos] as u32) << self.cnt;
            self.pos += 1;
            self.cnt += 8;
        }
        self.buf = val >> need;
        self.cnt -= need;
        Ok(val & ((1u32 << need) - 1))
    }
}

struct Huff {
    count: [u16; 16],
    symbol: Vec<u16>,
}

fn build(lengths: &[u16]) -> Result<Huff, String> {
    let mut count = [0u16; 16];
    for &l in lengths {
        count[l as usize] += 1;
    }
    let mut left: i32 = 1;
    for len in 1..16 {
        left <<= 1;
        left -= count[len] as i32;
        if left < 0 {
            return Err("invalid huffman table".into());
        }
    }
    let mut offs = [0u16; 16];
    for len in 1..15 {
        offs[len + 1] = offs[len] + count[len];
    }
    let mut symbol = vec![0u16; lengths.len()];
    for (sym, &l) in lengths.iter().enumerate() {
        if l != 0 {
            symbol[offs[l as usize] as usize] = sym as u16;
            offs[l as usize] += 1;
        }
    }
    Ok(Huff { count, symbol })
}

fn decode(br: &mut Bits, h: &Huff) -> Result<u16, String> {
    let (mut code, mut first, mut index) = (0i32, 0i32, 0i32);
    for len in 1..=15 {
        code |= br.bits(1)? as i32;
        let count = h.count[len] as i32;
        if code - count < first {
            return Ok(h.symbol[(index + (code - first)) as usize]);
        }
        index += count;
        first += count;
        first <<= 1;
        code <<= 1;
    }
    Err("invalid huffman code".into())
}

const LBASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LEXT: [u32; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DBASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DEXT: [u32; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

fn codes(br: &mut Bits, out: &mut Vec<u8>, lc: &Huff, dc: &Huff) -> Result<(), String> {
    loop {
        let sym = decode(br, lc)? as usize;
        if sym < 256 {
            out.push(sym as u8);
        } else if sym == 256 {
            return Ok(());
        } else {
            let s = sym - 257;
            if s >= 29 {
                return Err("bad length symbol".into());
            }
            let len = LBASE[s] as usize + br.bits(LEXT[s])? as usize;
            let ds = decode(br, dc)? as usize;
            if ds >= 30 {
                return Err("bad distance symbol".into());
            }
            let dist = DBASE[ds] as usize + br.bits(DEXT[ds])? as usize;
            if dist > out.len() {
                return Err("distance reaches before start of data".into());
            }
            for _ in 0..len {
                let b = out[out.len() - dist];
                out.push(b);
            }
        }
        if out.len() > MAX_OUT {
            return Err("decompressed data is too large".into());
        }
    }
}

fn inflate(br: &mut Bits) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    loop {
        let last = br.bits(1)?;
        match br.bits(2)? {
            0 => {
                br.buf = 0;
                br.cnt = 0;
                if br.pos + 4 > br.data.len() {
                    return Err("truncated stored block".into());
                }
                let len = br.data[br.pos] as usize | (br.data[br.pos + 1] as usize) << 8;
                let nlen = br.data[br.pos + 2] as usize | (br.data[br.pos + 3] as usize) << 8;
                if len != (!nlen & 0xffff) {
                    return Err("corrupt stored block".into());
                }
                br.pos += 4;
                if br.pos + len > br.data.len() {
                    return Err("truncated stored block".into());
                }
                out.extend_from_slice(&br.data[br.pos..br.pos + len]);
                br.pos += len;
            }
            1 => {
                let mut l = [0u16; 288];
                for (i, v) in l.iter_mut().enumerate() {
                    *v = match i {
                        0..=143 => 8,
                        144..=255 => 9,
                        256..=279 => 7,
                        _ => 8,
                    };
                }
                let lc = build(&l)?;
                let dc = build(&[5u16; 30])?;
                codes(br, &mut out, &lc, &dc)?;
            }
            2 => {
                let nlen = br.bits(5)? as usize + 257;
                let ndist = br.bits(5)? as usize + 1;
                let ncode = br.bits(4)? as usize + 4;
                if nlen > 286 || ndist > 30 {
                    return Err("bad dynamic block header".into());
                }
                const ORDER: [usize; 19] =
                    [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];
                let mut lengths = [0u16; 320];
                for &o in ORDER.iter().take(ncode) {
                    lengths[o] = br.bits(3)? as u16;
                }
                let cl = build(&lengths[..19])?;
                let mut idx = 0;
                while idx < nlen + ndist {
                    let sym = decode(br, &cl)?;
                    if sym < 16 {
                        lengths[idx] = sym;
                        idx += 1;
                    } else {
                        let (val, rep) = match sym {
                            16 => {
                                if idx == 0 {
                                    return Err("repeat with no previous length".into());
                                }
                                (lengths[idx - 1], 3 + br.bits(2)? as usize)
                            }
                            17 => (0, 3 + br.bits(3)? as usize),
                            _ => (0, 11 + br.bits(7)? as usize),
                        };
                        if idx + rep > nlen + ndist {
                            return Err("too many code lengths".into());
                        }
                        for _ in 0..rep {
                            lengths[idx] = val;
                            idx += 1;
                        }
                    }
                }
                let lc = build(&lengths[..nlen])?;
                let dc = build(&lengths[nlen..nlen + ndist])?;
                codes(br, &mut out, &lc, &dc)?;
            }
            _ => return Err("invalid block type".into()),
        }
        if last == 1 {
            return Ok(out);
        }
    }
}

pub fn gunzip(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() < 18 || data[0] != 0x1f || data[1] != 0x8b {
        return Err("not gzip data".into());
    }
    if data[2] != 8 {
        return Err("unsupported gzip compression method".into());
    }
    let flg = data[3];
    let mut pos = 10;
    if flg & 4 != 0 {
        if pos + 2 > data.len() {
            return Err("truncated gzip header".into());
        }
        pos += 2 + (data[pos] as usize | (data[pos + 1] as usize) << 8);
    }
    for bit in [8u8, 16u8] {
        if flg & bit != 0 {
            while pos < data.len() && data[pos] != 0 {
                pos += 1;
            }
            pos += 1;
        }
    }
    if flg & 2 != 0 {
        pos += 2;
    }
    if pos >= data.len() {
        return Err("truncated gzip header".into());
    }
    let mut br = Bits {
        data,
        pos,
        buf: 0,
        cnt: 0,
    };
    let out = inflate(&mut br)?;
    let t = br.pos;
    if t + 8 <= data.len() {
        let want = u32::from_le_bytes([data[t], data[t + 1], data[t + 2], data[t + 3]]);
        if crc32(&out) != want {
            return Err("gzip checksum mismatch (file corrupted?)".into());
        }
    }
    Ok(out)
}

/// Valid gzip made of stored (uncompressed) blocks. Bigger than real compression,
/// but every inflater accepts it, and it keeps this crate dependency-free.
/// Swap in `flate2` if you want smaller output.
pub fn gzip_stored(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 0xff];
    if data.is_empty() {
        out.extend_from_slice(&[1, 0, 0, 0xff, 0xff]);
    }
    let mut chunks = data.chunks(65535).peekable();
    while let Some(c) = chunks.next() {
        out.push(if chunks.peek().is_none() { 1 } else { 0 });
        let len = c.len() as u16;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(c);
    }
    out.extend_from_slice(&crc32(data).to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out
}

// ---------------------------------------------------------------- .gdlvl

pub(crate) enum Wrap {
    Gzip,
    B64 { urlsafe: bool },
    Plain,
}

pub(crate) fn utf8(b: Vec<u8>) -> Result<String, String> {
    String::from_utf8(b).map_err(|_| "decoded data is not valid UTF-8 text".to_string())
}

pub(crate) fn unwrap_file(file: &[u8]) -> Result<(Wrap, String), String> {
    if file.len() >= 2 && file[0] == 0x1f && file[1] == 0x8b {
        return Ok((Wrap::Gzip, utf8(gunzip(file)?)?));
    }
    if let Ok(text) = std::str::from_utf8(file) {
        let t = text.trim();
        if t.starts_with('<') {
            return Ok((Wrap::Plain, text.to_string()));
        }
        if !t.is_empty() && t.bytes().all(is_b64_byte) {
            let bytes = b64_decode(t)?;
            if bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
                let urlsafe = t.contains('-') || t.contains('_');
                return Ok((Wrap::B64 { urlsafe }, utf8(gunzip(&bytes)?)?));
            }
        }
    }
    let head: Vec<String> = file.iter().take(16).map(|b| format!("{b:02x}")).collect();
    Err(format!(
        "unrecognized file format (expected gzip, base64+gzip or XML); first bytes: {}",
        head.join(" ")
    ))
}

impl Wrap {
    pub(crate) fn name(&self) -> &'static str {
        match self {
            Wrap::Gzip => "gzip",
            Wrap::B64 { urlsafe: false } => "base64",
            Wrap::B64 { urlsafe: true } => "base64-urlsafe",
            Wrap::Plain => "xml",
        }
    }

    pub(crate) fn from_name(s: &str) -> Option<Wrap> {
        Some(match s {
            "gzip" => Wrap::Gzip,
            "base64" => Wrap::B64 { urlsafe: false },
            "base64-urlsafe" => Wrap::B64 { urlsafe: true },
            "xml" => Wrap::Plain,
            _ => return None,
        })
    }
}

pub(crate) fn rewrap(wrap: &Wrap, text: &str) -> Vec<u8> {
    match wrap {
        Wrap::Plain => text.as_bytes().to_vec(),
        Wrap::Gzip => gzip_stored(text.as_bytes()),
        Wrap::B64 { urlsafe } => b64_encode(&gzip_stored(text.as_bytes()), *urlsafe).into_bytes(),
    }
}

/// Byte range of the text inside `<k>k4</k><s>...</s>`.
pub(crate) fn find_k4(xml: &str) -> Option<(usize, usize)> {
    let key = "<k>k4</k>";
    let after = xml.find(key)? + key.len();
    let s_rel = xml[after..].find("<s>")?;
    if !xml[after..after + s_rel].trim().is_empty() {
        return None;
    }
    let start = after + s_rel + 3;
    let end = start + xml[start..].find("</s>")?;
    Some((start, end))
}

pub(crate) struct K4 {
    pub(crate) raw: String,
    pub(crate) compressed: bool,
}

pub(crate) fn read_k4(s: &str) -> Result<K4, String> {
    let t = s.trim();
    if t.starts_with("H4sI") {
        let raw = utf8(gunzip(&b64_decode(t)?)?)?;
        Ok(K4 { raw, compressed: true })
    } else {
        Ok(K4 { raw: t.to_string(), compressed: false })
    }
}

/// The level header and the object records (everything after the first `;`) of a .gdlvl file.
pub fn level_objects(file: &[u8]) -> Result<(String, String), String> {
    let (_, text) = unwrap_file(file)?;
    let (a, b) = find_k4(&text).ok_or_else(|| {
        "couldn't find the level string (<k>k4</k>) in this file".to_string()
    })?;
    let k = read_k4(&text[a..b])?;
    Ok(match k.raw.split_once(';') {
        Some((h, rest)) => (h.to_string(), rest.to_string()),
        None => (k.raw, String::new()),
    })
}

/// One object per line: index, object id, and the raw key,value record. Handy for
/// finding the IDs and property keys of anything you placed in the editor.
pub fn list_objects(file: &[u8]) -> Result<String, String> {
    let (header, objects) = level_objects(file)?;
    let mut out = format!("header: {header}\n");
    for (i, rec) in objects.split(';').filter(|r| !r.is_empty()).enumerate() {
        let parts: Vec<&str> = rec.split(',').collect();
        let id = parts
            .chunks(2)
            .find(|c| c[0] == "1")
            .and_then(|c| c.get(1))
            .copied()
            .unwrap_or("?");
        out.push_str(&format!("{i:>5}  id {id:<5}  {rec}\n"));
    }
    Ok(out)
}

/// Human-readable dump of a .gdlvl file: layers, XML, and the level-string summary.
pub fn inspect(file: &[u8]) -> Result<String, String> {
    let (wrap, text) = unwrap_file(file)?;
    let kind = match wrap {
        Wrap::Gzip => "gzip",
        Wrap::B64 { .. } => "base64(gzip)",
        Wrap::Plain => "plain XML",
    };
    let mut out = format!("outer layer: {kind}\n\n--- xml (k4 payload elided) ---\n");
    match find_k4(&text) {
        Some((a, b)) => {
            let k = read_k4(&text[a..b])?;
            out.push_str(&text[..a]);
            out.push_str(&format!("<<{} chars elided>>", b - a));
            out.push_str(&text[b..]);
            let objs = k.raw.split(';').filter(|s| !s.is_empty()).count().saturating_sub(1);
            out.push_str("\n\n--- summary ---\n");
            out.push_str(&format!(
                "k4: {} level string, {} chars decoded, about {} objects\n",
                if k.compressed { "compressed" } else { "raw" },
                k.raw.len(),
                objs
            ));
            out.push_str(&format!("level header: {}\n", k.raw.split(';').next().unwrap_or("")));
        }
        None => {
            out.push_str(&text);
            out.push_str("\n\n--- summary ---\nk4 (level string) not found in this XML.\n");
        }
    }
    Ok(out)
}

/// Append `objects` (like "1,1,2,15,3,15;") to the level string inside a .gdlvl file,
/// keeping the same outer wrapping and k4 compression the file already uses.
pub fn patch_level_file(file: &[u8], objects: &str) -> Result<Vec<u8>, String> {
    let (wrap, text) = unwrap_file(file)?;
    let (a, b) = find_k4(&text).ok_or_else(|| {
        "couldn't find <k>k4</k><s>…</s> (the level string) in this file; run `inspect` and check its layout".to_string()
    })?;
    let k = read_k4(&text[a..b])?;
    let mut raw = k.raw;
    if !raw.is_empty() && !raw.ends_with(';') {
        raw.push(';');
    }
    raw.push_str(objects);
    let new_k4 = if k.compressed {
        b64_encode(&gzip_stored(raw.as_bytes()), true)
    } else {
        raw
    };
    let mut out = String::with_capacity(text.len() + new_k4.len());
    out.push_str(&text[..a]);
    out.push_str(&new_k4);
    out.push_str(&text[b..]);
    Ok(rewrap(&wrap, &out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_roundtrip() {
        for n in 0..40 {
            let data: Vec<u8> = (0..n).map(|i| (i * 37 + 11) as u8).collect();
            for url in [false, true] {
                assert_eq!(b64_decode(&b64_encode(&data, url)).unwrap(), data);
            }
        }
        assert_eq!(b64_encode(b"Man", false), "TWFu");
        assert_eq!(b64_encode(b"Ma", false), "TWE=");
    }

    #[test]
    fn crc_known_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn gzip_roundtrip() {
        for n in [0usize, 1, 100, 65535, 65536, 200_000] {
            let data: Vec<u8> = (0..n).map(|i| (i % 251) as u8).collect();
            assert_eq!(gunzip(&gzip_stored(&data)).unwrap(), data, "n={n}");
        }
    }

    fn fake_file(wrap: u8, compressed_k4: bool) -> Vec<u8> {
        let level = "kS38,1_40_2_125;1,1,2,15,3,15;";
        let k4 = if compressed_k4 {
            b64_encode(&gzip_stored(level.as_bytes()), true)
        } else {
            level.to_string()
        };
        let xml = format!(
            "<?xml version=\"1.0\"?><plist version=\"1.0\"><dict><k>k2</k><s>T</s><k>k4</k><s>{k4}</s></dict></plist>"
        );
        match wrap {
            0 => xml.into_bytes(),
            1 => gzip_stored(xml.as_bytes()),
            _ => b64_encode(&gzip_stored(xml.as_bytes()), false).into_bytes(),
        }
    }

    #[test]
    fn patch_all_layer_combinations() {
        for wrap in 0..3 {
            for comp in [false, true] {
                let f = fake_file(wrap, comp);
                let out = patch_level_file(&f, "1,8,2,45,3,15;").unwrap();
                let report = inspect(&out).unwrap();
                assert!(report.contains("about 2 objects"), "wrap={wrap} comp={comp}\n{report}");
            }
        }
    }
}
