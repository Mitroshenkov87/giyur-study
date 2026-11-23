//! Minimal MCP server (JSON-RPC 2.0 over stdio, one message per line).
//!
//! `shorashim mcp` attaches to the running app's control channel;
//! `shorashim mcp --headless` drives an in-process headless session.
//! Every tool is one automation command (see `COMMANDS`).

use std::io::{BufRead, Write};
use std::path::PathBuf;

use serde_json::{json, Value};

use super::cli::{session_options, take_flag, take_opt};
use super::client::{self, Client};
use super::session::Session;

enum Backend {
    Headless(Box<Session>),
    Attach {
        file: PathBuf,
        client: Option<Client>,
    },
}

impl Backend {
    fn call(&mut self, req: &Value) -> Value {
        match self {
            Backend::Headless(s) => s.run(req),
            Backend::Attach { file, client } => {
                // Reconnect once if the app restarted.
                for _ in 0..2 {
                    if client.is_none() {
                        match Client::connect(file) {
                            Ok(c) => *client = Some(c),
                            Err(e) => return json!({"ok": false, "error": e}),
                        }
                    }
                    match client.as_mut().map(|c| c.call(req)) {
                        Some(Ok(v)) => return v,
                        _ => *client = None,
                    }
                }
                json!({"ok": false, "error": "lost the connection to the app"})
            }
        }
    }
}

fn obj(props: Value, required: &[&str]) -> Value {
    json!({"type": "object", "properties": props, "required": required})
}

fn tools(attach: bool) -> Vec<Value> {
    let state_flag =
        json!({"type": "boolean", "description": "Return the full screen state after the action."});
    let mut t = vec![
        ("state", "Read the current screen: route, visible texts, clickable controls with stable ids, input fields, progress, settings and toasts.",
            obj(json!({"texts": {"type": "boolean", "description": "Include all visible texts (default true)."}}), &[])),
        ("screens", "List navigation targets: sidebar items, route syntax, sections and lessons with ids.", obj(json!({}), &[])),
        ("navigate", "Go to a screen. Routes: map, calendar[:today|month|settings|YYYY-MM-DD], section:<id>, lesson:<id>, cards:<lessonId>, deck:<sectionId>, cards:review, exam[:<sectionId>], examrun:section:<id>[:n], examrun:general[:n], examrun:beitdin[:n], timed, timedrun, progress, settings, sources, interview[:practice|rehearsal], level, welcome; or a sidebar label like «Уроки» or «Календарь» (old labels such as «Карта» also work).",
            obj(json!({"to": {"type": "string"}, "state": state_flag}), &["to"])),
        ("back", "Go back to the previous screen («Назад»).", obj(json!({"state": state_flag}), &[])),
        ("press", "Press a control listed by `state`: by id (exact), label (exact visible text, case-insensitive) or contains (substring). nth picks among several matches.",
            obj(json!({"id": {"type": "string"}, "label": {"type": "string"}, "contains": {"type": "string"}, "nth": {"type": "integer", "minimum": 0}, "state": state_flag}), &[])),
        ("answer", "Answer the current question: choice (0-based, in the order shown), tf (true/false question), text (typed answer, submitted), knew (flashcard: remembered or not), self_grade (0..1 for open answers). next=true then moves to the next question.",
            obj(json!({"choice": {"type": "integer", "minimum": 0}, "tf": {"type": "boolean"}, "text": {"type": "string"}, "knew": {"type": "boolean"}, "self_grade": {"type": "number", "minimum": 0, "maximum": 1}, "auto": {"type": "boolean", "description": "Give any answer (smoke tests)."}, "next": {"type": "boolean"}, "state": state_flag}), &[])),
        ("type", "Type text into an input field (index from `state`). Replaces the current text unless append=true; submit=true presses Enter.",
            obj(json!({"text": {"type": "string"}, "input": {"type": "integer", "minimum": 0}, "append": {"type": "boolean"}, "submit": {"type": "boolean"}, "state": state_flag}), &["text"])),
        ("select", "Choose an option in a drop-down from `state.choices`: choice = its index (or target = its shown text), option = option text.",
            obj(json!({"choice": {"type": "integer", "minimum": 0, "description": "Index from state.choices"}, "target": {"type": "string", "description": "Shown text of the drop-down (substring)"}, "option": {"type": "string"}, "nth": {"type": "integer", "minimum": 0}, "state": state_flag}), &["option"])),
        ("set", "Change a setting.",
            obj(json!({"key": {"type": "string", "enum": ["theme", "font_step", "translit", "unlock_all", "timed_sound", "timed_auto", "timed_seconds", "daily_goal"]}, "value": {"description": "theme: dark|leather (parchment = old name); font_step: 0..3; booleans; timed_seconds: 20|30; daily_goal: 10|20|30|50"}}), &["key", "value"])),
        ("screenshot", "Save a WebP screenshot (the real window when attached; the rendered page when headless). inline=true also returns the image.",
            obj(json!({"path": {"type": "string", "description": "Output .webp path"}, "full_page": {"type": "boolean"}, "width": {"type": "number"}, "height": {"type": "number"}, "inline": {"type": "boolean"}}), &[])),
        ("wait", "Let time pass so timers run (the timed quiz shows its options after a short pause).",
            obj(json!({"ms": {"type": "integer", "minimum": 0, "maximum": 60000}}), &[])),
        ("quit", "Close the app (headless: end the session).", obj(json!({}), &[])),
    ];
    if attach {
        t.push((
            "scroll",
            "Scroll the main column of the window: to = top | bottom | 0..1.",
            obj(
                json!({"to": {"description": "top | bottom | number 0..1"}}),
                &["to"],
            ),
        ));
        t.push((
            "launch",
            "Start the Shorashim app with its control channel on and wait until it answers.",
            obj(
                json!({"args": {"type": "array", "items": {"type": "string"}}}),
                &[],
            ),
        ));
    }
    t.into_iter()
        .map(|(name, d, schema)| json!({"name": name, "description": d, "inputSchema": schema}))
        .collect()
}

fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (u32::from(c[0]) << 16)
            | (u32::from(*c.get(1).unwrap_or(&0)) << 8)
            | u32::from(*c.get(2).unwrap_or(&0));
        for i in 0..4 {
            if i <= c.len() {
                s.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                s.push('=');
            }
        }
    }
    s
}

fn call_tool(backend: &mut Backend, name: &str, args: &Value) -> Value {
    if name == "launch" {
        if let Backend::Attach { file, client } = backend {
            let extra: Vec<String> = args["args"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            *client = None;
            let v =
                client::launch(file, &extra).unwrap_or_else(|e| json!({"ok": false, "error": e}));
            return tool_result(&v, None);
        }
    }
    let mut req = args.as_object().cloned().unwrap_or_default();
    let inline = req
        .remove("inline")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    req.insert("cmd".into(), json!(name));
    let v = backend.call(&Value::Object(req));
    let image = if inline && name == "screenshot" && v["ok"] == json!(true) {
        v["path"]
            .as_str()
            .and_then(|p| std::fs::read(p).ok())
            .map(|b| json!({"type": "image", "data": base64(&b), "mimeType": "image/webp"}))
    } else {
        None
    };
    tool_result(&v, image)
}

fn tool_result(v: &Value, image: Option<Value>) -> Value {
    let mut content =
        vec![json!({"type": "text", "text": serde_json::to_string_pretty(v).unwrap_or_default()})];
    content.extend(image);
    json!({"content": content, "isError": v["ok"] != json!(true)})
}

pub fn main(args: &[String]) -> i32 {
    let mut args = args.to_vec();
    let headless = take_flag(&mut args, "--headless");
    let file = client::control_file(take_opt(&mut args, "--file").map(PathBuf::from));
    let mut backend = if headless {
        match session_options(&mut args) {
            Ok(o) => Backend::Headless(Box::new(Session::new(o))),
            Err(e) => {
                eprintln!("{e}");
                return 2;
            }
        }
    } else {
        Backend::Attach { file, client: None }
    };
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let msg: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let _ = writeln!(
                    out,
                    "{}",
                    json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": e.to_string()}})
                );
                let _ = out.flush();
                continue;
            }
        };
        let Some(id) = msg.get("id").cloned() else {
            continue; // notification
        };
        let method = msg["method"].as_str().unwrap_or_default();
        let params = &msg["params"];
        let result = match method {
            "initialize" => Ok(json!({
                "protocolVersion": params["protocolVersion"].as_str().unwrap_or("2025-06-18"),
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "shorashim", "version": env!("CARGO_PKG_VERSION")},
                "instructions": "Drive the Shorashim study app: call `state` to see the screen (texts and controls with ids), then `navigate`, `press`, `answer`, `type`. Screenshots are WebP.",
            })),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({"tools": tools(matches!(backend, Backend::Attach { .. }))})),
            "tools/call" => {
                let name = params["name"].as_str().unwrap_or_default();
                let empty = json!({});
                let args = params.get("arguments").unwrap_or(&empty);
                Ok(call_tool(&mut backend, name, args))
            }
            "resources/list" => Ok(json!({"resources": []})),
            "prompts/list" => Ok(json!({"prompts": []})),
            _ => Err(json!({"code": -32601, "message": format!("method not found: {method}")})),
        };
        let reply = match result {
            Ok(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}),
            Err(e) => json!({"jsonrpc": "2.0", "id": id, "error": e}),
        };
        let _ = writeln!(out, "{reply}");
        let _ = out.flush();
        if let Backend::Headless(s) = &backend {
            if s.quit {
                break;
            }
        }
    }
    0
}

#[cfg(test)]
mod tests {
    #[test]
    fn base64_matches_known_values() {
        assert_eq!(super::base64(b""), "");
        assert_eq!(super::base64(b"f"), "Zg==");
        assert_eq!(super::base64(b"fo"), "Zm8=");
        assert_eq!(super::base64(b"foo"), "Zm9v");
        assert_eq!(super::base64(b"foobar"), "Zm9vYmFy");
    }
}
