//! Local control channel of the windowed app.
//!
//! Off unless the app is started with `--control` or `SHORASHIM_CONTROL=1`.
//! Listens on 127.0.0.1 only (random port), and every request must carry
//! the random token written to `control.json` next to the progress file
//! (user-only permissions). One JSON object per line in, one per line out.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::time::Duration;

use iced::futures::SinkExt;
use iced::{Subscription, Task};
use serde_json::{json, Value};

use super::{execute, Effect, Host};
use crate::ui::app::Ctx;

pub const CONTROL_FILE: &str = "control.json";
/// Longest accepted request line.
const MAX_LINE: u64 = 1 << 20;

/// `--control` on the command line or `SHORASHIM_CONTROL=1`.
pub fn enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| {
        std::env::args().any(|a| a == "--control")
            || std::env::var("SHORASHIM_CONTROL").is_ok_and(|v| v == "1" || v == "true")
    })
}

/// Where `control.json` lives for a given progress file.
pub fn control_path_for(progress: &Path) -> PathBuf {
    progress
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(CONTROL_FILE)
}

/// One request from a client, with its reply slot.
#[derive(Clone)]
pub struct Request {
    body: Arc<Value>,
    reply: Arc<Mutex<Option<mpsc::Sender<Value>>>>,
}

impl std::fmt::Debug for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Request({})", self.body)
    }
}

impl Request {
    pub fn body(&self) -> &Value {
        &self.body
    }
    /// Send the reply (only the first call counts).
    pub fn respond(&self, v: Value) {
        if let Some(tx) = self.reply.lock().ok().and_then(|mut g| g.take()) {
            let _ = tx.send(v);
        }
    }
}

/// Message type the windowed app wraps (`Message::Control`).
#[derive(Debug, Clone)]
pub enum ControlMsg {
    Request(Request),
    /// An asynchronous effect (screenshot) finished; nothing to do.
    Done,
}

/// The control channel as an iced subscription (none when disabled).
pub fn subscription(ctx: &Ctx) -> Subscription<ControlMsg> {
    if !enabled() {
        return Subscription::none();
    }
    let file = control_path_for(&ctx.path);
    Subscription::run_with(file, |file| {
        let file = file.clone();
        iced::stream::channel(32, async move |out| {
            if let Err(e) = start(&file, out) {
                eprintln!("shorashim: control channel not started: {e}");
            }
            std::future::pending::<()>().await;
        })
    })
}

fn random_token() -> String {
    use rand::RngCore;
    let mut b = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut b);
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn write_private(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let _ = std::fs::remove_file(path);
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(path)?;
    f.write_all(text.as_bytes())
}

/// Remove our control file (if it is still ours).
pub fn cleanup(path: &Path) {
    if let Ok(t) = std::fs::read_to_string(path) {
        if let Ok(v) = serde_json::from_str::<Value>(&t) {
            if v["pid"] == json!(std::process::id()) {
                let _ = std::fs::remove_file(path);
            }
        }
    }
}

static FILE: OnceLock<PathBuf> = OnceLock::new();

/// Path of the control file this process wrote, if any.
pub fn active_file() -> Option<&'static PathBuf> {
    FILE.get()
}

fn start(
    file: &Path,
    out: iced::futures::channel::mpsc::Sender<ControlMsg>,
) -> std::io::Result<()> {
    let port: u16 = std::env::var("SHORASHIM_CONTROL_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(0);
    // Loopback only: never reachable from the network.
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    let addr = listener.local_addr()?;
    let token = random_token();
    let info = json!({
        "app": "shorashim",
        "version": env!("CARGO_PKG_VERSION"),
        "pid": std::process::id(),
        "addr": addr.to_string(),
        "port": addr.port(),
        "token": token,
    });
    write_private(
        file,
        &serde_json::to_string_pretty(&info).unwrap_or_default(),
    )?;
    let _ = FILE.set(file.to_path_buf());
    let token = Arc::new(token);
    std::thread::Builder::new()
        .name("shorashim-control".into())
        .spawn(move || {
            for stream in listener.incoming().flatten() {
                let (token, out) = (token.clone(), out.clone());
                let _ = std::thread::Builder::new()
                    .name("shorashim-control-conn".into())
                    .spawn(move || serve(stream, &token, out));
            }
        })?;
    Ok(())
}

/// Constant-time comparison (the token is a secret).
fn same(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |acc, (x, y)| acc | (x ^ y))
            == 0
}

fn serve(
    stream: TcpStream,
    token: &str,
    mut out: iced::futures::channel::mpsc::Sender<ControlMsg>,
) {
    let peer_ok = stream
        .peer_addr()
        .map(|a| a.ip().is_loopback())
        .unwrap_or(false);
    if !peer_ok {
        return;
    }
    let _ = stream.set_read_timeout(Some(Duration::from_secs(600)));
    let Ok(mut writer) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(stream);
    loop {
        let mut line = String::new();
        match (&mut reader).take(MAX_LINE).read_line(&mut line) {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        // Not JSON (e.g. a browser's HTTP request): drop the connection.
        let Ok(Value::Object(mut obj)) = serde_json::from_str::<Value>(t) else {
            let _ = writeln!(
                writer,
                "{}",
                json!({"ok": false, "error": "expected one JSON object per line"})
            );
            return;
        };
        let given = obj
            .remove("token")
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_default();
        if !same(&given, token) {
            std::thread::sleep(Duration::from_millis(250));
            let _ = writeln!(
                writer,
                "{}",
                json!({"ok": false, "error": "bad or missing token"})
            );
            return;
        }
        let (tx, rx) = mpsc::channel();
        let req = Request {
            body: Arc::new(Value::Object(obj)),
            reply: Arc::new(Mutex::new(Some(tx))),
        };
        if iced::futures::executor::block_on(out.send(ControlMsg::Request(req))).is_err() {
            return;
        }
        let reply = rx
            .recv_timeout(Duration::from_secs(120))
            .unwrap_or_else(|_| json!({"ok": false, "error": "the app did not answer in time"}));
        if writeln!(writer, "{reply}").is_err() {
            return;
        }
    }
}

/// Handle a request in the windowed app's `update`: run the command and
/// turn its effect into a task. `extra` are the tasks the dispatched
/// messages produced; `wrap` lifts a [`ControlMsg`] into the app message.
pub fn handle<M: Send + 'static>(
    host: &mut dyn Host,
    req: Request,
    extra: Vec<Task<M>>,
    scroll_id: &'static str,
    wrap: fn(ControlMsg) -> M,
) -> (Task<M>, Option<Effect>) {
    let out = execute(host, req.body());
    let mut tasks = extra;
    let effect = out.effect.clone();
    match out.effect {
        None => req.respond(out.reply),
        Some(Effect::Scroll(y)) => {
            req.respond(out.reply);
            tasks.push(iced::widget::operation::snap_to(
                scroll_id,
                iced::widget::operation::RelativeOffset {
                    x: None,
                    y: Some(y),
                },
            ));
        }
        Some(Effect::Wait(ms)) => {
            let reply = out.reply;
            tasks.push(Task::perform(
                async move { std::thread::sleep(Duration::from_millis(ms)) },
                move |()| {
                    req.respond(reply.clone());
                    wrap(ControlMsg::Done)
                },
            ));
        }
        Some(Effect::Quit) => {
            req.respond(out.reply);
            if let Some(f) = active_file() {
                cleanup(f);
            }
            tasks.push(iced::exit());
        }
        Some(Effect::Screenshot { path, .. }) => {
            let reply = out.reply;
            tasks.push(
                iced::window::latest()
                    .then(|id| match id {
                        Some(id) => iced::window::screenshot(id).map(Some),
                        None => Task::done(None),
                    })
                    .map(move |shot| {
                        let (req, path, mut reply) = (req.clone(), path.clone(), reply.clone());
                        // Encoding a big lossless WebP takes a moment: off the UI thread.
                        std::thread::spawn(move || {
                            let res = match shot {
                                Some(s) => super::probe::write_webp(
                                    &path,
                                    &s.rgba,
                                    s.size.width,
                                    s.size.height,
                                )
                                .map(|_| (s.size.width, s.size.height)),
                                None => Err("no window".to_string()),
                            };
                            match res {
                                Ok((w, h)) => {
                                    reply["width"] = json!(w);
                                    reply["height"] = json!(h);
                                    req.respond(reply);
                                }
                                Err(e) => req.respond(json!({"ok": false, "error": e})),
                            }
                        });
                        wrap(ControlMsg::Done)
                    }),
            );
        }
    }
    (Task::batch(tasks), effect)
}
