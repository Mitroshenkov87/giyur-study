//! Command-line entry points: `shorashim ctl …`, `shorashim mcp`,
//! `shorashim --headless …`. Called first thing in `main`.

use std::io::{BufRead, Write};
use std::path::PathBuf;

use serde_json::{json, Map, Value};

use super::client::{self, Client};
use super::session::{Options, Session};

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
        Some("ctl" | "mcp" | "--headless" | "headless" | "--automation-help" | "validate-content")
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
        "--automation-help" => {
            attach_console();
            print!("{USAGE}");
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
    if let Err(e) = crate::content_store::init(crate::content_store::DEFAULT_LANG) {
        eprintln!("course content not loaded: {e}");
        return 2;
    }
    let script = take_opt(&mut args, "--script");
    let keep_going = take_flag(&mut args, "--keep-going");
    let opts = match session_options(&mut args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };
    if let Some(extra) = args.first() {
        eprintln!("unknown option {extra}\n{USAGE}");
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

/// `validate-content [DIR] [--lang ru]`: check one language folder (or
/// `content/` with language folders inside, or the copy the app would use)
/// and print what is in it. Exit 0 = good, 1 = a broken file (named).
fn validate_content(args: &[String]) -> i32 {
    use crate::content_store as cs;
    let mut args = args.to_vec();
    let lang = take_opt(&mut args, "--lang").unwrap_or_else(|| cs::DEFAULT_LANG.to_string());
    let roots = match args.first() {
        Some(d) => vec![PathBuf::from(d)],
        None => cs::candidate_roots(),
    };
    let result = cs::locate_in(&roots, &lang).and_then(|dir| cs::load_dir(&dir, &lang));
    match result {
        Ok(c) => {
            println!("OK {}: {}", c.dir.display(), cs::summary(&c));
            0
        }
        Err(e) => {
            eprintln!("ОШИБКА {e}");
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
        eprint!("{USAGE}");
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
