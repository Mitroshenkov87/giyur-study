#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! `Shorashim-Setup.exe`: the single file people download.
//!
//! This small program carries the app as a payload appended to its own
//! file (made by `tools/make_setup.py`, docs/CONTENT-UPDATES.md): the
//! program `shorashim.exe`, `shorashim-ctl.exe`, the course files
//! `content/<lang>/…`, the music and the licences. It unpacks them into a
//! temporary folder, runs the app's own installer (`shorashim.exe
//! --install`: copies everything to `%LOCALAPPDATA%\Programs\Shorashim`,
//! shortcuts, «Приложения и возможности») and starts the installed app.
//!
//! `--quiet` skips the question, `--no-launch` only installs.
//!
//! Because the payload is just appended, a content fix only repacks the
//! setup (seconds); nothing is compiled.
//!
//! Payload format: src/install/payload.rs.

use std::path::PathBuf;

use shorashim::install::payload::unpack;

const TITLE: &str = "Установка «Шорашим»";

fn message(level: rfd::MessageLevel, text: &str) {
    rfd::MessageDialog::new()
        .set_title(TITLE)
        .set_level(level)
        .set_description(text)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

fn run() -> Result<Option<PathBuf>, String> {
    let me = std::env::current_exe().map_err(|e| format!("Не найден файл установщика: {e}"))?;
    let tmp = std::env::temp_dir().join(format!("Shorashim-Setup-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let result = (|| {
        let u = unpack(&me, &tmp)?;
        let exe = u.dir.join(shorashim::install::EXE_NAME);
        if !exe.is_file() {
            return Err("В установщике нет shorashim.exe. Скачайте его заново.".to_string());
        }
        let quiet = std::env::args().any(|a| a == "--quiet" || a == "/S");
        if !quiet {
            let ask = format!(
                "Установить «Шорашим» {}?\n\nПрограмма и материалы курса (версия {}) будут установлены для этого пользователя, без прав администратора. Ваш прогресс не изменится.",
                u.version, u.content_version
            );
            let yes = rfd::MessageDialog::new()
                .set_title(TITLE)
                .set_level(rfd::MessageLevel::Info)
                .set_description(ask)
                .set_buttons(rfd::MessageButtons::OkCancelCustom(
                    "Установить".into(),
                    "Отмена".into(),
                ))
                .show();
            if !matches!(yes, rfd::MessageDialogResult::Ok)
                && !matches!(&yes, rfd::MessageDialogResult::Custom(s) if s == "Установить")
            {
                return Ok(None);
            }
        }
        let out = std::process::Command::new(&exe)
            .args(["--install", "--desktop", "--quiet"])
            .output()
            .map_err(|e| format!("Не удалось запустить установку: {e}"))?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            let err = err.trim().trim_start_matches("install failed: ");
            return Err(format!("Установка не удалась: {err}"));
        }
        let installed = shorashim::install::status()
            .installed
            .map(|i| i.exe)
            .ok_or("Установка не удалась: программа не появилась в папке установки.")?;
        Ok(Some(installed))
    })();
    let _ = std::fs::remove_dir_all(&tmp);
    result
}

fn main() {
    if !cfg!(windows) {
        eprintln!("Shorashim-Setup.exe работает только в Windows.");
        std::process::exit(2);
    }
    match run() {
        Ok(Some(exe)) => {
            // `--no-launch`: install only (CI checks the setup this way).
            if !std::env::args().any(|a| a == "--no-launch") {
                let _ = std::process::Command::new(exe).spawn();
            }
        }
        Ok(None) => {}
        Err(e) => {
            message(rfd::MessageLevel::Error, &e);
            std::process::exit(1);
        }
    }
}
