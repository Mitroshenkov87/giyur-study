//! «Открытый режим рецензента»: a mode for a rabbi, a mentor or a beta
//! tester who wants to look at every part of the app without walking the
//! learning path first.
//!
//! While it is on:
//! - every progression gate is lifted. All gates ask [`active`], the one
//!   central check: sections and lessons (`content::Library::is_unlocked`,
//!   the lesson guard), the collection objects and their 3D views
//!   (`collection::is_unlocked`), badges («Успехи»), the first-run guard,
//!   any «Маалот» step (jump buttons in the run), any exam question (free
//!   navigation and «Показать ответ»), any «Предмет дня»;
//! - progress is not counted. The shell (`ui::app::Ctx`) keeps the real
//!   progress aside and the screens work on a sandbox copy that is never
//!   written to disk; XP, badges and collection objects are not awarded
//!   even in the sandbox (`game::award`, `game::check_badges`,
//!   `collection::sync` do nothing). Turning the mode off brings back the
//!   real progress exactly as it was. Only settings (theme, font, …)
//!   changed in the mode are kept.
//!
//! Turn it on in «Настройки», with `--reviewer` / `SHORASHIM_REVIEWER=1`
//! (for this run only), or with the automation command `reviewer`.

use crate::storage::State;

/// Name of the mode (the settings switch, the banner).
pub const TITLE: &str = "Открытый режим рецензента";
/// The caption under the settings switch.
pub const CAPTION: &str =
    "Режим без ограничений: можно переходить в любые разделы и части приложения. Прогресс не засчитывается.";

/// The central check every progression gate asks.
pub fn active(state: &State) -> bool {
    state.settings.reviewer_mode
}

/// `--reviewer` on the command line or `SHORASHIM_REVIEWER=1`: start in the
/// mode for this run only (the stored setting is not changed).
pub fn requested_at_start() -> bool {
    let env = crate::util::env_hook("REVIEWER")
        .map(|v| matches!(v.trim(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false);
    env || std::env::args().skip(1).any(|a| a == "--reviewer")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Library;

    #[test]
    fn the_check_follows_the_setting() {
        let mut s = State::default();
        assert!(!active(&s));
        s.settings.reviewer_mode = true;
        assert!(active(&s));
    }

    #[test]
    fn every_gate_opens_in_the_mode() {
        let lib = Library::current();
        let mut s = State {
            onboarded: true,
            ..Default::default()
        };
        s.settings.level = Some(crate::storage::Level::Reform);
        let locked_before = lib
            .sections
            .iter()
            .filter(|sec| sec.available && !lib.is_unlocked(sec, &s))
            .count();
        assert!(locked_before > 0, "a fresh profile has locked sections");
        assert_eq!(crate::collection::count(&s), 0);
        s.settings.reviewer_mode = true;
        for sec in lib.sections.iter().filter(|sec| sec.available) {
            assert!(lib.is_unlocked(sec, &s), "{}", sec.id);
        }
        assert_eq!(crate::collection::count(&s), crate::collection::ITEMS.len());
        for b in crate::game::badge_list(&lib) {
            assert!(crate::game::badge_open(&s, &b.id), "{}", b.id);
        }
    }

    #[test]
    fn nothing_is_awarded_in_the_mode() {
        let lib = Library::current();
        let mut s = State::default();
        s.settings.reviewer_mode = true;
        let today = chrono::NaiveDate::from_ymd_opt(2026, 11, 23).unwrap();
        let out = crate::game::award(&lib, &mut s, 50, today);
        assert_eq!(out.amount, 0);
        assert_eq!(s.xp_total, 0);
        assert!(s.xp.is_empty() && s.goal_days.is_empty() && s.badges.is_empty());
        assert!(crate::game::check_badges(&lib, &mut s, today).is_empty());
        assert!(crate::collection::sync(&mut s, "2026-11-23").is_empty());
        assert!(!s.extra.contains_key("collection"));
    }
}
