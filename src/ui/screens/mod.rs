//! All screens and the dispatch enum used by the shell.
pub mod calendar;
pub mod cards;
pub mod exam;
pub mod lesson;
pub mod level;
pub mod map;
pub mod progress;
pub mod section;
pub mod settings;
pub mod sources;
pub mod timed;
pub mod welcome;

use iced::{Element, Subscription};

use crate::ui::app::{Action, Ctx, Route};

/// The live screen for the current route.
pub enum Current {
    Welcome(welcome::Screen),
    Level(level::Screen),
    Map(map::Screen),
    Calendar(calendar::Screen),
    Section(section::Screen),
    Sources(sources::Screen),
    Lesson(lesson::Screen),
    Cards(Box<cards::Screen>),
    Exam(exam::Screen),
    Timed(timed::Screen),
    Progress(progress::Screen),
    Settings(settings::Screen),
}

#[derive(Debug, Clone)]
pub enum Msg {
    Welcome(welcome::Msg),
    Level(level::Msg),
    Map(map::Msg),
    Calendar(calendar::Msg),
    Section(section::Msg),
    Sources(sources::Msg),
    Lesson(lesson::Msg),
    Cards(cards::Msg),
    Exam(exam::Msg),
    Timed(timed::Msg),
    Progress(progress::Msg),
    Settings(settings::Msg),
}

impl Current {
    pub fn open(ctx: &mut Ctx, route: &Route) -> Current {
        match route {
            Route::Welcome => Current::Welcome(welcome::Screen::new(ctx, route)),
            Route::Level { .. } => Current::Level(level::Screen::new(ctx, route)),
            Route::Map => Current::Map(map::Screen::new(ctx, route)),
            Route::Calendar => Current::Calendar(calendar::Screen::new(ctx, route)),
            Route::Section(_) => Current::Section(section::Screen::new(ctx, route)),
            Route::Sources => Current::Sources(sources::Screen::new(ctx, route)),
            Route::Lesson(_) => Current::Lesson(lesson::Screen::new(ctx, route)),
            Route::Cards(_) => Current::Cards(Box::new(cards::Screen::new(ctx, route))),
            Route::Exam(_) | Route::ExamRun(_) => Current::Exam(exam::Screen::new(ctx, route)),
            Route::Timed | Route::TimedRun { .. } => Current::Timed(timed::Screen::new(ctx, route)),
            Route::Progress => Current::Progress(progress::Screen::new(ctx, route)),
            Route::Settings => Current::Settings(settings::Screen::new(ctx, route)),
        }
    }

    pub fn update(&mut self, ctx: &mut Ctx, msg: Msg) -> Action {
        match (self, msg) {
            (Current::Welcome(s), Msg::Welcome(m)) => s.update(ctx, m),
            (Current::Level(s), Msg::Level(m)) => s.update(ctx, m),
            (Current::Map(s), Msg::Map(m)) => s.update(ctx, m),
            (Current::Calendar(s), Msg::Calendar(m)) => s.update(ctx, m),
            (Current::Section(s), Msg::Section(m)) => s.update(ctx, m),
            (Current::Sources(s), Msg::Sources(m)) => s.update(ctx, m),
            (Current::Lesson(s), Msg::Lesson(m)) => s.update(ctx, m),
            (Current::Cards(s), Msg::Cards(m)) => s.update(ctx, m),
            (Current::Exam(s), Msg::Exam(m)) => s.update(ctx, m),
            (Current::Timed(s), Msg::Timed(m)) => s.update(ctx, m),
            (Current::Progress(s), Msg::Progress(m)) => s.update(ctx, m),
            (Current::Settings(s), Msg::Settings(m)) => s.update(ctx, m),
            _ => Action::None, // stale message from a previous screen
        }
    }

    pub fn view<'a>(&'a self, ctx: &'a Ctx) -> Element<'a, Msg> {
        match self {
            Current::Welcome(s) => s.view(ctx).map(Msg::Welcome),
            Current::Level(s) => s.view(ctx).map(Msg::Level),
            Current::Map(s) => s.view(ctx).map(Msg::Map),
            Current::Calendar(s) => s.view(ctx).map(Msg::Calendar),
            Current::Section(s) => s.view(ctx).map(Msg::Section),
            Current::Sources(s) => s.view(ctx).map(Msg::Sources),
            Current::Lesson(s) => s.view(ctx).map(Msg::Lesson),
            Current::Cards(s) => s.view(ctx).map(Msg::Cards),
            Current::Exam(s) => s.view(ctx).map(Msg::Exam),
            Current::Timed(s) => s.view(ctx).map(Msg::Timed),
            Current::Progress(s) => s.view(ctx).map(Msg::Progress),
            Current::Settings(s) => s.view(ctx).map(Msg::Settings),
        }
    }

    /// The screen's modal dialog layer, if any (rendered by the shell above
    /// the whole window, so it centres on the window, not the page).
    pub fn dialog<'a>(&'a self, ctx: &'a Ctx) -> Option<Element<'a, Msg>> {
        match self {
            Current::Level(s) => s.dialog(ctx).map(|d| d.map(Msg::Level)),
            Current::Settings(s) => s.dialog(ctx).map(|d| d.map(Msg::Settings)),
            Current::Exam(s) => s.dialog(ctx).map(|d| d.map(Msg::Exam)),
            Current::Timed(s) => s.dialog(ctx).map(|d| d.map(Msg::Timed)),
            _ => None,
        }
    }

    pub fn subscription(&self) -> Subscription<Msg> {
        match self {
            Current::Welcome(s) => s.subscription().map(Msg::Welcome),
            Current::Level(s) => s.subscription().map(Msg::Level),
            Current::Map(s) => s.subscription().map(Msg::Map),
            Current::Calendar(s) => s.subscription().map(Msg::Calendar),
            Current::Section(s) => s.subscription().map(Msg::Section),
            Current::Sources(s) => s.subscription().map(Msg::Sources),
            Current::Lesson(s) => s.subscription().map(Msg::Lesson),
            Current::Cards(s) => s.subscription().map(Msg::Cards),
            Current::Exam(s) => s.subscription().map(Msg::Exam),
            Current::Timed(s) => s.subscription().map(Msg::Timed),
            Current::Progress(s) => s.subscription().map(Msg::Progress),
            Current::Settings(s) => s.subscription().map(Msg::Settings),
        }
    }

    /// Called before the screen is replaced by another route.
    pub fn leave(&mut self, ctx: &mut Ctx) {
        if let Current::Timed(s) = self {
            s.leave(ctx);
        }
    }

    /// Screens where the sidebar is hidden (first-run welcome / level choice).
    pub fn bare(&self, ctx: &Ctx) -> bool {
        match self {
            Current::Welcome(_) => true,
            Current::Level(_) => !ctx.state.onboarded,
            _ => false,
        }
    }
}
