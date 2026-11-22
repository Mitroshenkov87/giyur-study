//! Built-in installer: the same `shorashim.exe` runs portable, and
//! «Настройки → Установить в систему» copies it to a per-user location
//! (no admin rights), adds Start-menu / desktop shortcuts and an entry in
//! «Приложения и возможности». `--uninstall` (or «Удалить из системы»)
//! reverses it. Progress always stays in the data folder
//! (`%APPDATA%\Shorashim`) and is never touched by install or uninstall.
//!
//! Windows only for now; on other systems [`status`] reports
//! [`Support::Unsupported`]. A Linux self-install (a `.desktop` file and an
//! icon under `~/.local/share`, AppImage-style) is planned later.

use std::path::{Path, PathBuf};

#[cfg(windows)]
mod win;

/// Folder name under `%LOCALAPPDATA%\Programs`.
pub const DIR_NAME: &str = "Shorashim";
/// Installed executable name.
pub const EXE_NAME: &str = "shorashim.exe";
/// Shortcut file name (Start menu and desktop).
pub const LINK_NAME: &str = "Шорашим.lnk";
/// Uninstall registry key under `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall`.
pub const UNINSTALL_KEY: &str = "Shorashim";
/// Shown in «Приложения и возможности».
pub const DISPLAY_NAME: &str = "Шорашим";
pub const PUBLISHER: &str = "Aleksandr Mitroshenkov";
pub const URL: &str = "https://github.com/Mitroshenkov87/shorashim";

/// This build's version.
pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Whether installing is possible on this system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Support {
    Supported,
    /// Not on this OS yet (the reason is shown in Settings).
    Unsupported(&'static str),
}

/// What the registry / install folder says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installed {
    pub dir: PathBuf,
    pub exe: PathBuf,
    pub version: String,
}

/// Snapshot used by the Settings card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    pub support: Support,
    pub installed: Option<Installed>,
    /// This process is the installed copy.
    pub running_installed: bool,
    /// A desktop shortcut exists (kept as is on update by default).
    pub desktop_link: bool,
}

/// What Settings offers for a [`Status`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Offer {
    /// Not installed: «Установить в систему».
    Install,
    /// Running a newer portable exe than the installed one.
    Update {
        from: String,
        to: String,
    },
    /// Installed copy is the same or newer; only «Удалить из системы».
    UpToDate {
        version: String,
        running_installed: bool,
    },
    Unsupported(&'static str),
}

impl Status {
    pub fn offer(&self) -> Offer {
        if let Support::Unsupported(why) = self.support {
            return Offer::Unsupported(why);
        }
        match &self.installed {
            None => Offer::Install,
            Some(i) if !self.running_installed && newer(current_version(), &i.version) => {
                Offer::Update {
                    from: i.version.clone(),
                    to: current_version().to_string(),
                }
            }
            Some(i) => Offer::UpToDate {
                version: i.version.clone(),
                running_installed: self.running_installed,
            },
        }
    }
}

/// Install options chosen in Settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    pub desktop_shortcut: bool,
}

/// After uninstalling: whether the app must quit (the installed copy
/// deletes its own folder only after it exits).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Uninstalled {
    Done,
    QuitToFinish,
}

/// `a > b` for dotted numeric versions («0.10.0» > «0.9.3»); a missing or
/// unparsable installed version counts as older.
pub fn newer(a: &str, b: &str) -> bool {
    fn parse(v: &str) -> Option<Vec<u64>> {
        v.trim()
            .trim_start_matches('v')
            .split(['.', '-', '+'])
            .take(3)
            .map(|p| p.parse().ok())
            .collect()
    }
    match (parse(a), parse(b)) {
        (Some(x), Some(y)) => x > y,
        (Some(_), None) => true,
        _ => false,
    }
}

/// `exe` lives inside `dir` (case-insensitive on Windows paths).
pub fn is_inside(exe: &Path, dir: &Path) -> bool {
    let norm = |p: &Path| {
        let s = p.to_string_lossy().replace('/', "\\");
        let s = s.trim_start_matches(r"\\?\").to_string();
        if cfg!(windows) {
            s.to_lowercase()
        } else {
            s
        }
    };
    let (e, d) = (norm(exe), norm(dir));
    let d = d.trim_end_matches('\\');
    e.len() > d.len() && e.starts_with(d) && e[d.len()..].starts_with('\\')
}

/// The per-user install folder for a `%LOCALAPPDATA%` root.
pub fn install_dir_in(local_app_data: &Path) -> PathBuf {
    local_app_data.join("Programs").join(DIR_NAME)
}

/// Registry values of the uninstall entry, in write order.
pub fn uninstall_values(dir: &Path, version: &str, size_kb: u32) -> Vec<(&'static str, RegValue)> {
    let exe = dir.join(EXE_NAME);
    let exe_s = exe.display().to_string();
    vec![
        ("DisplayName", RegValue::Str(DISPLAY_NAME.into())),
        ("DisplayVersion", RegValue::Str(version.into())),
        ("Publisher", RegValue::Str(PUBLISHER.into())),
        ("DisplayIcon", RegValue::Str(format!("{exe_s},0"))),
        ("InstallLocation", RegValue::Str(dir.display().to_string())),
        (
            "UninstallString",
            RegValue::Str(format!("\"{exe_s}\" --uninstall")),
        ),
        (
            "QuietUninstallString",
            RegValue::Str(format!("\"{exe_s}\" --uninstall --quiet")),
        ),
        ("URLInfoAbout", RegValue::Str(URL.into())),
        ("HelpLink", RegValue::Str(URL.into())),
        (
            "Comments",
            RegValue::Str("Основы иудаизма и подготовка к гиюру".into()),
        ),
        ("NoModify", RegValue::Dword(1)),
        ("NoRepair", RegValue::Dword(1)),
        ("EstimatedSize", RegValue::Dword(size_kb)),
    ]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegValue {
    Str(String),
    Dword(u32),
}

/// Current install state.
pub fn status() -> Status {
    // Screenshot hook (debug builds only): SHORASHIM_FAKE_INSTALL=
    // none | old | same — the Settings card as on Windows.
    #[cfg(debug_assertions)]
    if let Some(f) = crate::util::env_hook("FAKE_INSTALL") {
        let dir = PathBuf::from(r"C:\Users\user\AppData\Local\Programs\Shorashim");
        let installed = |v: &str| Installed {
            exe: dir.join(EXE_NAME),
            dir: dir.clone(),
            version: v.into(),
        };
        return Status {
            support: Support::Supported,
            installed: match f.as_str() {
                "old" => Some(installed("0.3.0")),
                "same" => Some(installed(current_version())),
                _ => None,
            },
            running_installed: false,
            desktop_link: true,
        };
    }
    #[cfg(windows)]
    {
        win::status()
    }
    #[cfg(not(windows))]
    {
        Status {
            support: Support::Unsupported(
                "Установка в систему пока есть только в Windows. Для Linux позже появится установка ярлыка в меню приложений (.desktop), как у AppImage.",
            ),
            installed: None,
            running_installed: false,
            desktop_link: false,
        }
    }
}

/// Copy LICENSE, NOTICE and the `music/` playlist from beside a portable
/// exe into the install folder. Missing sidecars are skipped.
pub fn copy_sidecars(from_parent: &Path, to_dir: &Path) {
    for f in ["LICENSE", "NOTICE"] {
        let src = from_parent.join(f);
        if src.is_file() {
            let _ = std::fs::copy(&src, to_dir.join(f));
        }
    }
    if let Some(src) = crate::music::sidecar_dir(from_parent) {
        let dest = to_dir.join("music");
        let _ = crate::music::copy_dir(&src, &dest);
    }
}

/// Install (or update) this exe. Returns the installed exe path.
pub fn install(opts: Options) -> Result<PathBuf, String> {
    #[cfg(windows)]
    {
        win::install(opts)
    }
    #[cfg(not(windows))]
    {
        let _ = opts;
        Err("Установка в систему пока доступна только в Windows.".into())
    }
}

/// Remove shortcuts, the uninstall entry and the program folder (not progress).
pub fn uninstall() -> Result<Uninstalled, String> {
    #[cfg(windows)]
    {
        win::uninstall()
    }
    #[cfg(not(windows))]
    {
        Err("Установка в систему пока доступна только в Windows.".into())
    }
}

/// Start-up housekeeping: delete a leftover `shorashim.exe.old` from an
/// update of a running installed copy. Never fails.
pub fn cleanup_after_update() {
    #[cfg(windows)]
    win::cleanup_after_update();
}

/// Handle `--uninstall [--quiet]` (Apps & features runs the
/// UninstallString) and `--install [--desktop] [--quiet]` (scripted
/// install) before the UI starts. Returns true when a flag was handled.
pub fn handle_cli(args: &[String]) -> bool {
    let has = |f: &str| args.iter().any(|a| a == f);
    let quiet = has("--quiet");
    if has("--install") {
        let res = install(Options {
            desktop_shortcut: has("--desktop"),
        });
        match &res {
            Ok(p) => eprintln!("installed: {}", p.display()),
            Err(e) => eprintln!("install failed: {e}"),
        }
        #[cfg(windows)]
        if !quiet {
            win::report(res.map(|_| {
                "«Шорашим» установлен: ярлык в меню «Пуск», запись в «Приложениях».".to_string()
            }));
        }
        return true;
    }
    if !has("--uninstall") {
        return false;
    }
    #[cfg(windows)]
    win::cli_uninstall(quiet);
    #[cfg(not(windows))]
    {
        let _ = quiet;
        eprintln!("--uninstall: установка в систему пока есть только в Windows");
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_order() {
        assert!(newer("0.5.0", "0.4.0"));
        assert!(newer("0.10.0", "0.9.9"));
        assert!(newer("1.0.0", "0.99.0"));
        assert!(!newer("0.4.0", "0.4.0"));
        assert!(!newer("0.4.0", "0.5.0"));
        assert!(newer("0.5.0", "garbage"));
        assert!(newer("v0.5.1", "0.5.0"));
    }

    #[test]
    fn inside_install_dir() {
        let d = Path::new(r"C:\Users\a\AppData\Local\Programs\Shorashim");
        assert!(is_inside(&d.join("shorashim.exe"), d));
        assert!(!is_inside(
            Path::new(r"C:\Users\a\AppData\Local\Programs\Shorashim2\shorashim.exe"),
            d
        ));
        assert!(!is_inside(
            Path::new(r"C:\Users\a\Downloads\shorashim.exe"),
            d
        ));
        assert!(is_inside(
            Path::new(r"\\?\C:\Users\a\AppData\Local\Programs\Shorashim\shorashim.exe"),
            d
        ));
    }

    #[test]
    fn offers() {
        let inst = |v: &str| Installed {
            dir: PathBuf::from("x"),
            exe: PathBuf::from("x/shorashim.exe"),
            version: v.into(),
        };
        let st = |i: Option<Installed>, running| Status {
            support: Support::Supported,
            installed: i,
            running_installed: running,
            desktop_link: false,
        };
        assert_eq!(st(None, false).offer(), Offer::Install);
        assert_eq!(
            st(Some(inst("0.0.1")), false).offer(),
            Offer::Update {
                from: "0.0.1".into(),
                to: current_version().into()
            }
        );
        assert!(matches!(
            st(Some(inst("0.0.1")), true).offer(),
            Offer::UpToDate { .. }
        ));
        assert!(matches!(
            st(Some(inst("99.0.0")), false).offer(),
            Offer::UpToDate { .. }
        ));
        let un = Status {
            support: Support::Unsupported("x"),
            installed: None,
            running_installed: false,
            desktop_link: false,
        };
        assert_eq!(un.offer(), Offer::Unsupported("x"));
    }

    #[test]
    fn uninstall_entry_points_at_installed_exe() {
        let d = Path::new(r"C:\U\AppData\Local\Programs\Shorashim");
        let v = uninstall_values(d, "0.5.0", 30_000);
        let get = |k: &str| v.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("DisplayName"), Some(RegValue::Str("Шорашим".into())));
        assert_eq!(get("DisplayVersion"), Some(RegValue::Str("0.5.0".into())));
        match get("UninstallString") {
            Some(RegValue::Str(s)) => {
                assert!(s.ends_with("--uninstall"));
                assert!(s.contains("Shorashim"));
                assert!(s.starts_with('"'));
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(get("EstimatedSize"), Some(RegValue::Dword(30_000)));
        assert_eq!(get("NoModify"), Some(RegValue::Dword(1)));
    }

    #[test]
    fn install_dir_is_per_user_programs() {
        let d = install_dir_in(Path::new(r"C:\U\AppData\Local"));
        assert!(d.ends_with("Shorashim"));
        assert!(d.to_string_lossy().contains("Programs"));
    }

    #[cfg(not(windows))]
    #[test]
    fn non_windows_is_a_stub() {
        assert!(matches!(status().offer(), Offer::Unsupported(_)));
        assert!(install(Options {
            desktop_shortcut: false
        })
        .is_err());
        assert!(handle_cli(&["x".into(), "--uninstall".into()]));
        assert!(!handle_cli(&["x".into()]));
    }
    #[test]
    fn copy_sidecars_copies_license_and_music() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("shorashim-sidecar-{n}"));
        let from = root.join("portable");
        let to = root.join("installed");
        std::fs::create_dir_all(from.join("music")).unwrap();
        std::fs::create_dir_all(&to).unwrap();
        std::fs::write(from.join("LICENSE"), b"L").unwrap();
        std::fs::write(from.join("NOTICE"), b"N").unwrap();
        std::fs::write(from.join("music").join("manifest.json"), b"{}").unwrap();
        std::fs::write(from.join("music").join("a.ogg"), b"ogg").unwrap();
        copy_sidecars(&from, &to);
        assert_eq!(std::fs::read(to.join("LICENSE")).unwrap(), b"L");
        assert_eq!(std::fs::read(to.join("NOTICE")).unwrap(), b"N");
        assert!(to.join("music").join("manifest.json").is_file());
        assert!(to.join("music").join("a.ogg").is_file());
        let _ = std::fs::remove_dir_all(&root);
    }
}
