#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! Шорашим (Shorashim) — native desktop app (iced).

use std::time::{Duration, Instant};

use iced::widget::{button, column, container, image, row, scrollable, stack, text, Column};
use iced::{Alignment, Element, Font, Length, Subscription, Task, Theme};

use shorashim::ui::app::{Action, Ctx, Deck, Route};
use shorashim::ui::screens::{self, Current};
use shorashim::ui::theme::{self, ThemeKind};
use shorashim::ui::{assets, fonts, layout, widgets};

const MAIN_SCROLL: &str = "main-scroll";
const ASIDE_SCROLL: &str = "aside-scroll";

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
    /// Screenshot hook `SHORASHIM_SCROLL_Y=<px>`: scroll the main column.
    ScrollTo(f32),
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
        // A newer portable exe than the installed copy: point to Settings.
        if let shorashim::install::Offer::Update { from, to } = shorashim::install::status().offer()
        {
            ctx.toast(
                &format!("Установлена версия {from}"),
                &format!("Эта копия новее ({to}). Обновить установленную можно в «Настройках»."),
                "",
            );
        }
        theme::set_current(theme_kind(&ctx));
        let route = if ctx.state.onboarded {
            shorashim::util::env_hook("START_ROUTE")
                .and_then(|r| parse_route(&r))
                .unwrap_or(Route::Map)
        } else {
            Route::Welcome
        };
        let screen = Current::open(&mut ctx, &route);
        (
            App {
                ctx,
                route,
                history: Vec::new(),
                screen,
            },
            // Screenshot hook: scroll once the first frame is laid out.
            match shorashim::util::env_hook("SCROLL_Y").and_then(|v| v.parse::<f32>().ok()) {
                Some(y) => Task::perform(
                    async { std::thread::sleep(Duration::from_millis(1500)) },
                    move |_| Message::ScrollTo(y),
                ),
                None => Task::none(),
            },
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
                Action::Quit => return iced::exit(),
            },
            Message::Nav(r) => return self.go(r),
            Message::Tick(now) => self.ctx.toasts.retain(|t| t.until > now),
            Message::ScrollTo(y) => {
                return iced::widget::operation::scroll_to(
                    MAIN_SCROLL,
                    iced::widget::operation::AbsoluteOffset {
                        x: None,
                        y: Some(y),
                    },
                )
            }
        }
        Task::none()
    }

    fn sidebar(&self) -> Element<'_, Message> {
        let p = theme::current();
        let items: [(&str, Route); 8] = [
            ("Карта", Route::Map),
            ("Календарь", Route::Calendar),
            ("Повторение", Route::Cards(Deck::Review)),
            ("Маалот", Route::Timed),
            ("Тесты", Route::Exam(None)),
            ("Прогресс", Route::Progress),
            ("Настройки", Route::Settings),
            ("Источники", Route::Sources),
        ];
        let active = nav_group(&self.route);
        let mut col = Column::new()
            .spacing(2)
            .padding([28, 12])
            .width(layout::NAV);
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
                    text("Шорашим").font(fonts::SERIF_BOLD).size(18),
                    text("основы иудаизма").size(12).color(p.muted),
                ]
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding([8, 10])
        .style(move |_, s| theme::button_nav(&p, s))
        .on_press(Message::Nav(Route::Map));
        col = col.push(brand).push(iced::widget::space().height(14));
        for (i, (label, route)) in items.into_iter().enumerate() {
            let is_active = active == Some(i);
            // 3 px brass left bar on the active item (transparent strut else).
            let bar = container(iced::widget::space())
                .width(3)
                .height(20)
                .style(move |_| iced::widget::container::Style {
                    background: is_active.then_some(iced::Background::Color(p.gold)),
                    border: iced::Border {
                        radius: 2.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                });
            let b = button(
                container(
                    row![bar, text(label).size(15).shaping(text::Shaping::Advanced)]
                        .spacing(10)
                        .align_y(Alignment::Center),
                )
                .center_y(Length::Fill),
            )
            .width(Length::Fill)
            .height(theme::size::NAV)
            .padding([0, 12])
            .style(move |_, s| {
                if is_active {
                    theme::button_nav_active(&p, s)
                } else {
                    theme::button_nav(&p, s)
                }
            })
            .on_press(Message::Nav(route));
            col = col.push(b);
        }
        let panel = container(col)
            .width(layout::NAV)
            .height(Length::Fill)
            .style(move |_| theme::sidebar(&p));
        // Leather (dark) / darker klaf (parchment) under the side veil: the
        // sidebar reads as a different material than the window.
        match assets::img(p.side_texture) {
            Some(handle) => stack![
                image(handle)
                    .width(layout::NAV)
                    .height(Length::Fill)
                    .content_fit(iced::ContentFit::Fill)
                    .filter_method(image::FilterMethod::Linear),
                panel,
            ]
            .into(),
            None => panel.into(),
        }
    }

    /// The main grid for a window `width` (see `layout::compute`).
    fn grid(&self, width: f32, want_aside: bool, p: theme::Palette) -> Element<'_, Message> {
        let g = layout::compute(width, want_aside);
        let hairline = move || {
            container(iced::widget::space())
                .width(1)
                .height(Length::Fill)
                .style(move |_| theme::divider_v(&p))
        };
        let body = container(self.screen.view(&self.ctx).map(Message::Screen))
            .padding([28.0, layout::CONTENT_PAD])
            .width(g.content)
            .clip(true);
        // The scrollable spans everything right of the nav, so the wheel
        // works over the margins and the scrollbar sits at the window edge;
        // the aside is drawn on top, fixed, in the reserved space.
        let reserved = if g.aside { layout::ASIDE } else { 0.0 } + g.right;
        let scroll = scrollable(row![body, iced::widget::space().width(reserved)])
            .id(MAIN_SCROLL)
            .width(Length::Fill)
            .height(Length::Fill);
        // Keep the widget tree the same shape at every width (iced matches
        // widget state by position), so resizing across a breakpoint does
        // not reset the scroll position or input focus.
        let lead = if g.left > 0.0 { g.left - 1.0 } else { 0.0 };
        let lead_line: Element<'_, Message> = if g.left > 0.0 {
            hairline().into()
        } else {
            iced::widget::space().width(0).into()
        };
        let base = row![
            iced::widget::space().width(lead),
            lead_line,
            self.sidebar(),
            hairline(),
            scroll,
        ]
        .height(Length::Fill);
        let overlay: Element<'_, Message> = if g.aside {
            let aside = shorashim::ui::aside::panel(&self.ctx, Message::Nav(Route::Calendar));
            row![
                iced::widget::space().width(g.aside_x()),
                scrollable(aside).id(ASIDE_SCROLL).height(Length::Fill),
            ]
            .into()
        } else {
            iced::widget::space().into()
        };
        stack![base, overlay].into()
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
        let p = theme::current();
        let bare = self.screen.bare(&self.ctx);
        let content: Element<'_, Message> = if bare {
            // Shrink-height scrollable: centres when the content fits,
            // scrolls when it doesn't (welcome / first-run level choice).
            let inner = container(
                container(self.screen.view(&self.ctx).map(Message::Screen))
                    .padding([28.0, layout::CONTENT_PAD])
                    .width(Length::Fill)
                    .max_width(layout::content_col()),
            )
            .center_x(Length::Fill);
            container(scrollable(inner).height(Length::Shrink))
                .center(Length::Fill)
                .into()
        } else {
            // One grid for every screen (src/ui/layout.rs): the nav, the
            // reading column and the optional quote panel move together.
            let want_aside = shorashim::ui::aside::enabled(&self.ctx);
            iced::widget::responsive(move |size| self.grid(size.width, want_aside, p)).into()
        };
        // Window background: the theme texture under a solid veil and two
        // crossed lamp gradients; bg0 alone is the fallback (no texture).
        let mut bg = stack![container(iced::widget::space())
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| theme::app_background(&p))];
        if let Some(handle) = assets::img(p.texture) {
            bg = bg.push(
                image(handle)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .content_fit(iced::ContentFit::Fill)
                    .filter_method(image::FilterMethod::Linear),
            );
        }
        if p.veil.a > 0.0 {
            bg = bg.push(
                container(iced::widget::space())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(move |_| theme::veil(&p)),
            );
        }
        if p.lamp.a > 0.0 {
            bg = bg
                .push(
                    container(iced::widget::space())
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .style(move |_| theme::lamp_v(&p)),
                )
                .push(
                    container(iced::widget::space())
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .style(move |_| theme::lamp_h(&p)),
                );
        }
        if p.vignette.a > 0.0 {
            bg = bg.push(
                container(iced::widget::space())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(move |_| theme::vignette(&p)),
            );
        }
        bg = bg.push(content);

        // Always the same root widget, so scroll position and focus survive
        // toasts and dialogs appearing and expiring.
        let mut layers = stack![bg, self.toasts()];
        if let Some(d) = self.screen.dialog(&self.ctx) {
            layers = layers.push(iced::widget::opaque(d.map(Message::Screen)));
        }
        layers.into()
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
        "Шорашим — основы иудаизма и подготовка к гиюру".to_string()
    }
}

/// Debug/screenshot hook: `SHORASHIM_START_ROUTE=section:shabbat` etc.
fn parse_route(r: &str) -> Option<Route> {
    let (k, v) = r.split_once(':').unwrap_or((r, ""));
    let v = v.to_string();
    Some(match k {
        "map" => Route::Map,
        "calendar" => {
            // Screenshot hook: «calendar», «calendar:month», «calendar:settings»
            // or «calendar:YYYY-MM-DD»; the screen reads the env vars below.
            match v.as_str() {
                "today" | "month" | "settings" => std::env::set_var("SHORASHIM_CALENDAR_TAB", &v),
                d if chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").is_ok() => {
                    std::env::set_var("SHORASHIM_CALENDAR_DATE", d)
                }
                _ => {}
            }
            Route::Calendar
        }
        "level" => Route::Level { reopened: true },
        "section" => Route::Section(v),
        "lesson" => Route::Lesson(v),
        "cards" if v == "review" => Route::Cards(Deck::Review),
        "cards" => Route::Cards(Deck::Lesson(v)),
        "deck" => Route::Cards(Deck::Section(v)),
        "exam" => Route::Exam(if v.is_empty() { None } else { Some(v) }),
        "examrun" => Route::ExamRun(shorashim::ui::app::ExamSpec::Section { id: v, count: 20 }),
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
        Route::Calendar => 1,
        Route::Cards(Deck::Review) => 2,
        Route::Cards(_) => 0,
        Route::Timed | Route::TimedRun { .. } => 3,
        Route::Exam(_) | Route::ExamRun(_) => 4,
        Route::Progress => 5,
        Route::Settings | Route::Level { .. } => 6,
        Route::Sources => 7,
        Route::Welcome => return None,
    })
}

fn main() -> iced::Result {
    // «Приложения и возможности» runs `shorashim.exe --uninstall`.
    let args: Vec<String> = std::env::args().collect();
    if shorashim::install::handle_cli(&args) {
        return Ok(());
    }
    shorashim::install::cleanup_after_update();
    // Screenshot hook: `SHORASHIM_WINDOW_SIZE=1440x900`.
    let size = shorashim::util::env_hook("WINDOW_SIZE")
        .and_then(|v| {
            let (w, h) = v.split_once('x')?;
            Some(iced::Size::new(w.parse().ok()?, h.parse().ok()?))
        })
        .unwrap_or(iced::Size::new(1200.0, 820.0));
    let mut settings = iced::window::Settings {
        size,
        min_size: Some(iced::Size::new(720.0, 560.0)),
        icon: assets::window_icon(),
        ..Default::default()
    };
    settings.exit_on_close_request = true;
    let app = iced::application(App::new, App::update, App::view)
        .title(App::title)
        .theme(App::theme)
        .subscription(App::subscription)
        // «Размер шрифта» A− / A / A+ / A++ (PWA root sizes 15/17/19/21.5 px).
        .scale_factor(|app: &App| {
            const STEPS: [f32; 4] = [15.0, 17.0, 19.0, 21.5];
            STEPS[app.ctx.state.settings.font_step.clamp(0, 3) as usize] / 17.0
        })
        .window(settings)
        .default_font(Font::with_name("Noto Sans"));
    // Register the embedded fonts up front, so the very first frame already
    // uses them (no font-load race on the first paint).
    fonts::all()
        .into_iter()
        .fold(app, |app, font| app.font(font))
        .run()
}
