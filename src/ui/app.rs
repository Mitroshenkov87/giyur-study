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
    Section(String),
    Lesson(String),
    Cards(Deck),
    /// «Тесты» setup; optional preselected section.
    Exam(Option<String>),
    /// Test runner for a prepared session (section, general, beit-din, retry).
    ExamRun(ExamSpec),
    /// Timed quiz-show intro «Экзамен».
    Timed,
    /// Timed run; `retry_ids` = «Повторить ошибки (n)» from the results screen.
    TimedRun {
        retry_ids: Option<Vec<String>>,
    },
    Progress,
    Settings,
    Sources,
}

/// Which flashcards to study.
#[derive(Debug, Clone, PartialEq)]
pub enum Deck {
    Lesson(String),
    Section(String),
    /// All due cards («Повторение»).
    Review,
}

/// What exam to start in the runner.
#[derive(Debug, Clone, PartialEq)]
pub enum ExamSpec {
    Section(String),
    General { count: usize },
    BeitDin { count: usize },
    Retry { ids: Vec<String> },
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
}

impl Ctx {
    pub fn load() -> Ctx {
        let path = std::env::var_os("GIYUR_STUDY_DATA")
            .map(PathBuf::from)
            .unwrap_or_else(storage::default_path);
        let today = chrono::Local::now().date_naive();
        let (state, writable) = storage::load_checked(&path, &crate::util::day_key(today));
        let mut ctx = Ctx {
            lib: Library::embedded(),
            state,
            path,
            toasts: Vec::new(),
            today_override: None,
            test_timer: std::env::var("GIYUR_TEST_TIMER")
                .ok()
                .and_then(|v| v.parse().ok()),
            writable,
        };
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
        ctx.save();
        ctx
    }

    pub fn today(&self) -> NaiveDate {
        self.today_override
            .unwrap_or_else(|| chrono::Local::now().date_naive())
    }

    pub fn today_key(&self) -> String {
        crate::util::day_key(self.today())
    }

    /// Persist progress (atomic write). Errors are reported as a toast.
    pub fn save(&mut self) {
        if !self.writable {
            return;
        }
        if let Err(e) = storage::save(&self.path, &self.state) {
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
        self.toasts.push(Toast {
            title: c.title,
            text: c.text,
            icon: c.icon,
            badge_id: None,
            until: self.next_slot(4200),
        });
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
