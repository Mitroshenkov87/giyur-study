#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! Шорашим (Shorashim) — native desktop app (iced).

use std::time::{Duration, Instant};

use iced::widget::{button, column, container, image, row, scrollable, stack, text, Column};
use iced::{Alignment, Element, Font, Length, Subscription, Task, Theme};

use shorashim::storage;
use shorashim::ui::app::{Action, Ctx, Deck, Route};
use shorashim::ui::screens::{self, Current};
use shorashim::ui::theme::{self, ThemeKind};
use shorashim::ui::{assets, fonts, layout, nav, widgets};

use shorashim::automation::server::{self as control, ControlMsg};

const MAIN_SCROLL: &str = "main-scroll";
const ASIDE_SCROLL: &str = "aside-scroll";

struct App {
    ctx: Ctx,
    route: Route,
    history: Vec<Route>,
    screen: Current,
    /// Sidebar item under the mouse (its 3D icon shows the hover frame).
    nav_hover: Option<usize>,
    music: shorashim::music::Controller,
}

#[derive(Debug, Clone)]
enum Message {
    Screen(screens::Msg),
    Nav(Route),
    Tick(Instant),
    /// Automation control channel (`--control`, docs/AUTOMATION.md).
    Control(ControlMsg),
    /// Screenshot hook `SHORASHIM_SCROLL_Y=<px>`: scroll the main column.
    ScrollTo(f32),
    /// Mouse entered (`true`) or left (`false`) a sidebar item.
    NavHover(usize, bool),
}

fn theme_kind(ctx: &Ctx) -> ThemeKind {
    let t = ctx.state.settings.theme.as_str();
    if t == storage::LIGHT_THEME || t == storage::LEGACY_LIGHT_THEME {
        ThemeKind::Leather
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
        let mut app = App {
            ctx,
            route,
            history: Vec::new(),
            screen,
            // Screenshot hook `SHORASHIM_NAV_HOVER=<index>`: show one
            // sidebar icon in its hover frame without a mouse.
            nav_hover: shorashim::util::env_hook("NAV_HOVER").and_then(|v| v.parse().ok()),
            music: shorashim::music::Controller::start(),
        };
        app.maybe_music_hint();
        app.sync_music();
        (
            app,
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
        // The sidebar may be rebuilt under the cursor; don't leave a stuck hover.
        self.nav_hover = None;
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
        self.screen.leave(&mut self.ctx);
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
            Message::NavHover(i, inside) => {
                if inside {
                    self.nav_hover = Some(i);
                } else if self.nav_hover == Some(i) {
                    self.nav_hover = None;
                }
            }
            Message::Screen(m) => match self.screen.update(&mut self.ctx, m) {
                Action::None => self.sync_music(),
                Action::Go(r) => return self.go(r),
                Action::Back => {
                    let r = self.history.pop().unwrap_or(Route::Map);
                    self.screen = Current::open(&mut self.ctx, &r);
                    self.route = r;
                }
                Action::SettingsChanged => {
                    theme::set_current(theme_kind(&self.ctx));
                    self.sync_music();
                }
                Action::Quit => {
                    self.music.set_enabled(false);
                    return iced::exit();
                }
                Action::MusicNext => {
                    self.music.next();
                    self.sync_music();
                }
            },
            Message::Nav(r) => return self.go(r),
            Message::Tick(now) => {
                self.ctx.toasts.retain(|t| t.until > now);
                self.sync_music();
            }
            Message::Control(ControlMsg::Request(req)) => {
                let mut host = GuiHost {
                    app: self,
                    tasks: Vec::new(),
                };
                let tasks = std::mem::take(&mut host.tasks);
                let (task, _) =
                    control::handle(&mut host, req, tasks, MAIN_SCROLL, Message::Control);
                return Task::batch([task, Task::batch(host.tasks)]);
            }
            Message::Control(ControlMsg::Done) => {}
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

    fn maybe_music_hint(&mut self) {
        if self.ctx.state.onboarded
            && !self.ctx.state.settings.music_hint_seen
            && self.ctx.music_available
        {
            self.ctx.toast(
                "Фоновая музыка",
                "В «Настройках» можно включить тихие инструментальные мелодии. По умолчанию она выключена.",
                "",
            );
            self.ctx.state.settings.music_hint_seen = true;
            self.ctx.save();
        }
    }

    fn sync_music(&mut self) {
        self.music
            .set_enabled(self.ctx.state.settings.music_enabled);
        self.music.set_volume(self.ctx.state.settings.music_volume);
        let st = self.music.status();
        self.ctx.music_title = st.title;
        self.ctx.music_error = st.error;
        if st.track_count > 0 {
            self.ctx.music_available = st.available;
            self.ctx.music_track_count = st.track_count;
        }
    }

    fn sidebar(&self) -> Element<'_, Message> {
        let p = theme::side_palette(theme::current());
        let active = nav::group(&self.route);
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
        let main_p = theme::current();
        for (i, item) in nav::ITEMS.iter().enumerate() {
            let is_active = active == Some(i);
            if i > 0 {
                // Thin divider between items, inset from the edges.
                col = col.push(
                    container(
                        container(iced::widget::space())
                            .width(Length::Fill)
                            .height(1)
                            .style(move |_| theme::nav_divider(&p)),
                    )
                    .padding([0, 12]),
                );
            }
            // 3 px brass left bar on the active item (transparent strut else).
            let bar = container(iced::widget::space())
                .width(3)
                .height(30)
                .style(move |_| iced::widget::container::Style {
                    background: is_active.then_some(iced::Background::Color(p.gold)),
                    border: iced::Border {
                        radius: 2.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                });
            // Pre-rendered 3D icon for the theme; the hover frame is the same
            // object turned 15° with a brighter key light (a cheap "tilt").
            let hovered = self.nav_hover == Some(i);
            let art_name = format!(
                "icon-{}-{}{}",
                item.art,
                theme::art_variant(),
                if hovered { "-hover" } else { "" }
            );
            // The hovered icon also grows a little inside a fixed box ("pop"),
            // so the row never shifts.
            let box_px = theme::size::NAV_ICON_HOVER;
            let px = if hovered {
                theme::size::NAV_ICON_HOVER
            } else {
                theme::size::NAV_ICON
            };
            let icon: Element<'_, Message> = match assets::art(&art_name) {
                Some(h) => container(
                    image(h)
                        .width(px)
                        .height(px)
                        .filter_method(image::FilterMethod::Linear),
                )
                .center_x(box_px)
                .center_y(box_px)
                .into(),
                None => iced::widget::space().width(box_px).height(box_px).into(),
            };
            let label = column![
                text(item.label).size(15).shaping(text::Shaping::Advanced),
                text(item.subtitle)
                    .size(11.5)
                    .color(p.muted)
                    .shaping(text::Shaping::Advanced),
            ]
            .spacing(1);
            let b = button(
                container(row![bar, icon, label].spacing(8).align_y(Alignment::Center))
                    .center_y(Length::Fill),
            )
            .width(Length::Fill)
            .height(theme::size::NAV_ITEM)
            .padding([0, 12])
            .style(move |_, s| {
                if is_active {
                    theme::button_nav_active(&p, s)
                } else {
                    theme::button_nav(&p, s)
                }
            })
            .on_press(Message::Nav(nav::route(i)));
            let tip = container(
                text(item.tooltip)
                    .size(13)
                    .color(main_p.text)
                    .shaping(text::Shaping::Advanced),
            )
            .padding([7, 11])
            .max_width(300);
            let b = iced::widget::mouse_area(b)
                .on_enter(Message::NavHover(i, true))
                .on_exit(Message::NavHover(i, false));
            col = col.push(
                iced::widget::tooltip(b, tip, iced::widget::tooltip::Position::Right)
                    .gap(6)
                    .style(move |_| theme::tooltip(&main_p)),
            );
        }
        let panel = container(col)
            .width(layout::NAV)
            .height(Length::Fill)
            .style(move |_| theme::sidebar(&p));
        // Leather (dark) / dark goatskin («Кожа») under the side veil: the
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
        // The scrollable spans everything right of the nav, so the wheel
        // works over the margins and the scrollbar sits at the window edge;
        // the aside is drawn on top, fixed, in the reserved space.
        let reserved = if g.aside { layout::ASIDE } else { 0.0 } + g.right;
        let page = layout::page(
            self.screen.view(&self.ctx).map(Message::Screen),
            g.content,
            reserved,
        );
        let scroll = scrollable(page)
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
        let screen = Subscription::batch([
            self.screen.subscription().map(Message::Screen),
            control::subscription(&self.ctx).map(Message::Control),
        ]);
        let tick = !self.ctx.toasts.is_empty()
            || matches!(self.route, Route::Settings)
            || self.ctx.state.settings.music_enabled;
        if tick {
            Subscription::batch([
                screen,
                iced::time::every(Duration::from_millis(250)).map(Message::Tick),
            ])
        } else {
            screen
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
        // A sidebar label, current or pre-v0.5 («Уроки», «Карта», …).
        _ => return nav::find(r).map(nav::route),
    })
}

/// The windowed app as an automation host: commands go through the same
/// `update` as clicks and sidebar navigation.
struct GuiHost<'a> {
    app: &'a mut App,
    tasks: Vec<Task<Message>>,
}

impl shorashim::automation::Host for GuiHost<'_> {
    fn ctx(&self) -> &Ctx {
        &self.app.ctx
    }
    fn ctx_mut(&mut self) -> &mut Ctx {
        &mut self.app.ctx
    }
    fn screen(&self) -> &Current {
        &self.app.screen
    }
    fn route(&self) -> &Route {
        &self.app.route
    }
    fn dispatch(&mut self, msg: screens::Msg) {
        let t = self.app.update(Message::Screen(msg));
        self.tasks.push(t);
    }
    fn navigate(&mut self, route: Route) {
        let t = self.app.update(Message::Nav(route));
        self.tasks.push(t);
    }
    fn back(&mut self) {
        let r = self.app.history.pop().unwrap_or(Route::Map);
        self.app.screen = Current::open(&mut self.app.ctx, &r);
        self.app.route = r;
    }
    fn settings_changed(&mut self) {
        theme::set_current(theme_kind(&self.app.ctx));
    }
    fn windowed(&self) -> bool {
        true
    }
}

fn main() -> iced::Result {
    // `shorashim ctl …`, `shorashim mcp`, `shorashim --headless …`.
    if let Some(code) = shorashim::automation::cli::main_hook() {
        std::process::exit(code);
    }
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
    let result = fonts::all()
        .into_iter()
        .fold(app, |app, font| app.font(font))
        .run();
    // The control channel's token file is only valid while the app runs.
    if let Some(file) = control::active_file() {
        control::cleanup(file);
    }
    result
}
