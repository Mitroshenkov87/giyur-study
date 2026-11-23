//! Headless session: the app's state and screens without a window. Runs the
//! same `Current::update` / navigation logic as the windowed shell
//! (src/main.rs `App::go` / `App::update`), so scripts test real behaviour.

use std::io::{BufRead, Write};
use std::path::PathBuf;

use chrono::NaiveDate;
use serde_json::{json, Value};

use super::{execute, headless_screenshot, Effect, Host};
use crate::content::Library;
use crate::game;
use crate::storage;
use crate::ui::app::{Action, Ctx, Route};
use crate::ui::screens::{self, Current};
use crate::ui::theme;

pub struct Session {
    pub ctx: Ctx,
    pub route: Route,
    pub history: Vec<Route>,
    pub screen: Current,
    pub quit: bool,
    /// Keeps an ephemeral data folder alive for the session.
    _tmp: Option<TempDir>,
}

/// A throw-away data folder (removed on drop).
struct TempDir(PathBuf);
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Options for a headless session.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Progress file; `None` = a fresh throw-away one (never the user's).
    pub data: Option<PathBuf>,
    /// Fixed "today" (calendar, streaks) for reproducible runs.
    pub today: Option<NaiveDate>,
    /// Start in the «Открытый режим рецензента» (`--reviewer`).
    pub reviewer: bool,
}

impl Session {
    pub fn new(opts: Options) -> Session {
        let (path, tmp) = match opts.data {
            Some(p) => (p, None),
            None => {
                let dir = std::env::temp_dir().join(format!(
                    "shorashim-headless-{}-{}",
                    std::process::id(),
                    rand::random::<u64>()
                ));
                let _ = std::fs::create_dir_all(&dir);
                (dir.join("progress.json"), Some(TempDir(dir)))
            }
        };
        let today = opts
            .today
            .unwrap_or_else(|| chrono::Local::now().date_naive());
        let (state, writable) = storage::load_checked(&path, &crate::util::day_key(today));
        let mut ctx = Ctx {
            lib: Library::embedded(),
            state,
            path,
            toasts: Vec::new(),
            today_override: opts.today,
            test_timer: None,
            writable,
            music_title: None,
            music_available: false,
            music_error: None,
            music_track_count: 0,
            sandbox: None,
        };
        game::apply_freezes(&mut ctx.state, today);
        let _ = game::check_badges(&ctx.lib, &mut ctx.state, today);
        ctx.save();
        if opts.reviewer {
            ctx.set_reviewer(true, false);
        } else {
            ctx.start_reviewer_if_asked();
        }
        theme::set_current(super::theme_kind(&ctx));
        let route = if ctx.state.onboarded {
            Route::Map
        } else {
            Route::Welcome
        };
        let screen = Current::open(&mut ctx, &route);
        Session {
            ctx,
            route,
            history: Vec::new(),
            screen,
            quit: false,
            _tmp: tmp,
        }
    }

    /// Same rules as the windowed shell's `go`.
    fn go(&mut self, route: Route) {
        let route = if !self.ctx.state.onboarded
            && !matches!(route, Route::Welcome | Route::Level { .. })
        {
            Route::Welcome
        } else {
            route
        };
        if route != self.route {
            self.history.push(self.route.clone());
            if self.history.len() > 50 {
                self.history.remove(0);
            }
        }
        self.screen = Current::open(&mut self.ctx, &route);
        self.route = route;
    }

    /// Run one JSON command; effects (screenshots, quit) are done here.
    pub fn run(&mut self, req: &Value) -> Value {
        let out = execute(self, req);
        let mut reply = out.reply;
        match out.effect {
            Some(Effect::Screenshot {
                path,
                full_page,
                size,
            }) => match headless_screenshot(self, &path, full_page, size) {
                Ok((w, h)) => {
                    reply["width"] = json!(w);
                    reply["height"] = json!(h);
                }
                Err(e) => reply = json!({"ok": false, "error": e}),
            },
            Some(Effect::Quit) => self.quit = true,
            Some(Effect::Wait(ms)) => self.wait(ms),
            Some(Effect::Scroll(_)) | None => {}
        }
        if let Some(expect) = req.get("expect") {
            if reply["ok"] == json!(true) {
                if let Err(e) = self.check(expect, &reply) {
                    reply = json!({"ok": false, "error": e, "reply": reply});
                }
            }
        }
        reply
    }

    /// Sleep, feeding the clock-driven messages the windowed app gets from
    /// its subscriptions (100 ms ticks).
    fn wait(&mut self, ms: u64) {
        let end = std::time::Instant::now() + std::time::Duration::from_millis(ms);
        loop {
            let now = std::time::Instant::now();
            match &self.screen {
                Current::Timed(_) => {
                    self.dispatch(screens::Msg::Timed(screens::timed::Msg::Tick(now)))
                }
                Current::Calendar(_) => {
                    self.dispatch(screens::Msg::Calendar(screens::calendar::Msg::GeoTick))
                }
                _ => {}
            }
            if now >= end {
                break;
            }
            std::thread::sleep((end - now).min(std::time::Duration::from_millis(100)));
        }
    }

    /// Script assertions: `"expect": {"screen": "lesson", "route": "lesson:",
    /// "text": "Шаббат", "control": "Далее", "ok": true}`.
    fn check(&mut self, expect: &Value, reply: &Value) -> Result<(), String> {
        let Some(e) = expect.as_object() else {
            return Err("expect must be an object".into());
        };
        let screen = super::screen_name(&self.screen);
        let route = super::route_str(&self.route);
        if let Some(s) = e.get("screen").and_then(Value::as_str) {
            if s != screen {
                return Err(format!("expected screen «{s}», got «{screen}»"));
            }
        }
        if let Some(r) = e.get("route").and_then(Value::as_str) {
            if !route.starts_with(r) {
                return Err(format!("expected route starting «{r}», got «{route}»"));
            }
        }
        let wants_text = e.get("text").and_then(Value::as_str);
        let wants_control = e.get("control").and_then(Value::as_str);
        if wants_text.is_some() || wants_control.is_some() {
            let view = super::inspect(self);
            if let Some(t) = wants_text {
                let t = t.to_lowercase();
                if !view
                    .texts
                    .iter()
                    .any(|x| x.text.to_lowercase().contains(&t))
                {
                    return Err(format!("expected text «{t}» on «{route}»"));
                }
            }
            if let Some(c) = wants_control {
                let c = c.to_lowercase();
                if !view.controls.iter().any(|x| {
                    x.id.to_lowercase().contains(&c)
                        || x.label.iter().any(|l| l.to_lowercase().contains(&c))
                }) {
                    return Err(format!("expected control «{c}» on «{route}»"));
                }
            }
        }
        if let Some(min) = e.get("min_xp").and_then(Value::as_i64) {
            if self.ctx.state.xp_total < min {
                return Err(format!(
                    "expected xp_total ≥ {min}, got {}",
                    self.ctx.state.xp_total
                ));
            }
        }
        if let Some(obj) = e.get("reply").and_then(Value::as_object) {
            for (k, v) in obj {
                if !reply.get(k).is_some_and(|got| subset(v, got)) {
                    return Err(format!("expected reply.{k} ⊇ {v}, got {:?}", reply.get(k)));
                }
            }
        }
        Ok(())
    }

    /// Run JSON lines from `input`, one reply line per command to `output`.
    /// Blank lines and `#` / `//` comments are skipped. Returns the number of
    /// failed commands.
    pub fn run_lines(
        &mut self,
        input: impl BufRead,
        mut output: impl Write,
        stop_on_error: bool,
    ) -> usize {
        let mut failed = 0;
        for (n, line) in input.lines().enumerate() {
            let Ok(line) = line else { break };
            let t = line.trim();
            if t.is_empty() || t.starts_with('#') || t.starts_with("//") {
                continue;
            }
            let reply = match serde_json::from_str::<Value>(t) {
                Ok(req) => self.run(&req),
                Err(e) => json!({"ok": false, "error": format!("line {}: bad JSON: {e}", n + 1)}),
            };
            let ok = reply["ok"] == json!(true);
            let mut reply = reply;
            if !ok {
                failed += 1;
                reply["line"] = json!(n + 1);
            }
            let _ = writeln!(output, "{reply}");
            let _ = output.flush();
            if (!ok && stop_on_error) || self.quit {
                break;
            }
        }
        failed
    }
}

impl Host for Session {
    fn ctx(&self) -> &Ctx {
        &self.ctx
    }
    fn ctx_mut(&mut self) -> &mut Ctx {
        &mut self.ctx
    }
    fn screen(&self) -> &Current {
        &self.screen
    }
    fn route(&self) -> &Route {
        &self.route
    }
    fn dispatch(&mut self, msg: screens::Msg) {
        match self.screen.update(&mut self.ctx, msg) {
            Action::Go(r) => self.go(r),
            Action::Back => self.back(),
            Action::SettingsChanged => self.settings_changed(),
            #[allow(unreachable_patterns)]
            _ => {}
        }
    }
    fn navigate(&mut self, route: Route) {
        self.go(route);
    }
    fn back(&mut self) {
        let r = self.history.pop().unwrap_or(Route::Map);
        self.screen = Current::open(&mut self.ctx, &r);
        self.route = r;
    }
    fn settings_changed(&mut self) {
        theme::set_current(super::theme_kind(&self.ctx));
    }
    fn windowed(&self) -> bool {
        false
    }
}

/// `want` matches `got` when every key of a `want` object is in `got` with a
/// matching value (recursively); other values must be equal.
fn subset(want: &Value, got: &Value) -> bool {
    match (want, got) {
        (Value::Object(w), Value::Object(g)) => w
            .iter()
            .all(|(k, v)| g.get(k).is_some_and(|gv| subset(v, gv))),
        _ => want == got,
    }
}
