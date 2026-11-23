//! Windows side of the installer: per-user folder, shortcuts (IShellLinkW),
//! HKCU uninstall entry, self-removal after exit.

use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};

use windows::core::{Interface, HSTRING, PCWSTR};
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, IPersistFile, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegGetValueW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_WRITE, REG_DWORD, REG_OPTION_NON_VOLATILE, REG_SZ, RRF_RT_REG_SZ,
};
use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

use super::{
    current_version, install_dir_in, is_inside, os_display_name, os_link_name, os_tagline,
    uninstall_values, Installed, Options, RegValue, Status, Support, Uninstalled, EXE_NAME,
    LINK_NAMES, UNINSTALL_KEY,
};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const DETACHED_PROCESS: u32 = 0x0000_0008;

fn key_path() -> String {
    format!(r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{UNINSTALL_KEY}")
}

fn install_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| install_dir_in(&d))
}

fn start_menu_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join(r"Microsoft\Windows\Start Menu\Programs"))
}

fn start_menu_link() -> Option<PathBuf> {
    start_menu_dir().map(|d| d.join(os_link_name()))
}

fn desktop_link() -> Option<PathBuf> {
    dirs::desktop_dir().map(|d| d.join(os_link_name()))
}

fn all_start_menu_links() -> Vec<PathBuf> {
    start_menu_dir()
        .into_iter()
        .flat_map(|d| LINK_NAMES.iter().map(move |n| d.join(n)))
        .collect()
}

fn all_desktop_links() -> Vec<PathBuf> {
    dirs::desktop_dir()
        .into_iter()
        .flat_map(|d| LINK_NAMES.iter().map(move |n| d.join(n)))
        .collect()
}

fn read_value(name: &str) -> Option<String> {
    let key = HSTRING::from(key_path());
    let value = HSTRING::from(name);
    let mut buf = vec![0u16; 1024];
    let mut size = (buf.len() * 2) as u32;
    let rc = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            &key,
            &value,
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    if rc != ERROR_SUCCESS {
        return None;
    }
    let len = (size as usize / 2).saturating_sub(1).min(buf.len());
    Some(String::from_utf16_lossy(&buf[..len]))
}

pub fn status() -> Status {
    let me = std::env::current_exe().ok();
    let installed = read_value("InstallLocation")
        .map(PathBuf::from)
        .or_else(install_dir)
        .map(|dir| Installed {
            exe: dir.join(EXE_NAME),
            version: read_value("DisplayVersion").unwrap_or_default(),
            dir,
        })
        .filter(|i| i.exe.exists() && read_value("DisplayName").is_some());
    let running_installed = match (&me, &installed) {
        (Some(me), Some(i)) => is_inside(me, &i.dir),
        _ => false,
    };
    Status {
        support: Support::Supported,
        installed,
        running_installed,
        desktop_link: all_desktop_links().iter().any(|l| l.exists()),
    }
}

pub fn install(opts: Options) -> Result<PathBuf, String> {
    let me = std::env::current_exe().map_err(|e| {
        crate::trf!(
            "Не найден исполняемый файл: {e}",
            "Could not find the program file: {e}"
        )
    })?;
    let dir = install_dir().ok_or_else(|| {
        crate::i18n::tr("Не найдена папка %LOCALAPPDATA%").to_string()
    })?;
    std::fs::create_dir_all(&dir).map_err(|e| {
        crate::trf!("Не удалось создать папку: {e}", "Could not create the folder: {e}")
    })?;
    let target = dir.join(EXE_NAME);
    if !is_inside(&me, &dir) {
        // A running installed copy cannot be overwritten, but it can be
        // renamed; the .old file is removed on its next start.
        let old = dir.join(format!("{EXE_NAME}.old"));
        let _ = std::fs::remove_file(&old);
        if target.exists() {
            std::fs::rename(&target, &old)
                .map_err(|e| {
                    crate::trf!(
                        "Не удалось заменить установленную копию: {e}",
                        "Could not replace the installed copy: {e}"
                    )
                })?;
        }
        if let Err(e) = std::fs::copy(&me, &target) {
            let _ = std::fs::rename(&old, &target);
            return Err(crate::trf!(
                "Не удалось скопировать программу: {e}",
                "Could not copy the program: {e}"
            ));
        }
        let _ = std::fs::remove_file(&old);
    }
    if let Some(parent) = me.parent() {
        crate::install::copy_sidecars(parent, &dir)?;
    }

    let mut links = Vec::new();
    if let Some(l) = start_menu_link() {
        links.push(l);
    }
    if opts.desktop_shortcut {
        if let Some(l) = desktop_link() {
            links.push(l);
        }
    }
    // Drop the leftover shortcut of the other language (and a desktop
    // shortcut the user turned off).
    for l in all_start_menu_links()
        .into_iter()
        .chain(all_desktop_links())
    {
        if !links.contains(&l) && l.exists() {
            let _ = std::fs::remove_file(l);
        }
    }
    make_links(&target, &dir, &links)?;

    let size_kb = std::fs::metadata(&target)
        .map(|m| (m.len() / 1024) as u32)
        .unwrap_or(0);
    write_uninstall_entry(&dir, size_kb)?;
    Ok(target)
}

/// Shortcuts are created on a helper thread with its own COM apartment,
/// independent of whatever the UI thread initialised.
fn make_links(target: &Path, dir: &Path, links: &[PathBuf]) -> Result<(), String> {
    let (target, dir, links) = (target.to_path_buf(), dir.to_path_buf(), links.to_vec());
    std::thread::spawn(move || unsafe {
        let init = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let res = (|| -> windows::core::Result<()> {
            for link in &links {
                if let Some(parent) = link.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let sl: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
                sl.SetPath(&HSTRING::from(target.as_os_str()))?;
                sl.SetWorkingDirectory(&HSTRING::from(dir.as_os_str()))?;
                sl.SetDescription(&HSTRING::from(format!(
                    "{} — {}",
                    os_display_name(),
                    os_tagline()
                )))?;
                sl.SetIconLocation(&HSTRING::from(target.as_os_str()), 0)?;
                let pf: IPersistFile = sl.cast()?;
                pf.Save(&HSTRING::from(link.as_os_str()), true)?;
            }
            Ok(())
        })();
        if init.is_ok() {
            CoUninitialize();
        }
        res.map_err(|e| {
            crate::trf!(
                "Не удалось создать ярлык: {e}",
                "Could not create the shortcut: {e}"
            )
        })
    })
    .join()
    .map_err(|_| crate::i18n::tr("Не удалось создать ярлык").to_string())?
}

fn write_uninstall_entry(dir: &Path, size_kb: u32) -> Result<(), String> {
    let mut key = HKEY::default();
    let rc = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            &HSTRING::from(key_path()),
            None,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut key,
            None,
        )
    };
    if rc != ERROR_SUCCESS {
        return Err(crate::trf!(
            "Не удалось записать в реестр (код {})",
            "Could not write to the registry (code {})",
            rc.0
        ));
    }
    let mut result = Ok(());
    for (name, value) in uninstall_values(dir, current_version(), size_kb) {
        let rc = match value {
            RegValue::Str(s) => {
                let wide: Vec<u16> = s.encode_utf16().chain(Some(0)).collect();
                let bytes: Vec<u8> = wide.iter().flat_map(|w| w.to_le_bytes()).collect();
                unsafe { RegSetValueExW(key, &HSTRING::from(name), None, REG_SZ, Some(&bytes)) }
            }
            RegValue::Dword(v) => unsafe {
                RegSetValueExW(
                    key,
                    &HSTRING::from(name),
                    None,
                    REG_DWORD,
                    Some(&v.to_le_bytes()),
                )
            },
        };
        if rc != ERROR_SUCCESS {
            result = Err(crate::trf!(
                "Не удалось записать {name} в реестр (код {})",
                "Could not write {name} to the registry (code {})",
                rc.0
            ));
            break;
        }
    }
    unsafe {
        let _ = RegCloseKey(key);
    }
    result
}

pub fn uninstall() -> Result<Uninstalled, String> {
    let st = status();
    let dir = st
        .installed
        .as_ref()
        .map(|i| i.dir.clone())
        .or_else(install_dir)
        .ok_or_else(|| crate::i18n::tr("Не найдена папка установки").to_string())?;
    for l in all_start_menu_links()
        .into_iter()
        .chain(all_desktop_links())
    {
        if l.exists() {
            let _ = std::fs::remove_file(l);
        }
    }
    let rc = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, &HSTRING::from(key_path())) };
    let _ = rc; // missing key = already gone
    let me = std::env::current_exe().ok();
    if me.as_ref().is_some_and(|m| is_inside(m, &dir)) {
        schedule_removal(&dir)?;
        Ok(Uninstalled::QuitToFinish)
    } else {
        if dir.exists() {
            std::fs::remove_dir_all(&dir)
                .map_err(|e| {
                    crate::trf!(
                        "Не удалось удалить папку программы: {e}",
                        "Could not delete the program folder: {e}"
                    )
                })?;
        }
        Ok(Uninstalled::Done)
    }
}

/// The running exe cannot delete itself: a hidden `cmd` waits a few
/// seconds for this process to exit, then removes the folder.
fn schedule_removal(dir: &Path) -> Result<(), String> {
    let script = format!(
        "ping 127.0.0.1 -n 4 >nul & rmdir /s /q \"{}\"",
        dir.display()
    );
    std::process::Command::new("cmd.exe")
        .arg("/C")
        .raw_arg(&script)
        .current_dir(std::env::temp_dir())
        .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS)
        .spawn()
        .map(|_| ())
        .map_err(|e| {
            crate::trf!(
                "Не удалось запланировать удаление папки: {e}",
                "Could not schedule deletion of the folder: {e}"
            )
        })
}

pub fn cleanup_after_update() {
    if let Ok(me) = std::env::current_exe() {
        if let Some(dir) = me.parent() {
            let _ = std::fs::remove_file(dir.join(format!("{EXE_NAME}.old")));
        }
    }
}

/// `--uninstall` from «Приложения и возможности»: confirm, uninstall, report.
pub fn cli_uninstall(quiet: bool) {
    crate::i18n::apply_os_lang();
    let data_dir = dirs::config_dir()
        .map(|d| d.join(crate::storage::DATA_DIR).display().to_string())
        .unwrap_or_default();
    if !quiet {
        let name = os_display_name();
        let ask = crate::i18n::with_lang(
            crate::i18n::env_override().unwrap_or_else(crate::i18n::detect_system),
            || {
                crate::trf!(
                    "Удалить «{name}» с этого компьютера?\n\nБудут удалены программа, ярлыки и запись в «Приложениях». Ваш прогресс останется в папке:\n{data_dir}",
                    "Remove {name} from this computer?\n\nThe program, shortcuts and the Apps & features entry will be deleted. Your progress stays in:\n{data_dir}"
                )
            },
        );
        let remove = crate::i18n::tr_or("Удалить", "Remove").to_string();
        let cancel = crate::i18n::tr_or("Отмена", "Cancel").to_string();
        let ok = rfd::MessageDialog::new()
            .set_title(name)
            .set_level(rfd::MessageLevel::Warning)
            .set_description(ask)
            .set_buttons(rfd::MessageButtons::OkCancelCustom(remove.clone(), cancel))
            .show();
        let confirmed = match ok {
            rfd::MessageDialogResult::Ok => true,
            rfd::MessageDialogResult::Custom(s) => s == remove || s == "Удалить" || s == "Remove",
            _ => false,
        };
        if !confirmed {
            return;
        }
    }
    let res = uninstall();
    if quiet {
        return;
    }
    report(res.map(|_| {
        let name = os_display_name();
        crate::i18n::with_lang(
            crate::i18n::env_override().unwrap_or_else(crate::i18n::detect_system),
            || {
                crate::trf!(
                    "«{name}» удалён. Прогресс сохранён — он вернётся, если установить программу снова.",
                    "{name} was removed. Your progress is kept and will return if you install the app again."
                )
            },
        )
    }));
}

/// A native message box with the outcome of a CLI install / uninstall.
pub fn report(res: Result<String, String>) {
    let (level, text) = match res {
        Ok(t) => (rfd::MessageLevel::Info, t),
        Err(e) => (rfd::MessageLevel::Error, e),
    };
    rfd::MessageDialog::new()
        .set_title(os_display_name())
        .set_level(level)
        .set_description(text)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}
