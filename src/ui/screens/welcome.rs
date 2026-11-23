//! Welcome screen (PARITY §1): hero with overlaid text, feature list,
//! rabbi-review notice and the «Начать» button.

use iced::advanced::layout::{self, Node};
use iced::advanced::renderer::{self, Renderer as _};
use iced::advanced::widget::{Tree, Widget};
use iced::widget::{column, container, row, space, stack, Column};
use iced::{Alignment, Background, Border, Color, Element, Length, Rectangle, Size};

use crate::i18n::{app_name, tr};
use crate::ui::app::{Action, Ctx, Route};
use crate::ui::screens::map::{alpha, icons, txt, v_gradient};
use crate::ui::{assets, fonts, theme, widgets};

pub struct Screen;

#[derive(Debug, Clone)]
pub enum Msg {
    Start,
}

impl Screen {
    pub fn new(_ctx: &mut Ctx, _route: &Route) -> Self {
        Screen
    }

    pub fn update(&mut self, _ctx: &mut Ctx, msg: Msg) -> Action {
        match msg {
            Msg::Start => Action::Go(Route::Level { reopened: false }),
        }
    }

    pub fn subscription(&self) -> iced::Subscription<Msg> {
        iced::Subscription::none()
    }

    pub fn view<'a>(&'a self, _ctx: &'a Ctx) -> Element<'a, Msg> {
        let p = theme::current();

        // ---- Hero (welcome.webp with fallback) ----
        // The photo is 16:9. iced's Image + ContentFit::Cover lays out or
        // paints a 16:9 box even when given a shorter height, and tiny-skia
        // draws images after container quads while ignoring the image's own
        // clip bounds — so the picture used to run over «8 разделов».
        // `clipped` forces a 300px layout box and scissors with `with_layer`.
        let hero_height = 300.0;
        let hero_inner: Element<'a, Msg> = match assets::img("welcome") {
            Some(handle) => {
                // Cover-crop the 3840×2160 art to the 820×300 hero aspect
                // (3840 × 300 / 820 ≈ 1405) so Cover cannot overflow vertically
                // at the content column width.
                const SRC_W: u32 = 3840;
                const SRC_H: u32 = 2160;
                const CROP_H: u32 = 1405;
                iced::widget::image(handle)
                    .width(Length::Fill)
                    .height(hero_height)
                    .content_fit(iced::ContentFit::Cover)
                    .crop(Rectangle {
                        x: 0,
                        y: (SRC_H - CROP_H) / 2,
                        width: SRC_W,
                        height: CROP_H,
                    })
                    .border_radius(theme::radius::HERO)
                    .into()
            }
            None => container(icons::icon(
                icons::Kind::StarDavid,
                110.0,
                Color::from_rgb8(0xe6, 0xc3, 0x6a),
            ))
            .width(Length::Fill)
            .height(hero_height)
            .style(move |_| iced::widget::container::Style {
                background: Some(v_gradient(
                    Color::from_rgb8(0x1b, 0x25, 0x47),
                    Color::from_rgb8(0x0b, 0x10, 0x20),
                )),
                ..iced::widget::container::Style::default()
            })
            .into(),
        };

        // Title overlaid at the bottom (fixed light colours: the hero art is
        // dark in both themes), on a smooth bottom-only scrim — no hard edge.
        let over_text = Color::from_rgb8(0xec, 0xe7, 0xda);
        let over_gold = Color::from_rgb8(0xe6, 0xc3, 0x6a);
        let overlay = container(
            column![
                txt(tr("Добро пожаловать").to_uppercase())
                    .size(theme::text::KICKER)
                    .font(fonts::SANS_BOLD)
                    .color(over_gold),
                txt(app_name())
                    .size(42.0)
                    .font(fonts::SERIF_BOLD)
                    .color(over_text),
                container(
                    txt(tr("Основы иудаизма и спокойная, последовательная подготовка к гиюру и собеседованию в бейт-дине."))
                        .size(theme::text::H3)
                        .color(over_text)
                        .width(Length::Fill),
                )
                .max_width(640.0),
            ]
            .spacing(8),
        )
        .padding([20.0, 24.0])
        .width(Length::Fill);

        let scrim = container(space::vertical().height(hero_height))
            .width(Length::Fill)
            .height(hero_height)
            .style(|_| iced::widget::container::Style {
                background: Some(hero_scrim()),
                ..iced::widget::container::Style::default()
            });

        let hero = clipped(
            hero_height,
            container(stack![
                hero_inner,
                scrim,
                container(overlay)
                    .width(Length::Fill)
                    .height(hero_height)
                    .align_y(iced::alignment::Vertical::Bottom),
            ])
            .width(Length::Fill)
            .height(hero_height)
            .max_height(hero_height)
            .clip(true)
            .style(move |_| iced::widget::container::Style {
                border: Border {
                    color: p.line,
                    width: 1.0,
                    radius: theme::radius::HERO.into(),
                },
                ..iced::widget::container::Style::default()
            }),
        );

        // ---- Body card (below the hero, not overlapping it) ----
        let feature = |kind: icons::Kind, body: &'a str| {
            row![
                icons::icon(kind, 20.0, p.gold2),
                widgets::rich_paragraph(body),
            ]
            .spacing(12)
            .align_y(Alignment::Center)
        };
        let body_card = container(
            column![
                feature(
                    icons::Kind::Book,
                    tr("**8 разделов** — от основ веры до бейт-дина, уроки по 5–10 минут."),
                ),
                feature(
                    icons::Kind::Cards,
                    tr("**Карточки** с интервальным повторением: 1 → 3 → 7 → 14 → 30 дней."),
                ),
                feature(
                    icons::Kind::Exam,
                    tr("**Тесты и пробный бейт-дин** с разбором ошибок."),
                ),
                container(
                    row![
                        icons::icon(icons::Kind::Sources, 20.0, p.gold2),
                        txt(tr("Материалы требуют проверки раввином вашей общины. Приложение — помощник в учёбе, а не замена учителю."))
                            .size(theme::text::SMALL + 1.0)
                            .color(p.text)
                            .width(Length::Fill),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center),
                )
                .padding([12.0, 16.0])
                .width(Length::Fill)
                .style(move |_| iced::widget::container::Style {
                    background: Some(Background::Color(p.gold_soft)),
                    border: Border {
                        color: alpha(p.gold, 0.25),
                        width: 1.0,
                        radius: theme::radius::SMALL.into(),
                    },
                    ..iced::widget::container::Style::default()
                }),
                txt(tr("Всё работает без интернета; прогресс хранится только на этом устройстве."))
                    .size(theme::text::SMALL)
                    .color(p.muted),
                widgets::centered(widgets::btn(
                    tr("Начать"),
                    widgets::Btn::Primary,
                    widgets::BtnSize::Lg,
                    Some(Msg::Start),
                )),
            ]
            .spacing(16),
        )
        .padding(theme::spacing::CARD_PAD)
        .width(Length::Fill)
        .style(move |_| {
            // Solid surface so the feature list stays readable on the card,
            // not on the hero art. (theme::card is a 90 % veil.)
            let mut style = theme::card(&p);
            style.background = Some(Background::Color(p.surface));
            style
        });

        // A stack would clip the card (it sizes to the hero), so keep them
        // as a column: hero, then a normal gap, then the list.
        Column::new()
            .push(hero)
            .push(body_card)
            .spacing(20)
            .width(Length::Fill)
            .into()
    }
}

/// Bottom-only darkening behind the welcome title: transparent through the
/// upper half, then a smooth fade. Two-stop 0→0.92 over the full height
/// made a hard band when the layer failed to fill the hero.
fn hero_scrim() -> Background {
    Background::Gradient(iced::Gradient::Linear(
        iced::gradient::Linear::new(iced::Radians(std::f32::consts::FRAC_PI_2))
            .add_stop(0.0, Color::from_rgba8(8, 11, 22, 0.0))
            .add_stop(0.50, Color::from_rgba8(8, 11, 22, 0.0))
            .add_stop(0.78, Color::from_rgba8(8, 11, 22, 0.45))
            .add_stop(1.0, Color::from_rgba8(8, 11, 22, 0.82)),
    ))
}

/// Fixed-height scissor: layout is exactly `height` px, and children are
/// drawn inside a `with_layer` so a Cover-fitted image cannot paint over
/// the widgets below (tiny-skia paints images after quads and ignores the
/// Image widget's clip rectangle).
fn clipped<'a, Message: 'a>(
    height: f32,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    Element::new(Clipped {
        height,
        content: content.into(),
    })
}

struct Clipped<'a, Message> {
    height: f32,
    content: Element<'a, Message>,
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for Clipped<'_, Message> {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fixed(self.height))
    }

    fn size_hint(&self) -> Size<Length> {
        self.size()
    }

    fn tag(&self) -> iced::advanced::widget::tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> iced::advanced::widget::tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut Tree) {
        self.content.as_widget().diff(tree);
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> Node {
        let limits = limits
            .width(Length::Fill)
            .height(Length::Fixed(self.height));
        let content = self
            .content
            .as_widget_mut()
            .layout(tree, renderer, &limits.loose());
        let size = limits.resolve(Length::Fill, Length::Fixed(self.height), content.size());
        Node::with_children(size, vec![content])
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: iced::mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let Some(content_layout) = layout.children().next() else {
            return;
        };
        renderer.with_layer(bounds, |renderer| {
            self.content.as_widget().draw(
                tree,
                renderer,
                theme,
                style,
                content_layout,
                cursor,
                &bounds,
            );
        });
    }
}
