//! Sidebar navigation items: one-word label, a grey one-line subtitle and a
//! fuller hover description. The single source of the names, so the shell,
//! tests and command control agree (old labels are kept as aliases).

use crate::i18n::{self, tr};
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
pub const ITEMS: [Item; 10] = [
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
        label: "Земля",
        subtitle: "Эрец-Исраэль",
        tooltip: "Карта всей Эрец-Исраэль: Иудея и Шомрон, Голан, Аза и единый Иерусалим, вечная неделимая столица. Слой «Границы по Торе»",
        old_label: None,
        art: "land",
    },
    Item {
        label: "Молитвы",
        subtitle: "на каждый день",
        tooltip: "Молитвы и благословения на каждый день: иврит, чтение русскими буквами и перевод, чтобы молиться и по-русски",
        old_label: None,
        art: "prayers",
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
        subtitle: "лестница Маалот",
        tooltip: "Викторина «Маалот»: 15 ступеней от лёгких вопросов к трудным, три подсказки, можно остановиться и сохранить пройденное",
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
        tooltip: "Уровень изучения, тема, размер шрифта, таймер Маалот и установка программы",
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
        2 => Route::Eretz,
        3 => Route::Prayers(None),
        4 => Route::Cards(Deck::Review),
        5 => Route::Timed,
        6 => Route::Exam(None),
        7 => Route::Progress,
        8 => Route::Settings,
        _ => Route::Sources,
    }
}

/// Which item is highlighted for a route (`None` on the welcome screen).
pub fn group(r: &Route) -> Option<usize> {
    Some(match r {
        Route::Map | Route::Section(_) | Route::Lesson(_) | Route::Interview(_) => 0,
        Route::Calendar => 1,
        Route::Eretz => 2,
        Route::Prayers(_) => 3,
        Route::Cards(Deck::Review) => 4,
        Route::Cards(_) => 0,
        Route::Timed | Route::TimedRun { .. } => 5,
        Route::Exam(_) | Route::ExamRun(_) => 6,
        Route::Progress => 7,
        Route::Settings | Route::Level { .. } => 8,
        Route::Sources => 9,
        Route::Welcome => return None,
    })
}

/// Item index for a sidebar label, current or pre-v0.5 (case-insensitive).
/// English names of the current labels match too; the stored Russian text
/// stays the lookup key, and pre-v0.5 aliases stay Russian so «Экзамен»
/// does not steal the `exam` route.
pub fn find(label: &str) -> Option<usize> {
    let l = label.trim().to_lowercase();
    ITEMS.iter().position(|it| {
        same(&l, it.label)
            || it.old_label.is_some_and(|o| same(&l, o))
            || i18n::en(it.label).is_some_and(|e| same(&l, e))
    })
}

fn same(input_lower: &str, candidate: &str) -> bool {
    input_lower == candidate.to_lowercase()
}

/// Sidebar label in the current language. The table itself stays Russian.
pub fn label(it: &Item) -> &'static str {
    tr(it.label)
}

/// Grey subtitle in the current language.
pub fn subtitle(it: &Item) -> &'static str {
    tr(it.subtitle)
}

/// Hover tooltip in the current language.
pub fn tooltip(it: &Item) -> &'static str {
    tr(it.tooltip)
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
        assert_eq!(find("Земля"), Some(2));
        assert_eq!(find("повторение"), Some(4));
        assert_eq!(find("Экзамен"), Some(5));
        assert_eq!(find("Прогресс"), Some(7));
        assert_eq!(find("молитвы"), Some(3));
        assert_eq!(find("Нет такого"), None);
    }
}
