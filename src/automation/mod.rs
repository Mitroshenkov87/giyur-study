//! Command-based control of the app, for testing bots and CI.
//!
//! Commands are JSON objects (`{"cmd": "press", "label": "Начать"}`). They
//! map onto the app's real `Message`s: `press` publishes exactly what a
//! mouse click on that control would publish (found by simulating the click
//! on a headless copy of the current screen), `navigate` is the sidebar /
//! route change, `answer` sends the quiz messages. So every command runs the
//! same `update` code as a user's click.
//!
//! The same [`execute`] serves three front ends:
//! - the GUI app's local control channel (`--control`, see [`server`]),
//! - the headless session (`--headless`, see [`session`]) used by CI,
//! - the `ctl` CLI and the `mcp` stdio server (see [`cli`], [`mcp`]).
//!
//! Docs: docs/AUTOMATION.md.

pub mod cli;
pub mod client;
pub mod mcp;
pub mod probe;
pub mod server;
pub mod session;

use std::path::PathBuf;

use iced::widget::{container, scrollable, stack};
use iced::{Element, Length, Size};
use serde_json::{json, Map, Value};

use crate::ui::app::{Ctx, Deck, ExamSpec, Route};
use crate::ui::screens::{self, Current};
use crate::ui::theme::{self, ThemeKind};

/// Width of the probed content column (the app's reading column).
pub const PROBE_WIDTH: f32 = 1040.0;
/// Tall enough that a whole page is "on screen" for the click probe.
const PROBE_HEIGHT: f32 = 24000.0;

/// What the front end hosting the app must provide.
pub trait Host {
    fn ctx(&self) -> &Ctx;
    fn ctx_mut(&mut self) -> &mut Ctx;
    fn screen(&self) -> &Current;
    fn route(&self) -> &Route;
    /// Feed a screen message through the app's normal `update`.
    fn dispatch(&mut self, msg: screens::Msg);
    /// Go to a route like a sidebar click (`Message::Nav`).
    fn navigate(&mut self, route: Route);
    /// «Назад»: previous route, or the map.
    fn back(&mut self);
    /// Theme / font / level changed outside a screen.
    fn settings_changed(&mut self);
    /// True for the windowed app (screenshots of the real window, scroll).
    fn windowed(&self) -> bool;
}

/// Side effects only the front end can perform.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    /// Save a WebP screenshot. `full_page`: headless only, whole page height.
    Screenshot {
        path: PathBuf,
        full_page: bool,
        size: Option<Size>,
    },
    /// Scroll the main column (0 = top, 1 = bottom). Windowed only.
    Scroll(f32),
    /// Close the app (after the reply is sent).
    Quit,
}

/// Reply plus an optional effect for the front end.
#[derive(Debug, Clone)]
pub struct Outcome {
    pub reply: Value,
    pub effect: Option<Effect>,
}

impl Outcome {
    fn ok(v: Value) -> Outcome {
        let mut v = v;
        if let Some(o) = v.as_object_mut() {
            o.insert("ok".into(), Value::Bool(true));
        }
        Outcome {
            reply: v,
            effect: None,
        }
    }
    fn err(msg: impl Into<String>) -> Outcome {
        Outcome {
            reply: json!({"ok": false, "error": msg.into()}),
            effect: None,
        }
    }
}

/// The sidebar items, in order (label, route string).
pub const NAV: [(&str, &str); 8] = [
    ("Карта", "map"),
    ("Календарь", "calendar"),
    ("Повторение", "cards:review"),
    ("Экзамен", "timed"),
    ("Тесты", "exam"),
    ("Прогресс", "progress"),
    ("Настройки", "settings"),
    ("Источники", "sources"),
];

/// Command list for `help` and the MCP tool catalogue.
pub const COMMANDS: &[(&str, &str)] = &[
    ("help", "This list."),
    ("screens", "Routes, sidebar items, sections and lessons you can navigate to."),
    ("state", "Current screen: route, visible texts, clickable controls (stable ids), inputs, progress, settings, toasts. Args: texts (bool, default true)."),
    ("navigate", "Go to a route: to = \"map\" | \"calendar[:today|month|settings|YYYY-MM-DD]\" | \"section:<id>\" | \"lesson:<id>\" | \"cards:<lessonId>\" | \"deck:<sectionId>\" | \"cards:review\" | \"exam[:<sectionId>]\" | \"examrun:section:<id>[:n]\" | \"examrun:general[:n]\" | \"examrun:beitdin[:n]\" | \"timed\" | \"timedrun\" | \"progress\" | \"settings\" | \"sources\" | \"level\" | \"welcome\", or a sidebar label («Календарь»)."),
    ("back", "«Назад»: previous screen."),
    ("press", "Press a control from `state`: id (exact), or label (exact, case-insensitive), or contains (substring); nth picks among several matches (0-based)."),
    ("answer", "Answer the current question: choice (0-based option), tf (true/false), text (typed answer, submitted), knew (flashcard: flips if needed, then grades), self_grade (0..1, open answers); next=true then presses «Далее»."),
    ("type", "Type into an input field: text, input (index from `state`, default 0), append (keep current text), submit (press Enter)."),
    ("select", "Pick from a drop-down: target (text of the closed pick list), option (text of the option)."),
    ("set", "Change a setting: key = theme (dark|parchment) | font_step (0..3) | translit | unlock_all | timed_sound | timed_auto (bool) | timed_seconds (20|30) | daily_goal (10|20|30|50), value."),
    ("screenshot", "Save a WebP screenshot: path (.webp). Windowed: the real window. Headless: the page; full_page=true for the whole page, width/height optional."),
    ("scroll", "Windowed only: to = \"top\" | \"bottom\" | 0..1."),
    ("quit", "Close the app (headless: end the session)."),
];

// ---------------------------------------------------------------- routes

/// Route grammar shared with the `SHORASHIM_START_ROUTE` hook.
/// Returns the route plus an optional calendar tab / date to open after.
pub fn parse_route(r: &str) -> Option<(Route, Option<screens::calendar::Msg>)> {
    let r = r.trim();
    if let Some((_, route)) = NAV
        .iter()
        .find(|(label, _)| label.to_lowercase() == r.to_lowercase())
    {
        return parse_route(route);
    }
    let (k, v) = r.split_once(':').unwrap_or((r, ""));
    let v = v.to_string();
    let count = |s: Option<&str>, d: usize| s.and_then(|c| c.parse().ok()).unwrap_or(d);
    let mut follow = None;
    let route = match k {
        "welcome" => Route::Welcome,
        "level" => Route::Level { reopened: true },
        "map" => Route::Map,
        "calendar" => {
            use screens::calendar::{Msg as C, Tab};
            follow = match v.as_str() {
                "" => None,
                "today" => Some(C::Tab(Tab::Today)),
                "month" => Some(C::Tab(Tab::Month)),
                "settings" => Some(C::Tab(Tab::Settings)),
                d => Some(C::PickDate(
                    chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()?,
                )),
            };
            Route::Calendar
        }
        "section" if !v.is_empty() => Route::Section(v),
        "lesson" if !v.is_empty() => Route::Lesson(v),
        "cards" if v == "review" => Route::Cards(Deck::Review),
        "review" => Route::Cards(Deck::Review),
        "cards" if !v.is_empty() => Route::Cards(Deck::Lesson(v)),
        "deck" if !v.is_empty() => Route::Cards(Deck::Section(v)),
        "exam" => Route::Exam(if v.is_empty() { None } else { Some(v) }),
        "examrun" => {
            let mut parts = v.split(':');
            match parts.next()? {
                "section" => {
                    let id = parts.next()?.to_string();
                    Route::ExamRun(ExamSpec::Section {
                        id,
                        count: count(parts.next(), 10),
                    })
                }
                "general" => Route::ExamRun(ExamSpec::General {
                    count: count(parts.next(), 10),
                }),
                "beitdin" => Route::ExamRun(ExamSpec::BeitDin {
                    count: count(parts.next(), 5),
                }),
                _ => return None,
            }
        }
        "timed" => Route::Timed,
        "timedrun" => Route::TimedRun { retry_ids: None },
        "progress" => Route::Progress,
        "settings" => Route::Settings,
        "sources" => Route::Sources,
        _ => return None,
    };
    Some((route, follow))
}

/// Inverse of [`parse_route`] (best effort for exam runs).
pub fn route_str(r: &Route) -> String {
    match r {
        Route::Welcome => "welcome".into(),
        Route::Level { .. } => "level".into(),
        Route::Map => "map".into(),
        Route::Calendar => "calendar".into(),
        Route::Section(id) => format!("section:{id}"),
        Route::Lesson(id) => format!("lesson:{id}"),
        Route::Cards(Deck::Review) => "cards:review".into(),
        Route::Cards(Deck::Lesson(id)) => format!("cards:{id}"),
        Route::Cards(Deck::Section(id)) => format!("deck:{id}"),
        Route::Exam(None) => "exam".into(),
        Route::Exam(Some(id)) => format!("exam:{id}"),
        Route::ExamRun(spec) => match spec {
            ExamSpec::Section { id, count } => format!("examrun:section:{id}:{count}"),
            ExamSpec::General { count } => format!("examrun:general:{count}"),
            ExamSpec::BeitDin { count } => format!("examrun:beitdin:{count}"),
            ExamSpec::Retry { .. } => "examrun:retry".into(),
            ExamSpec::SelfCheck { section_id, .. } => format!("examrun:selfcheck:{section_id}"),
            ExamSpec::Rehearsal { section_id, .. } => format!("examrun:rehearsal:{section_id}"),
        },
        Route::Timed => "timed".into(),
        Route::TimedRun { .. } => "timedrun".into(),
        Route::Progress => "progress".into(),
        Route::Settings => "settings".into(),
        Route::Sources => "sources".into(),
    }
}

pub fn screen_name(s: &Current) -> &'static str {
    match s {
        Current::Welcome(_) => "welcome",
        Current::Level(_) => "level",
        Current::Map(_) => "map",
        Current::Calendar(_) => "calendar",
        Current::Section(_) => "section",
        Current::Sources(_) => "sources",
        Current::Lesson(_) => "lesson",
        Current::Cards(_) => "cards",
        Current::Exam(_) => "exam",
        Current::Timed(_) => "timed",
        Current::Progress(_) => "progress",
        Current::Settings(_) => "settings",
    }
}

// ------------------------------------------------------------ page probe

pub fn theme_kind(ctx: &Ctx) -> ThemeKind {
    if ctx.state.settings.theme == "parchment" {
        ThemeKind::Parchment
    } else {
        ThemeKind::Dark
    }
}

/// The current screen as the shell shows it: padded reading column in a
/// scrollable, with the screen's dialog (if any) on top.
pub fn page<'a>(screen: &'a Current, ctx: &'a Ctx) -> Element<'a, screens::Msg> {
    let p = theme::current();
    let inner = container(
        container(screen.view(ctx))
            .padding([28, 32])
            .width(Length::Fill)
            .max_width(PROBE_WIDTH),
    )
    .center_x(Length::Fill);
    let main = container(scrollable(inner).height(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| theme::app_background(&p));
    match screen.dialog(ctx) {
        Some(d) => stack![main, iced::widget::opaque(d)].into(),
        None => main.into(),
    }
}

/// A clickable control found by the probe.
#[derive(Debug, Clone)]
pub struct Control {
    /// Stable id: the Debug form of the message(s) a click publishes.
    pub id: String,
    /// Visible texts inside the control.
    pub label: Vec<String>,
    pub msgs: Vec<screens::Msg>,
}

/// Everything a bot needs to see on the current screen.
pub struct View {
    pub texts: Vec<probe::TextItem>,
    pub inputs: Vec<probe::InputItem>,
    pub controls: Vec<Control>,
    pub content_height: Option<f32>,
}

fn fnv(s: &str) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for b in s.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

fn control_id(msgs: &[screens::Msg]) -> String {
    let key = msgs
        .iter()
        .map(probe::debug_of)
        .collect::<Vec<_>>()
        .join(" + ");
    if key.chars().count() <= 72 && !key.contains('\n') {
        key
    } else {
        let head: String = key.chars().take(48).collect();
        format!("{head}…#{:08x}", fnv(&key))
    }
}

pub fn inspect(host: &dyn Host) -> View {
    let (screen, ctx) = (host.screen(), host.ctx());
    let scan = probe::scan(
        &|| page(screen, ctx),
        Size::new(PROBE_WIDTH, PROBE_HEIGHT),
    );
    let mut controls: Vec<Control> = Vec::new();
    for (t, msgs) in scan.texts.iter().zip(scan.clicks) {
        if msgs.is_empty() {
            continue;
        }
        let id = control_id(&msgs);
        match controls.iter_mut().find(|c| c.id == id) {
            Some(c) => c.label.push(t.text.clone()),
            None => controls.push(Control {
                id,
                label: vec![t.text.clone()],
                msgs,
            }),
        }
    }
    View {
        texts: scan.texts,
        inputs: scan.inputs,
        controls,
        content_height: scan.content_height,
    }
}

// ----------------------------------------------------------------- state

fn progress_json(ctx: &Ctx) -> Value {
    let s = &ctx.state;
    let today = ctx.today();
    json!({
        "xp_total": s.xp_total,
        "xp_today": crate::game::today_xp(s, today),
        "daily_goal": crate::game::goal(s),
        "streak": crate::game::streak(s, today),
        "freezes": s.freezes,
        "lessons_read": s.lessons_read.len(),
        "cards_seen": s.cards.len(),
        "questions_seen": s.questions.len(),
        "exams_taken": s.exams.len(),
        "badges": s.badges.len(),
        "best": s.best,
    })
}

fn settings_json(ctx: &Ctx) -> Value {
    let s = &ctx.state.settings;
    json!({
        "level": s.level.as_ref().map(|l| format!("{l:?}").to_lowercase()),
        "theme": s.theme,
        "font_step": s.font_step,
        "translit": s.translit,
        "unlock_all": s.unlock_all,
        "timed_seconds": s.timed_seconds,
        "timed_sound": s.timed_sound,
        "timed_auto": s.timed_auto,
        "daily_goal": s.daily_goal,
    })
}

fn brief(host: &dyn Host) -> Value {
    let ctx = host.ctx();
    json!({
        "screen": screen_name(host.screen()),
        "route": route_str(host.route()),
        "dialog": host.screen().dialog(ctx).is_some(),
        "toasts": ctx.toasts.iter().map(|t| json!({"title": t.title, "text": t.text})).collect::<Vec<_>>(),
    })
}

fn state_json(host: &dyn Host, with_texts: bool) -> Value {
    let view = inspect(host);
    let ctx = host.ctx();
    let mut v = brief(host);
    let o = v.as_object_mut().expect("object");
    o.insert("onboarded".into(), json!(ctx.state.onboarded));
    o.insert("today".into(), json!(ctx.today_key()));
    if with_texts {
        o.insert(
            "texts".into(),
            json!(view.texts.iter().map(|t| t.text.clone()).collect::<Vec<_>>()),
        );
    }
    o.insert(
        "controls".into(),
        json!(view
            .controls
            .iter()
            .map(|c| json!({"id": c.id, "label": c.label.join(" · ")}))
            .collect::<Vec<_>>()),
    );
    o.insert(
        "inputs".into(),
        json!(view
            .inputs
            .iter()
            .enumerate()
            .map(|(i, f)| json!({"index": i, "kind": f.kind, "text": f.text}))
            .collect::<Vec<_>>()),
    );
    o.insert("progress".into(), progress_json(ctx));
    o.insert("settings".into(), settings_json(ctx));
    v
}

fn screens_json(ctx: &Ctx) -> Value {
    let user = crate::content::Library::user_level(&ctx.state);
    let sections: Vec<Value> = ctx
        .lib
        .sections
        .iter()
        .map(|s| {
            let lessons: Vec<Value> = s
                .data
                .as_ref()
                .map(|d| {
                    d.lessons
                        .iter()
                        .filter(|l| crate::content::Library::visible(l.min_level, None, user))
                        .map(|l| json!({"id": l.id, "title": l.title}))
                        .collect()
                })
                .unwrap_or_default();
            json!({
                "id": s.id,
                "title": s.title,
                "available": s.available,
                "unlocked": ctx.lib.is_unlocked(s, &ctx.state),
                "lessons": lessons,
            })
        })
        .collect();
    json!({
        "nav": NAV.iter().map(|(l, r)| json!({"label": l, "route": r})).collect::<Vec<_>>(),
        "routes": ["welcome", "level", "map", "calendar", "calendar:today", "calendar:month", "calendar:settings", "calendar:YYYY-MM-DD", "section:<id>", "lesson:<id>", "cards:<lessonId>", "deck:<sectionId>", "cards:review", "exam", "exam:<sectionId>", "examrun:section:<id>[:n]", "examrun:general[:n]", "examrun:beitdin[:n]", "timed", "timedrun", "progress", "settings", "sources"],
        "sections": sections,
    })
}

// -------------------------------------------------------------- commands

fn arg_str<'v>(a: &'v Map<String, Value>, k: &str) -> Option<&'v str> {
    a.get(k).and_then(Value::as_str)
}

fn arg_bool(a: &Map<String, Value>, k: &str) -> Option<bool> {
    match a.get(k)? {
        Value::Bool(b) => Some(*b),
        Value::String(s) => match s.as_str() {
            "true" | "1" | "yes" | "on" => Some(true),
            "false" | "0" | "no" | "off" => Some(false),
            _ => None,
        },
        Value::Number(n) => Some(n.as_i64() != Some(0)),
        _ => None,
    }
}

fn arg_num(a: &Map<String, Value>, k: &str) -> Option<f64> {
    match a.get(k)? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

fn find_control<'v>(view: &'v View, a: &Map<String, Value>) -> Result<&'v Control, String> {
    let nth = arg_num(a, "nth").unwrap_or(0.0) as usize;
    let matches: Vec<&Control> = if let Some(id) = arg_str(a, "id") {
        view.controls.iter().filter(|c| c.id == id).collect()
    } else if let Some(l) = arg_str(a, "label") {
        let l = l.trim().to_lowercase();
        view.controls
            .iter()
            .filter(|c| c.label.iter().any(|t| t.to_lowercase() == l))
            .collect()
    } else if let Some(s) = arg_str(a, "contains") {
        let s = s.to_lowercase();
        view.controls
            .iter()
            .filter(|c| {
                c.label.iter().any(|t| t.to_lowercase().contains(&s))
                    || c.id.to_lowercase().contains(&s)
            })
            .collect()
    } else {
        return Err("press needs id, label or contains".into());
    };
    matches.get(nth).copied().ok_or_else(|| {
        let avail: Vec<String> = view
            .controls
            .iter()
            .map(|c| c.label.join(" · "))
            .collect();
        format!(
            "no such control (found {} matches); available: {}",
            matches.len(),
            avail.join(" | ")
        )
    })
}

fn has_control(view: &View, id: &str) -> bool {
    view.controls.iter().any(|c| c.id == id)
}

fn answer(host: &mut dyn Host, a: &Map<String, Value>) -> Result<(), String> {
    use screens::Msg as S;
    let screen = screen_name(host.screen());
    let mut msgs: Vec<S> = Vec::new();
    if let Some(c) = arg_num(a, "choice") {
        let i = c as usize;
        msgs.push(match screen {
            "exam" => S::Exam(screens::exam::Msg::PickMc(i)),
            "timed" => S::Timed(screens::timed::Msg::Pick(i)),
            _ => return Err(format!("choice: no question on screen «{screen}»")),
        });
    } else if let Some(b) = arg_bool(a, "tf") {
        if screen != "exam" {
            return Err(format!("tf: no true/false question on screen «{screen}»"));
        }
        msgs.push(S::Exam(screens::exam::Msg::PickTf(b)));
    } else if let Some(t) = arg_str(a, "text") {
        if screen != "exam" {
            return Err(format!("text: no typed question on screen «{screen}»"));
        }
        msgs.push(S::Exam(screens::exam::Msg::Typed(t.to_string())));
        msgs.push(S::Exam(screens::exam::Msg::TypedSubmit));
    } else if let Some(knew) = arg_bool(a, "knew") {
        if screen != "cards" {
            return Err(format!("knew: no flashcard on screen «{screen}»"));
        }
        // Grade buttons only work on a flipped card; flip first if needed.
        let view = inspect(host);
        let grade_id = control_id(&[S::Cards(screens::cards::Msg::Grade(knew))]);
        if !has_control(&view, &grade_id) {
            msgs.push(S::Cards(screens::cards::Msg::Flip));
        }
        msgs.push(S::Cards(screens::cards::Msg::Grade(knew)));
    } else if let Some(g) = arg_num(a, "self_grade") {
        if screen != "exam" {
            return Err(format!("self_grade: no open question on screen «{screen}»"));
        }
        msgs.push(S::Exam(screens::exam::Msg::OpenReveal));
        msgs.push(S::Exam(screens::exam::Msg::SelfGrade(g.clamp(0.0, 1.0))));
    } else if !arg_bool(a, "next").unwrap_or(false) {
        return Err("answer needs choice, tf, text, knew or self_grade".into());
    }
    if arg_bool(a, "next").unwrap_or(false) {
        msgs.push(match screen {
            "exam" => S::Exam(screens::exam::Msg::Next),
            "timed" => S::Timed(screens::timed::Msg::Next),
            _ => return Err(format!("next: nothing to advance on «{screen}»")),
        });
    }
    for m in msgs {
        host.dispatch(m);
    }
    Ok(())
}

fn set_setting(host: &mut dyn Host, a: &Map<String, Value>) -> Result<(), String> {
    let key = arg_str(a, "key").ok_or("set needs key")?.to_string();
    let val = a.get("value").ok_or("set needs value")?.clone();
    let mut tmp = Map::new();
    tmp.insert("v".into(), val.clone());
    let b = || arg_bool(&tmp, "v").ok_or_else(|| format!("{key}: expected true/false"));
    let n = || arg_num(&tmp, "v").ok_or_else(|| format!("{key}: expected a number"));
    {
        let s = &mut host.ctx_mut().state.settings;
        match key.as_str() {
            "theme" => {
                let t = val.as_str().unwrap_or_default();
                if !["dark", "parchment"].contains(&t) {
                    return Err("theme: dark | parchment".into());
                }
                s.theme = t.to_string();
            }
            "font_step" => s.font_step = (n()? as i64).clamp(0, 3),
            "translit" => s.translit = b()?,
            "unlock_all" => s.unlock_all = b()?,
            "timed_sound" => s.timed_sound = b()?,
            "timed_auto" => s.timed_auto = b()?,
            "timed_seconds" => s.timed_seconds = n()? as i64,
            "daily_goal" => s.daily_goal = n()? as i64,
            _ => return Err(format!("unknown setting «{key}»")),
        }
    }
    host.ctx_mut().save();
    host.settings_changed();
    Ok(())
}

fn input_steps(view: &View, a: &Map<String, Value>) -> Result<Vec<probe::Step>, String> {
    let text = arg_str(a, "text").ok_or("type needs text")?;
    let ix = arg_num(a, "input").unwrap_or(0.0) as usize;
    let field = view.inputs.get(ix).ok_or_else(|| {
        format!(
            "no input #{ix} on this screen ({} inputs)",
            view.inputs.len()
        )
    })?;
    let mut steps = vec![probe::Step::Click(field.bounds.center())];
    if !arg_bool(a, "append").unwrap_or(false) {
        steps.push(probe::Step::SelectAll);
    }
    steps.push(probe::Step::Type(text.to_string()));
    if arg_bool(a, "submit").unwrap_or(false) {
        steps.push(probe::Step::Enter);
    }
    Ok(steps)
}

/// Run one command against `host`.
pub fn execute(host: &mut dyn Host, req: &Value) -> Outcome {
    let empty = Map::new();
    let a = req.as_object().unwrap_or(&empty);
    let Some(cmd) = arg_str(a, "cmd") else {
        return Outcome::err("missing \"cmd\"; try {\"cmd\":\"help\"}");
    };
    let after = |host: &mut dyn Host| {
        if arg_bool(a, "state").unwrap_or(false) {
            Outcome::ok(state_json(host, true))
        } else {
            Outcome::ok(brief(host))
        }
    };
    match cmd {
        "help" => Outcome::ok(json!({
            "commands": COMMANDS.iter().map(|(c, d)| json!({"cmd": c, "about": d})).collect::<Vec<_>>(),
        })),
        "screens" => Outcome::ok(screens_json(host.ctx())),
        "state" => Outcome::ok(state_json(host, arg_bool(a, "texts").unwrap_or(true))),
        "navigate" | "nav" | "go" => {
            let Some(to) = arg_str(a, "to").or_else(|| arg_str(a, "route")) else {
                return Outcome::err("navigate needs to");
            };
            let Some((route, follow)) = parse_route(to) else {
                return Outcome::err(format!("unknown route «{to}»; see {{\"cmd\":\"screens\"}}"));
            };
            host.navigate(route);
            if let Some(m) = follow {
                if matches!(host.screen(), Current::Calendar(_)) {
                    host.dispatch(screens::Msg::Calendar(m));
                }
            }
            after(host)
        }
        "back" => {
            host.back();
            after(host)
        }
        "press" | "click" => {
            let view = inspect(host);
            match find_control(&view, a) {
                Ok(c) => {
                    let (id, msgs) = (c.id.clone(), c.msgs.clone());
                    for m in msgs {
                        host.dispatch(m);
                    }
                    let mut o = after(host);
                    o.reply["pressed"] = json!(id);
                    o
                }
                Err(e) => Outcome::err(e),
            }
        }
        "answer" => match answer(host, a) {
            Ok(()) => after(host),
            Err(e) => Outcome::err(e),
        },
        "type" => {
            let view = inspect(host);
            let steps = match input_steps(&view, a) {
                Ok(s) => s,
                Err(e) => return Outcome::err(e),
            };
            let msgs = {
                let (screen, ctx) = (host.screen(), host.ctx());
                probe::interact(
                    page(screen, ctx),
                    Size::new(PROBE_WIDTH, PROBE_HEIGHT),
                    &steps,
                )
            };
            if msgs.is_empty() {
                return Outcome::err("the field published nothing (read-only or not editable)");
            }
            let n = msgs.len();
            for m in msgs {
                host.dispatch(m);
            }
            let mut o = after(host);
            o.reply["messages"] = json!(n);
            o
        }
        "select" => {
            let (Some(target), Some(option)) = (arg_str(a, "target"), arg_str(a, "option")) else {
                return Outcome::err("select needs target and option");
            };
            let view = inspect(host);
            let want = target.to_lowercase();
            let Some(t) = view
                .texts
                .iter()
                .find(|t| t.text.to_lowercase() == want)
                .or_else(|| view.texts.iter().find(|t| t.text.to_lowercase().contains(&want)))
            else {
                return Outcome::err(format!("no text «{target}» on this screen"));
            };
            let at = t.bounds.center();
            let res = {
                let (screen, ctx) = (host.screen(), host.ctx());
                probe::click_then_pick(
                    page(screen, ctx),
                    Size::new(PROBE_WIDTH, PROBE_HEIGHT),
                    at,
                    option,
                )
            };
            match res {
                Ok(msgs) if !msgs.is_empty() => {
                    for m in msgs {
                        host.dispatch(m);
                    }
                    after(host)
                }
                Ok(_) => Outcome::err("picking the option published nothing"),
                Err(e) => Outcome::err(e),
            }
        }
        "set" => match set_setting(host, a) {
            Ok(()) => {
                let mut o = after(host);
                o.reply["settings"] = settings_json(host.ctx());
                o
            }
            Err(e) => Outcome::err(e),
        },
        "screenshot" => {
            let path = arg_str(a, "path")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    std::env::temp_dir().join(format!(
                        "shorashim-{}.webp",
                        chrono::Local::now().format("%Y%m%d-%H%M%S%.3f")
                    ))
                });
            let path = if path.extension().and_then(|e| e.to_str()) == Some("webp") {
                path
            } else {
                path.with_extension("webp")
            };
            let size = match (arg_num(a, "width"), arg_num(a, "height")) {
                (Some(w), Some(h)) => Some(Size::new(w as f32, h as f32)),
                _ => None,
            };
            let mut o = Outcome::ok(json!({"path": path}));
            o.effect = Some(Effect::Screenshot {
                path,
                full_page: arg_bool(a, "full_page").unwrap_or(false),
                size,
            });
            o
        }
        "scroll" => {
            if !host.windowed() {
                return Outcome::err("scroll works in the windowed app only");
            }
            let to = match a.get("to") {
                Some(Value::String(s)) if s == "top" => 0.0,
                Some(Value::String(s)) if s == "bottom" => 1.0,
                Some(v) => match v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())) {
                    Some(f) => f.clamp(0.0, 1.0) as f32,
                    None => return Outcome::err("scroll: to = top | bottom | 0..1"),
                },
                None => 0.0,
            };
            let mut o = Outcome::ok(json!({"scrolled": to}));
            o.effect = Some(Effect::Scroll(to));
            o
        }
        "quit" | "exit" => {
            let mut o = Outcome::ok(json!({"bye": true}));
            o.effect = Some(Effect::Quit);
            o
        }
        other => Outcome::err(format!("unknown command «{other}»; try {{\"cmd\":\"help\"}}")),
    }
}

/// Render the current page headlessly to a WebP file.
pub fn headless_screenshot(
    host: &dyn Host,
    path: &std::path::Path,
    full_page: bool,
    size: Option<Size>,
) -> Result<(u32, u32), String> {
    let (screen, ctx) = (host.screen(), host.ctx());
    let mut size = size.unwrap_or(Size::new(PROBE_WIDTH, 820.0));
    if full_page {
        let view = inspect(host);
        if let Some(h) = view.content_height {
            size.height = (h + 4.0).max(size.height);
        }
    }
    let scale = {
        const STEPS: [f32; 4] = [15.0, 17.0, 19.0, 21.5];
        STEPS[ctx.state.settings.font_step.clamp(0, 3) as usize] / 17.0
    };
    let kind = theme_kind(ctx);
    let p = theme::current();
    let (rgba, w, h) = probe::render(
        page(screen, ctx),
        size,
        scale,
        &theme::iced_theme(kind),
        p.bg0,
    );
    probe::write_webp(path, &rgba, w, h)?;
    Ok((w, h))
}
