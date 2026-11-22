//! Content access, level filtering, progress & unlocking ported from
//! js/content.js. Content is embedded at build time from content/*.json.

use std::collections::HashMap;

use chrono::NaiveDate;
use serde::Deserialize;

use crate::srs;
use crate::storage::{Level, State};

// ---- Serde types for the JSON schema (docs/CONTENT-SCHEMA.md) ----

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub content_version: Option<String>,
    #[serde(default)]
    pub review_status: Option<String>,
    #[serde(default)]
    pub sections: Vec<ManifestSection>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManifestSection {
    pub id: String,
    #[serde(default)]
    pub file: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Source {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub movement: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Differs {
    pub level: Level,
    pub text: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockCommon {
    #[serde(default)]
    pub min_level: Option<Level>,
    #[serde(default)]
    pub differs: Vec<Differs>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum Block {
    #[serde(rename = "h")]
    H {
        text: String,
        #[serde(flatten)]
        common: BlockCommon,
    },
    #[serde(rename = "p")]
    P {
        text: String,
        #[serde(flatten)]
        common: BlockCommon,
    },
    #[serde(rename = "list")]
    List {
        #[serde(default)]
        items: Vec<String>,
        #[serde(default)]
        ordered: bool,
        #[serde(flatten)]
        common: BlockCommon,
    },
    #[serde(rename = "hebrew")]
    Hebrew {
        he: String,
        #[serde(default)]
        translit: Option<String>,
        #[serde(default)]
        ru: Option<String>,
        #[serde(rename = "ref", default)]
        ref_: Option<String>,
        #[serde(flatten)]
        common: BlockCommon,
    },
    #[serde(rename = "quote")]
    Quote {
        text: String,
        #[serde(rename = "ref", default)]
        ref_: Option<String>,
        #[serde(flatten)]
        common: BlockCommon,
    },
    #[serde(rename = "note")]
    Note {
        text: String,
        #[serde(flatten)]
        common: BlockCommon,
    },
    #[serde(rename = "term")]
    Term {
        term: String,
        #[serde(default)]
        he: Option<String>,
        #[serde(default)]
        text: Option<String>,
        #[serde(flatten)]
        common: BlockCommon,
    },
    #[serde(other)]
    Unknown,
}

impl Block {
    pub fn min_level(&self) -> Option<Level> {
        match self {
            Block::H { common, .. }
            | Block::P { common, .. }
            | Block::List { common, .. }
            | Block::Hebrew { common, .. }
            | Block::Quote { common, .. }
            | Block::Note { common, .. }
            | Block::Term { common, .. } => common.min_level,
            Block::Unknown => None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    pub id: String,
    #[serde(default)]
    pub front: String,
    #[serde(default)]
    pub back: String,
    #[serde(default)]
    pub min_level: Option<Level>,
    #[serde(default)]
    pub differs: Vec<Differs>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lesson {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub min_level: Option<Level>,
    #[serde(default)]
    pub minutes: Option<i64>,
    #[serde(default)]
    pub source_refs: Vec<String>,
    #[serde(default)]
    pub citations: Vec<String>,
    #[serde(default)]
    pub blocks: Vec<Block>,
    #[serde(default)]
    pub cards: Vec<Card>,
    /// Lesson-level differences by movement (js/ui/lesson.js renders L.differs).
    #[serde(default)]
    pub differs: Vec<Differs>,
    /// «Главное»: 3–5 key ideas that restate the lesson (docs/STRUCTURE.md).
    #[serde(default)]
    pub key_ideas: Vec<String>,
}

/// Section overview shown at the top of the «Читать» mode (docs/STRUCTURE.md).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    /// Why this section matters for understanding Judaism.
    #[serde(default)]
    pub why: String,
    /// What the beit din usually checks on this topic.
    #[serde(default)]
    pub exam_focus: String,
    /// «После раздела вы сможете …»
    #[serde(default)]
    pub goals: Vec<String>,
}

/// «Углубиться»: further reading that points at one of the section sources.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Deeper {
    pub title: String,
    #[serde(default)]
    pub source_ref: Option<String>,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub min_level: Option<Level>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QCommon {
    pub id: String,
    #[serde(default)]
    pub lesson_id: Option<String>,
    #[serde(default)]
    pub min_level: Option<Level>,
    #[serde(default)]
    pub difficulty: Option<Difficulty>,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub explanation: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub differs: Vec<Differs>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "format")]
pub enum Question {
    #[serde(rename = "mc")]
    Mc {
        #[serde(flatten)]
        common: QCommon,
        #[serde(default)]
        options: Vec<String>,
        #[serde(default)]
        answer: usize,
    },
    #[serde(rename = "tf")]
    Tf {
        #[serde(flatten)]
        common: QCommon,
        #[serde(default)]
        answer: bool,
    },
    #[serde(rename = "match")]
    Match {
        #[serde(flatten)]
        common: QCommon,
        #[serde(default)]
        pairs: Vec<(String, String)>,
    },
    #[serde(rename = "typed")]
    Typed {
        #[serde(flatten)]
        common: QCommon,
        #[serde(default)]
        answer: String,
        #[serde(default)]
        accept: Vec<String>,
    },
    #[serde(rename = "open")]
    Open {
        #[serde(flatten)]
        common: QCommon,
        #[serde(default)]
        model_answer: String,
        #[serde(default)]
        key_points: Vec<String>,
    },
}

impl Question {
    pub fn common(&self) -> &QCommon {
        match self {
            Question::Mc { common, .. }
            | Question::Tf { common, .. }
            | Question::Match { common, .. }
            | Question::Typed { common, .. }
            | Question::Open { common, .. } => common,
        }
    }

    pub fn id(&self) -> &str {
        &self.common().id
    }

    /// Open (beit-din interview) question with a model answer.
    pub fn is_open(&self) -> bool {
        matches!(self, Question::Open { .. })
    }

    pub fn format(&self) -> &'static str {
        match self {
            Question::Mc { .. } => "mc",
            Question::Tf { .. } => "tf",
            Question::Match { .. } => "match",
            Question::Typed { .. } => "typed",
            Question::Open { .. } => "open",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Section {
    pub id: String,
    #[serde(default)]
    pub order: Option<i64>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub subtitle: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub intro: Option<String>,
    #[serde(default)]
    pub sources: Vec<Source>,
    #[serde(default)]
    pub lessons: Vec<Lesson>,
    #[serde(default)]
    pub questions: Vec<Question>,
    #[serde(default)]
    pub overview: Option<Overview>,
    #[serde(default)]
    pub deeper: Vec<Deeper>,
}

// ---- Level info (exact strings from js/content.js) ----

#[derive(Debug, Clone, Copy)]
pub struct LevelInfo {
    pub title: &'static str,
    pub short: &'static str,
    pub differs_label: &'static str,
    pub text: &'static str,
}

pub fn level_info(level: Level) -> &'static LevelInfo {
    match level {
        Level::Reform => &LevelInfo {
            title: "Реформистский",
            short: "Реформа",
            differs_label: "В реформистском иудаизме иначе:",
            text: "База: общее для всех течений — вера, история, праздники, этика и смысл заповедей.",
        },
        Level::Conservative => &LevelInfo {
            title: "Консервативный (Масорти)",
            short: "Масорти",
            differs_label: "В консервативном иудаизме иначе:",
            text: "Всё из базового уровня плюс то, что требуют Масорти: соблюдение Шаббата, кашрута, молитвы.",
        },
        Level::Orthodox => &LevelInfo {
            title: "Ортодоксальный",
            short: "Ортодоксия",
            differs_label: "В ортодоксальном иудаизме иначе:",
            text: "Полный уровень: включает всё предыдущее и подробную галахическую практику.",
        },
    }
}

/// Placeholder metadata for sections not yet present («Скоро»), js/content.js DEFAULT_SECTIONS.
pub struct SectionDef {
    pub id: &'static str,
    pub title: &'static str,
    pub subtitle: &'static str,
    pub icon: &'static str,
}

pub const DEFAULT_SECTIONS: &[SectionDef] = &[
    SectionDef {
        id: "osnovy",
        title: "Основы",
        subtitle: "Кто такой еврей, Тора и вера",
        icon: "📜",
    },
    SectionDef {
        id: "ivrit",
        title: "Иврит и молитва",
        subtitle: "Алфавит, Шма, Амида, сидур",
        icon: "א",
    },
    SectionDef {
        id: "shabbat",
        title: "Шаббат",
        subtitle: "Святость седьмого дня",
        icon: "🕯️",
    },
    SectionDef {
        id: "kashrut",
        title: "Кашрут",
        subtitle: "Законы пищи",
        icon: "🍞",
    },
    SectionDef {
        id: "prazdniki",
        title: "Праздники",
        subtitle: "Еврейский календарь",
        icon: "🌙",
    },
    SectionDef {
        id: "semya",
        title: "Жизненный цикл и семья",
        subtitle: "От брит-милы до траура",
        icon: "🏠",
    },
    SectionDef {
        id: "etika",
        title: "Этика",
        subtitle: "Пиркей Авот, цдака, речь",
        icon: "⚖️",
    },
    SectionDef {
        id: "beit-din",
        title: "Бейт-дин",
        subtitle: "Как проходит собеседование",
        icon: "🏛️",
    },
];

/// Passing score for section tests (js/content.js C.PASS).
pub const PASS: f64 = 80.0;

// ---- Library ----

#[derive(Debug, Clone, Copy)]
pub struct LessonLoc {
    pub sec: usize,
    pub lesson: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct CardLoc {
    pub sec: usize,
    pub lesson: usize,
    pub card: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct QLoc {
    pub sec: usize,
    pub q: usize,
}

pub struct SectionMeta {
    pub id: String,
    pub index: usize,
    pub data: Option<Section>,
    pub available: bool,
    pub title: String,
    pub subtitle: String,
    pub icon: String,
}

pub struct Library {
    pub manifest: Manifest,
    pub sections: Vec<SectionMeta>,
    card_ix: HashMap<String, CardLoc>,
    question_ix: HashMap<String, QLoc>,
    lesson_ix: HashMap<String, LessonLoc>,
}

impl Library {
    /// Load the content embedded at build time.
    pub fn embedded() -> Library {
        const MANIFEST: &str = include_str!("../content/manifest.json");
        const SECTIONS: &[&str] = &[
            include_str!("../content/sections/01-osnovy.json"),
            include_str!("../content/sections/02-ivrit.json"),
            include_str!("../content/sections/03-shabbat.json"),
            include_str!("../content/sections/04-kashrut.json"),
            include_str!("../content/sections/05-prazdniki.json"),
            include_str!("../content/sections/06-semya.json"),
            include_str!("../content/sections/07-etika.json"),
            include_str!("../content/sections/08-beit-din.json"),
        ];
        Library::from_json(MANIFEST, SECTIONS).expect("embedded content must parse")
    }

    /// Build from raw JSON strings (manifest + section files).
    pub fn from_json(manifest: &str, sections: &[&str]) -> Result<Library, serde_json::Error> {
        let manifest: Manifest = serde_json::from_str(manifest)?;
        let mut by_id: HashMap<String, Section> = HashMap::new();
        let mut bundle_order: Vec<String> = Vec::new();
        for raw in sections {
            let s: Section = serde_json::from_str(raw)?;
            bundle_order.push(s.id.clone());
            by_id.insert(s.id.clone(), s);
        }

        let mut order: Vec<String> = if manifest.sections.is_empty() {
            DEFAULT_SECTIONS.iter().map(|d| d.id.to_string()).collect()
        } else {
            manifest.sections.iter().map(|m| m.id.clone()).collect()
        };
        // Sections bundled but missing from the manifest keep bundle order
        // (js/content.js), so index/unlock chain are deterministic.
        for id in &bundle_order {
            if !order.contains(id) {
                order.push(id.clone());
            }
        }

        let mut metas = Vec::new();
        for (i, id) in order.iter().enumerate() {
            let data = by_id.remove(id);
            let def = DEFAULT_SECTIONS.iter().find(|d| d.id == id);
            let fallback_title = def.map(|d| d.title).unwrap_or(id.as_str()).to_string();
            let fallback_subtitle = def.map(|d| d.subtitle).unwrap_or("").to_string();
            let fallback_icon = def.map(|d| d.icon).unwrap_or("✡").to_string();
            let available = data.is_some();
            // JS: (data && data.title) || d.title — an empty string is missing.
            let non_empty = |s: &Option<String>| s.clone().filter(|s| !s.is_empty());
            metas.push(SectionMeta {
                id: id.clone(),
                index: i,
                title: data
                    .as_ref()
                    .and_then(|d| non_empty(&d.title))
                    .unwrap_or(fallback_title),
                subtitle: data
                    .as_ref()
                    .and_then(|d| non_empty(&d.subtitle))
                    .unwrap_or(fallback_subtitle),
                icon: data
                    .as_ref()
                    .and_then(|d| non_empty(&d.icon))
                    .unwrap_or(fallback_icon),
                available,
                data,
            });
        }

        let mut card_ix = HashMap::new();
        let mut question_ix = HashMap::new();
        let mut lesson_ix = HashMap::new();
        for (si, sec) in metas.iter().enumerate() {
            if let Some(data) = &sec.data {
                for (li, lesson) in data.lessons.iter().enumerate() {
                    lesson_ix.insert(
                        lesson.id.clone(),
                        LessonLoc {
                            sec: si,
                            lesson: li,
                        },
                    );
                    for (ci, card) in lesson.cards.iter().enumerate() {
                        card_ix.insert(
                            card.id.clone(),
                            CardLoc {
                                sec: si,
                                lesson: li,
                                card: ci,
                            },
                        );
                    }
                }
                for (qi, q) in data.questions.iter().enumerate() {
                    question_ix.insert(q.id().to_string(), QLoc { sec: si, q: qi });
                }
            }
        }

        Ok(Library {
            manifest,
            sections: metas,
            card_ix,
            question_ix,
            lesson_ix,
        })
    }

    // ---- Lookups ----

    pub fn section(&self, id: &str) -> Option<&SectionMeta> {
        self.sections.iter().find(|s| s.id == id)
    }

    pub fn card(&self, id: &str) -> Option<(&Card, &Lesson, &SectionMeta)> {
        let loc = self.card_ix.get(id)?;
        let sec = &self.sections[loc.sec];
        let lesson = &sec.data.as_ref()?.lessons[loc.lesson];
        Some((&lesson.cards[loc.card], lesson, sec))
    }

    pub fn question(&self, id: &str) -> Option<(&Question, &SectionMeta)> {
        let loc = self.question_ix.get(id)?;
        let sec = &self.sections[loc.sec];
        Some((&sec.data.as_ref()?.questions[loc.q], sec))
    }

    pub fn lesson(&self, id: &str) -> Option<(&Lesson, &SectionMeta)> {
        let loc = self.lesson_ix.get(id)?;
        let sec = &self.sections[loc.sec];
        Some((&sec.data.as_ref()?.lessons[loc.lesson], sec))
    }

    // ---- Level filtering ----

    /// js/content.js C.userLevel: null level → 3 (orthodox).
    pub fn user_level(state: &State) -> Level {
        state.settings.level.unwrap_or(Level::Orthodox)
    }

    /// js/content.js C.visible: item minLevel ≤ user level.
    pub fn visible(item: Option<Level>, inherited: Option<Level>, user: Level) -> bool {
        item.or(inherited).unwrap_or(Level::Reform).rank() <= user.rank()
    }

    /// Lessons of a section visible at the user level.
    pub fn lessons<'a>(&self, sec: &'a SectionMeta, user: Level) -> Vec<&'a Lesson> {
        match &sec.data {
            None => Vec::new(),
            Some(data) => data
                .lessons
                .iter()
                .filter(|l| Self::visible(l.min_level, None, user))
                .collect(),
        }
    }

    /// Cards of a lesson; cards inherit the lesson minLevel.
    pub fn lesson_cards<'a>(&self, lesson: &'a Lesson, user: Level) -> Vec<&'a Card> {
        lesson
            .cards
            .iter()
            .filter(|c| Self::visible(c.min_level, lesson.min_level, user))
            .collect()
    }

    pub fn section_cards<'a>(&self, sec: &'a SectionMeta, user: Level) -> Vec<&'a Card> {
        let mut out = Vec::new();
        for l in self.lessons(sec, user) {
            out.extend(self.lesson_cards(l, user));
        }
        out
    }

    /// Blocks of a lesson visible at the user level (blocks inherit lesson minLevel).
    pub fn lesson_blocks<'a>(&self, lesson: &'a Lesson, user: Level) -> Vec<&'a Block> {
        lesson
            .blocks
            .iter()
            .filter(|b| Self::visible(b.min_level(), lesson.min_level, user))
            .collect()
    }

    /// Questions of a section (js/content.js C.questions).
    pub fn questions<'a>(&self, sec: &'a SectionMeta, user: Level) -> Vec<&'a Question> {
        let Some(data) = &sec.data else {
            return Vec::new();
        };
        let lesson_visible: HashMap<&str, bool> = self
            .lessons(sec, user)
            .iter()
            .map(|l| (l.id.as_str(), true))
            .collect();
        data.questions
            .iter()
            .filter(|q| {
                let c = q.common();
                Self::visible(c.min_level, None, user)
                    && (match &c.lesson_id {
                        None => true,
                        Some(lid) => {
                            lesson_visible.contains_key(lid.as_str())
                                || !self.lesson_ix.contains_key(lid)
                        }
                    })
            })
            .collect()
    }

    /// Questions of one lesson visible at the user level, in bank order.
    pub fn lesson_questions<'a>(
        &self,
        sec: &'a SectionMeta,
        lesson_id: &str,
        user: Level,
    ) -> Vec<&'a Question> {
        self.questions(sec, user)
            .into_iter()
            .filter(|q| q.common().lesson_id.as_deref() == Some(lesson_id))
            .collect()
    }

    /// «Что спрашивают на бейт-дине»: the lesson's open (interview) questions.
    pub fn lesson_interview<'a>(
        &self,
        sec: &'a SectionMeta,
        lesson_id: &str,
        user: Level,
    ) -> Vec<&'a Question> {
        self.lesson_questions(sec, lesson_id, user)
            .into_iter()
            .filter(|q| q.is_open())
            .collect()
    }

    /// The section's open (interview) questions.
    pub fn section_interview<'a>(&self, sec: &'a SectionMeta, user: Level) -> Vec<&'a Question> {
        self.questions(sec, user)
            .into_iter()
            .filter(|q| q.is_open())
            .collect()
    }

    /// «Углубиться» items visible at the user level.
    pub fn deeper<'a>(&self, sec: &'a SectionMeta, user: Level) -> Vec<&'a Deeper> {
        match &sec.data {
            None => Vec::new(),
            Some(d) => d
                .deeper
                .iter()
                .filter(|x| Self::visible(x.min_level, None, user))
                .collect(),
        }
    }

    pub fn overview<'a>(&self, sec: &'a SectionMeta) -> Option<&'a Overview> {
        sec.data.as_ref().and_then(|d| d.overview.as_ref())
    }

    pub fn sources<'a>(&self, sec: &'a SectionMeta, ids: Option<&[String]>) -> Vec<&'a Source> {
        let list: Vec<&Source> = match &sec.data {
            Some(d) => d.sources.iter().collect(),
            None => Vec::new(),
        };
        match ids {
            None => list,
            Some(ids) => list.into_iter().filter(|s| ids.contains(&s.id)).collect(),
        }
    }

    pub fn source_title<'a>(&self, sec: &'a SectionMeta, id: &'a str) -> &'a str {
        self.sources(sec, None)
            .iter()
            .find(|s| s.id == id)
            .and_then(|s| s.title.as_deref())
            .unwrap_or(id)
    }

    // ---- Progress ----

    pub fn is_read(state: &State, lesson_id: &str) -> bool {
        state.lessons_read.contains_key(lesson_id)
    }

    pub fn best(state: &State, sec_id: &str) -> Option<f64> {
        state.best.get(sec_id).copied()
    }

    /// js/content.js C.sectionStats.
    pub fn section_stats(&self, sec: &SectionMeta, user: Level, state: &State) -> SectionStats {
        let lessons = self.lessons(sec, user);
        let cards = self.section_cards(sec, user);
        let read = lessons
            .iter()
            .filter(|l| Self::is_read(state, &l.id))
            .count();
        let mastered = cards
            .iter()
            .filter(|c| srs::is_mastered(state, &c.id))
            .count();
        let best = Self::best(state, &sec.id);
        let all_read = !lessons.is_empty() && read == lessons.len();
        let passed = best.map(|b| b >= PASS).unwrap_or(false);
        // Completion: 60% reading, 40% test (scaled to the 80% threshold).
        let pct = if !lessons.is_empty() {
            0.6 * read as f64 / lessons.len() as f64 + 0.4 * (best.unwrap_or(0.0) / PASS).min(1.0)
        } else {
            0.0
        };
        SectionStats {
            lessons: lessons.len(),
            read,
            cards: cards.len(),
            mastered,
            best,
            all_read,
            passed,
            complete: all_read && passed,
            pct: (pct * 100.0).round() as i64,
        }
    }

    /// Section N+1 unlocks when the previous *available* section is complete
    /// (js/content.js C.isUnlocked).
    pub fn is_unlocked(&self, sec: &SectionMeta, state: &State) -> bool {
        if !sec.available {
            return false;
        }
        if state.settings.unlock_all {
            return true;
        }
        let user = Self::user_level(state);
        for i in (0..sec.index).rev() {
            let prev = &self.sections[i];
            if prev.available {
                return self.section_stats(prev, user, state).complete;
            }
        }
        true // first available section
    }

    pub fn unlocked_sections<'a>(&'a self, state: &State) -> Vec<&'a SectionMeta> {
        self.sections
            .iter()
            .filter(|s| self.is_unlocked(s, state))
            .collect()
    }

    // ---- Daily review ----

    /// Cards due today across unlocked sections (js/content.js C.dueCards).
    pub fn due_cards<'a>(&'a self, state: &State, today: NaiveDate) -> Vec<&'a Card> {
        let user = Self::user_level(state);
        let mut out = Vec::new();
        for sec in self.unlocked_sections(state) {
            for c in self.section_cards(sec, user) {
                if srs::is_due(state, &c.id, today) {
                    out.push(c);
                }
            }
        }
        out
    }
}

pub struct SectionStats {
    pub lessons: usize,
    pub read: usize,
    pub cards: usize,
    pub mastered: usize,
    pub best: Option<f64>,
    pub all_read: bool,
    pub passed: bool,
    pub complete: bool,
    pub pct: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::srs;
    use chrono::NaiveDate;

    fn lib() -> Library {
        Library::embedded()
    }

    #[test]
    fn embedded_parses_and_has_questions_with_difficulty() {
        let lib = lib();
        assert_eq!(lib.sections.len(), 8);
        for sec in &lib.sections {
            assert!(sec.available, "section {} must be available", sec.id);
            let data = sec.data.as_ref().unwrap();
            assert!(
                !data.questions.is_empty(),
                "section {} has no questions",
                sec.id
            );
            for q in &data.questions {
                assert!(
                    q.common().difficulty.is_some(),
                    "question {} has no difficulty",
                    q.id()
                );
            }
            // Manifest order respected.
            assert_eq!(
                sec.index,
                lib.sections.iter().position(|s| s.id == sec.id).unwrap()
            );
        }
        // Manifest order matches the canonical default order.
        let ids: Vec<&str> = lib.sections.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(
            ids,
            vec![
                "osnovy",
                "ivrit",
                "shabbat",
                "kashrut",
                "prazdniki",
                "semya",
                "etika",
                "beit-din"
            ]
        );
    }

    #[test]
    fn level_filtering() {
        let lib = lib();
        let mut state = State::default();
        // Null level → orthodox (JS: GS.LEVELS[null] || 3).
        assert_eq!(Library::user_level(&state), Level::Orthodox);
        let sec = lib.section("osnovy").unwrap();
        let all: Vec<_> = lib.lessons(sec, Level::Orthodox);
        state.settings.level = Some(Level::Reform);
        let base: Vec<_> = lib.lessons(sec, Level::Reform);
        assert!(base.len() <= all.len());
        for l in &base {
            assert!(l.min_level.unwrap_or(Level::Reform).rank() <= 1);
        }
        // Cards inherit lesson minLevel.
        for l in &all {
            let cards = lib.lesson_cards(l, Level::Reform);
            for c in cards {
                let eff = c.min_level.or(l.min_level).unwrap_or(Level::Reform);
                assert!(eff.rank() <= 1);
            }
        }
    }

    #[test]
    fn question_index_and_lookup() {
        let lib = lib();
        let sec = lib.section("osnovy").unwrap();
        let q = &sec.data.as_ref().unwrap().questions[0];
        let (found, fsec) = lib.question(q.id()).unwrap();
        assert_eq!(found.id(), q.id());
        assert_eq!(fsec.id, "osnovy");
    }

    #[test]
    fn unlock_logic() {
        let lib = lib();
        let mut state = State::default();
        let first = &lib.sections[0];
        let second = &lib.sections[1];
        // First section open, second locked.
        assert!(lib.is_unlocked(first, &state));
        assert!(!lib.is_unlocked(second, &state));
        // Complete the first section: read all lessons + best ≥ 80.
        let user = Library::user_level(&state);
        for l in lib.lessons(first, user) {
            state
                .lessons_read
                .insert(l.id.clone(), "2026-11-22".to_string());
        }
        state.best.insert(first.id.clone(), 80.0);
        assert!(lib.is_unlocked(second, &state));
        // unlockAll overrides.
        let mut s2 = State::default();
        s2.settings.unlock_all = true;
        assert!(lib.is_unlocked(second, &s2));
    }

    #[test]
    fn level_change_never_resets_progress() {
        let lib = lib();
        let mut state = State::default();
        state
            .lessons_read
            .insert("x".to_string(), "2026-11-22".to_string());
        state.best.insert("osnovy".to_string(), 90.0);
        state.settings.level = Some(Level::Reform);
        let read_before = state.lessons_read.len();
        let best_before = state.best["osnovy"];
        // Switching level only changes filtering, not stored progress.
        state.settings.level = Some(Level::Orthodox);
        assert_eq!(state.lessons_read.len(), read_before);
        assert_eq!(state.best["osnovy"], best_before);
        let sec = lib.section("osnovy").unwrap();
        assert!(lib
            .section_stats(sec, Level::Orthodox, &state)
            .best
            .is_some());
    }

    #[test]
    fn section_stats_pct() {
        let lib = lib();
        let mut state = State::default();
        let sec = lib.section("osnovy").unwrap();
        let user = Level::Orthodox;
        let s0 = lib.section_stats(sec, user, &state);
        assert_eq!(s0.pct, 0);
        assert!(!s0.complete);
        let lessons = lib.lessons(sec, user);
        for l in &lessons {
            state
                .lessons_read
                .insert(l.id.clone(), "2026-11-22".to_string());
        }
        let s1 = lib.section_stats(sec, user, &state);
        assert!(s1.all_read);
        assert_eq!(s1.pct, 60); // 60% reading done, no test yet
        assert!(!s1.complete);
        state.best.insert(sec.id.clone(), 80.0);
        let s2 = lib.section_stats(sec, user, &state);
        assert!(s2.passed);
        assert!(s2.complete);
        assert_eq!(s2.pct, 100);
    }

    #[test]
    fn due_cards_uses_unlocked_sections() {
        let lib = lib();
        let mut state = State::default();
        let today = NaiveDate::from_ymd_opt(2026, 11, 22).unwrap();
        // Grade one card of the first section as due today.
        let sec = lib.section("osnovy").unwrap();
        let card = lib.section_cards(sec, Level::Orthodox)[0];
        let cid = card.id.clone();
        srs::grade(&mut state, &cid, false, today); // due tomorrow
        assert!(lib.due_cards(&state, today).is_empty());
        let tomorrow = NaiveDate::from_ymd_opt(2026, 11, 23).unwrap();
        let due = lib.due_cards(&state, tomorrow);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].id, cid);
    }

    #[test]
    fn level_info_strings() {
        assert_eq!(level_info(Level::Reform).title, "Реформистский");
        assert_eq!(level_info(Level::Conservative).short, "Масорти");
        assert_eq!(
            level_info(Level::Orthodox).differs_label,
            "В ортодоксальном иудаизме иначе:"
        );
    }

    #[test]
    fn from_json_tolerates_missing_optional_fields() {
        let manifest = r#"{"sections":[{"id":"x","file":"sections/x.json"}]}"#;
        let section = r#"{
            "id": "x",
            "lessons": [{"id": "x-1", "blocks": [
                {"type": "p", "text": "Абзац"},
                {"type": "something-new", "foo": 1}
            ], "cards": [{"id": "x-1-c1"}]}],
            "questions": [
                {"id": "x-q1", "format": "tf", "difficulty": "easy", "answer": true},
                {"id": "x-q2", "format": "open", "difficulty": "hard"}
            ]
        }"#;
        let lib = Library::from_json(manifest, &[section]).unwrap();
        assert_eq!(lib.sections.len(), 1);
        let sec = &lib.sections[0];
        assert!(sec.available);
        assert_eq!(sec.title, "x"); // fallback to id when no default def
        assert_eq!(sec.icon, "✡");
        let lessons = lib.lessons(sec, Level::Orthodox);
        assert_eq!(lessons.len(), 1);
        assert_eq!(lessons[0].blocks.len(), 2);
        assert_eq!(lib.questions(sec, Level::Orthodox).len(), 2);
    }

    #[test]
    fn unlisted_sections_keep_bundle_order() {
        // Bundled sections missing from the manifest are appended in the order
        // of the input slice, not in HashMap order (js/content.js).
        let manifest = r#"{"sections":[{"id":"b"}]}"#;
        let sec = |id: &str| format!(r#"{{"id":"{id}"}}"#);
        let owned: Vec<String> = ["z1", "b", "z2", "z3"].iter().map(|id| sec(id)).collect();
        let refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
        let lib = Library::from_json(manifest, &refs).unwrap();
        let ids: Vec<&str> = lib.sections.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, vec!["b", "z1", "z2", "z3"]);
        assert_eq!(lib.sections[1].index, 1);
    }

    #[test]
    fn empty_title_falls_back_like_js() {
        // JS: (data && data.title) || d.title — "" counts as missing.
        let manifest = r#"{"sections":[{"id":"osnovy"},{"id":"x"}]}"#;
        let section = r#"{"id":"osnovy","title":"","subtitle":"","icon":""}"#;
        let lib = Library::from_json(manifest, &[section]).unwrap();
        let sec = lib.section("osnovy").unwrap();
        assert_eq!(sec.title, "Основы");
        assert_eq!(sec.subtitle, "Кто такой еврей, Тора и вера");
        assert_eq!(sec.icon, "📜");
    }

    #[test]
    fn lesson_differs_parsed() {
        let manifest = r#"{"sections":[{"id":"x"}]}"#;
        let section = r#"{
            "id": "x",
            "lessons": [{
                "id": "x-1",
                "differs": [{"level": "reform", "text": "Иначе в реформе"}]
            }]
        }"#;
        let lib = Library::from_json(manifest, &[section]).unwrap();
        let (lesson, _) = lib.lesson("x-1").unwrap();
        assert_eq!(lesson.differs.len(), 1);
        assert_eq!(lesson.differs[0].level, Level::Reform);
        assert_eq!(lesson.differs[0].text, "Иначе в реформе");
    }
}
