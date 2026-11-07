use std::path::{Path, PathBuf};
use std::{env, fs, process, thread};

const USAGE: &str = "\
gdl - a tiny scripting language for Geometry Dash levels

usage:
  gdl run <script.gdl>                         run a script, print the objects it makes
  gdl inspect <level.gdlvl>                    decode a level file and show its layout
  gdl patch <level.gdlvl> <script.gdl> [-o out]  add the script's objects to a level file
";

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    // big stack so deeply recursive scripts don't overflow the native stack
    let handle = thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(move || real_main(args))
        .expect("failed to start thread");
    process::exit(handle.join().unwrap_or(1));
}

fn read(path: &str) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|e| format!("can't read {path}: {e}"))
}

fn read_text(path: &str) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("can't read {path}: {e}"))
}

fn real_main(args: Vec<String>) -> i32 {
    let res = match args.first().map(|s| s.as_str()) {
        Some("run") if args.len() == 2 => cmd_run(&args[1]),
        Some("inspect") if args.len() == 2 => cmd_inspect(&args[1]),
        Some("patch") if args.len() >= 3 => cmd_patch(&args[1..]),
        _ => {
            eprint!("{USAGE}");
            return 2;
        }
    };
    match res {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

fn cmd_run(script: &str) -> Result<(), String> {
    let out = gdl_core::run(&read_text(script)?)?;
    for l in &out.logs {
        eprintln!("print: {l}");
    }
    eprintln!("{} object(s)", out.objects.len());
    println!("{}", out.objects_string());
    Ok(())
}

fn cmd_inspect(file: &str) -> Result<(), String> {
    print!("{}", gdl_core::codec::inspect(&read(file)?)?);
    Ok(())
}

fn cmd_patch(args: &[String]) -> Result<(), String> {
    let mut pos: Vec<&str> = Vec::new();
    let mut out_path: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "-o" {
            out_path = Some(args.get(i + 1).ok_or("-o needs a path")?.clone());
            i += 2;
        } else {
            pos.push(&args[i]);
            i += 1;
        }
    }
    if pos.len() != 2 {
        return Err("patch needs a level file and a script".into());
    }
    let (level, script) = (pos[0], pos[1]);
    let run = gdl_core::run(&read_text(script)?)?;
    for l in &run.logs {
        eprintln!("print: {l}");
    }
    let patched = gdl_core::codec::patch_level_file(&read(level)?, &run.objects_string())?;
    let dest: PathBuf = match out_path {
        Some(p) => PathBuf::from(p),
        None => {
            let p = Path::new(level);
            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("level");
            let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("gdlvl");
            p.with_file_name(format!("{stem}.patched.{ext}"))
        }
    };
    fs::write(&dest, patched).map_err(|e| format!("can't write {}: {e}", dest.display()))?;
    eprintln!("added {} object(s) -> {}", run.objects.len(), dest.display());
    Ok(())
}
