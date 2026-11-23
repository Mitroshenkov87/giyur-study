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
        reviewer: false,
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
fn interview_trainer() {
    run_script("09-interview.jsonl");
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
    run_script("10-prayers.jsonl");
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

/// Progress as stored, without the settings (which the reviewer mode may
/// change: the switch itself, theme, font).
fn progress_without_settings(v: &Value) -> Value {
    let mut v = v.clone();
    v.as_object_mut().expect("object").remove("settings");
    v
}

fn file_json(path: &std::path::Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).expect("progress file")).expect("json")
}

/// «Открытый режим рецензента»: every gate opens, and whatever is done in
/// it — exams, «Маалот» runs, flashcards, lessons — never reaches the real
/// progress; switching it off restores exactly the state before.
#[test]
fn reviewer_mode_opens_everything_and_leaves_progress_untouched() {
    let dir = std::env::temp_dir().join(format!("shorashim-reviewer-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("progress.json");
    let mut s = Session::new(Options {
        data: Some(path.clone()),
        today: chrono::NaiveDate::from_ymd_opt(2026, 11, 22),
        reviewer: false,
    });
    let ok = |s: &mut Session, c: Value| {
        let r = s.run(&c);
        assert_eq!(r["ok"], json!(true), "{c} → {r}");
        r
    };
    ok(&mut s, json!({"cmd": "press", "label": "Начать"}));
    ok(&mut s, json!({"cmd": "press", "contains": "Реформистский"}));
    // Some real progress first: a read lesson and XP.
    let lesson_id = {
        let lib = &s.ctx.lib;
        lib.lessons(&lib.sections[0], shorashim::storage::Level::Reform)[0]
            .id
            .clone()
    };
    s.ctx
        .state
        .lessons_read
        .insert(lesson_id, "2026-11-22".to_string());
    s.ctx.award(12);
    let before_state = serde_json::to_value(&s.ctx.state).unwrap();
    let before_file = file_json(&path);
    assert_eq!(before_file["xpTotal"], json!(12));
    let locked: Vec<String> = s
        .ctx
        .lib
        .sections
        .iter()
        .filter(|sec| sec.available && !s.ctx.lib.is_unlocked(sec, &s.ctx.state))
        .map(|sec| sec.id.clone())
        .collect();
    assert!(!locked.is_empty());

    // On.
    let r = ok(&mut s, json!({"cmd": "reviewer", "on": true}));
    assert_eq!(r["reviewer"], json!(true));
    assert_eq!(r["progress"]["xp_total"], json!(12));
    assert_eq!(file_json(&path)["settings"]["reviewerMode"], json!(true));

    // Every gate is open.
    for id in &locked {
        let r = ok(
            &mut s,
            json!({"cmd": "navigate", "to": format!("section:{id}"), "state": true}),
        );
        assert_eq!(r["screen"], json!("section"));
        assert!(!r.to_string().contains("пока закрыт"), "{id}: {r}");
    }
    ok(&mut s, json!({"cmd": "navigate", "to": "progress"}));
    assert_eq!(
        shorashim::collection::count(&s.ctx.state),
        shorashim::collection::ITEMS.len()
    );

    // An exam: free navigation, answers revealed, run to the results.
    ok(
        &mut s,
        json!({"cmd": "navigate", "to": format!("examrun:section:{}:4", locked[0])}),
    );
    ok(&mut s, json!({"cmd": "press", "id": "Exam(Jump(3))"}));
    let r = ok(
        &mut s,
        json!({"cmd": "press", "id": "Exam(Peek)", "state": true}),
    );
    assert!(r.to_string().contains("Правильный ответ"), "{r}");
    ok(&mut s, json!({"cmd": "press", "id": "Exam(Jump(1))"}));
    for _ in 0..4 {
        ok(&mut s, json!({"cmd": "answer", "auto": true, "next": true}));
    }
    assert!(
        s.screen.leave_guard(&s.ctx).is_none(),
        "no confirm in the mode"
    );
    // «Маалот»: straight to step 12, a wrong answer, the results.
    ok(
        &mut s,
        json!({"cmd": "set", "key": "timed_seconds", "value": 0}),
    );
    ok(&mut s, json!({"cmd": "navigate", "to": "timed"}));
    ok(&mut s, json!({"cmd": "press", "id": "Timed(Jump(12))"}));
    ok(&mut s, json!({"cmd": "wait", "ms": 2600}));
    let r = ok(&mut s, json!({"cmd": "state"}));
    assert!(
        r.to_string().contains("Ступень 12 из 15") || r.to_string().contains("СТУПЕНЬ 12 ИЗ 15"),
        "{r}"
    );
    ok(&mut s, json!({"cmd": "answer", "auto": true}));
    // Flashcards of a locked section, a lesson marked read.
    ok(
        &mut s,
        json!({"cmd": "navigate", "to": format!("deck:{}", locked[0])}),
    );
    let _ = s.run(&json!({"cmd": "answer", "knew": true}));
    let _ = s.run(&json!({"cmd": "answer", "knew": true}));
    let l2 = {
        let lib = &s.ctx.lib;
        let sec = lib.section(&locked[0]).unwrap();
        lib.lessons(sec, shorashim::storage::Level::Orthodox)[0]
            .id
            .clone()
    };
    ok(
        &mut s,
        json!({"cmd": "navigate", "to": format!("lesson:{l2}")}),
    );
    let _ = s.run(&json!({"cmd": "press", "contains": "Отметить прочитанным"}));

    // Nothing reached the file, and no XP even in the sandbox.
    assert_eq!(
        progress_without_settings(&file_json(&path)),
        progress_without_settings(&before_file)
    );
    assert_eq!(s.ctx.state.xp_total, 12);
    let r = ok(&mut s, json!({"cmd": "reviewer"}));
    assert_eq!(r["progress"]["xp_total"], json!(12));
    assert_eq!(
        r["progress"]["exams_taken"],
        json!(before_state["exams"].as_array().map_or(0, |a| a.len()))
    );

    // Off: exactly the state before.
    let r = ok(&mut s, json!({"cmd": "reviewer", "on": false}));
    assert_eq!(r["reviewer"], json!(false));
    // Progress exactly as before; settings changed in the mode are kept.
    let after_state = serde_json::to_value(&s.ctx.state).unwrap();
    assert_eq!(
        progress_without_settings(&after_state),
        progress_without_settings(&before_state)
    );
    assert_eq!(after_state["settings"]["timedSeconds"], json!(0));
    assert_eq!(after_state["settings"]["reviewerMode"], json!(false));
    assert_eq!(
        progress_without_settings(&file_json(&path)),
        progress_without_settings(&before_file)
    );
    assert_eq!(file_json(&path)["settings"], after_state["settings"]);
    for id in &locked {
        let sec = s.ctx.lib.section(id).unwrap();
        assert!(
            !s.ctx.lib.is_unlocked(sec, &s.ctx.state),
            "{id} locked again"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// A sidebar jump out of an exam in progress asks «Прервать экзамен?»;
/// the «Маалот» climb asks too; the results do not.
#[test]
fn leaving_a_run_asks_first() {
    let mut s = session();
    s.run(&json!({"cmd": "press", "label": "Начать"}));
    s.run(&json!({"cmd": "press", "contains": "Ортодоксальный"}));
    s.run(&json!({"cmd": "navigate", "to": "examrun:section:osnovy:2"}));
    let g = s.screen.leave_guard(&s.ctx).expect("exam run guard");
    assert_eq!(g.title, "Прервать экзамен?");
    assert_eq!(g.body, "Прогресс этой попытки не сохранится.");
    s.run(&json!({"cmd": "answer", "auto": true, "next": true}));
    s.run(&json!({"cmd": "answer", "auto": true, "next": true}));
    assert!(
        s.screen.leave_guard(&s.ctx).is_none(),
        "results: nothing to lose"
    );
    s.run(&json!({"cmd": "set", "key": "timed_seconds", "value": 0}));
    s.run(&json!({"cmd": "navigate", "to": "timedrun"}));
    let g = s.screen.leave_guard(&s.ctx).expect("ladder guard");
    assert!(g.title.contains("Маалот"), "{g:?}");
    // In the reviewer mode nothing asks.
    s.run(&json!({"cmd": "reviewer", "on": true, "persist": false}));
    s.run(&json!({"cmd": "navigate", "to": "examrun:section:osnovy:2"}));
    assert!(s.screen.leave_guard(&s.ctx).is_none());
}

/// `--reviewer` (headless) starts in the mode for this run only.
#[test]
fn reviewer_flag_is_for_this_run_only() {
    let dir = std::env::temp_dir().join(format!("shorashim-reviewer-flag-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("progress.json");
    let s = Session::new(Options {
        data: Some(path.clone()),
        today: chrono::NaiveDate::from_ymd_opt(2026, 11, 22),
        reviewer: true,
    });
    assert!(s.ctx.reviewer());
    assert_eq!(file_json(&path)["settings"]["reviewerMode"], json!(false));
    drop(s);
    let s = Session::new(Options {
        data: Some(path.clone()),
        today: chrono::NaiveDate::from_ymd_opt(2026, 11, 22),
        reviewer: false,
    });
    assert!(!s.ctx.reviewer());
    let _ = std::fs::remove_dir_all(&dir);
}
