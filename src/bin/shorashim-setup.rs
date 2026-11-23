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

use shorashim::i18n;
use shorashim::install::payload::unpack;
use shorashim::trf;

fn title() -> String {
    trf!("Установка «Шорашим»", "Install Shorashim")
}

fn message(level: rfd::MessageLevel, text: &str) {
    rfd::MessageDialog::new()
        .set_title(title())
        .set_level(level)
        .set_description(text)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

fn run() -> Result<Option<PathBuf>, String> {
    let me = std::env::current_exe().map_err(|e| {
        trf!(
            "Не найден файл установщика: {e}",
            "Setup file not found: {e}"
        )
    })?;
    let tmp = std::env::temp_dir().join(format!("Shorashim-Setup-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let result = (|| {
        let u = unpack(&me, &tmp)?;
        let exe = u.dir.join(shorashim::install::EXE_NAME);
        if !exe.is_file() {
            return Err(trf!(
                "В установщике нет shorashim.exe. Скачайте его заново.",
                "This setup has no shorashim.exe. Please download it again."
            ));
        }
        let quiet = std::env::args().any(|a| a == "--quiet" || a == "/S");
        if !quiet {
            let name = i18n::app_name();
            let ver = &u.version;
            let cv = &u.content_version;
            let ask = trf!(
                "Установить «{name}» {ver}?\n\nПрограмма и материалы курса (версия {cv}) будут установлены для этого пользователя, без прав администратора. Ваш прогресс не изменится.",
                "Install {name} {ver}?\n\nThe app and the course files (version {cv}) will be installed for this user, without administrator rights. Your progress will not change."
            );
            let install = i18n::tr_or("Установить", "Install").to_string();
            let cancel = i18n::tr_or("Отмена", "Cancel").to_string();
            let yes = rfd::MessageDialog::new()
                .set_title(title())
                .set_level(rfd::MessageLevel::Info)
                .set_description(ask)
                .set_buttons(rfd::MessageButtons::OkCancelCustom(install.clone(), cancel))
                .show();
            if !matches!(yes, rfd::MessageDialogResult::Ok)
                && !matches!(&yes, rfd::MessageDialogResult::Custom(s) if s.as_str() == install || s == "Установить" || s == "Install")
            {
                return Ok(None);
            }
        }
        let out = std::process::Command::new(&exe)
            .args(["--install", "--desktop", "--quiet"])
            .output()
            .map_err(|e| {
                trf!(
                    "Не удалось запустить установку: {e}",
                    "Could not start the installer: {e}"
                )
            })?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            let err = err.trim().trim_start_matches("install failed: ");
            return Err(trf!("Установка не удалась: {err}", "Install failed: {err}"));
        }
        let installed = shorashim::install::status()
            .installed
            .map(|i| i.exe)
            .ok_or_else(|| {
                trf!(
                    "Установка не удалась: программа не появилась в папке установки.",
                    "Install failed: the app did not appear in the install folder."
                )
            })?;
        Ok(Some(installed))
    })();
    let _ = std::fs::remove_dir_all(&tmp);
    result
}

fn main() {
    i18n::apply_os_lang();
    if !cfg!(windows) {
        eprintln!(
            "{}",
            i18n::tr_or(
                "Shorashim-Setup.exe работает только в Windows.",
                "Shorashim-Setup.exe runs only on Windows.",
            )
        );
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
