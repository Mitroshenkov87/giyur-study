//! Settings screen (PARITY §12): movement, theme, font size, learning
//! switches, timed-exam options, daily goal, data export/import/reset and
//! the «О приложении» card. Confirm dialogs follow §0.3.

use iced::alignment::Horizontal;
use iced::widget::text::{IntoFragment, Shaping};
use iced::widget::{
    button, column, container, mouse_area, row, rule, slider, space, stack, text, Column, Text,
};
use iced::{Alignment, Background, Border, Color, Element, Length, Shadow, Theme};

use crate::content;
use crate::game;
use crate::i18n::{tr, Lang};
use crate::install;
use crate::storage::{Level, State};
use crate::ui::app::{Action, Ctx, Route};
use crate::ui::fonts::{HEBREW, SANS, SANS_BOLD, SERIF_BOLD};
use crate::ui::screens::calendar;
use crate::ui::theme::{self, Palette};
use crate::ui::widgets;

fn palette() -> Palette {
    theme::current()
}

fn alpha(color: Color, a: f32) -> Color {
    Color { a, ..color }
}

/// Plain text with advanced shaping (Cyrillic + Hebrew).
fn txt<'a>(content: impl IntoFragment<'a>) -> Text<'a> {
    text(content).shaping(Shaping::Advanced)
}

fn plain_container(background: Option<Color>) -> container::Style {
    container::Style {
        text_color: None,
        background: background.map(Background::Color),
        border: Border::default(),
        shadow: Shadow::default(),
        snap: true,
    }
}

/// Which confirmation dialog is open (§0.3).
#[derive(Debug, Clone)]
enum Confirm {
    /// Holds the file contents chosen via the open dialog.
    Import(String),
    Reset,
    Uninstall,
}

pub struct Screen {
    confirm: Option<Confirm>,
    /// Install state, read when the screen opens and after each change.
    install: install::Status,
    /// «Ярлык на рабочем столе» for the next install / update.
    desktop_shortcut: bool,
}

#[derive(Debug, Clone)]
pub enum Msg {
    ChangeLevel,
    SetLang(Lang),
    SetTheme(&'static str),
    SetFont(i64),
    ToggleUnlockAll,
    /// «Открытый режим рецензента» (src/reviewer.rs).
    ToggleReviewer,
    ToggleTranslit,
    SetSeconds(i64),
    SetGoal(i64),
    Export,
    ImportPick,
    AskReset,
    /// «Сбросить прохождение» → «Только …» (the movement in use).
    ResetCurrent,
    ConfirmOk,
    ConfirmCancel,
    GoProgress,
    GoSources,
    GoCalendar,
    ToggleQuotePanel,
    ToggleModels3d,
    ToggleDesktopShortcut,
    Install,
    AskUninstall,
    ToggleMusic,
    SetVolume(i64),
    NextTrack,
    Esc,
}

impl Screen {
    pub fn new(_ctx: &mut Ctx, _route: &Route) -> Self {
        let install = install::status();
        // New install: desktop shortcut on; update: keep what is there.
        let desktop_shortcut = install.installed.is_none() || install.desktop_link;
        Screen {
            confirm: None,
            install,
            desktop_shortcut,
        }
    }

    pub fn update(&mut self, ctx: &mut Ctx, msg: Msg) -> Action {
        match msg {
            Msg::ChangeLevel => Action::Go(Route::Level { reopened: true }),
            Msg::SetLang(lang) => {
                if crate::i18n::lang() == lang && ctx.state.settings.lang == Some(lang) {
                    return Action::None;
                }
                match ctx.switch_lang(lang) {
                    Ok(()) => {
                        ctx.toast(crate::i18n::tr_or("Сохранено", "Saved"), "", "");
                        Action::LanguageChanged
                    }
                    Err(e) => {
                        ctx.toast(
                            crate::i18n::tr_or(
                                "Не удалось сменить язык",
                                "Could not switch language",
                            ),
                            &e.to_string(),
                            "⚠",
                        );
                        Action::None
                    }
                }
            }
            Msg::SetTheme(v) => {
                ctx.state.settings.theme = v.to_string();
                ctx.save();
                Action::SettingsChanged
            }
            Msg::SetFont(step) => {
                ctx.state.settings.font_step = step.clamp(0, 3);
                ctx.save();
                Action::SettingsChanged
            }
            Msg::ToggleUnlockAll => {
                ctx.state.settings.unlock_all = !ctx.state.settings.unlock_all;
                ctx.save();
                ctx.toast(tr("Сохранено"), "", "");
                Action::None
            }
            Msg::ToggleReviewer => {
                let on = !ctx.reviewer();
                ctx.set_reviewer(on, true);
                if on {
                    ctx.toast(
                        tr("Открытый режим рецензента включён"),
                        tr("Открыто всё. Ваш прогресс отложен и не изменится."),
                        "",
                    );
                } else {
                    ctx.toast(
                        tr("Открытый режим рецензента выключен"),
                        tr("Ваш прогресс — в точности как до включения."),
                        "",
                    );
                }
                Action::SettingsChanged
            }
            Msg::ToggleTranslit => {
                ctx.state.settings.translit = !ctx.state.settings.translit;
                ctx.save();
                ctx.toast(tr("Сохранено"), "", "");
                Action::None
            }
            Msg::SetSeconds(v) => {
                ctx.state.settings.timed_seconds = crate::ladder::sanitize_seconds(v);
                ctx.save();
                Action::None
            }
            Msg::SetGoal(xp) => {
                ctx.state.settings.daily_goal = xp;
                // The new goal may already be reached today (js/ui/settings.js).
                ctx.award(0);
                ctx.toast(
                    &crate::trf!("Цель дня: {xp} XP", "Daily goal: {xp} XP"),
                    "",
                    "",
                );
                Action::None
            }
            Msg::Export => {
                let name = format!("shorashim-progress-{}.json", ctx.today_key());
                let picked = rfd::FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .set_file_name(&name)
                    .save_file();
                if let Some(path) = picked {
                    // The real progress, also in the «Открытый режим рецензента».
                    match std::fs::write(&path, ctx.real_state().export_json()) {
                        Ok(()) => ctx.toast(tr("Файл прогресса сохранён"), "", ""),
                        Err(_) => ctx.toast(tr("Не удалось сохранить файл"), "", ""),
                    }
                }
                Action::None
            }
            Msg::ImportPick if ctx.reviewer() => {
                ctx.toast(
                    tr("Недоступно в открытом режиме рецензента"),
                    tr("Выключите режим, чтобы импортировать прогресс."),
                    "",
                );
                Action::None
            }
            Msg::AskReset if ctx.reviewer() => {
                ctx.toast(
                    tr("Недоступно в открытом режиме рецензента"),
                    tr("Выключите режим, чтобы сбросить прохождение."),
                    "",
                );
                Action::None
            }
            Msg::ImportPick => {
                let picked = rfd::FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .pick_file();
                if let Some(path) = picked {
                    match std::fs::read(&path) {
                        // A non-UTF-8 file fails like invalid JSON (PWA's
                        // FileReader → JSON.parse path), not like a read error.
                        Ok(bytes) => match String::from_utf8(bytes) {
                            Ok(text) => self.confirm = Some(Confirm::Import(text)),
                            Err(_) => ctx.toast(tr("Файл не является корректным JSON."), "", ""),
                        },
                        Err(_) => ctx.toast(tr("Не удалось прочитать файл"), "", ""),
                    }
                }
                Action::None
            }
            Msg::AskReset => {
                self.confirm = Some(Confirm::Reset);
                Action::None
            }
            Msg::ConfirmOk => match self.confirm.take() {
                Some(Confirm::Import(text)) => match State::import_json(&text, &ctx.today_key()) {
                    Ok(state) => {
                        ctx.state = state;
                        let today = ctx.today();
                        let _ = crate::game::check_badges(&ctx.lib, &mut ctx.state, today);
                        ctx.writable = true;
                        ctx.save();
                        ctx.toast(tr("Прогресс импортирован"), "", "");
                        Action::SettingsChanged
                    }
                    Err(e) => {
                        ctx.toast(&e, "", "");
                        Action::None
                    }
                },
                Some(Confirm::Reset) => self.reset(ctx, crate::tracks::Reset::All),
                Some(Confirm::Uninstall) => {
                    let res = install::uninstall();
                    self.install = install::status();
                    match res {
                        Ok(install::Uninstalled::QuitToFinish) => Action::Quit,
                        Ok(install::Uninstalled::Done) => {
                            ctx.toast(
                                tr("Удалено из системы"),
                                tr("Ярлыки и запись в «Приложениях» удалены. Прогресс сохранён."),
                                "",
                            );
                            Action::None
                        }
                        Err(e) => {
                            ctx.toast(tr("Не удалось удалить"), &e, "⚠");
                            Action::None
                        }
                    }
                }
                None => Action::None,
            },
            Msg::ResetCurrent => {
                self.confirm = None;
                self.reset(ctx, crate::tracks::Reset::Current)
            }
            Msg::ConfirmCancel | Msg::Esc => {
                self.confirm = None;
                Action::None
            }
            Msg::GoProgress => Action::Go(Route::Progress),
            Msg::GoSources => Action::Go(Route::Sources),
            Msg::GoCalendar => {
                calendar::open_tab(calendar::Tab::Settings);
                Action::Go(Route::Calendar)
            }
            Msg::ToggleModels3d => {
                let on = !crate::ui::model3d::enabled(ctx);
                crate::ui::model3d::set_enabled(ctx, on);
                ctx.save();
                ctx.toast(tr("Сохранено"), "", "");
                Action::None
            }
            Msg::ToggleQuotePanel => {
                let on = !crate::ui::aside::enabled(ctx);
                crate::ui::aside::set_enabled(ctx, on);
                ctx.save();
                ctx.toast(tr("Сохранено"), "", "");
                Action::None
            }
            Msg::ToggleDesktopShortcut => {
                self.desktop_shortcut = !self.desktop_shortcut;
                Action::None
            }
            Msg::Install => {
                let updating = matches!(self.install.offer(), install::Offer::Update { .. });
                let res = install::install(install::Options {
                    desktop_shortcut: self.desktop_shortcut,
                });
                self.install = install::status();
                match res {
                    Ok(_) => {
                        let name = crate::i18n::app_name();
                        let detail = if self.desktop_shortcut {
                            crate::trf!(
                                "«{name}» теперь есть в меню «Пуск» и на рабочем столе. Прогресс общий для всех копий.",
                                "{name} is now in the Start menu and on the desktop. Progress is shared by every copy."
                            )
                        } else {
                            crate::trf!(
                                "«{name}» теперь есть в меню «Пуск». Прогресс общий для всех копий.",
                                "{name} is now in the Start menu. Progress is shared by every copy."
                            )
                        };
                        ctx.toast(
                            if updating {
                                tr("Установленная копия обновлена")
                            } else {
                                tr("Установлено в систему")
                            },
                            &detail,
                            "",
                        );
                    }
                    Err(e) => ctx.toast(tr("Не удалось установить"), &e, "⚠"),
                }
                Action::None
            }
            Msg::AskUninstall => {
                self.confirm = Some(Confirm::Uninstall);
                Action::None
            }
            Msg::ToggleMusic => {
                ctx.state.settings.music_enabled = !ctx.state.settings.music_enabled;
                ctx.save();
                ctx.toast(tr("Сохранено"), "", "");
                Action::None
            }
            Msg::SetVolume(v) => {
                ctx.state.settings.music_volume = v.clamp(0, 100);
                ctx.save();
                Action::None
            }
            Msg::NextTrack => Action::MusicNext,
        }
    }

    pub fn subscription(&self) -> iced::Subscription<Msg> {
        if self.confirm.is_some() {
            iced::keyboard::listen().filter_map(|event| match event {
                iced::keyboard::Event::KeyPressed {
                    key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
                    ..
                } => Some(Msg::Esc),
                _ => None,
            })
        } else {
            iced::Subscription::none()
        }
    }

    pub fn view<'a>(&'a self, ctx: &'a Ctx) -> Element<'a, Msg> {
        let p = palette();
        column![
            widgets::page_header(tr("Настройки"), None),
            self.general_card(ctx, p),
            self.music_card(ctx, p),
            self.install_card(p),
            self.goal_card(ctx, p),
            self.data_card(p),
            self.calendar_card(p),
            self.about_card(ctx, p),
        ]
        .spacing(16)
        .width(Length::Fill)
        .into()
    }

    /// «Сбросить прохождение» (GS-29): the progress of the movement in use
    /// or of every movement; settings (theme, font, music, the movement
    /// itself) stay. The cleared track starts again with the Torah scroll.
    fn reset(&mut self, ctx: &mut Ctx, what: crate::tracks::Reset) -> Action {
        let today = ctx.today_key();
        crate::tracks::reset(&mut ctx.state, what, &today);
        ctx.save();
        match what {
            crate::tracks::Reset::Current => {
                let title = tr(content::level_info(crate::tracks::current(&ctx.state)).title);
                ctx.toast(
                    tr("Прохождение сброшено"),
                    &crate::trf!(
                        "Течение «{title}» начинается заново. Другие течения не тронуты.",
                        "The {title} movement starts over. The others are left as they are."
                    ),
                    "",
                )
            }
            crate::tracks::Reset::All => ctx.toast(
                tr("Прохождение сброшено"),
                tr("Все течения начинаются заново."),
                "",
            ),
        }
        Action::Go(Route::Map)
    }

    /// The import / reset confirm dialog, rendered as a shell layer (§0.3).
    pub fn dialog<'a>(&'a self, ctx: &'a Ctx) -> Option<Element<'a, Msg>> {
        match &self.confirm {
            None => None,
            Some(Confirm::Import(_)) => Some(self.dialog_view(
                tr("Импортировать прогресс?"),
                tr("Текущий прогресс на этом устройстве будет заменён данными из файла."),
                tr("Импортировать"),
                false,
                None,
            )),
            Some(Confirm::Uninstall) => {
                let name = crate::i18n::app_name();
                Some(self.dialog_view(
                    crate::trf!("Удалить «{name}» из системы?", "Remove {name} from this computer?"),
                    if self.install.running_installed {
                        tr("Будут удалены установленная программа, ярлыки и запись в «Приложениях». Программа закроется. Прогресс останется на этом компьютере.")
                    } else {
                        tr("Будут удалены установленная копия, ярлыки и запись в «Приложениях». Эта портативная копия и прогресс останутся.")
                    },
                    tr("Удалить"),
                    true,
                    None,
                ))
            }
            Some(Confirm::Reset) => {
                let title = tr(content::level_info(crate::tracks::current(&ctx.state)).title);
                Some(self.dialog_view(
                    tr("Сбросить прохождение?"),
                    tr("Будут удалены прочитанные уроки, результаты тестов и экзаменов, «Маалот», карточки, опыт, значки и коллекция. Тема, шрифт и музыка останутся. Это нельзя отменить (кроме импорта ранее сохранённого файла)."),
                    tr("Все течения"),
                    true,
                    Some((
                        crate::trf!("Только «{title}»", "{title} only"),
                        Msg::ResetCurrent,
                    )),
                ))
            }
        }
    }

    // ---- Cards -------------------------------------------------------------

    fn general_card<'a>(&'a self, ctx: &'a Ctx, p: Palette) -> Element<'a, Msg> {
        let s = &ctx.state.settings;
        let level = s.level.unwrap_or(Level::Orthodox);
        let level_title = tr(content::level_info(level).title);

        let current_lang = ctx.state.settings.lang.unwrap_or_else(crate::i18n::lang);
        let mut lang_row = iced::widget::Row::new().spacing(4);
        for l in Lang::ALL {
            lang_row = lang_row.push(widgets::choice(
                l.native_name(),
                current_lang == l,
                false,
                Msg::SetLang(l),
            ));
        }

        let rows = column![
            set_row(
                crate::i18n::tr_or("Язык", "Language"),
                crate::i18n::tr_or(
                    "Язык интерфейса и материалов курса",
                    "Interface and course language",
                )
                .to_string(),
                lang_row.into(),
            ),
            divider(p),
            set_row(
                tr("Течение"),
                crate::trf!(
                    "{level_title} · у каждого течения своё прохождение",
                    "{level_title} · each movement keeps its own progress"
                ),
                widgets::btn(
                    tr("Сменить течение"),
                    widgets::Btn::Secondary,
                    widgets::BtnSize::Sm,
                    Some(Msg::ChangeLevel),
                )
                .into(),
            ),
            divider(p),
            set_row(
                tr("Тема"),
                tr("Тёмная — основная").to_string(),
                row![
                    widgets::choice(
                        tr("Тёмная"),
                        s.theme != crate::storage::LIGHT_THEME,
                        false,
                        Msg::SetTheme("dark")
                    ),
                    widgets::choice(
                        tr("Кожа"),
                        s.theme == crate::storage::LIGHT_THEME,
                        false,
                        Msg::SetTheme(crate::storage::LIGHT_THEME)
                    ),
                ]
                .spacing(4)
                .into(),
            ),
            divider(p),
            set_row(
                tr("Размер шрифта"),
                tr("Иврит масштабируется тоже").to_string(),
                row![
                    widgets::choice("A−", s.font_step == 0, false, Msg::SetFont(0)),
                    widgets::choice("A", s.font_step == 1, false, Msg::SetFont(1)),
                    widgets::choice("A+", s.font_step == 2, false, Msg::SetFont(2)),
                    widgets::choice("A++", s.font_step == 3, false, Msg::SetFont(3)),
                ]
                .spacing(4)
                .into(),
            ),
            divider(p),
            set_row(
                tr(crate::reviewer::TITLE),
                tr(crate::reviewer::CAPTION).to_string(),
                switch(ctx.reviewer(), Msg::ToggleReviewer, p),
            ),
            divider(p),
            set_row(
                tr("Открыть все разделы"),
                tr("Свободное изучение без последовательной разблокировки").to_string(),
                switch(s.unlock_all, Msg::ToggleUnlockAll, p),
            ),
            divider(p),
            set_row(
                tr("Показывать транслитерацию"),
                tr("Русская транскрипция под текстом на иврите").to_string(),
                switch(s.translit, Msg::ToggleTranslit, p),
            ),
            divider(p),
            set_row(
                tr("Цитата дня"),
                tr("Панель справа на широком окне: цитата и сегодняшняя дата").to_string(),
                switch(crate::ui::aside::enabled(ctx), Msg::ToggleQuotePanel, p),
            ),
            divider(p),
            set_row(
                tr("Объёмные модели"),
                tr("Живые 3D-предметы, которые можно вращать мышью (нужна видеокарта)").to_string(),
                switch(crate::ui::model3d::enabled(ctx), Msg::ToggleModels3d, p),
            ),
            divider(p),
            set_row(
                tr("Маалот"),
                tr("Таймер на вопрос в лестнице знаний").to_string(),
                {
                    let cur = crate::ladder::timer_seconds(&ctx.state);
                    let mut r = iced::widget::Row::new().spacing(4);
                    for (v, label) in crate::ladder::TIMER_CHOICES {
                        r = r.push(widgets::choice(
                            tr(label),
                            cur == v,
                            false,
                            Msg::SetSeconds(v),
                        ));
                    }
                    r.into()
                },
            ),
            container(
                txt("שְׁמַע יִשְׂרָאֵל")
                    .size(theme::text::HEBREW)
                    .font(HEBREW)
                    .color(p.hebrew)
                    // RTL paragraphs are already flush right in cosmic-text;
                    // keep Left + Fill (see widgets::hebrew_block).
                    .align_x(Horizontal::Left)
                    .width(Length::Fill),
            )
            .padding([12.0, 16.0])
            .width(Length::Fill)
            .style(move |_| theme::hebrew_block(&palette())),
        ]
        .spacing(14);
        widgets::card(rows)
    }

    fn goal_card<'a>(&'a self, ctx: &'a Ctx, p: Palette) -> Element<'a, Msg> {
        let current = game::goal(&ctx.state);
        let mut segs = Column::new().spacing(6).width(Length::Fill);
        for g in game::GOALS {
            segs = segs.push(goal_segment(g.title, g.xp, current == g.xp, p));
        }

        let note = row![
            txt(tr("XP даются за уроки, карточки, тесты и викторину. За каждые 7 дней серии — заморозка (до двух), она сохранит серию, если вы пропустите день."))
                .size(theme::text::SMALL)
                .color(p.muted)
                .width(Length::Fill),
        ]
        .width(Length::Fill);

        let link = button(
            txt(tr("Подробнее в разделе «Успехи»"))
                .size(theme::text::SMALL)
                .color(p.gold2),
        )
        .padding(0)
        .style(|_, _| iced::widget::button::Style::default())
        .on_press(Msg::GoProgress);

        widgets::card(
            column![
                widgets::heading(tr("Цель и серия"), 3),
                column![
                    txt(tr("Цель дня")).size(theme::text::BODY).font(SANS_BOLD),
                    txt(tr(
                        "Сколько XP набирать в день. Серия растёт, когда цель выполнена."
                    ))
                    .size(theme::text::SMALL)
                    .color(p.muted),
                ]
                .spacing(2),
                segs,
                note,
                link,
            ]
            .spacing(12),
        )
    }

    fn data_card<'a>(&'a self, p: Palette) -> Element<'a, Msg> {
        let export = widgets::btn(
            tr("Экспорт прогресса"),
            widgets::Btn::Secondary,
            widgets::BtnSize::Sm,
            Some(Msg::Export),
        );
        let import = widgets::btn(
            tr("Импорт прогресса"),
            widgets::Btn::Secondary,
            widgets::BtnSize::Sm,
            Some(Msg::ImportPick),
        );
        let reset = widgets::btn(
            tr("Сбросить прохождение"),
            widgets::Btn::Danger,
            widgets::BtnSize::Sm,
            Some(Msg::AskReset),
        );

        widgets::card(
            column![
                widgets::heading(tr("Данные"), 3),
                txt(tr("Прогресс хранится только на этом устройстве."))
                    .size(theme::text::SMALL)
                    .color(p.muted),
                row![export, import, reset].spacing(10).wrap(),
            ]
            .spacing(12),
        )
    }

    fn music_card<'a>(&'a self, ctx: &'a Ctx, p: Palette) -> Element<'a, Msg> {
        let s = &ctx.state.settings;
        let now = if !ctx.music_available {
            tr("Файлы музыки не найдены. Они идут в портативном архиве и копируются при установке (папка music рядом с программой).").to_string()
        } else if let Some(err) = &ctx.music_error {
            err.clone()
        } else if !s.music_enabled {
            tr("Сейчас выключена.").to_string()
        } else if let Some(title) = &ctx.music_title {
            crate::trf!("Сейчас звучит: {title}", "Now playing: {title}")
        } else {
            tr("Сейчас включается…").to_string()
        };
        let vol = s.music_volume.clamp(0, 100);
        let next = widgets::btn(
            tr("Следующий трек"),
            widgets::Btn::Secondary,
            widgets::BtnSize::Sm,
            if s.music_enabled && ctx.music_available {
                Some(Msg::NextTrack)
            } else {
                None
            },
        );
        widgets::card(
            column![
                widgets::heading(tr("Музыка"), 3),
                txt(tr(
                    "Тихие инструментальные мелодии на фоне занятий. По умолчанию выключена."
                ))
                .size(theme::text::SMALL)
                .color(p.muted)
                .width(Length::Fill),
                set_row(
                    tr("Включить музыку"),
                    if ctx.music_track_count > 1 {
                        crate::trf!(
                            "Плейлист из {} мелодий, без повторов, пока не прозвучат все",
                            "A playlist of {} melodies, with no repeat until every one has played",
                            ctx.music_track_count
                        )
                    } else {
                        tr("Один трек, он будет плавно начинаться заново").to_string()
                    },
                    switch(s.music_enabled, Msg::ToggleMusic, p),
                ),
                txt(now)
                    .size(theme::text::SMALL)
                    .color(p.muted)
                    .width(Length::Fill),
                set_row(tr("Громкость"), format!("{vol} %"), volume_slider(vol),),
                next,
                txt(tr("Мелодии созданы в Grok Imagine."))
                    .size(theme::text::SMALL)
                    .color(p.muted)
                    .width(Length::Fill),
            ]
            .spacing(12),
        )
    }

    fn install_card<'a>(&'a self, p: Palette) -> Element<'a, Msg> {
        let note = |s: String| -> Element<'a, Msg> {
            txt(s)
                .size(theme::text::SMALL)
                .color(p.muted)
                .width(Length::Fill)
                .into()
        };
        let mut col = Column::new()
            .spacing(12)
            .width(Length::Fill)
            .push(widgets::heading(tr("Установка на компьютер"), 3));
        let uninstall = || {
            widgets::btn(
                tr("Удалить из системы"),
                widgets::Btn::Danger,
                widgets::BtnSize::Sm,
                Some(Msg::AskUninstall),
            )
        };
        let desktop = || {
            set_row(
                tr("Ярлык на рабочем столе"),
                tr("Ярлык в меню «Пуск» создаётся всегда").to_string(),
                switch(self.desktop_shortcut, Msg::ToggleDesktopShortcut, p),
            )
        };
        match self.install.offer() {
            install::Offer::Install => {
                col = col
                    .push(note(
                        tr("Сейчас программа работает как портативная — прямо из этого файла. Её можно установить для вашей учётной записи (права администратора не нужны): ярлык в меню «Пуск», запись в «Приложениях», удаление в один клик. Прогресс хранится отдельно и не пропадёт.")
                            .into(),
                    ))
                    .push(desktop())
                    .push(widgets::btn(
                        tr("Установить в систему"),
                        widgets::Btn::Primary,
                        widgets::BtnSize::Md,
                        Some(Msg::Install),
                    ));
            }
            install::Offer::Update { from, to } => {
                col = col
                    .push(note(crate::trf!(
                        "В системе установлена версия {from}, а сейчас запущена более новая — {to}. Можно обновить установленную копию; прогресс сохранится.",
                        "Version {from} is installed, and this copy is newer — {to}. You can update the installed copy; progress is kept."
                    )))
                    .push(desktop())
                    .push(
                        row![
                            widgets::btn(
                                tr("Обновить установленную копию"),
                                widgets::Btn::Primary,
                                widgets::BtnSize::Md,
                                Some(Msg::Install),
                            ),
                            uninstall(),
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center)
                        .wrap(),
                    );
            }
            install::Offer::UpToDate {
                version,
                running_installed,
            } => {
                let place = self
                    .install
                    .installed
                    .as_ref()
                    .map(|i| i.dir.display().to_string())
                    .unwrap_or_default();
                let detail = if running_installed {
                    crate::trf!(
                        "Установлено в систему, версия {version}: {place}. Запущена установленная копия.",
                        "Installed, version {version}: {place}. This is the installed copy."
                    )
                } else {
                    crate::trf!(
                        "Установлено в систему, версия {version}: {place}. Сейчас запущена портативная копия; установленная открывается из меню «Пуск».",
                        "Installed, version {version}: {place}. This is the portable copy; the installed one opens from the Start menu."
                    )
                };
                col = col.push(note(detail)).push(uninstall());
            }
            install::Offer::Unsupported(why) => {
                col = col.push(note(why.to_string()));
            }
        }
        widgets::card(col)
    }

    fn calendar_card<'a>(&'a self, p: Palette) -> Element<'a, Msg> {
        widgets::card(
            column![
                widgets::heading(tr("Еврейский календарь"), 3),
                txt(tr(
                    "Израиль или диаспора, местоположение для времени свечей и постов"
                ))
                .size(theme::text::SMALL)
                .color(p.muted),
                widgets::btn(
                    tr("Настройки календаря"),
                    widgets::Btn::Secondary,
                    widgets::BtnSize::Sm,
                    Some(Msg::GoCalendar),
                ),
            ]
            .spacing(12),
        )
    }

    fn about_card<'a>(&'a self, ctx: &'a Ctx, p: Palette) -> Element<'a, Msg> {
        let m = &ctx.lib.manifest;
        let available = ctx.lib.sections.iter().filter(|s| s.available).count();
        let src_count: usize = ctx
            .lib
            .sections
            .iter()
            .map(|s| ctx.lib.sources(s, None).len())
            .sum();

        let kv = |key: &'static str, value: String| -> Element<'a, Msg> {
            row![
                txt(key)
                    .size(theme::text::SMALL + 0.5)
                    .color(p.muted)
                    .width(Length::Fixed(170.0)),
                txt(value)
                    .size(theme::text::SMALL + 0.5)
                    .width(Length::Fill),
            ]
            .spacing(10)
            .into()
        };

        let content_version = match &m.content_version {
            Some(cv) => match &m.version {
                Some(v) => format!("{cv} ({v})"),
                None => cv.clone(),
            },
            None => "—".to_string(),
        };
        let review_status = m
            .review_status
            .clone()
            .unwrap_or_else(|| tr("требует проверки раввином").to_string());

        let notice = container(
            txt(tr("Материалы требуют проверки раввином вашей общины. Окончательные решения о гиюре принимает бейт-дин."))
                .size(theme::text::SMALL + 0.5)
                .color(p.text)
                .width(Length::Fill),
        )
        .padding([12.0, 16.0])
        .width(Length::Fill)
        .style(move |_| container::Style {
            border: Border {
                radius: theme::radius::SMALL.into(),
                ..Border::default()
            },
            ..plain_container(Some(p.gold_soft))
        });

        let sources_btn = widgets::btn(
            crate::trf!("Источники ({src_count})", "Sources ({src_count})"),
            widgets::Btn::Secondary,
            widgets::BtnSize::Sm,
            Some(Msg::GoSources),
        );

        widgets::card(
            column![
                widgets::heading(tr("О приложении"), 3),
                column![
                    kv(
                        tr("Версия приложения"),
                        install::current_version().to_string(),
                    ),
                    kv(tr("Версия содержания"), content_version),
                    kv(tr("Статус материалов"), review_status),
                    kv(
                        tr("Разделов готово"),
                        crate::trf!(
                            "{} из {}",
                            "{} of {}",
                            available,
                            ctx.lib.sections.len()
                        ),
                    ),
                ]
                .spacing(8),
                notice,
                txt(tr("Открытый исходный код (Apache-2.0) и документация: github.com/Mitroshenkov87/shorashim"))
                    .size(theme::text::SMALL)
                    .color(p.muted),
                sources_btn,
            ]
            .spacing(12),
        )
    }

    /// The §0.3 confirm dialog as a standalone overlay layer.
    fn dialog_view<'a>(
        &'a self,
        title: impl IntoFragment<'a> + 'a,
        body: impl IntoFragment<'a> + 'a,
        ok_label: impl IntoFragment<'a> + 'a,
        danger: bool,
        middle: Option<(String, Msg)>,
    ) -> Element<'a, Msg> {
        let p = palette();
        let backdrop = mouse_area(
            container(space::horizontal())
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_| plain_container(Some(Color::from_rgba8(3, 5, 12, 0.38)))),
        )
        .on_press(Msg::ConfirmCancel);

        let cancel = widgets::btn(
            tr("Отмена"),
            widgets::Btn::Ghost,
            widgets::BtnSize::Md,
            Some(Msg::ConfirmCancel),
        );
        let ok_kind = if danger {
            widgets::Btn::Danger
        } else {
            widgets::Btn::Primary
        };
        let ok = widgets::btn(
            ok_label,
            ok_kind,
            widgets::BtnSize::Md,
            Some(Msg::ConfirmOk),
        );

        let modal = container(
            column![
                txt(title)
                    .size(theme::text::H3)
                    .font(SERIF_BOLD)
                    .color(p.text),
                txt(body)
                    .size(theme::text::SMALL + 1.0)
                    .color(p.muted)
                    .width(Length::Fill),
                {
                    let mut r = row![space::horizontal().width(Length::Fill), cancel]
                        .spacing(10)
                        .align_y(Alignment::Center);
                    if let Some((label, msg)) = middle {
                        r = r.push(widgets::btn(
                            label,
                            widgets::Btn::Secondary,
                            widgets::BtnSize::Md,
                            Some(msg),
                        ));
                    }
                    r.push(ok).wrap()
                },
            ]
            .spacing(14),
        )
        .padding(24.0)
        .max_width(460.0)
        .style(move |_| container::Style {
            background: Some(Background::Color(p.surface)),
            border: Border {
                color: p.line2,
                width: 1.0,
                radius: theme::radius::MODAL.into(),
            },
            shadow: Shadow {
                color: alpha(Color::BLACK, 0.4),
                offset: iced::Vector::new(0.0, 10.0),
                blur_radius: 30.0,
            },
            text_color: None,
            snap: true,
        });

        stack![
            backdrop,
            container(widgets::modal_fit(modal))
                .center(Length::Fill)
                .padding(20.0)
        ]
        .into()
    }
}

// ---------------------------------------------------------------------------
// Controls
// ---------------------------------------------------------------------------

/// Hairline separator between settings rows.
fn divider<'a>(p: Palette) -> Element<'a, Msg> {
    rule::horizontal(1)
        .style(move |_theme: &Theme| theme::divider_h(&p))
        .into()
}

/// Bold label + small muted sub on the left, control on the right (§12).
fn set_row<'a>(label: &'static str, sub: String, control: Element<'a, Msg>) -> Element<'a, Msg> {
    row![
        column![
            txt(tr(label)).size(theme::text::BODY).font(SANS_BOLD),
            txt(sub).size(theme::text::SMALL).color(palette().muted),
        ]
        .spacing(2)
        .width(Length::Fill),
        control,
    ]
    .spacing(16)
    .align_y(Alignment::Center)
    .into()
}

/// Daily-goal segment: full-width row «{title} … {xp} XP» (§12 card 2).
fn goal_segment<'a>(title: &'static str, xp: i64, selected: bool, p: Palette) -> Element<'a, Msg> {
    button(
        container(
            row![
                txt(tr(title))
                    .size(theme::text::BODY)
                    .font(if selected { SANS_BOLD } else { SANS })
                    .color(if selected { p.gold2 } else { p.text }),
                space::horizontal().width(Length::Fill),
                txt(format!("{xp} XP"))
                    .size(theme::text::SMALL)
                    .color(if selected { p.gold2 } else { p.muted }),
            ]
            .align_y(Alignment::Center),
        )
        .center_y(Length::Fill),
    )
    .padding([0.0, 14.0])
    .height(theme::size::BTN_MD)
    .width(Length::Fill)
    .style(move |_, status| theme::choice(&palette(), selected, false, status))
    .on_press(Msg::SetGoal(xp))
    .into()
}

fn volume_slider(value: i64) -> Element<'static, Msg> {
    let p = palette();
    slider(0..=100, value as u8, |v| Msg::SetVolume(i64::from(v)))
        .width(Length::Fixed(180.0))
        .style(move |_theme, _status| {
            use iced::widget::slider::{Handle, HandleShape, Rail, Style};
            Style {
                rail: Rail {
                    backgrounds: (
                        iced::Background::Color(p.gold2),
                        iced::Background::Color(p.surface3),
                    ),
                    width: 4.0,
                    border: Border {
                        radius: 2.0.into(),
                        ..Border::default()
                    },
                },
                handle: Handle {
                    shape: HandleShape::Circle { radius: 8.0 },
                    background: iced::Background::Color(p.gold2),
                    border_width: 0.0,
                    border_color: Color::TRANSPARENT,
                },
            }
        })
        .into()
}

/// The sliding switch (§T.6: 50×28 track, 20 px knob, slides 22 px).
fn switch<'a>(on: bool, msg: Msg, p: Palette) -> Element<'a, Msg> {
    let knob_color = if on { p.gold2 } else { p.muted };
    let knob = container(space::horizontal())
        .width(Length::Fixed(20.0))
        .height(Length::Fixed(20.0))
        .style(move |_| container::Style {
            border: Border {
                radius: theme::radius::PILL.into(),
                ..Border::default()
            },
            ..plain_container(Some(knob_color))
        });
    let inner: Element<'a, Msg> = if on {
        row![space::horizontal().width(Length::Fixed(22.0)), knob].into()
    } else {
        row![knob, space::horizontal().width(Length::Fixed(22.0))].into()
    };
    let button = button(
        container(inner)
            .padding(4.0)
            .width(Length::Fixed(50.0))
            .height(Length::Fixed(28.0))
            .style(move |_| container::Style {
                background: Some(Background::Color(if on { p.gold_soft } else { p.surface3 })),
                border: Border {
                    color: if on { p.gold } else { Color::TRANSPARENT },
                    width: 1.0,
                    radius: theme::radius::PILL.into(),
                },
                shadow: Shadow::default(),
                text_color: None,
                snap: true,
            }),
    )
    .padding(0)
    .style(|_, _| iced::widget::button::Style::default())
    .on_press(msg);
    // The switch has no text of its own: report its state so the automation
    // probe lists it as a control (id = its message, e.g. Settings(ToggleTranslit)).
    crate::automation::reported(button, if on { tr("вкл") } else { tr("выкл") })
}
