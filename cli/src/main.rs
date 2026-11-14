use std::path::{Path, PathBuf};
use std::{env, fs, process, thread};

const USAGE: &str = "\
gdl - a tiny scripting language for Geometry Dash levels

usage:
  gdl run <script.gdl>                         run a script, print the objects it makes
  gdl inspect <level.gdlvl>                    decode a level file and show its layout
  gdl preview <script.gdl>                     the JSON the web preview draws (objects + script lines)
  gdl base <level.gdlvl>                       the same JSON for objects already in a level
  gdl objects <level.gdlvl>                    list every object (id + raw properties); use it to
                                               look up the IDs/keys of anything you placed in-game
  gdl patch <level.gdlvl> <script.gdl> [-o out]  add the script's objects to a level file

projects (.gdlrust, one plain-text file: script + level objects + level wrapper):
  gdl new <name>                               start a project: writes <name>.gdlrust
  gdl convert <level.gdlvl> [-o out] [--force] .gdlvl -> .gdlrust (never overwrites without --force)
  gdl convert <project.gdlrust> [-o out]       .gdlrust -> .gdlvl
  gdl compile <project.gdlrust> [-o out] [--into level.gdlvl] [--force]
                                               run the script and write the level back out

reference:
  gdl blocks                                   list the named block variants (block(\"spike\", x, y))
  gdl docs triggers|blocks                     print the generated references (docs/TRIGGERS.md, docs/BLOCKS.md)
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
        Some("--help") | Some("-h") | Some("help") => {
            print!("{USAGE}");
            return 0;
        }
        Some("run") if args.len() == 2 => cmd_run(&args[1]),
        Some("inspect") if args.len() == 2 => cmd_inspect(&args[1]),
        Some("preview") if args.len() == 2 => {
            read_text(&args[1]).map(|src| println!("{}", gdl_core::preview_json(&src)))
        }
        Some("base") if args.len() == 2 => {
            read(&args[1]).map(|f| println!("{}", gdl_core::base_json(&f)))
        }
        Some("objects") if args.len() == 2 => {
            read(&args[1]).and_then(|f| gdl_core::codec::list_objects(&f)).map(|s| print!("{s}"))
        }
        Some("patch") if args.len() >= 3 => cmd_patch(&args[1..]),
        Some("new") if args.len() == 2 => cmd_new(&args[1]),
        Some("convert") if args.len() >= 2 => cmd_convert(&args[1..]),
        Some("compile") if args.len() >= 2 => cmd_compile(&args[1..]),
        Some("blocks") if args.len() == 1 => {
            print!("{}", gdl_core::blocks::table());
            Ok(())
        }
        Some("docs") if args.len() == 2 && args[1] == "triggers" => {
            print!("{}", gdl_core::triggers::markdown());
            Ok(())
        }
        Some("docs") if args.len() == 2 && args[1] == "blocks" => {
            print!("{}", gdl_core::blocks::markdown());
            Ok(())
        }
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
    let out = if script.ends_with(".gdlrust") {
        let p = gdl_core::project::Project::parse(&read_text(script)?)?;
        let (g, c) = p.next_free_ids();
        gdl_core::run_with(&p.script, g, c)?
    } else {
        gdl_core::run(&read_text(script)?)?
    };
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

// ---------------------------------------------------------------- projects

struct Opts {
    pos: Vec<String>,
    out: Option<String>,
    into: Option<String>,
    force: bool,
}

fn parse_opts(args: &[String]) -> Result<Opts, String> {
    let mut o = Opts { pos: Vec::new(), out: None, into: None, force: false };
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" => {
                o.out = Some(args.get(i + 1).ok_or("-o needs a path")?.clone());
                i += 2;
            }
            "--into" => {
                o.into = Some(args.get(i + 1).ok_or("--into needs a level file")?.clone());
                i += 2;
            }
            "--force" => {
                o.force = true;
                i += 1;
            }
            a if a.starts_with('-') && a.len() > 1 => return Err(format!("unknown option {a}")),
            _ => {
                o.pos.push(args[i].clone());
                i += 1;
            }
        }
    }
    Ok(o)
}

/// Write a file, refusing to replace an existing one unless `force` is set. Projects and
/// levels are people's work; silently overwriting them would be the worst possible default.
fn write_new(path: &Path, bytes: &[u8], force: bool) -> Result<(), String> {
    if path.exists() && !force {
        return Err(format!("{} already exists (use --force to overwrite it, or -o to pick another name)", path.display()));
    }
    fs::write(path, bytes).map_err(|e| format!("can't write {}: {e}", path.display()))
}

fn stem_of(path: &str) -> String {
    Path::new(path).file_stem().and_then(|s| s.to_str()).unwrap_or("level").to_string()
}

fn sibling(path: &str, new_name: String) -> PathBuf {
    Path::new(path).with_file_name(new_name)
}

fn cmd_new(name: &str) -> Result<(), String> {
    let file = if name.ends_with(".gdlrust") { name.to_string() } else { format!("{name}.gdlrust") };
    let project = gdl_core::project::Project::new(&stem_of(&file));
    write_new(Path::new(&file), project.to_text()?.as_bytes(), false)?;
    eprintln!("created {file}");
    eprintln!("next: gdl run {file}   then   gdl compile {file} --into YourLevel.gdlvl");
    Ok(())
}

fn cmd_convert(args: &[String]) -> Result<(), String> {
    let o = parse_opts(args)?;
    if o.pos.len() != 1 {
        return Err("convert needs exactly one input file".into());
    }
    let input = &o.pos[0];
    if input.ends_with(".gdlrust") {
        return cmd_compile(args);
    }
    let bytes = read(input)?;
    let project = gdl_core::project::Project::from_level(&bytes, &stem_of(input))?;
    let dest = match &o.out {
        Some(p) => PathBuf::from(p),
        None => sibling(input, format!("{}.gdlrust", stem_of(input))),
    };
    write_new(&dest, project.to_text()?.as_bytes(), o.force)?;
    eprintln!(
        "{} -> {}  ({} object(s) kept, header {} chars)",
        input,
        dest.display(),
        project.objects.len(),
        project.header.len()
    );
    eprintln!("edit the script section, then: gdl compile {}", dest.display());
    Ok(())
}

fn cmd_compile(args: &[String]) -> Result<(), String> {
    let o = parse_opts(args)?;
    if o.pos.len() != 1 {
        return Err("compile needs exactly one .gdlrust file".into());
    }
    let input = &o.pos[0];
    let project = gdl_core::project::Project::parse(&read_text(input)?)?;
    let compiled = match &o.into {
        Some(level) => project.compile_into(&read(level)?)?,
        None => project.compile()?,
    };
    for l in &compiled.logs {
        eprintln!("print: {l}");
    }
    // never default to the original level's name: the output is always a new file
    let dest = match &o.out {
        Some(p) => PathBuf::from(p),
        None => match &o.into {
            Some(level) => sibling(level, format!("{}.compiled.gdlvl", stem_of(level))),
            None => sibling(input, format!("{}.compiled.gdlvl", stem_of(input))),
        },
    };
    write_new(&dest, &compiled.bytes, o.force)?;
    eprintln!("{} object(s) from the script -> {}", compiled.script_objects, dest.display());
    Ok(())
}
