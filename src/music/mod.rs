//! Background music: a folder of OGG Vorbis tracks plus a JSON manifest,
//! played with shuffle and a short crossfade.
//!
//! Tracks live next to the executable (`music/`), not inside it, so a new
//! melody can be dropped in without a rebuild. The portable zip and the
//! built-in installer both copy that folder. During development the same
//! files are read from `assets/music/` in the crate root.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use serde::Deserialize;

mod player;
mod playlist;

pub use player::{Controller, Status, CROSSFADE};
pub use playlist::{ShuffleBag, Track};

/// On-disk playlist description (`manifest.json`).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestFile {
    #[serde(default)]
    credit: Option<String>,
    #[serde(default = "default_crossfade_secs")]
    crossfade_seconds: f32,
    #[serde(default)]
    tracks: Vec<ManifestTrack>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestTrack {
    id: String,
    /// Russian display name shown in Settings.
    title: String,
    /// Original Grok Imagine title, kept for credit/reference.
    #[serde(default)]
    #[allow(dead_code)]
    grok_title: Option<String>,
    file: String,
    #[serde(default)]
    duration_seconds: Option<f32>,
}

fn default_crossfade_secs() -> f32 {
    5.0
}

/// Loaded playlist. Missing files are skipped, not fatal.
#[derive(Debug, Clone, Default)]
pub struct Catalog {
    pub credit: Option<String>,
    pub crossfade: Duration,
    pub tracks: Vec<Track>,
    pub dir: Option<PathBuf>,
}

impl Catalog {
    /// Load `manifest.json` from `dir` and keep only tracks whose files exist.
    pub fn from_dir(dir: &Path) -> Self {
        let text = match fs::read_to_string(dir.join("manifest.json")) {
            Ok(t) => t,
            Err(_) => return Catalog::default(),
        };
        let file: ManifestFile = match serde_json::from_str(&text) {
            Ok(f) => f,
            Err(_) => return Catalog::default(),
        };
        let secs = file.crossfade_seconds.clamp(2.0, 8.0);
        let tracks = file
            .tracks
            .into_iter()
            .filter_map(|t| {
                if t.id.is_empty() || t.file.is_empty() {
                    return None;
                }
                let path = dir.join(&t.file);
                if !path.is_file() {
                    return None;
                }
                Some(Track {
                    id: t.id,
                    title: if t.title.is_empty() {
                        t.file.clone()
                    } else {
                        t.title
                    },
                    path,
                    duration: t
                        .duration_seconds
                        .filter(|s| *s > 0.0)
                        .map(Duration::from_secs_f32),
                })
            })
            .collect();
        Catalog {
            credit: file.credit,
            crossfade: Duration::from_secs_f32(secs),
            tracks,
            dir: Some(dir.to_path_buf()),
        }
    }

    pub fn load() -> Self {
        match find_music_dir() {
            Some(dir) => Catalog::from_dir(&dir),
            None => Catalog::default(),
        }
    }
}

static CATALOG: OnceLock<Catalog> = OnceLock::new();

/// Process-wide catalog, loaded once at first use.
pub fn catalog() -> &'static Catalog {
    CATALOG.get_or_init(Catalog::load)
}

/// Directory that contains `manifest.json`, searched in this order:
/// next to the exe (`music/`, then `assets/music/`), the current directory,
/// and the crate's `assets/music` (so `cargo run` from the worktree works).
pub fn find_music_dir() -> Option<PathBuf> {
    let mut cands = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(p) = exe.parent() {
            cands.push(p.join("music"));
            cands.push(p.join("assets").join("music"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        cands.push(cwd.join("music"));
        cands.push(cwd.join("assets").join("music"));
    }
    cands.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join("music"),
    );
    first_dir_with_manifest(&cands)
}

/// First existing folder in `candidates` that has a `manifest.json`.
pub fn first_dir_with_manifest(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates
        .iter()
        .find(|p| p.join("manifest.json").is_file())
        .cloned()
}

/// `music/` or `assets/music/` sitting next to a portable exe (installer).
pub fn sidecar_dir(exe_parent: &Path) -> Option<PathBuf> {
    first_dir_with_manifest(&[
        exe_parent.join("music"),
        exe_parent.join("assets").join("music"),
    ])
}

/// Recursively copy a directory. Existing files at the destination are replaced.
pub fn copy_dir(src: &Path, dest: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let to = dest.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &to)?;
        } else {
            fs::copy(entry.path(), to)?;
        }
    }
    Ok(())
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
        let dir = std::env::temp_dir().join(format!("shorashim-music-{name}-{n}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn bundled_manifest_loads() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join("music");
        let c = Catalog::from_dir(&dir);
        assert!(
            !c.tracks.is_empty(),
            "assets/music must ship at least one track"
        );
        assert_eq!(c.tracks.len(), 10);
        assert_eq!(c.tracks[0].id, "quiet-freygish-study");
        assert_eq!(c.tracks[0].title, "Фрейгиш");
        assert!(c.tracks[0].path.ends_with("quiet-freygish-study.ogg"));
        assert!(c.tracks[0].path.is_file());
        let ids: Vec<_> = c.tracks.iter().map(|t| t.id.as_str()).collect();
        let mut unique = ids.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), ids.len(), "duplicate track id");
        for tr in &c.tracks {
            assert!(!tr.title.is_empty(), "{}", tr.id);
            assert!(tr.path.is_file(), "{}", tr.path.display());
        }
        assert!(c.credit.as_deref().unwrap_or("").contains("Grok Imagine"));
    }

    #[test]
    fn missing_files_are_skipped() {
        let dir = scratch("skip");
        fs::write(
            dir.join("manifest.json"),
            r#"{
              "tracks": [
                {"id": "ok", "title": "Ok", "file": "ok.ogg"},
                {"id": "gone", "title": "Gone", "file": "missing.ogg"},
                {"id": "", "title": "Empty", "file": "ok.ogg"}
              ]
            }"#,
        )
        .unwrap();
        fs::write(dir.join("ok.ogg"), b"not-really-ogg").unwrap();
        let c = Catalog::from_dir(&dir);
        assert_eq!(c.tracks.len(), 1);
        assert_eq!(c.tracks[0].id, "ok");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn first_dir_with_manifest_picks_the_first_hit() {
        let a = scratch("a");
        let b = scratch("b");
        fs::write(b.join("manifest.json"), "{}").unwrap();
        let found = first_dir_with_manifest(&[a.clone(), b.clone()]).unwrap();
        assert_eq!(found, b);
        assert!(first_dir_with_manifest(std::slice::from_ref(&a)).is_none());
        let _ = fs::remove_dir_all(&a);
        let _ = fs::remove_dir_all(&b);
    }
}
