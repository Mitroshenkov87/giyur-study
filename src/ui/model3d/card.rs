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

/// Largest model size on a card (logical px): the full 820 px reading
/// column gives about this.
pub const MAX_SIZE: f32 = 280.0;
/// Smallest model size beside the text; below it the card stacks.
const MIN_SIDE_SIZE: f32 = 190.0;
/// Body text needs at least this much width beside the model.
const MIN_BODY: f32 = 300.0;

/// Model size for a card in a reading column `reading` px wide: about a
/// third of the card, 190–280 px. Screenshot hook `SHORASHIM_3D_SIZE=<px>`.
pub fn size_for(reading: f32) -> f32 {
    if let Some(s) = crate::util::env_hook("3D_SIZE").and_then(|v| v.parse::<f32>().ok()) {
        return s;
    }
    let inner = reading - 2.0 * theme::spacing::CARD_PAD;
    (inner * 0.36).clamp(MIN_SIDE_SIZE, MAX_SIZE)
}

/// Whether the model goes above the text (narrow windows).
fn stacked(reading: f32, size: f32) -> bool {
    reading - 2.0 * theme::spacing::CARD_PAD - size - GAP < MIN_BODY
}

const GAP: f32 = 24.0;

pub fn card<'a, M: Clone + 'a>(
    ctx: &Ctx,
    model: Model,
    kicker: Option<&'a str>,
    caption_open: bool,
    msgs: Msgs<M>,
) -> Element<'a, M> {
    let p = theme::current();
    let info = model.info();
    let reading = crate::ui::layout::reading_width();
    let size = size_for(reading);
    let stack = stacked(reading, size);
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
    if stack {
        widgets::card(
            column![container(view).center_x(Length::Fill), body]
                .spacing(16)
                .width(Length::Fill),
        )
    } else {
        widgets::card(row![view, body].spacing(GAP).align_y(Alignment::Center))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_size_scales_with_the_reading_column() {
        // Full reading column (1280 and 1920 px windows): the largest size.
        let full = size_for(crate::ui::layout::CONTENT);
        assert!((260.0..=MAX_SIZE).contains(&full), "{full}");
        assert!(!stacked(crate::ui::layout::CONTENT, full));
        // Narrower windows shrink the model, then stack it above the text.
        let mid = size_for(600.0);
        assert!(mid < full && mid >= MIN_SIDE_SIZE);
        assert!(!stacked(600.0, mid));
        assert!(stacked(420.0, size_for(420.0)));
    }
}
