//! A reusable «3D object» card: the live model (or its still), a kicker,
//! title, lead text, an optional caption toggled by a click on the model,
//! the interaction hint and an optional link button.

use iced::widget::text::{self, Shaping};
use iced::widget::{button, column, container, row, Column};
use iced::{Alignment, Background, Border, Color, Element, Length};

use super::Model;
use crate::ui::app::Ctx;
use crate::ui::{fonts, theme, widgets};

fn txt<'a>(content: impl text::IntoFragment<'a>) -> iced::widget::Text<'a> {
    iced::widget::text(content).shaping(Shaping::Advanced)
}

/// Messages a card can send.
pub struct Msgs<M> {
    /// The model was drawn by the GPU (rebuild without the placeholder).
    pub ready: M,
    /// Click on the model: show / hide the caption.
    pub toggle_caption: M,
    /// Optional link button (label, message).
    pub link: Option<(String, M)>,
}

/// Default model size on cards (logical px). Screenshot hook
/// `SHORASHIM_3D_SIZE=<px>`.
pub fn default_size() -> f32 {
    crate::util::env_hook("3D_SIZE")
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(190.0)
}

pub fn card<'a, M: Clone + 'a>(
    ctx: &Ctx,
    model: Model,
    kicker: Option<&'a str>,
    caption_open: bool,
    msgs: Msgs<M>,
) -> Element<'a, M> {
    let p = theme::current();
    let info = model.info();
    let size = default_size();
    let live = super::enabled(ctx);
    let view = if live {
        super::view(
            model,
            size,
            p.texture == "bg-dark",
            msgs.ready,
            Some(msgs.toggle_caption),
        )
    } else {
        super::still(model, size)
    };
    let hint = if live && super::software_forced() {
        "Щелчок — подпись."
    } else if live {
        "Тяните — повернуть, колесо — приблизить, щелчок — подпись, двойной щелчок — исходный вид."
    } else {
        "Объёмные модели выключены в настройках."
    };
    let mut body: Column<'a, M> = column![
        widgets::kicker(kicker.unwrap_or(info.kicker)),
        widgets::heading(info.title, 3),
        txt(info.lead)
            .size(theme::text::BODY)
            .color(p.text)
            .width(Length::Fill),
    ]
    .spacing(8)
    .width(Length::Fill);
    if caption_open {
        body = body.push(
            container(
                txt(info.caption)
                    .size(theme::text::SMALL)
                    .color(p.text)
                    .width(Length::Fill),
            )
            .padding([10.0, 12.0])
            .width(Length::Fill)
            .style(move |_| container::Style {
                background: Some(Background::Color(Color { a: 0.10, ..p.gold })),
                border: Border {
                    color: Color { a: 0.55, ..p.gold },
                    width: 1.0,
                    radius: 10.0.into(),
                },
                ..container::Style::default()
            }),
        );
    }
    body = body.push(txt(hint).size(theme::text::SMALL).color(p.muted));
    if let Some((label, msg)) = msgs.link {
        body = body.push(
            button(txt(label).size(theme::text::SMALL).font(fonts::SANS_BOLD))
                .padding([8.0, 16.0])
                .style(move |_, s| theme::button_secondary(&p, s))
                .on_press(msg),
        );
    }
    widgets::card(row![view, body].spacing(20).align_y(Alignment::Center))
}

/// Placeholder card for an object whose model is not good enough yet.
pub fn placeholder<'a, M: 'a>(title: &'a str, note: &'a str) -> Element<'a, M> {
    let p = theme::current();
    widgets::card(
        column![
            widgets::heading(title, 3),
            txt(note).size(theme::text::SMALL).color(p.muted),
        ]
        .spacing(8)
        .width(Length::Fill),
    )
}
