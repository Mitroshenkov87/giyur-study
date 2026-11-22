//! Client side of the control channel (`shorashim ctl`, `shorashim mcp`).

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::server::CONTROL_FILE;

/// The control file to use: explicit path, `SHORASHIM_CONTROL_FILE`, the
/// folder of `SHORASHIM_DATA`, or the normal data folder.
pub fn control_file(explicit: Option<PathBuf>) -> PathBuf {
    if let Some(p) = explicit {
        return p;
    }
    if let Some(p) = std::env::var_os("SHORASHIM_CONTROL_FILE") {
        return PathBuf::from(p);
    }
    if let Some(p) =
        std::env::var_os("SHORASHIM_DATA").or_else(|| std::env::var_os("GIYUR_STUDY_DATA"))
    {
        return super::server::control_path_for(&PathBuf::from(p));
    }
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(crate::storage::DATA_DIR)
        .join(CONTROL_FILE)
}

pub struct Client {
    writer: TcpStream,
    reader: BufReader<TcpStream>,
    token: String,
}

impl Client {
    pub fn connect(file: &std::path::Path) -> Result<Client, String> {
        let text = std::fs::read_to_string(file).map_err(|_| {
            format!(
                "no control file at {} — start the app with `shorashim --control` (or SHORASHIM_CONTROL=1)",
                file.display()
            )
        })?;
        let info: Value =
            serde_json::from_str(&text).map_err(|e| format!("bad control file: {e}"))?;
        let addr = info["addr"].as_str().ok_or("control file has no addr")?;
        let token = info["token"]
            .as_str()
            .ok_or("control file has no token")?
            .to_string();
        let addr: std::net::SocketAddr = addr.parse().map_err(|e| format!("bad addr: {e}"))?;
        if !addr.ip().is_loopback() {
            return Err("refusing a non-loopback control address".into());
        }
        let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(3))
            .map_err(|e| format!("the app is not running or not listening ({e})"))?;
        let _ = stream.set_read_timeout(Some(Duration::from_secs(150)));
        let reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
        Ok(Client {
            writer: stream,
            reader,
            token,
        })
    }

    pub fn call(&mut self, req: &Value) -> Result<Value, String> {
        let mut req = req.clone();
        if let Some(o) = req.as_object_mut() {
            o.insert("token".into(), json!(self.token));
        }
        writeln!(self.writer, "{req}").map_err(|e| e.to_string())?;
        let mut line = String::new();
        self.reader
            .read_line(&mut line)
            .map_err(|e| e.to_string())?;
        if line.trim().is_empty() {
            return Err("the app closed the connection".into());
        }
        serde_json::from_str(line.trim()).map_err(|e| e.to_string())
    }
}

/// Start the windowed app with the control channel on and wait until it
/// answers. Extra `args` are passed through.
pub fn launch(file: &std::path::Path, args: &[String]) -> Result<Value, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    // A companion binary (shorashim-ctl) starts the main app next to it.
    let exe = match exe.file_stem().and_then(|s| s.to_str()) {
        Some(s) if s.starts_with("shorashim-ctl") => {
            let name = if cfg!(windows) {
                "shorashim.exe"
            } else {
                "shorashim"
            };
            exe.with_file_name(name)
        }
        _ => exe,
    };
    let old = std::fs::read_to_string(file).ok();
    let child = std::process::Command::new(&exe)
        .arg("--control")
        .args(args)
        .env("SHORASHIM_CONTROL", "1")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("cannot start {}: {e}", exe.display()))?;
    let pid = child.id();
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_secs(30) {
        std::thread::sleep(Duration::from_millis(200));
        let now = std::fs::read_to_string(file).ok();
        if now.is_some() && now != old {
            if let Ok(mut c) = Client::connect(file) {
                if let Ok(v) = c.call(&json!({"cmd": "state", "texts": false})) {
                    return Ok(
                        json!({"ok": true, "pid": pid, "screen": v["screen"], "route": v["route"]}),
                    );
                }
            }
        }
    }
    Err("the app did not open its control channel within 30 s".into())
}
