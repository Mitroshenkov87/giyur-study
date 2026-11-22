//! Right-hand panel on wide windows: «Цитата дня» and a short «Сегодня»
//! card (Gregorian + Hebrew date, fast, weekly portion) that opens the
//! calendar. See [`crate::ui::layout`] for when it is shown.

use iced::widget::text::Shaping;
use iced::widget::{button, column, container, text, Column};
use iced::{Element, Length};

use crate::calendar::{days, hebrew, parasha, prefs};
use crate::quotes;
use crate::ui::app::Ctx;
use crate::ui::fonts::{HEBREW, SANS_BOLD, SERIF_BOLD};
use crate::ui::layout;
use crate::ui::theme;
use crate::ui::widgets;

/// Settings `extra` key: show the panel on wide windows (default on).
pub const SETTING_KEY: &str = "quotePanel";

/// Whether the user wants the panel (absent = yes).
pub fn enabled(ctx: &Ctx) -> bool {
    ctx.state
        .settings
        .extra
        .get(SETTING_KEY)
        .and_then(|v| v.as_bool())
        .unwrap_or(true)
}

/// Turn the panel on or off (caller saves).
pub fn set_enabled(ctx: &mut Ctx, on: bool) {
    ctx.state
        .settings
        .extra
        .insert(SETTING_KEY.to_string(), serde_json::Value::Bool(on));
}

/// The panel itself; `open_calendar` is sent by the «Сегодня» card.
pub fn panel<'a, M: Clone + 'a>(ctx: &Ctx, open_calendar: M) -> Element<'a, M> {
    let p = theme::current();
    let today = ctx.today();
    let mut col = Column::new().spacing(16).width(Length::Fill);

    if let Some(q) = quotes::for_date(today) {
        col = col.push(
            container(
                column![
                    widgets::kicker("Цитата дня"),
                    text(format!("«{}»", q.text))
                        .shaping(Shaping::Advanced)
                        .font(SERIF_BOLD)
                        .size(theme::text::BODY + 1.0)
                        .line_height(1.45)
                        .color(p.text)
                        .width(Length::Fill),
                    text(quotes::attribution(q))
                        .shaping(Shaping::Advanced)
                        .size(theme::text::SMALL)
                        .color(p.gold2)
                        .width(Length::Fill),
                ]
                .spacing(12),
            )
            .padding(theme::spacing::CARD_PAD)
            .width(Length::Fill)
            .style(move |_| theme::hero(&theme::current())),
        );
    }

    let diaspora = prefs::CalendarPrefs::load(&ctx.state.settings).diaspora;
    let info = days::day_info(today, diaspora);
    let mut day = Column::new().spacing(6).width(Length::Fill);
    day = day.push(widgets::kicker("Сегодня"));
    day = day.push(
        text(format!(
            "{}, {}",
            hebrew::weekday_ru(today),
            hebrew::format_gregorian_ru(today)
        ))
        .shaping(Shaping::Advanced)
        .font(SANS_BOLD)
        .size(theme::text::BODY)
        .color(p.text)
        .width(Length::Fill),
    );
    day = day.push(
        text(info.hebrew.format_ru())
            .shaping(Shaping::Advanced)
            .size(theme::text::SMALL + 0.5)
            .color(p.gold2)
            .width(Length::Fill),
    );
    day = day.push(
        text(info.hebrew.format_he())
            .shaping(Shaping::Advanced)
            .font(HEBREW)
            .size(20.0)
            .color(p.hebrew)
            .width(Length::Fill),
    );
    let notable: Vec<&str> = info
        .events
        .iter()
        .filter(|e| e.is_notable() || e.kind == days::Kind::RoshChodesh)
        .map(|e| e.title.as_str())
        .collect();
    if !notable.is_empty() {
        day = day.push(
            text(notable.join(" · "))
                .shaping(Shaping::Advanced)
                .size(theme::text::SMALL)
                .color(p.text)
                .width(Length::Fill),
        );
    }
    day = day.push(
        text(if info.fast().is_some() {
            "Сегодня пост"
        } else {
            "Сегодня поста нет"
        })
        .shaping(Shaping::Advanced)
        .size(theme::text::SMALL)
        .color(if info.fast().is_some() {
            p.bad
        } else {
            p.muted
        }),
    );
    let (_, reading, next) = parasha::weekly(today, diaspora);
    let portion = if reading == parasha::Reading::Festival {
        next.map(|(_, r)| format!("Недельная глава: праздничное чтение, затем {}", r.name_ru()))
    } else {
        Some(format!("Недельная глава: {}", reading.name_ru()))
    };
    if let Some(s) = portion {
        day = day.push(
            text(s)
                .shaping(Shaping::Advanced)
                .size(theme::text::SMALL)
                .color(p.muted)
                .width(Length::Fill),
        );
    }
    col = col.push(
        button(day)
            .padding(theme::spacing::CARD_PAD)
            .width(Length::Fill)
            .style(move |_, s| theme::aside_card(&theme::current(), s))
            .on_press(open_calendar),
    );

    container(col)
        .width(layout::ASIDE)
        .padding([28.0, 24.0])
        .into()
}
