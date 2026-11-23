//! Headless automation scripts (tests/scripts/*.jsonl) for the main flows:
//! navigation, a lesson, flashcards, a section test, the exams, the
//! calendar and settings. Each line is a command; `expect` asserts on the
//! result (docs/AUTOMATION.md).

use std::io::BufReader;

use serde_json::{json, Value};
use shorashim::automation::session::{Options, Session};

fn session() -> Session {
    Session::new(Options {
        data: None,
        today: chrono::NaiveDate::from_ymd_opt(2026, 11, 22),
    })
}

fn run_script(name: &str) {
    let path = format!("{}/tests/scripts/{name}", env!("CARGO_MANIFEST_DIR"));
    let file = std::fs::File::open(&path).expect("script");
    let mut s = session();
    let mut out = Vec::new();
    let failed = s.run_lines(BufReader::new(file), &mut out, true);
    let log = String::from_utf8_lossy(&out);
    assert_eq!(failed, 0, "{name} failed:\n{log}");
}

#[test]
fn navigate_sections() {
    run_script("01-navigate.jsonl");
}

#[test]
fn lesson() {
    run_script("02-lesson.jsonl");
}

#[test]
fn flashcards() {
    run_script("03-flashcards.jsonl");
}

#[test]
fn section_test() {
    run_script("04-section-test.jsonl");
}

#[test]
fn exams() {
    run_script("05-exam.jsonl");
}

#[test]
fn calendar() {
    run_script("06-calendar.jsonl");
}

#[test]
fn settings() {
    run_script("07-settings.jsonl");
}

#[test]
fn exam_setup_mode_tiles() {
    run_script("08-exam-setup-modes.jsonl");
}

#[test]
fn errors_are_replies_not_panics() {
    let mut s = session();
    for req in [
        json!({}),
        json!({"cmd": "nope"}),
        json!({"cmd": "navigate", "to": "nowhere"}),
        json!({"cmd": "press", "label": "no such button"}),
        json!({"cmd": "answer", "choice": 0}),
        json!({"cmd": "type", "text": "x"}),
        json!({"cmd": "select", "option": "x"}),
        json!({"cmd": "set", "key": "theme", "value": "neon"}),
        json!({"cmd": "scroll", "to": "top"}),
    ] {
        let r = s.run(&req);
        assert_eq!(r["ok"], json!(false), "{req} → {r}");
        assert!(r["error"].is_string(), "{req} → {r}");
    }
    // First run: anything but the welcome / level screens bounces to welcome.
    let r = s.run(&json!({"cmd": "navigate", "to": "map"}));
    assert_eq!(r["screen"], json!("welcome"));
}

#[test]
fn links_and_file_dialogs_are_not_pressed_headless() {
    let mut s = session();
    for c in [
        json!({"cmd": "press", "label": "Начать"}),
        json!({"cmd": "press", "contains": "Ортодоксальный"}),
        json!({"cmd": "navigate", "to": "settings"}),
    ] {
        assert_eq!(s.run(&c)["ok"], json!(true));
    }
    let r = s.run(&json!({"cmd": "press", "contains": "Экспорт", "allow_external": true}));
    assert_eq!(r["ok"], json!(false), "{r}");
    assert!(
        r["error"]
            .as_str()
            .unwrap_or_default()
            .contains("file dialog"),
        "{r}"
    );
}

#[test]
fn state_lists_ids_that_press_accepts() {
    let mut s = session();
    let st = s.run(&json!({"cmd": "state"}));
    let controls = st["controls"].as_array().expect("controls");
    assert!(!controls.is_empty());
    let id = controls[0]["id"].as_str().expect("id").to_string();
    let r = s.run(&json!({"cmd": "press", "id": id}));
    assert_eq!(r["ok"], json!(true), "{r}");
    assert_eq!(r["screen"], json!("level"));
}

#[test]
fn headless_screenshot_is_webp() {
    let dir = std::env::temp_dir().join(format!("shorashim-shot-{}", std::process::id()));
    let path = dir.join("welcome.png"); // the extension is forced to .webp
    let mut s = session();
    let r = s.run(&json!({"cmd": "screenshot", "path": path, "full_page": true}));
    assert_eq!(r["ok"], json!(true), "{r}");
    let saved = std::path::PathBuf::from(r["path"].as_str().expect("path"));
    assert_eq!(saved.extension().and_then(|e| e.to_str()), Some("webp"));
    let bytes = std::fs::read(&saved).expect("file");
    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WEBP");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn mcp_server_lists_and_calls_tools() {
    // Drive `shorashim mcp --headless` through stdio like an MCP client.
    let exe = env!("CARGO_BIN_EXE_shorashim");
    let mut child = std::process::Command::new(exe)
        .args(["mcp", "--headless", "--today", "2026-11-22"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("spawn mcp");
    let input = [
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "test", "version": "0"}}}),
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
        json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "press", "arguments": {"label": "Начать"}}}),
        json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {"name": "quit", "arguments": {}}}),
    ];
    {
        use std::io::Write;
        let stdin = child.stdin.as_mut().expect("stdin");
        for m in &input {
            writeln!(stdin, "{m}").expect("write");
        }
    }
    let out = child.wait_with_output().expect("output");
    let replies: Vec<Value> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| serde_json::from_str(l).expect("json"))
        .collect();
    assert_eq!(replies.len(), 4, "{replies:?}");
    assert_eq!(
        replies[0]["result"]["serverInfo"]["name"],
        json!("shorashim")
    );
    let tools = replies[1]["result"]["tools"].as_array().expect("tools");
    assert!(tools.iter().any(|t| t["name"] == json!("press")));
    assert_eq!(replies[2]["result"]["isError"], json!(false));
    let text = replies[2]["result"]["content"][0]["text"]
        .as_str()
        .expect("text");
    assert!(text.contains("\"screen\": \"level\""), "{text}");
}

#[test]
fn prayers() {
    run_script("09-prayers.jsonl");
}

/// The «Показывать» toggle really hides the other forms, and the choice is
/// saved in the progress file (`extra.prayers`), not lost on navigation.
#[test]
fn prayers_display_toggle_hides_and_persists() {
    let mut s = session();
    s.run(&json!({"cmd": "press", "label": "Начать"}));
    s.run(&json!({"cmd": "press", "contains": "Ортодоксальный"}));
    let texts = |s: &mut Session| -> String {
        let r = s.run(&json!({"cmd": "state"}));
        r["texts"].to_string()
    };
    s.run(&json!({"cmd": "navigate", "to": "prayers:modeh-ani"}));
    let all = texts(&mut s);
    assert!(all.contains("מוֹדֶה אֲנִי"), "{all}");
    assert!(all.contains("Модэ ани лефанеха"));
    assert!(all.contains("Благодарю Тебя"));
    s.run(&json!({"cmd": "press", "label": "Русский"}));
    let ru = texts(&mut s);
    assert!(ru.contains("Благодарю Тебя"));
    assert!(
        !ru.contains("Модэ ани лефанеха"),
        "translit shown in ru mode"
    );
    assert!(!ru.contains("שֶׁהֶחֱזַרְתָּ"), "Hebrew shown in ru mode");
    // Another screen and back: still Russian only.
    s.run(&json!({"cmd": "navigate", "to": "map"}));
    s.run(&json!({"cmd": "navigate", "to": "prayers:modeh-ani"}));
    let again = texts(&mut s);
    assert!(!again.contains("שֶׁהֶחֱזַרְתָּ"));
    s.run(&json!({"cmd": "press", "label": "Иврит"}));
    let he = texts(&mut s);
    assert!(he.contains("שֶׁהֶחֱזַרְתָּ"));
    assert!(!he.contains("Благодарю Тебя"));
    // An unknown prayer id is refused, not silently opened.
    let bad = s.run(&json!({"cmd": "navigate", "to": "prayers:nope"}));
    assert_eq!(bad["ok"], false, "{bad}");
}
