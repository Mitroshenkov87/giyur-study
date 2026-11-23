//! Application shell: shared context, routes, navigation, toasts.
//!
//! Contract for screens (src/ui/screens/<name>.rs):
//! ```ignore
//! pub struct Screen { /* view state */ }
//! #[derive(Debug, Clone)] pub enum Msg { .. }
//! impl Screen {
//!     pub fn new(ctx: &mut Ctx /*, route args */) -> Self;
//!     pub fn update(&mut self, ctx: &mut Ctx, msg: Msg) -> Action;
//!     pub fn view<'a>(&'a self, ctx: &'a Ctx) -> Element<'a, Msg>;
//!     // optional: pub fn subscription(&self) -> Subscription<Msg>;
//! }
//! ```
//! Screens mutate `ctx.state` through the core modules (srs, exam, game) and
//! call `ctx.save()` after each change; celebrations go to `ctx.celebrate()`.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use chrono::NaiveDate;

use crate::content::Library;
use crate::game::{self, AwardOutcome, Celebration};
use crate::storage::{self, State};

/// Navigation targets (mirror of the PWA hash routes, docs/PARITY.md §0.1).
#[derive(Debug, Clone, PartialEq)]
pub enum Route {
    Welcome,
    /// Movement choice; `reopened` = came from map chip / settings («Назад» shown).
    Level {
        reopened: bool,
    },
    Map,
    /// «Еврейский календарь» (Сегодня / Месяц / Настройки).
    Calendar,
    Section(String),
    Lesson(String),
    Cards(Deck),
    /// «Тесты» setup; optional preselected section.
    Exam(Option<String>),
    /// Test runner for a prepared session (section, general, beit-din, retry).
    ExamRun(ExamSpec),
    /// «Викторина»: the «Маалот» ladder setup.
    Timed,
    /// Timed run; `retry_ids` = «Повторить ошибки (n)» from the results screen.
    TimedRun {
        retry_ids: Option<Vec<String>>,
    },
    Progress,
    Settings,
    Sources,
    /// «Тренажёр собеседования с бейт-дином» (src/ui/screens/interview.rs).
    Interview(crate::interview::Mode),
    /// «Молитвы»: the list, or one prayer by id (src/prayers.rs).
    Prayers(Option<String>),
}

/// Which flashcards to study.
#[derive(Debug, Clone, PartialEq)]
pub enum Deck {
    Lesson(String),
    Section(String),
    /// All due cards («Карточки»).
    Review,
}

/// What exam to start in the runner.
#[derive(Debug, Clone, PartialEq)]
pub enum ExamSpec {
    Section {
        id: String,
        count: usize,
    },
    General {
        count: usize,
    },
    BeitDin {
        count: usize,
    },
    /// «Повторить ошибки»; `section_id` keeps the retry tied to its section
    /// (kicker, «К разделу» button, history entry).
    Retry {
        ids: Vec<String>,
        section_id: Option<String>,
    },
    /// «Самопроверка» of one lesson (question ids picked by study::self_check_ids).
    SelfCheck {
        section_id: String,
        ids: Vec<String>,
    },
    /// «Репетиция бейт-дина» of one section (study::rehearsal_ids).
    Rehearsal {
        section_id: String,
        ids: Vec<String>,
    },
}

/// What a screen asks the shell to do after an update.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    None,
    Go(Route),
    /// Return to the previous route (or the map).
    Back,
    /// Theme or level changed: rebuild everything that depends on settings.
    SettingsChanged,
    /// Close the app (after «Удалить из системы» from the installed copy).
    Quit,
    /// Skip to the next background track (settings «Следующий трек»).
    MusicNext,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub title: String,
    pub text: String,
    pub icon: String,
    pub badge_id: Option<String>,
    pub until: Instant,
}

/// Shared state handed to every screen.
pub struct Ctx {
    pub lib: Library,
    pub state: State,
    pub path: PathBuf,
    pub toasts: Vec<Toast>,
    /// Test hook: fixed "today" (None = real local date).
    pub today_override: Option<NaiveDate>,
    /// Test hook: timed-exam seconds override (PWA `?testTimer=`).
    pub test_timer: Option<u32>,
    /// False when an existing progress file could not be read: never
    /// overwrite it (PARITY: storage unavailable → progress not saved).
    pub writable: bool,
    /// Current background-track title, filled by the shell from the player.
    pub music_title: Option<String>,
    pub music_available: bool,
    pub music_error: Option<String>,
    pub music_track_count: usize,
    /// «Открытый режим рецензента» (src/reviewer.rs): the real progress,
    /// kept aside while the screens work on a sandbox copy in `state`.
    pub sandbox: Option<Sandbox>,
}

/// The real progress while the «Открытый режим рецензента» is on.
pub struct Sandbox {
    /// Exactly what was there when the mode was switched on; only its
    /// settings follow the ones changed in the mode.
    pub real: State,
    /// Whether the mode is stored in the progress file (switched on in
    /// «Настройки») or only for this run (`--reviewer`).
    pub persist: bool,
}

impl Ctx {
    pub fn load() -> Ctx {
        // A file or a folder (then progress.json inside it).
        let path = storage::data_path_from_env().unwrap_or_else(storage::default_path);
        let today = chrono::Local::now().date_naive();
        let (state, writable) = storage::load_checked(&path, &crate::util::day_key(today));
        let mut ctx = Ctx {
            lib: Library::embedded(),
            state,
            path,
            toasts: Vec::new(),
            today_override: None,
            test_timer: crate::util::env_hook("TEST_TIMER").and_then(|v| v.parse().ok()),
            writable,
            music_title: None,
            music_available: false,
            music_error: None,
            music_track_count: 0,
            sandbox: None,
        };
        let cat = crate::music::catalog();
        ctx.music_available = !cat.tracks.is_empty();
        ctx.music_track_count = cat.tracks.len();
        if !writable {
            ctx.toast(
                "Файл прогресса недоступен",
                "Прогресс не сохранится, пока файл занят или повреждён.",
                "⚠",
            );
        }
        // js/game.js G.init: spend streak freezes for missed days, then
        // unlock already-earned badges quietly.
        let today = ctx.today();
        game::apply_freezes(&mut ctx.state, today);
        let _ = game::check_badges(&ctx.lib, &mut ctx.state, today);
        // Objects earned before the collection existed join it quietly.
        crate::collection::sync(&mut ctx.state, &crate::util::day_key(today));
        ctx.save();
        ctx.start_reviewer_if_asked();
        ctx
    }

    /// Enter the «Открытый режим рецензента» at start-up when the progress
    /// file has it on, or `--reviewer` / `SHORASHIM_REVIEWER=1` asks for it.
    pub fn start_reviewer_if_asked(&mut self) {
        if crate::reviewer::active(&self.state) {
            self.set_reviewer(true, true);
        } else if crate::reviewer::requested_at_start() {
            self.set_reviewer(true, false);
        }
    }

    /// Whether the «Открытый режим рецензента» is on.
    pub fn reviewer(&self) -> bool {
        self.sandbox.is_some()
    }

    /// The real progress (the one kept aside while the mode is on).
    pub fn real_state(&self) -> &State {
        self.sandbox.as_ref().map_or(&self.state, |s| &s.real)
    }

    /// Switch the «Открытый режим рецензента». On: the real progress is put
    /// aside and the screens get a copy that is never saved. Off: the real
    /// progress comes back exactly as it was (with the current settings).
    /// `persist`: keep the mode on across restarts (the settings switch).
    pub fn set_reviewer(&mut self, on: bool, persist: bool) {
        match (on, self.sandbox.is_some()) {
            (true, false) => {
                let real = self.state.clone();
                self.state.settings.reviewer_mode = true;
                self.sandbox = Some(Sandbox { real, persist });
                self.save();
            }
            (true, true) => {
                if let Some(sb) = &mut self.sandbox {
                    sb.persist |= persist;
                }
                self.save();
            }
            (false, true) => {
                let sb = self.sandbox.take().expect("checked");
                let mut real = sb.real;
                real.settings = self.state.settings.clone();
                real.settings.reviewer_mode = false;
                self.state = real;
                self.save();
            }
            (false, false) => {
                self.state.settings.reviewer_mode = false;
                self.save();
            }
        }
    }

    pub fn today(&self) -> NaiveDate {
        self.today_override
            .unwrap_or_else(|| chrono::Local::now().date_naive())
    }

    pub fn today_key(&self) -> String {
        crate::util::day_key(self.today())
    }

    /// Persist progress (atomic write). Errors are reported as a toast.
    /// While the «Открытый режим рецензента» is on only the settings reach
    /// the file: the real progress is written back unchanged.
    pub fn save(&mut self) {
        if !self.writable {
            return;
        }
        let res = match &mut self.sandbox {
            Some(sb) => {
                sb.real.settings = self.state.settings.clone();
                sb.real.settings.reviewer_mode = sb.persist;
                storage::save(&self.path, &sb.real)
            }
            None => storage::save(&self.path, &self.state),
        };
        if let Err(e) = res {
            self.toast("Не удалось сохранить прогресс", &e.to_string(), "⚠");
        }
    }

    /// Record learning activity for today (js/storage.js `touch`) and save.
    pub fn activity(&mut self) {
        let key = self.today_key();
        self.state.touch(&key);
        self.save();
    }

    /// Toasts are shown one at a time: each queued toast gets its own full
    /// display time after the previous one ends.
    fn next_slot(&self, ms: u64) -> Instant {
        let now = Instant::now();
        let start = self.toasts.last().map_or(now, |t| t.until.max(now));
        start + Duration::from_millis(ms)
    }

    /// Close the toast on screen (a click on it); the next ones move up and
    /// keep their full time.
    pub fn dismiss_toast(&mut self, now: Instant) {
        if self.toasts.is_empty() {
            return;
        }
        let first = self.toasts.remove(0);
        let gained = first.until.saturating_duration_since(now);
        for t in &mut self.toasts {
            t.until -= gained;
        }
    }

    /// Hold every toast for `by` (they were raised while the start-up
    /// splash covered the window): the first one starts its time now.
    pub fn delay_toasts(&mut self, by: Duration) {
        for t in &mut self.toasts {
            t.until += by;
        }
    }

    pub fn toast(&mut self, title: &str, text: &str, icon: &str) {
        self.toasts.push(Toast {
            title: title.into(),
            text: text.into(),
            icon: icon.into(),
            badge_id: None,
            until: self.next_slot(4200),
        });
    }

    pub fn celebrate(&mut self, c: Celebration) {
        if self.reviewer() {
            return;
        }
        self.toasts.push(Toast {
            title: c.title,
            text: c.text,
            icon: c.icon,
            badge_id: None,
            until: self.next_slot(4200),
        });
    }

    /// Add the «Успехи» collection objects whose rule is met now (a lesson
    /// read, a «Маалот» step), announce each one with a toast and save.
    pub fn sync_collection(&mut self) {
        let today = self.today_key();
        let new = crate::collection::sync(&mut self.state, &today);
        if new.is_empty() {
            return;
        }
        for key in new {
            if let Some(m) = crate::ui::model3d::Model::from_key(key) {
                let text = format!("«{}» — откройте его в разделе «Успехи».", m.info().title);
                self.toasts.push(Toast {
                    title: "Новый предмет в коллекции".into(),
                    text,
                    icon: "✓".into(),
                    badge_id: None,
                    until: self.next_slot(4200),
                });
            }
        }
        self.save();
    }

    /// Award XP, then show goal / badge celebrations and save.
    pub fn award(&mut self, amount: i64) -> AwardOutcome {
        let today = self.today();
        let out = game::award(&self.lib, &mut self.state, amount, today);
        if let Some(c) = out.celebration.clone() {
            self.celebrate(c);
        }
        for b in &out.new_badges {
            let mut c = b.celebration();
            c.is_badge = true;
            self.toasts.push(Toast {
                title: c.title,
                text: c.text,
                icon: c.icon,
                badge_id: Some(b.id.clone()),
                until: self.next_slot(5200),
            });
        }
        self.save();
        out
    }
}
