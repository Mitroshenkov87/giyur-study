//! The payload of `Shorashim-Setup.exe` (src/bin/shorashim-setup.rs):
//! files appended to the setup stub by `tools/make_setup.py`.
//!
//! Layout (all integers little-endian):
//! ```text
//! entry*  : u32 path_len, path (UTF-8, '/'), u64 size, u64 packed_len, zlib bytes
//! footer  : u64 payload_len, b"SHRSETP1"
//! ```
//! The entry `setup.json` ({"version", "contentVersion"}) describes the
//! payload and is not unpacked.

use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};

pub const MAGIC: &[u8; 8] = b"SHRSETP1";
pub const INFO: &str = "setup.json";

/// One unpacked payload.
pub struct Unpacked {
    pub dir: PathBuf,
    pub version: String,
    pub content_version: String,
}

fn safe(path: &str) -> bool {
    !path.is_empty()
        && Path::new(path)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}

fn read_u32(r: &mut impl Read) -> std::io::Result<u32> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b)?;
    Ok(u32::from_le_bytes(b))
}

fn read_u64(r: &mut impl Read) -> std::io::Result<u64> {
    let mut b = [0u8; 8];
    r.read_exact(&mut b)?;
    Ok(u64::from_le_bytes(b))
}

/// Unpack the payload appended to `setup` into `dir`.
pub fn unpack(setup: &Path, dir: &Path) -> Result<Unpacked, String> {
    let bad = |e: std::io::Error| {
        crate::trf!(
            "Установщик повреждён ({e}). Скачайте его заново.",
            "The installer is damaged ({e}). Please download it again."
        )
    };
    let mut f = File::open(setup).map_err(bad)?;
    let len = f.metadata().map_err(bad)?.len();
    if len < 16 {
        return Err(crate::i18n::tr("Установщик повреждён. Скачайте его заново.").into());
    }
    f.seek(SeekFrom::Start(len - 16)).map_err(bad)?;
    let payload_len = read_u64(&mut f).map_err(bad)?;
    let mut magic = [0u8; 8];
    f.read_exact(&mut magic).map_err(bad)?;
    if &magic != MAGIC || payload_len > len - 16 {
        return Err(
            crate::i18n::tr("В установщике нет файлов программы. Скачайте его заново.").into(),
        );
    }
    let start = len - 16 - payload_len;
    f.seek(SeekFrom::Start(start)).map_err(bad)?;
    let mut r = BufReader::new(f).take(payload_len);
    let mut info = serde_json::Value::Null;
    loop {
        let plen = match read_u32(&mut r) {
            Ok(n) => n as usize,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(bad(e)),
        };
        let mut pb = vec![0u8; plen];
        r.read_exact(&mut pb).map_err(bad)?;
        let path = String::from_utf8(pb)
            .map_err(|_| crate::i18n::tr("Установщик повреждён.").to_string())?;
        let size = read_u64(&mut r).map_err(bad)?;
        let packed = read_u64(&mut r).map_err(bad)?;
        if !safe(&path) {
            return Err(crate::trf!(
                "Установщик повреждён: неверный путь «{path}».",
                "The installer is damaged: invalid path “{path}”."
            ));
        }
        let mut data = Vec::with_capacity(size.min(1 << 28) as usize);
        flate2::read::ZlibDecoder::new((&mut r).take(packed))
            .read_to_end(&mut data)
            .map_err(bad)?;
        if data.len() as u64 != size {
            return Err(crate::trf!(
                "Установщик повреждён: файл «{path}» неполный.",
                "The installer is damaged: the file “{path}” is incomplete."
            ));
        }
        if path == INFO {
            info = serde_json::from_slice(&data).unwrap_or_default();
            continue;
        }
        let out = dir.join(&path);
        if let Some(p) = out.parent() {
            std::fs::create_dir_all(p).map_err(|e| {
                crate::trf!("Не удалось распаковать: {e}", "Could not unpack: {e}")
            })?;
        }
        File::create(&out)
            .and_then(|mut o| o.write_all(&data))
            .map_err(|e| {
                crate::trf!(
                    "Не удалось распаковать «{path}»: {e}",
                    "Could not unpack “{path}”: {e}"
                )
            })?;
    }
    let s = |k: &str| info[k].as_str().unwrap_or("").to_string();
    Ok(Unpacked {
        dir: dir.to_path_buf(),
        version: s("version"),
        content_version: s("contentVersion"),
    })
}

/// Append `entries` to `stub` as a setup (the same format
/// tools/make_setup.py writes; used by tests).
pub fn pack(stub: &[u8], entries: &[(&str, &[u8])]) -> Vec<u8> {
    use flate2::write::ZlibEncoder;
    let mut payload = Vec::new();
    for (name, data) in entries {
        let mut z = ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        z.write_all(data).expect("in memory");
        let packed = z.finish().expect("in memory");
        payload.extend_from_slice(&(name.len() as u32).to_le_bytes());
        payload.extend_from_slice(name.as_bytes());
        payload.extend_from_slice(&(data.len() as u64).to_le_bytes());
        payload.extend_from_slice(&(packed.len() as u64).to_le_bytes());
        payload.extend_from_slice(&packed);
    }
    let mut out = stub.to_vec();
    out.extend_from_slice(&payload);
    out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    out.extend_from_slice(MAGIC);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn scratch(name: &str) -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let d = std::env::temp_dir().join(format!("shorashim-payload-{name}-{n}"));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn round_trip() {
        let d = scratch("rt");
        let setup = d.join("setup.exe");
        let blob = pack(
            b"MZstub",
            &[
                (
                    INFO,
                    br#"{"version":"0.5.6","contentVersion":"2026-11-23"}"#,
                ),
                ("shorashim.exe", b"exe"),
                ("content/ru/manifest.json", b"{}"),
            ],
        );
        std::fs::write(&setup, blob).unwrap();
        let u = unpack(&setup, &d.join("out")).unwrap();
        assert_eq!(
            (u.version.as_str(), u.content_version.as_str()),
            ("0.5.6", "2026-11-23")
        );
        assert_eq!(std::fs::read(d.join("out/shorashim.exe")).unwrap(), b"exe");
        assert_eq!(
            std::fs::read(d.join("out/content/ru/manifest.json")).unwrap(),
            b"{}"
        );
        assert!(!d.join("out").join(INFO).exists());
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn refuses_broken_and_escaping_payloads() {
        let d = scratch("bad");
        let setup = d.join("setup.exe");
        std::fs::write(&setup, b"MZ just a stub").unwrap();
        assert!(unpack(&setup, &d.join("out")).is_err());
        std::fs::write(&setup, pack(b"MZ", &[("../evil.txt", b"x")])).unwrap();
        assert!(unpack(&setup, &d.join("out")).is_err());
        assert!(!d.join("evil.txt").exists());
        let mut blob = pack(b"MZ", &[("a.txt", b"hello")]);
        let n = blob.len();
        blob[n - 20] ^= 0xff; // corrupt the compressed bytes
        std::fs::write(&setup, blob).unwrap();
        assert!(unpack(&setup, &d.join("out2")).is_err());
        let _ = std::fs::remove_dir_all(d);
    }

    /// tools/make_setup.py writes what [`unpack`] reads (when Python is there).
    #[test]
    fn python_packer_matches() {
        let py = if cfg!(windows) { "python" } else { "python3" };
        if std::process::Command::new(py)
            .arg("--version")
            .output()
            .is_err()
        {
            eprintln!("no {py}; skipped");
            return;
        }
        let d = scratch("py");
        std::fs::write(d.join("stub.exe"), b"MZstub").unwrap();
        std::fs::write(d.join("app.exe"), b"app").unwrap();
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let tool = root.join("tools/make_setup.py");
        let out = d.join("Shorashim-Setup.exe");
        let st = std::process::Command::new(py)
            .arg(&tool)
            .args(["--stub"])
            .arg(d.join("stub.exe"))
            .args(["--exe"])
            .arg(d.join("app.exe"))
            .args(["--content"])
            .arg(root.join("content"))
            .args(["--out"])
            .arg(&out)
            .status()
            .unwrap();
        assert!(st.success());
        let u = unpack(&out, &d.join("full")).unwrap();
        assert_eq!(u.version, crate::install::current_version());
        assert_eq!(std::fs::read(d.join("full/shorashim.exe")).unwrap(), b"app");
        let c = crate::content_store::load_dir(&d.join("full/content/ru"), "ru").unwrap();
        assert_eq!(c.meta.content_version, u.content_version);

        // Content fix: repack from the setup, programs kept, content new.
        let fixed = d.join("fixed");
        crate::music::copy_dir(&root.join("content"), &fixed).unwrap();
        let m = fixed.join("ru/manifest.json");
        let raw = std::fs::read_to_string(&m).unwrap().replacen(
            &format!("\"contentVersion\":\"{}\"", u.content_version),
            "\"contentVersion\":\"9999-01-01\"",
            1,
        );
        std::fs::write(&m, raw).unwrap();
        let out2 = d.join("Shorashim-Setup-2.exe");
        let st = std::process::Command::new(py)
            .arg(&tool)
            .args(["--from"])
            .arg(&out)
            .args(["--content"])
            .arg(&fixed)
            .args(["--out"])
            .arg(&out2)
            .status()
            .unwrap();
        assert!(st.success());
        let u2 = unpack(&out2, &d.join("full2")).unwrap();
        assert_eq!(u2.content_version, "9999-01-01");
        assert_eq!(u2.version, u.version);
        assert_eq!(
            std::fs::read(d.join("full2/shorashim.exe")).unwrap(),
            b"app"
        );
        let _ = std::fs::remove_dir_all(d);
    }
}
