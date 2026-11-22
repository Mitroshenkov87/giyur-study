//! Sidebar navigation items: one-word label, a grey one-line subtitle and a
//! fuller hover description. The single source of the names, so the shell,
//! tests and command control agree (old labels are kept as aliases).

use crate::ui::app::{Deck, Route};

/// One sidebar entry.
#[derive(Debug, Clone, Copy)]
pub struct Item {
    /// One word, shown in the sidebar and as the screen's title.
    pub label: &'static str,
    /// Small grey line under the label.
    pub subtitle: &'static str,
    /// Hover tooltip.
    pub tooltip: &'static str,
    /// Label before v0.5 (for commands and old notes), if it changed.
    pub old_label: Option<&'static str>,
    /// Name of the item's rendered 3D icon (`assets/img/icons/<art>-<theme>.webp`).
    pub art: &'static str,
}

/// The sidebar, top to bottom (index = `group` of a route).
pub const ITEMS: [Item; 8] = [
    Item {
        label: "Уроки",
        subtitle: "тексты по темам",
        tooltip: "Разделы и уроки: тексты по темам с источниками, карточки и тесты каждого раздела",
        old_label: Some("Карта"),
        art: "lessons",
    },
    Item {
        label: "Календарь",
        subtitle: "еврейский год",
        tooltip: "Еврейский календарь",
        old_label: None,
        art: "calendar",
    },
    Item {
        label: "Карточки",
        subtitle: "интервальное повторение",
        tooltip: "Карточки с интервальным повторением: всё, что пора повторить сегодня",
        old_label: Some("Повторение"),
        art: "cards",
    },
    Item {
        label: "Викторина",
        subtitle: "вопросы на время",
        tooltip: "Викторина на время: вопрос, четыре варианта ответа и таймер",
        old_label: Some("Экзамен"),
        art: "quiz",
    },
    Item {
        label: "Тесты",
        subtitle: "проверка знаний",
        tooltip:
            "Тесты по разделам, общий экзамен и пробный бейт-дин: 10–50 вопросов разных форматов",
        old_label: None,
        art: "tests",
    },
    Item {
        label: "Успехи",
        subtitle: "уровни и статистика",
        tooltip: "Уровень, XP, серия дней, награды и статистика ответов",
        old_label: Some("Прогресс"),
        art: "progress",
    },
    Item {
        label: "Настройки",
        subtitle: "уровень, тема, шрифт",
        tooltip: "Уровень изучения, тема, размер шрифта, викторина и установка программы",
        old_label: None,
        art: "settings",
    },
    Item {
        label: "Источники",
        subtitle: "литература",
        tooltip: "Книги и сайты, на которые опираются уроки",
        old_label: None,
        art: "sources",
    },
];

/// The route an item opens.
pub fn route(index: usize) -> Route {
    match index {
        0 => Route::Map,
        1 => Route::Calendar,
        2 => Route::Cards(Deck::Review),
        3 => Route::Timed,
        4 => Route::Exam(None),
        5 => Route::Progress,
        6 => Route::Settings,
        _ => Route::Sources,
    }
}

/// Which item is highlighted for a route (`None` on the welcome screen).
pub fn group(r: &Route) -> Option<usize> {
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

/// Item index for a sidebar label, current or pre-v0.5 (case-insensitive).
pub fn find(label: &str) -> Option<usize> {
    let l = label.trim().to_lowercase();
    ITEMS.iter().position(|it| {
        it.label.to_lowercase() == l || it.old_label.is_some_and(|o| o.to_lowercase() == l)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_route_back_to_themselves() {
        for (i, it) in ITEMS.iter().enumerate() {
            assert_eq!(group(&route(i)), Some(i), "{}", it.label);
            assert_eq!(find(it.label), Some(i));
            assert!(!it.label.contains(' '), "one word: {}", it.label);
            for v in ["wood", "leather"] {
                for h in ["", "-hover"] {
                    let name = format!("icon-{}-{v}{h}", it.art);
                    assert!(crate::ui::assets::art(&name).is_some(), "{name}");
                }
            }
        }
        assert_eq!(find("Карта"), Some(0));
        assert_eq!(find("повторение"), Some(2));
        assert_eq!(find("Экзамен"), Some(3));
        assert_eq!(find("Прогресс"), Some(5));
        assert_eq!(find("Нет такого"), None);
    }
}
