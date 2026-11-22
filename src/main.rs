#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! Giyur Study — native desktop app (iced).

use std::time::{Duration, Instant};

use iced::widget::{button, column, container, image, row, scrollable, stack, text, Column};
use iced::{Alignment, Element, Font, Length, Subscription, Task, Theme};

use giyur_study::ui::app::{Action, Ctx, Deck, Route};
use giyur_study::ui::screens::{self, Current};
use giyur_study::ui::theme::{self, ThemeKind};
use giyur_study::ui::{assets, fonts, widgets};

const MAIN_SCROLL: &str = "main-scroll";

struct App {
    ctx: Ctx,
    route: Route,
    history: Vec<Route>,
    screen: Current,
}

#[derive(Debug, Clone)]
enum Message {
    Screen(screens::Msg),
    Nav(Route),
    Tick(Instant),
    FontLoaded,
}

fn theme_kind(ctx: &Ctx) -> ThemeKind {
    if ctx.state.settings.theme == "parchment" {
        ThemeKind::Parchment
    } else {
        ThemeKind::Dark
    }
}

impl App {
    fn new() -> (App, Task<Message>) {
        let mut ctx = Ctx::load();
        theme::set_current(theme_kind(&ctx));
        let route = if ctx.state.onboarded {
            std::env::var("GIYUR_START_ROUTE")
                .ok()
                .and_then(|r| parse_route(&r))
                .unwrap_or(Route::Map)
        } else {
            Route::Welcome
        };
        let screen = Current::open(&mut ctx, &route);
        let fonts = Task::batch(
            fonts::all()
                .into_iter()
                .map(|f| iced::font::load(f).map(|_| Message::FontLoaded)),
        );
        (
            App {
                ctx,
                route,
                history: Vec::new(),
                screen,
            },
            fonts,
        )
    }

    fn go(&mut self, route: Route) -> Task<Message> {
        // First-launch guard (PARITY §0.1).
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
        let keep_scroll = matches!(route, Route::ExamRun(_) | Route::TimedRun { .. });
        self.route = route;
        if keep_scroll {
            Task::none()
        } else {
            iced::widget::operation::snap_to(
                MAIN_SCROLL,
                iced::widget::operation::RelativeOffset::START,
            )
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Screen(m) => match self.screen.update(&mut self.ctx, m) {
                Action::None => {}
                Action::Go(r) => return self.go(r),
                Action::Back => {
                    let r = self.history.pop().unwrap_or(Route::Map);
                    self.screen = Current::open(&mut self.ctx, &r);
                    self.route = r;
                }
                Action::SettingsChanged => {
                    theme::set_current(theme_kind(&self.ctx));
                }
            },
            Message::Nav(r) => return self.go(r),
            Message::Tick(now) => self.ctx.toasts.retain(|t| t.until > now),
            Message::FontLoaded => {}
        }
        Task::none()
    }

    fn sidebar(&self) -> Element<'_, Message> {
        let p = theme::current();
        let items: [(&str, Route); 7] = [
            ("Карта", Route::Map),
            ("Повторение", Route::Cards(Deck::Review)),
            ("Экзамен", Route::Timed),
            ("Тесты", Route::Exam(None)),
            ("Прогресс", Route::Progress),
            ("Настройки", Route::Settings),
            ("Источники", Route::Sources),
        ];
        let active = nav_group(&self.route);
        let mut col = Column::new().spacing(4).padding([18, 12]).width(230);
        let brand = button(
            row![
                image(
                    assets::img("app-icon").unwrap_or_else(|| image::Handle::from_rgba(
                        1,
                        1,
                        vec![0; 4]
                    ))
                )
                .width(36)
                .height(36),
                column![
                    text("Giyur Study").font(fonts::SERIF_BOLD).size(18),
                    text("подготовка к гиюру").size(12).color(p.muted),
                ]
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .style(move |_, s| theme::button_ghost(&p, s))
        .on_press(Message::Nav(Route::Map));
        col = col.push(brand).push(iced::widget::space().height(14));
        for (i, (label, route)) in items.into_iter().enumerate() {
            let is_active = active == Some(i);
            let b = button(text(label).size(15).shaping(text::Shaping::Advanced))
                .width(Length::Fill)
                .padding([10, 14])
                .style(move |_, s| {
                    if is_active {
                        theme::button_nav_active(&p, s)
                    } else {
                        theme::button_ghost(&p, s)
                    }
                })
                .on_press(Message::Nav(route));
            col = col.push(b);
        }
        container(col)
            .height(Length::Fill)
            .style(move |_| theme::sidebar(&p))
            .into()
    }

    fn toasts(&self) -> Element<'_, Message> {
        let mut col = Column::new().spacing(8).align_x(Alignment::Center);
        if let Some(t) = self.ctx.toasts.first() {
            let mut r = row![].spacing(12).align_y(Alignment::Center);
            if let Some(h) = t
                .badge_id
                .as_ref()
                .and_then(|id| assets::img(&format!("badge-{id}")))
            {
                r = r.push(image(h).width(48).height(48));
            }
            r = r.push(column![
                text(&t.title)
                    .font(fonts::SERIF_BOLD)
                    .size(17)
                    .color(theme::current().gold2)
                    .shaping(text::Shaping::Advanced),
                text(&t.text)
                    .size(13)
                    .color(theme::current().muted)
                    .shaping(text::Shaping::Advanced),
            ]);
            col = col.push(widgets::card(r));
        }
        container(col)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::End)
            .padding(24)
            .into()
    }

    fn view(&self) -> Element<'_, Message> {
        let body: Element<'_, Message> = self.screen.view(&self.ctx).map(Message::Screen);
        let main = container(
            scrollable(
                container(body)
                    .padding([28, 32])
                    .max_width(980)
                    .center_x(Length::Fill),
            )
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill);
        let content: Element<'_, Message> = if self.screen.bare(&self.ctx) {
            main.into()
        } else {
            row![self.sidebar(), main].into()
        };
        let base = container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| theme::app_background(&theme::current()));
        // Always the same root widget, so scroll position and focus survive
        // toasts appearing and expiring.
        stack![base, self.toasts()].into()
    }

    fn subscription(&self) -> Subscription<Message> {
        let screen = self.screen.subscription().map(Message::Screen);
        if self.ctx.toasts.is_empty() {
            screen
        } else {
            Subscription::batch([
                screen,
                iced::time::every(Duration::from_millis(250)).map(Message::Tick),
            ])
        }
    }

    fn theme(&self) -> Theme {
        theme::iced_theme(theme_kind(&self.ctx))
    }

    fn title(&self) -> String {
        "Giyur Study — подготовка к гиюру".to_string()
    }
}

/// Debug/screenshot hook: `GIYUR_START_ROUTE=section:shabbat` etc.
fn parse_route(r: &str) -> Option<Route> {
    let (k, v) = r.split_once(':').unwrap_or((r, ""));
    let v = v.to_string();
    Some(match k {
        "map" => Route::Map,
        "level" => Route::Level { reopened: true },
        "section" => Route::Section(v),
        "lesson" => Route::Lesson(v),
        "cards" if v == "review" => Route::Cards(Deck::Review),
        "cards" => Route::Cards(Deck::Lesson(v)),
        "deck" => Route::Cards(Deck::Section(v)),
        "exam" => Route::Exam(if v.is_empty() { None } else { Some(v) }),
        "examrun" => Route::ExamRun(giyur_study::ui::app::ExamSpec::Section(v)),
        "timed" => Route::Timed,
        "timedrun" => Route::TimedRun { retry_ids: None },
        "progress" => Route::Progress,
        "settings" => Route::Settings,
        "sources" => Route::Sources,
        _ => return None,
    })
}

/// Which sidebar item is highlighted for a route.
fn nav_group(r: &Route) -> Option<usize> {
    Some(match r {
        Route::Map | Route::Section(_) | Route::Lesson(_) => 0,
        Route::Cards(Deck::Review) => 1,
        Route::Cards(_) => 0,
        Route::Timed | Route::TimedRun { .. } => 2,
        Route::Exam(_) | Route::ExamRun(_) => 3,
        Route::Progress => 4,
        Route::Settings | Route::Level { .. } => 5,
        Route::Sources => 6,
        Route::Welcome => return None,
    })
}

fn main() -> iced::Result {
    let mut settings = iced::window::Settings {
        size: iced::Size::new(1200.0, 820.0),
        min_size: Some(iced::Size::new(720.0, 560.0)),
        icon: assets::window_icon(),
        ..Default::default()
    };
    settings.exit_on_close_request = true;
    iced::application(App::new, App::update, App::view)
        .title(App::title)
        .theme(App::theme)
        .subscription(App::subscription)
        // «Размер шрифта» A− / A / A+ / A++ (PWA root sizes 15/17/19/21.5 px).
        .scale_factor(|app: &App| {
            const STEPS: [f32; 4] = [15.0, 17.0, 19.0, 21.5];
            STEPS[app.ctx.state.settings.font_step.clamp(0, 3) as usize] / 17.0
        })
        .window(settings)
        .default_font(Font::with_name("Noto Sans"))
        .run()
}
