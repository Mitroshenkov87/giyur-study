//! Command-line entry points: `shorashim ctl …`, `shorashim mcp`,
//! `shorashim --headless …`. Called first thing in `main`.

use std::io::{BufRead, Write};
use std::path::PathBuf;

use serde_json::{json, Map, Value};

use super::client::{self, Client};
use super::session::{Options, Session};

use crate::i18n::tr;

const USAGE: &str = "\
Shorashim automation (docs/AUTOMATION.md)

  shorashim --control                 start the app with the local control channel
  shorashim ctl <cmd> [key=value ...] send one command to the running app
  shorashim ctl --json '{\"cmd\":...}'  same, raw JSON
  shorashim ctl -                     JSON lines from stdin, one reply per line
  shorashim ctl launch                start the app with --control and wait for it
  shorashim mcp [--headless]          MCP server on stdio (attach to the app, or headless)
  shorashim --headless [--script FILE] [--data FILE] [--today YYYY-MM-DD] [--reviewer] [--keep-going]
                                      run JSON-line commands without a window
  shorashim validate-content [DIR]    check the course files (content/<lang>/) and exit
  shorashim validate-parity [DIR]     check content/en against content/ru (skip if en is absent)
  shorashim ctl reload-content        re-read edited course files in the running app
  shorashim --reviewer                start in the «Открытый режим рецензента» for this run
                                      (also SHORASHIM_REVIEWER=1; ctl launch --reviewer)
Options for ctl/mcp: --file PATH (control.json location)
";

/// Handle an automation subcommand; `None` = start the normal app.
pub fn main_hook() -> Option<i32> {
    main_hook_with(std::env::args().skip(1).collect())
}

/// Entry point of the console companion `shorashim-ctl`: the same commands,
/// and a bare `shorashim-ctl state` means `shorashim ctl state`.
pub fn companion_main() -> i32 {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let known = matches!(
        args.first().map(String::as_str),
        Some(
            "ctl"
                | "mcp"
                | "--headless"
                | "headless"
                | "--automation-help"
                | "validate-content"
                | "validate-parity"
        )
    );
    if !known {
        args.insert(0, "ctl".into());
    }
    main_hook_with(args).unwrap_or(0)
}

fn main_hook_with(args: Vec<String>) -> Option<i32> {
    let first = args.first().map(String::as_str)?;
    let code = match first {
        "ctl" => {
            attach_console();
            ctl(&args[1..])
        }
        "mcp" => {
            attach_console();
            super::mcp::main(&args[1..])
        }
        "--headless" | "headless" => {
            attach_console();
            headless(&args[1..])
        }
        "validate-content" | "--validate-content" => {
            attach_console();
            validate_content(&args[1..])
        }
        "validate-parity" | "--validate-parity" => {
            attach_console();
            validate_parity(&args[1..])
        }
        "--automation-help" => {
            attach_console();
            print!("{}", tr(USAGE));
            0
        }
        _ => return None,
    };
    Some(code)
}

/// Pull `--name value` out of `args`.
pub fn take_opt(args: &mut Vec<String>, name: &str) -> Option<String> {
    let i = args.iter().position(|a| a == name)?;
    if i + 1 >= args.len() {
        args.remove(i);
        return None;
    }
    let v = args.remove(i + 1);
    args.remove(i);
    Some(v)
}

pub fn take_flag(args: &mut Vec<String>, name: &str) -> bool {
    match args.iter().position(|a| a == name) {
        Some(i) => {
            args.remove(i);
            true
        }
        None => false,
    }
}

/// Headless options shared by `--headless` and `mcp --headless`.
pub fn session_options(args: &mut Vec<String>) -> Result<Options, String> {
    let data = take_opt(args, "--data").map(crate::storage::resolve_data_path);
    let today = match take_opt(args, "--today") {
        Some(d) => Some(
            chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d")
                .map_err(|_| format!("--today: expected YYYY-MM-DD, got {d}"))?,
        ),
        None => None,
    };
    let reviewer = take_flag(args, "--reviewer");
    Ok(Options {
        data,
        today,
        reviewer,
    })
}

fn headless(args: &[String]) -> i32 {
    let mut args = args.to_vec();
    let script = take_opt(&mut args, "--script");
    let keep_going = take_flag(&mut args, "--keep-going");
    let opts = match session_options(&mut args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };
    if let Err(e) = crate::content_store::start(opts.data.as_deref()) {
        eprintln!("course content not loaded: {e}");
        return 2;
    }
    if let Some(extra) = args.first() {
        eprintln!("unknown option {extra}\n{}", tr(USAGE));
        return 2;
    }
    let mut session = Session::new(opts);
    let stdout = std::io::stdout();
    let failed = match script.as_deref() {
        Some("-") | None => session.run_lines(std::io::stdin().lock(), stdout.lock(), false),
        Some(path) => match std::fs::File::open(path) {
            Ok(f) => session.run_lines(std::io::BufReader::new(f), stdout.lock(), !keep_going),
            Err(e) => {
                eprintln!("cannot open {path}: {e}");
                return 2;
            }
        },
    };
    i32::from(failed > 0)
}

/// `validate-content [DIR] [--lang ru]`: check the course files the way the
/// app reads them and print what is in each language. Without `--lang` every
/// language is checked (Russian and English must both be there) and
/// `content/en` must match `content/ru`: the same files, ids and fields.
/// Exit 0 = good, 1 = a broken or missing file (named).
fn validate_content(args: &[String]) -> i32 {
    use crate::content_store as cs;
    let mut args = args.to_vec();
    let only = take_opt(&mut args, "--lang");
    let roots = match args.first() {
        Some(d) => vec![PathBuf::from(d)],
        None => cs::candidate_roots(),
    };
    let langs: Vec<String> = match &only {
        Some(l) => vec![l.clone()],
        None => crate::i18n::Lang::ALL
            .iter()
            .map(|l| l.code().to_string())
            .collect(),
    };
    let mut failed = false;
    let mut root_used = None;
    for lang in &langs {
        match cs::locate_in(&roots, lang).and_then(|dir| cs::load_dir(&dir, lang)) {
            Ok(c) => {
                println!("OK {}: {}", c.dir.display(), cs::summary(&c));
                if root_used.is_none() {
                    root_used = c.dir.parent().map(PathBuf::from);
                }
            }
            Err(e) => {
                eprintln!("{}", crate::trf!("ОШИБКА {e}", "ERROR {e}"));
                failed = true;
            }
        }
    }
    if only.is_none() && !failed {
        if let Some(root) = root_used {
            match cs::check_parity(&root) {
                cs::ParityResult::Ok => println!("OK parity {} ru ↔ en", root.display()),
                cs::ParityResult::Skipped { missing } => {
                    eprintln!(
                        "{}",
                        crate::trf!(
                            "ОШИБКА нет {}",
                            "ERROR missing {}",
                            missing.display()
                        )
                    );
                    failed = true;
                }
                cs::ParityResult::Failed(issues) => {
                    for i in &issues {
                        eprintln!(
                            "{}",
                            crate::trf!("ОШИБКА {}: {}", "ERROR {}: {}", i.file, i.reason)
                        );
                    }
                    failed = true;
                }
            }
        }
    }
    i32::from(failed)
}

/// `validate-parity [DIR]`: `content/en` must have the same JSON files, ids
/// and fields as `content/ru`. Missing `en/` is a skip (exit 0): English
/// course files may land later.
fn validate_parity(args: &[String]) -> i32 {
    use crate::content_store as cs;
    let root = match args.first() {
        Some(d) => PathBuf::from(d),
        None => cs::candidate_roots()
            .into_iter()
            .find(|p| p.is_dir())
            .unwrap_or_else(|| PathBuf::from(cs::DIR_NAME)),
    };
    match cs::check_parity(&root) {
        cs::ParityResult::Ok => {
            if let Some(dir) = cs::lang_dir(&root, "en") {
                if let Err(e) = cs::load_dir(&dir, "en") {
                    eprintln!("{}", crate::trf!("ОШИБКА {e}", "ERROR {e}"));
                    return 1;
                }
            }
            println!("OK parity {} ru ↔ en", root.display());
            0
        }
        cs::ParityResult::Skipped { missing } => {
            println!(
                "SKIP no {} (English course files are optional until they exist)",
                missing.display()
            );
            0
        }
        cs::ParityResult::Failed(issues) => {
            for i in &issues {
                eprintln!(
                    "{}",
                    crate::trf!("ОШИБКА {}: {}", "ERROR {}: {}", i.file, i.reason)
                );
            }
            1
        }
    }
}

/// `key=value` → JSON (numbers / bools / JSON literals parsed, else string).
fn kv_args(cmd: &str, rest: &[String]) -> Result<Value, String> {
    let mut m = Map::new();
    m.insert("cmd".into(), json!(cmd));
    for (i, a) in rest.iter().enumerate() {
        match a.split_once('=') {
            Some((k, v)) => {
                let val = serde_json::from_str::<Value>(v).unwrap_or_else(|_| json!(v));
                m.insert(k.to_string(), val);
            }
            // A bare first word: the obvious argument of the command.
            None if i == 0 => {
                let key = match cmd {
                    "navigate" | "nav" | "go" => "to",
                    "press" | "click" => "label",
                    "type" => "text",
                    "screenshot" => "path",
                    "scroll" => "to",
                    _ => return Err(format!("expected key=value, got {a}")),
                };
                m.insert(key.into(), json!(a));
            }
            None => return Err(format!("expected key=value, got {a}")),
        }
    }
    Ok(Value::Object(m))
}

fn ctl(args: &[String]) -> i32 {
    let mut args = args.to_vec();
    let file = client::control_file(take_opt(&mut args, "--file").map(PathBuf::from));
    let raw = take_opt(&mut args, "--json");
    let Some(cmd) = raw.clone().or_else(|| args.first().cloned()) else {
        eprint!("{}", tr(USAGE));
        return 2;
    };
    if raw.is_none() && cmd == "launch" {
        return match client::launch(&file, &args[1..]) {
            Ok(v) => {
                println!("{v}");
                0
            }
            Err(e) => {
                eprintln!("{e}");
                1
            }
        };
    }
    let mut c = match Client::connect(&file) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    if raw.is_none() && cmd == "-" {
        let mut failed = 0;
        let stdout = std::io::stdout();
        let mut out = stdout.lock();
        for line in std::io::stdin().lock().lines() {
            let Ok(line) = line else { break };
            let t = line.trim();
            if t.is_empty() || t.starts_with('#') {
                continue;
            }
            let reply = match serde_json::from_str::<Value>(t) {
                Ok(req) => c
                    .call(&req)
                    .unwrap_or_else(|e| json!({"ok": false, "error": e})),
                Err(e) => json!({"ok": false, "error": format!("bad JSON: {e}")}),
            };
            if reply["ok"] != json!(true) {
                failed += 1;
            }
            let _ = writeln!(out, "{reply}");
        }
        return i32::from(failed > 0);
    }
    let req = match raw {
        Some(j) => match serde_json::from_str::<Value>(&j) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("--json: {e}");
                return 2;
            }
        },
        None => match kv_args(&cmd, &args[1..]) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("{e}");
                return 2;
            }
        },
    };
    match c.call(&req) {
        Ok(v) => {
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            i32::from(v["ok"] != json!(true))
        }
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

/// The release exe is a Windows GUI program (no console of its own). For
/// CLI use, attach to the console of the shell that started it, unless the
/// standard handles are already set (pipes from an MCP client, redirects).
#[cfg(all(windows, not(debug_assertions)))]
fn attach_console() {
    use std::os::windows::io::IntoRawHandle;
    type Handle = *mut core::ffi::c_void;
    #[link(name = "kernel32")]
    extern "system" {
        fn GetStdHandle(n: u32) -> Handle;
        fn SetStdHandle(n: u32, h: Handle) -> i32;
        fn AttachConsole(pid: u32) -> i32;
    }
    const STD_INPUT: u32 = -10i32 as u32;
    const STD_OUTPUT: u32 = -11i32 as u32;
    const STD_ERROR: u32 = -12i32 as u32;
    const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
    let missing = |n: u32| {
        // SAFETY: plain Win32 call with a valid constant.
        let h = unsafe { GetStdHandle(n) };
        h.is_null() || h as isize == -1
    };
    if !(missing(STD_OUTPUT) || missing(STD_ERROR)) {
        return;
    }
    // SAFETY: plain Win32 calls; handles come from files we just opened.
    unsafe {
        if AttachConsole(ATTACH_PARENT_PROCESS) == 0 {
            return;
        }
        for (n, name, write) in [
            (STD_OUTPUT, "CONOUT$", true),
            (STD_ERROR, "CONOUT$", true),
            (STD_INPUT, "CONIN$", false),
        ] {
            if missing(n) {
                if let Ok(f) = std::fs::OpenOptions::new()
                    .read(!write)
                    .write(write)
                    .open(name)
                {
                    SetStdHandle(n, f.into_raw_handle() as Handle);
                }
            }
        }
    }
}

#[cfg(not(all(windows, not(debug_assertions))))]
fn attach_console() {}
