//! Live 3D ritual objects (wgpu `shader` widget).
//!
//! The model is drawn offscreen (4× MSAA + depth) in `prepare` and composited
//! into iced's frame in `draw` as a premultiplied quad with a soft contact
//! shadow. Drag rotates it; when idle it turns slowly. Under the tiny-skia
//! renderer (no GPU) custom shaders are not drawn at all, so the caller keeps
//! a static WebP render underneath until [`gpu_ready`] turns true.

pub mod card;
mod glb;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};

use iced::mouse;
use iced::time::{Duration, Instant};
use iced::wgpu;
use iced::widget::image::Handle;
use iced::widget::shader::Viewport;
use iced::widget::shader::{self, Action};
use iced::{Element, Length, Point, Rectangle};

pub use glb::Mesh;

/// An embedded file under assets/3d. Each one sits behind its own `static`,
/// so the bytes exist once in the binary even when `glb()` / `still()` are
/// inlined into several codegen units (otherwise the Windows exe carried the
/// models twice).
macro_rules! asset {
    ($f:literal) => {{
        static BYTES: &[u8] = include_bytes!(concat!("../../../assets/3d/", $f));
        BYTES
    }};
}

/// Settings `extra` key «Объёмные модели» (absent = on).
pub const SETTING_KEY: &str = "models3d";

pub fn enabled(ctx: &crate::ui::app::Ctx) -> bool {
    ctx.state
        .settings
        .extra
        .get(SETTING_KEY)
        .and_then(|v| v.as_bool())
        .unwrap_or(true)
}

pub fn set_enabled(ctx: &mut crate::ui::app::Ctx, on: bool) {
    ctx.state
        .settings
        .extra
        .insert(SETTING_KEY.to_string(), serde_json::Value::Bool(on));
}

/// The embedded models (Tripo AI, decimated; see tools/3d).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Model {
    KiddushCup,
    ShabbatCandles,
    Menorah,
    TorahScroll,
    Shofar,
    Temple,
    MagenDavid,
    Hanukkiah,
    SederPlate,
    LulavEtrog,
    MegillatEsther,
    Sukkah,
    Tallit,
    Mezuzah,
    Siddur,
    Tefillin,
    Kotel,
    Chuppah,
    Ketubah,
    KosherDishes,
    WeddingGlass,
}

/// Card texts and view limits of one model.
pub struct Info {
    pub kicker: &'static str,
    pub title: &'static str,
    /// Always shown next to the model.
    pub lead: &'static str,
    /// Shown on a click on the model.
    pub caption: &'static str,
    /// Section the object belongs to (for the «object of the day» link).
    pub section: &'static str,
    pub section_title: &'static str,
    /// Wheel zoom ceiling (the Torah scroll stays far: its letters are
    /// decorative imitations and must not become readable).
    pub zoom_max: f32,
}

/// Bit per model for the per-model GPU-ready flags.
pub(crate) const ALL: [Model; 21] = [
    Model::KiddushCup,
    Model::ShabbatCandles,
    Model::Menorah,
    Model::TorahScroll,
    Model::Shofar,
    Model::Temple,
    Model::MagenDavid,
    Model::Hanukkiah,
    Model::SederPlate,
    Model::LulavEtrog,
    Model::MegillatEsther,
    Model::Sukkah,
    Model::Tallit,
    Model::Mezuzah,
    Model::Siddur,
    Model::Tefillin,
    Model::Kotel,
    Model::Chuppah,
    Model::Ketubah,
    Model::KosherDishes,
    Model::WeddingGlass,
];

enum Load {
    Loading,
    Ready(Arc<Mesh>),
    /// Failed to decode, or the forced software renderer (never drawn).
    Off,
}

fn slots() -> &'static std::sync::Mutex<HashMap<Model, Load>> {
    static S: OnceLock<std::sync::Mutex<HashMap<Model, Load>>> = OnceLock::new();
    S.get_or_init(Default::default)
}

impl Model {
    /// Objects that rotate as «Предмет дня» on «Уроки», one per day.
    pub const OF_THE_DAY: [Model; 21] = [
        Model::KiddushCup,
        Model::Menorah,
        Model::ShabbatCandles,
        Model::Shofar,
        Model::TorahScroll,
        Model::Temple,
        Model::MagenDavid,
        Model::Hanukkiah,
        Model::SederPlate,
        Model::LulavEtrog,
        Model::MegillatEsther,
        Model::Sukkah,
        Model::Tallit,
        Model::Mezuzah,
        Model::Siddur,
        Model::Tefillin,
        Model::Kotel,
        Model::Chuppah,
        Model::Ketubah,
        Model::KosherDishes,
        Model::WeddingGlass,
    ];

    pub fn of_the_day(day: chrono::NaiveDate) -> Model {
        use chrono::Datelike;
        let n = day
            .num_days_from_ce()
            .rem_euclid(Self::OF_THE_DAY.len() as i32);
        Self::OF_THE_DAY[n as usize]
    }

    /// Models shown on a section page, in order.
    pub fn for_section(id: &str) -> &'static [Model] {
        match id {
            "osnovy" => &[Model::TorahScroll, Model::Temple, Model::MagenDavid],
            "shabbat" => &[Model::ShabbatCandles, Model::KiddushCup],
            "ivrit" => &[
                Model::Siddur,
                Model::Tallit,
                Model::Tefillin,
                Model::Mezuzah,
            ],
            "prazdniki" => &[
                Model::Shofar,
                Model::LulavEtrog,
                Model::Sukkah,
                Model::Menorah,
                Model::Hanukkiah,
                Model::MegillatEsther,
                Model::SederPlate,
                Model::Temple,
                Model::Kotel,
            ],
            "kashrut" => &[Model::KosherDishes],
            "semya" => &[Model::Chuppah, Model::Ketubah, Model::WeddingGlass],
            _ => &[],
        }
    }

    fn bit(self) -> u32 {
        1 << ALL.iter().position(|m| *m == self).unwrap_or(0)
    }

    pub(crate) fn glb(self) -> &'static [u8] {
        match self {
            Model::KiddushCup => asset!("kiddush-cup.glb"),
            Model::ShabbatCandles => asset!("shabbat-candlesticks.glb"),
            Model::Menorah => asset!("menorah.glb"),
            Model::TorahScroll => asset!("torah-scroll.glb"),
            Model::Shofar => asset!("shofar.glb"),
            Model::Temple => asset!("temple-complex.glb"),
            Model::MagenDavid => asset!("magen-david.glb"),
            Model::Hanukkiah => asset!("hanukkiah.glb"),
            Model::SederPlate => asset!("seder-plate.glb"),
            Model::LulavEtrog => asset!("lulav-etrog.glb"),
            Model::MegillatEsther => asset!("megillat-esther.glb"),
            Model::Sukkah => asset!("sukkah.glb"),
            Model::Tallit => asset!("tallit.glb"),
            Model::Mezuzah => asset!("mezuzah.glb"),
            Model::Siddur => asset!("siddur.glb"),
            Model::Tefillin => asset!("tefillin.glb"),
            Model::Kotel => asset!("kotel.glb"),
            Model::Chuppah => asset!("chuppah.glb"),
            Model::Ketubah => asset!("ketubah.glb"),
            Model::KosherDishes => asset!("kosher-dishes.glb"),
            Model::WeddingGlass => asset!("wedding-glass.glb"),
        }
    }

    fn still(self) -> &'static [u8] {
        match self {
            Model::KiddushCup => asset!("kiddush-cup.webp"),
            Model::ShabbatCandles => asset!("shabbat-candlesticks.webp"),
            Model::Menorah => asset!("menorah.webp"),
            Model::TorahScroll => asset!("torah-scroll.webp"),
            Model::Shofar => asset!("shofar.webp"),
            Model::Temple => asset!("temple-complex.webp"),
            Model::MagenDavid => asset!("magen-david.webp"),
            Model::Hanukkiah => asset!("hanukkiah.webp"),
            Model::SederPlate => asset!("seder-plate.webp"),
            Model::LulavEtrog => asset!("lulav-etrog.webp"),
            Model::MegillatEsther => asset!("megillat-esther.webp"),
            Model::Sukkah => asset!("sukkah.webp"),
            Model::Tallit => asset!("tallit.webp"),
            Model::Mezuzah => asset!("mezuzah.webp"),
            Model::Siddur => asset!("siddur.webp"),
            Model::Tefillin => asset!("tefillin.webp"),
            Model::Kotel => asset!("kotel.webp"),
            Model::Chuppah => asset!("chuppah.webp"),
            Model::Ketubah => asset!("ketubah.webp"),
            Model::KosherDishes => asset!("kosher-dishes.webp"),
            Model::WeddingGlass => asset!("wedding-glass.webp"),
        }
    }

    pub fn info(self) -> Info {
        match self {
            Model::KiddushCup => Info {
                kicker: "Шаббат",
                title: "Бокал для кидуша",
                lead: "Кидуш — освящение субботы и праздников: благословение произносят над бокалом вина в начале трапезы.",
                caption: "Серебряный бокал с чеканным узором. Для кидуша его наполняют вином или виноградным соком; после благословения пьёт тот, кто произносил кидуш, и делится с сидящими за столом.",
                section: "shabbat",
                section_title: "Шаббат",
                zoom_max: 2.0,
            },
            Model::ShabbatCandles => Info {
                kicker: "Шаббат",
                title: "Субботние свечи",
                lead: "Перед наступлением Шаббата, до захода солнца, в доме зажигают свечи — не меньше двух.",
                caption: "Две свечи напоминают о двух словах заповеди о субботе: «Помни» (захор) и «Храни» (шамор). Обычно свечи зажигает хозяйка дома; зажгла — и для неё уже наступил Шаббат.",
                section: "shabbat",
                section_title: "Шаббат",
                zoom_max: 2.0,
            },
            Model::Menorah => Info {
                kicker: "Праздники · Ханука",
                title: "Менора Храма",
                lead: "Семисвечник из чистого золота, который зажигали в Храме. С ним связано чудо Хануки.",
                caption: "Это менора Храма — семь светильников, а не ханукия (у ханукии восемь светильников и шамаш). Когда Хасмонеи очистили Храм, нашёлся один кувшин чистого масла на день, а менора горела восемь дней. Поэтому Ханука длится восемь дней. Точную копию семисвечной меноры для дома по обычаю не делают.",
                section: "prazdniki",
                section_title: "Праздники",
                zoom_max: 2.0,
            },
            Model::TorahScroll => Info {
                kicker: "Основы",
                title: "Свиток Торы",
                lead: "Сефер Тора — Пятикнижие, написанное от руки писцом (сойфером) на пергаменте.",
                caption: "Свиток пишут гусиным пером на пергаменте из кожи кошерного животного, без единой ошибки. Его читают в синагоге в Шаббат, по понедельникам и четвергам, в праздники и новомесячья. Буквы на этой модели условные — это не настоящий текст.",
                section: "osnovy",
                section_title: "Основы",
                zoom_max: 1.15,
            },
            Model::Shofar => Info {
                kicker: "Праздники · Рош а-Шана",
                title: "Шофар",
                lead: "Рог, в который трубят в Рош а-Шана и в конце Йом Кипура.",
                caption: "Шофар делают из рога кошерного животного, лучше всего — барана, в память о баране, принесённом вместо Ицхака. Звуки шофара — ткиа, шварим и труа — будят сердце к тшуве.",
                section: "prazdniki",
                section_title: "Праздники",
                zoom_max: 2.0,
            },
            Model::Temple => Info {
                kicker: "Основы · История",
                title: "Второй Храм",
                lead: "Храм в Иерусалиме на Храмовой горе. Его перестроил царь Ирод, а римляне разрушили в 70 году (3830 год по еврейскому счёту), 9 ава — в Тиша бе-Ав.",
                caption: "В центре — Святилище (Эйхаль), где находилась Святая Святых. Перед ним — Двор священников с жертвенником, за ним Двор Израиля и Двор женщин. Всё это стояло на Храмовой горе, окружённой стенами. Это художественная реконструкция: детали приблизительные.",
                section: "osnovy",
                section_title: "Основы",
                zoom_max: 2.0,
            },
            Model::MagenDavid => Info {
                kicker: "Основы",
                title: "Маген Давид",
                lead: "Маген Давид — шестиконечная звезда, общепринятый знак еврейского народа.",
                caption: "Как общепринятый еврейский знак звезда закрепилась сравнительно недавно, в Новое время; сегодня её изображение стоит на флаге Государства Израиль. Это не заповедь Торы и не ритуальный предмет: у этого знака нет источника в Торе или в храмовой службе. «Маген Давид» — устоявшееся имя знака; достоверных сведений о том, что это был личный щит царя Давида, нет.",
                section: "osnovy",
                section_title: "Основы",
                zoom_max: 2.0,
            },
            Model::Hanukkiah => Info {
                kicker: "Праздники · Ханука",
                title: "Ханукия",
                lead: "Ханукия — светильник Хануки: восемь огней и шамаш, которым их зажигают.",
                caption: "У ханукии восемь светильников и ещё шамаш — служебный огонь, которым зажигают остальные. Это не семисвечная менора Храма. Принято каждый вечер добавлять по одному огню: в первый вечер зажигают один, в восьмой — все восемь.",
                section: "prazdniki",
                section_title: "Праздники",
                zoom_max: 2.0,
            },
            Model::SederPlate => Info {
                kicker: "Праздники · Песах",
                title: "Пасхальное блюдо",
                lead: "Кеара — специальное блюдо для седера, трапезы Песаха, на которое кладут символы праздника.",
                caption: "На кеару кладут зроа — запечённую кость, бейца — яйцо, марор — горькие травы, харосет — смесь фруктов, орехов и вина, и карпас — овощ, который обмакивают в солёную воду; часто добавляют ещё хазерет. К седеру также берут три мацы. Каждый предмет напоминает об Исходе из Египта. Расположение предметов в разных общинах бывает разным, поэтому на модели оно условное.",
                section: "prazdniki",
                section_title: "Праздники",
                zoom_max: 2.0,
            },
            Model::LulavEtrog => Info {
                kicker: "Праздники · Суккот",
                title: "Лулав и этрог",
                lead: "Четыре вида растений — арбаат ха-миним — берут в руки в праздник Суккот: лулав, хадас, арава и этрог.",
                caption: "В четыре вида входят лулав — нераскрывшаяся пальмовая ветвь, хадас — ветви мирта, арава — ветви ивы и этрог — плод цитрона. Лулав, хадас и арава обычно связывают в один пучок, а этрог держат отдельно. Все четыре вида берут вместе и потрясают в Суккот.",
                section: "prazdniki",
                section_title: "Праздники",
                zoom_max: 2.0,
            },
            Model::MegillatEsther => Info {
                kicker: "Праздники · Пурим",
                title: "Мегилат Эстер",
                lead: "Мегилат Эстер — свиток с историей Пурима, одна из книг Танаха; его читают в праздник вечером и утром.",
                caption: "Книга Эстер рассказывает, как Эстер и Мордехай спасли евреев от Амана. В Пурим свиток читают дважды: вечером и на следующее утро. Буквы на этой модели условные — это не настоящий текст.",
                section: "prazdniki",
                section_title: "Праздники",
                zoom_max: 1.15,
            },
            Model::Sukkah => Info {
                kicker: "Праздники · Суккот",
                title: "Сукка",
                lead: "Сукка — временная постройка под открытым небом; главная заповедь Суккота — жить в ней.",
                caption: "«Жить» в сукке значит прежде всего есть там, особенно хлеб и мучные блюда, а также проводить время и — кто может и в чьей общине так принято — спать. Сукка учит смирению и доверию: человек выходит из прочного дома под тонкую крышу и вспоминает, что его защищает не камень, а Всевышний. Перед едой произносят благословение «лешев ба-сукка». Украшения и устройство на этой модели условные.",
                section: "prazdniki",
                section_title: "Праздники",
                zoom_max: 2.0,
            },
            Model::Tallit => Info {
                kicker: "Иврит и молитва",
                title: "Талит",
                lead: "Талит (талит гадоль) — большое покрывало с цицит на четырёх углах, которое надевают во время утренней молитвы.",
                caption: "Цицит — кисти на четырёх углах одежды: «и увидите их, и вспомните все заповеди Господа, и исполните их» (Бемидбар 15:39). Талит гадоль надевают в Шахарит; талит катан носят весь день под одеждой. На этой модели — большой талит на подставке; узлы и обмотки цицит условные.",
                section: "ivrit",
                section_title: "Иврит и молитва",
                zoom_max: 2.0,
            },
            Model::Mezuzah => Info {
                kicker: "Иврит и молитва",
                title: "Мезуза",
                lead: "Мезуза — пергамент с двумя первыми отрывками Шма, свёрнутый в футляр и прикреплённый к дверному косяку.",
                caption: "Заповедь берётся из Шма: «И напиши их на дверных косяках дома твоего и на воротах твоих» (Дварим 6:9). Сойфер пишет на пергаменте (клафе) два отрывка: Дварим 6:4–9 и 11:13–21. Футляр крепят к правому косяку при входе: у ашкеназов наклонно, верхним концом внутрь помещения, у сефардов вертикально. На этой модели — футляр на отрезке косяка; самого пергамента не видно.",
                section: "ivrit",
                section_title: "Иврит и молитва",
                zoom_max: 2.0,
            },
            Model::Siddur => Info {
                kicker: "Иврит и молитва",
                title: "Сидур",
                lead: "Сидур — молитвенник на будни и Шаббат; само слово значит «порядок».",
                caption: "Сидур содержит установленный порядок ежедневных и шаббатних молитв. Для Рош а-Шана, Йом Кипура и паломнических праздников берут особый молитвенник — махзор. В разных общинах приняты свои нусахи — редакции текста и порядка. На этой модели — открытый сидур; буквы на страницах условные, это не настоящий текст молитвы.",
                section: "ivrit",
                section_title: "Иврит и молитва",
                zoom_max: 1.15,
            },
            Model::Tefillin => Info {
                kicker: "Иврит и молитва",
                title: "Тфилин",
                lead: "Тфилин — две чёрные кожаные коробочки с ремешками; внутри — четыре отрывка Торы.",
                caption: "Заповедь берётся из Шма: «и повяжи их как знак на руку твою, и будут они повязкой между глазами твоими» (Дварим 6:8). Тфилин шель яд надевают на бицепс слабой руки, напротив сердца; тфилин шель рош — на голову над линией волос. Их надевают в будни на утреннюю молитву; в Шаббат и праздники не надевают — эти дни сами «знак». На этой модели — головной тфилин (шель рош): коробочка (байт) на основании (титура). Буква шин на боковых гранях и ремешки на модели не показаны.",
                section: "ivrit",
                section_title: "Иврит и молитва",
                zoom_max: 2.0,
            },
            Model::Kotel => Info {
                kicker: "Праздники · Тиша бе-Ав",
                title: "Котель",
                lead: "Котель ха-Маарави — Западная стена Храмовой горы, место молитвы у сохранившейся стены Второго Храма.",
                caption: "Это подпорная стена Храмовой горы со времён Второго Храма; после разрушения Храма в 70 году (3830 год по еврейскому счёту) она стала главным местом молитвы у горы. В щели стены принято класть записки с просьбами. Разрушение Храма вспоминают в пост Тиша бе-Ав. На этой модели — небольшой фрагмент кладки с растениями в швах; это художественное приближение, а не точный вид стены, и записок на модели нет.",
                section: "prazdniki",
                section_title: "Праздники",
                zoom_max: 2.0,
            },
            Model::Chuppah => Info {
                kicker: "Жизненный цикл и семья",
                title: "Хупа",
                lead: "Хупа — свадебный балдахин, под которым проходит церемония еврейского брака.",
                caption: "Под хупой совершают кидушин и нисуин: благословения над вином, кольцо, кетуба. Часто полотно хупы — талит или белая ткань на четырёх шестах; под ним стоят жених и невеста. На этой модели — условная хупа (ткань с синими полосами, деревянные шесты, столик с бокалом); ткань тонкая, кисти неровные, устройство не притязает на точный ритуальный вид.",
                section: "semya",
                section_title: "Жизненный цикл и семья",
                zoom_max: 2.0,
            },
            Model::Ketubah => Info {
                kicker: "Жизненный цикл и семья",
                title: "Ктуба",
                lead: "Ктуба — брачный договор, который читают и передают невесте под хупой.",
                caption: "Ктуба закрепляет обязательства мужа перед женой: содержание, одежда и супружеские права, а также сумму, которая полагается жене в случае развода или смерти мужа. Её пишут до хупы, подписывают свидетели, читают под хупой и отдают невесте. Текст и украшения на этой модели условные: буквы не составляют настоящую ктубу, это не действительный документ.",
                section: "semya",
                section_title: "Жизненный цикл и семья",
                zoom_max: 1.15,
            },
            Model::KosherDishes => Info {
                kicker: "Кашрут",
                title: "Мясная и молочная посуда",
                lead: "В кашруте мясное (басар) и молочное (халав) не смешивают, и для них держат отдельную посуду.",
                caption: "Тора запрещает варить козлёнка в молоке его матери; отсюда закон не есть мясное с молочным и не готовить их вместе. Поэтому в кухне обычно две посуды: одна для мяса, другая для молока. Цвета на этой модели — условный учебный знак: красный набор (тарелка, пиала, вилка) обозначает мясное, синий (тарелка, пиала, ложка) — молочное. Так иногда помечают посуду для наглядности, но сам цвет не является законом. Наборы поставлены на одну доску только для показа; в доме их хранят отдельно.",
                section: "kashrut",
                section_title: "Кашрут",
                zoom_max: 2.0,
            },
            Model::WeddingGlass => Info {
                kicker: "Жизненный цикл и семья",
                title: "Свадебный бокал",
                lead: "Под хупой жених разбивает бокал в память о разрушенном Храме.",
                caption: "Даже в самый радостный день помнят о разрушении Храма в Иерусалиме. Жених наступает на бокал — обычно его заворачивают в салфетку или кладут в ткань — и разбивает его; гости отвечают «мазаль тов». На этой модели — бокал на деревянном подносе рядом со сложенной салфеткой. Стекло передано условно: блестящий бледно-голубой оттенок, без настоящей прозрачности.",
                section: "semya",
                section_title: "Жизненный цикл и семья",
                zoom_max: 2.0,
            },
        }
    }

    /// Kicker when the model is shown outside its home section.
    pub fn kicker_in(self, section: &str) -> Option<&'static str> {
        match (self, section) {
            (Model::Temple, "prazdniki") => Some("Праздники · Тиша бе-Ав"),
            _ => None,
        }
    }

    /// Exposure cut on the dark themes: the pale stone Temple glares on
    /// the dark background; the other models keep their light.
    fn dark_dim(self) -> f32 {
        match self {
            Model::Temple => 0.28,
            _ => 0.0,
        }
    }

    /// Start view (yaw, pitch): the buildings from a 3/4 aerial view, the
    /// objects from slightly above.
    fn view0(self) -> (f32, f32) {
        match self {
            Model::Temple => (YAW0, 0.62),
            // The plate is flat: a slightly higher view shows the wells and the foods.
            Model::SederPlate => (YAW0, 0.55),
            // The sukkah is a small building: a slightly higher view shows the schach and the table inside.
            Model::Sukkah => (YAW0, 0.48),
            // Open pages and the star on the cover: a slightly higher view than a standing object.
            Model::Siddur => (YAW0, 0.40),
            // The table and the hanging cloth are visible from a slightly higher view.
            Model::Chuppah => (YAW0, 0.40),
            // The bowls sit on a board: a higher view shows the insides and the utensils.
            Model::KosherDishes => (YAW0, 0.55),
            // Tray, napkin and glass are all visible from a slightly higher view.
            Model::WeddingGlass => (YAW0, 0.35),
            _ => (YAW0, PITCH0),
        }
    }

    /// Decoded mesh, or None while it is still loading / failed.
    fn mesh(self) -> Option<Arc<Mesh>> {
        match slots().lock().ok()?.get(&self) {
            Some(Load::Ready(m)) => Some(m.clone()),
            _ => None,
        }
    }

    /// Not decided yet: idle or decoding.
    fn loading(self) -> bool {
        slots()
            .lock()
            .map(|s| matches!(s.get(&self), None | Some(Load::Loading)))
            .unwrap_or(false)
    }

    /// Decode the GLB on a worker thread (textures + mips take ~0.2–0.5 s).
    /// Called from the card's view, so a model is decoded only when a
    /// screen that shows it is open.
    pub fn preload(self) {
        let Ok(mut s) = slots().lock() else { return };
        if s.contains_key(&self) {
            return;
        }
        // Forced software renderer: custom shaders never draw, so do not
        // spend the decode time and memory.
        if software_forced() {
            s.insert(self, Load::Off);
            return;
        }
        s.insert(self, Load::Loading);
        drop(s);
        std::thread::spawn(move || {
            let t = Instant::now();
            let load = match glb::parse(self.glb()) {
                Ok(m) => {
                    if stats_enabled() {
                        eprintln!(
                            "model3d: {:?} decoded in {:?} ({} verts, {} tris)",
                            self,
                            t.elapsed(),
                            m.vertices.len(),
                            m.indices.len() / 3
                        );
                    }
                    Load::Ready(Arc::new(m))
                }
                Err(e) => {
                    eprintln!("model3d: {self:?}: {e}");
                    Load::Off
                }
            };
            if let Ok(mut s) = slots().lock() {
                s.insert(self, load);
            }
        });
    }

    /// Static render (fallback for the tiny-skia renderer or when 3D is off).
    pub fn still_image(self) -> Handle {
        static CACHE: OnceLock<std::sync::Mutex<HashMap<Model, Handle>>> = OnceLock::new();
        let cache = CACHE.get_or_init(Default::default);
        let mut c = cache.lock().unwrap_or_else(|e| e.into_inner());
        c.entry(self)
            .or_insert_with(|| match image::load_from_memory(self.still()) {
                Ok(i) => {
                    let i = i.to_rgba8();
                    Handle::from_rgba(i.width(), i.height(), i.into_raw())
                }
                Err(_) => Handle::from_rgba(1, 1, vec![0; 4]),
            })
            .clone()
    }

    /// True once this model has been uploaded and drawn by wgpu.
    pub fn gpu_ready(self) -> bool {
        GPU_READY.load(Ordering::Relaxed) & self.bit() != 0
    }
}

/// Per-model bits: drawn by wgpu (see [`Model::gpu_ready`]).
static GPU_READY: AtomicU32 = AtomicU32::new(0);
/// `ICED_BACKEND` forces the tiny-skia renderer, where custom shaders never draw.
pub(crate) fn software_forced() -> bool {
    std::env::var("ICED_BACKEND")
        .map(|b| b.contains("tiny"))
        .unwrap_or(false)
}

/// Set by any `prepare` call: the wgpu renderer is active (tiny-skia never
/// calls it), even if the mesh is still decoding.
static WGPU_SEEN: AtomicBool = AtomicBool::new(false);
// Stats (SHORASHIM_3D_STATS=1): summed 3D-pass time incl. a GPU wait.
static PREP_NS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static PREP_N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn stats_enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| crate::util::env_hook("3D_STATS").is_some())
}

/// Screenshot hook `SHORASHIM_3D_YAW=<radians>`: fixed start angle, no spin.
fn fixed_yaw() -> Option<f32> {
    static Y: OnceLock<Option<f32>> = OnceLock::new();
    *Y.get_or_init(|| crate::util::env_hook("3D_YAW").and_then(|v| v.parse().ok()))
}

/// A rotatable model viewport. `on_ready` is published once when the GPU has
/// drawn it, so the caller can drop its static placeholder.
/// `on_click` (if any) is published on a single click (not a drag, not a
/// double click), e.g. to show a caption about the object.
pub fn view<'a, M: Clone + 'a>(
    model: Model,
    size: f32,
    warm: bool,
    on_ready: M,
    on_click: Option<M>,
) -> Element<'a, M> {
    model.preload();
    let still = model.still_image();
    let live = shader::Shader::new(Viewer {
        model,
        warm,
        on_ready,
        on_click,
    })
    .width(Length::Fixed(size))
    .height(Length::Fixed(size));
    // Same tree shape before and after the GPU is ready (only the bottom
    // layer changes), so the viewer keeps its angle and an ongoing drag
    // when the placeholder is dropped.
    let under: Element<'a, M> = if model.gpu_ready() {
        iced::widget::space()
            .width(Length::Fixed(size))
            .height(Length::Fixed(size))
            .into()
    } else {
        iced::widget::image(still)
            .width(Length::Fixed(size))
            .height(Length::Fixed(size))
            .into()
    };
    iced::widget::stack![under, live].into()
}

/// Static render only (3D switched off).
pub fn still<'a, M: 'a>(model: Model, size: f32) -> Element<'a, M> {
    iced::widget::image(model.still_image())
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .into()
}

struct Viewer<M> {
    model: Model,
    warm: bool,
    on_ready: M,
    on_click: Option<M>,
}

const YAW0: f32 = 0.7;
const PITCH0: f32 = 0.28;
const ZOOM_MIN: f32 = 0.8;
const DOUBLE_CLICK: Duration = Duration::from_millis(350);

pub struct State {
    yaw: f32,
    pitch: f32,
    zoom: f32,
    /// Smoothed towards `zoom_to` (wheel steps glide).
    zoom_to: f32,
    /// Hover glow 0..1, eased towards `hovered`.
    glow: f32,
    hovered: bool,
    drag: Option<(Point, f32, f32)>,
    /// Press position and time: a release close by is a click, not a drag.
    press: Option<(Point, Instant)>,
    last_move: Option<(Point, Instant)>,
    /// Angular velocity (yaw, pitch) in rad/s for the fling.
    vel: (f32, f32),
    /// A single click waits for the double-click window before it fires.
    pending_click: Option<Instant>,
    last_click: Option<Instant>,
    /// Double click: glide back to the initial view.
    resetting: bool,
    last: Option<Instant>,
    announced: bool,
    /// The model's start view was applied (State::default cannot know it).
    placed: bool,
    redraws: u32,
    // fps stats
    frames: u32,
    since: Option<Instant>,
}

impl Default for State {
    fn default() -> Self {
        State {
            yaw: fixed_yaw().unwrap_or(YAW0),
            pitch: PITCH0,
            zoom: 1.0,
            zoom_to: 1.0,
            glow: 0.0,
            hovered: false,
            drag: None,
            press: None,
            last_move: None,
            vel: (0.0, 0.0),
            pending_click: None,
            last_click: None,
            resetting: false,
            last: None,
            announced: false,
            placed: false,
            redraws: 0,
            frames: 0,
            since: None,
        }
    }
}

const IDLE_SPIN: f32 = 0.35; // rad/s

impl<M: Clone> shader::Program<M> for Viewer<M> {
    type State = State;
    type Primitive = Primitive;

    fn update(
        &self,
        st: &mut State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<M>> {
        let (yaw0, pitch0) = self.model.view0();
        if !st.placed {
            st.placed = true;
            st.yaw = fixed_yaw().unwrap_or(yaw0);
            st.pitch = pitch0;
        }
        let k = 3.2 / bounds.width.max(1.0) / st.zoom.sqrt();
        match event {
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let p = cursor.position_over(bounds)?;
                let now = Instant::now();
                st.drag = Some((p, st.yaw, st.pitch));
                st.press = Some((p, now));
                st.last_move = Some((p, now));
                st.vel = (0.0, 0.0);
                st.resetting = false;
                Some(Action::request_redraw().and_capture())
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { position }) => {
                let over = bounds.contains(*position);
                let hover_changed = over != st.hovered;
                st.hovered = over;
                let Some((start, yaw0, pitch0)) = st.drag else {
                    return hover_changed.then(Action::request_redraw);
                };
                st.yaw = yaw0 + (position.x - start.x) * k;
                st.pitch = (pitch0 + (position.y - start.y) * k).clamp(-0.5, 1.1);
                let now = Instant::now();
                if let Some((lp, lt)) = st.last_move {
                    let dt = (now - lt).as_secs_f32();
                    if dt > 0.004 {
                        // Light smoothing so one jittery event does not decide the fling.
                        let v = ((position.x - lp.x) * k / dt, (position.y - lp.y) * k / dt);
                        st.vel = (
                            (st.vel.0 * 0.4 + v.0 * 0.6).clamp(-10.0, 10.0),
                            (st.vel.1 * 0.4 + v.1 * 0.6).clamp(-4.0, 4.0),
                        );
                    }
                }
                st.last_move = Some((*position, now));
                Some(Action::request_redraw().and_capture())
            }
            iced::Event::Mouse(mouse::Event::CursorLeft) => {
                st.hovered = false;
                Some(Action::request_redraw())
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                st.drag?;
                st.drag = None;
                let now = Instant::now();
                // Released after holding still: no fling.
                if st
                    .last_move
                    .map(|(_, t)| now - t > Duration::from_millis(80))
                    .unwrap_or(true)
                {
                    st.vel = (0.0, 0.0);
                }
                let click = st.press.take().is_some_and(|(p, t)| {
                    let up = st.last_move.map(|(q, _)| q).unwrap_or(p);
                    (up.x - p.x).hypot(up.y - p.y) < 5.0 && now - t < Duration::from_millis(400)
                });
                if click {
                    st.vel = (0.0, 0.0);
                    if st.last_click.is_some_and(|t| now - t < DOUBLE_CLICK) {
                        // Double click: back to the initial view, no caption toggle.
                        st.last_click = None;
                        st.pending_click = None;
                        st.resetting = true;
                        st.zoom_to = 1.0;
                    } else {
                        st.last_click = Some(now);
                        st.pending_click = Some(now);
                    }
                }
                Some(Action::request_redraw().and_capture())
            }
            iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                cursor.position_over(bounds)?;
                let steps = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / 60.0,
                };
                let to =
                    (st.zoom_to * 1.15f32.powf(steps)).clamp(ZOOM_MIN, self.model.info().zoom_max);
                if (to - st.zoom_to).abs() < 1e-4 {
                    // At a limit: let the page scroll on.
                    return None;
                }
                st.zoom_to = to;
                st.resetting = false;
                Some(Action::request_redraw().and_capture())
            }
            iced::Event::Window(iced::window::Event::RedrawRequested(now)) => {
                let dt = st
                    .last
                    .map(|l| (*now - l).as_secs_f32().min(0.1))
                    .unwrap_or(0.0);
                st.last = Some(*now);
                if st.drag.is_none() {
                    if st.resetting {
                        // Glide to the start view along the shortest turn.
                        let tau = std::f32::consts::TAU;
                        let dy = ((yaw0 - st.yaw) % tau + tau + tau / 2.0) % tau - tau / 2.0;
                        let a = 1.0 - (-8.0 * dt).exp();
                        st.yaw += dy * a;
                        st.pitch += (pitch0 - st.pitch) * a;
                        if dy.abs() < 0.003 && (pitch0 - st.pitch).abs() < 0.003 {
                            st.yaw = yaw0;
                            st.pitch = pitch0;
                            st.resetting = false;
                        }
                    } else {
                        let spin = if fixed_yaw().is_none() && !st.hovered {
                            IDLE_SPIN
                        } else {
                            0.0
                        };
                        st.yaw += (spin + st.vel.0) * dt;
                        st.pitch = (st.pitch + st.vel.1 * dt).clamp(-0.5, 1.1);
                    }
                    // Inertia: the fling dies out in about a second.
                    let decay = (-2.5 * dt).exp();
                    st.vel = (st.vel.0 * decay, st.vel.1 * decay);
                }
                st.zoom += (st.zoom_to - st.zoom) * (1.0 - (-12.0 * dt).exp());
                let glow_to = if st.hovered || st.drag.is_some() {
                    1.0
                } else {
                    0.0
                };
                st.glow += (glow_to - st.glow) * (1.0 - (-10.0 * dt).exp());
                if stats_enabled() {
                    st.frames += 1;
                    let since = *st.since.get_or_insert(*now);
                    let el = (*now - since).as_secs_f32();
                    if el >= 5.0 {
                        let (ns, n) = (
                            PREP_NS.swap(0, Ordering::Relaxed),
                            PREP_N.swap(0, Ordering::Relaxed).max(1),
                        );
                        eprintln!(
                            "model3d: {:.1} fps over {:.1}s ({:.2} ms/frame); 3D pass avg {:.2} ms over {} renders",
                            st.frames as f32 / el,
                            el,
                            el * 1000.0 / st.frames as f32,
                            ns as f64 / n as f64 / 1e6,
                            n
                        );
                        st.frames = 0;
                        st.since = Some(*now);
                    }
                }
                if self.model.gpu_ready() && !st.announced {
                    st.announced = true;
                    return Some(Action::publish(self.on_ready.clone()));
                }
                if st.pending_click.is_some_and(|t| *now - t >= DOUBLE_CLICK) {
                    st.pending_click = None;
                    if let Some(m) = &self.on_click {
                        return Some(Action::publish(m.clone()));
                    }
                }
                // Keep animating only where it can be seen (wgpu); under
                // tiny-skia the static picture stays and nothing redraws.
                st.redraws = st.redraws.saturating_add(1);
                let loading = self.model.loading();
                let waiting_upload =
                    WGPU_SEEN.load(Ordering::Relaxed) && self.model.mesh().is_some();
                if self.model.gpu_ready() || loading || waiting_upload || st.redraws < 10 {
                    // ~60 fps while touched, flung, zooming or gliding back;
                    // ~30 fps for the idle spin.
                    let busy = st.drag.is_some()
                        || st.vel.0.abs() + st.vel.1.abs() > 0.05
                        || st.resetting
                        || (st.zoom_to - st.zoom).abs() > 0.002
                        || (st.glow > 0.01 && st.glow < 0.99);
                    let gap = if busy { 16 } else { 33 };
                    Some(Action::request_redraw_at(*now + Duration::from_millis(gap)))
                } else if st.pending_click.is_some() {
                    Some(Action::request_redraw_at(*now + DOUBLE_CLICK))
                } else {
                    // Decoded (or skipped) and no wgpu draw after the first
                    // frames: tiny-skia, so stop asking for frames.
                    None
                }
            }
            _ => None,
        }
    }

    fn draw(&self, st: &State, _cursor: mouse::Cursor, _bounds: Rectangle) -> Primitive {
        Primitive {
            model: self.model,
            yaw: st.yaw,
            pitch: st.pitch,
            zoom: st.zoom,
            glow: st.glow,
            warm: self.warm,
        }
    }

    fn mouse_interaction(
        &self,
        st: &State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if st.drag.is_some() {
            mouse::Interaction::Grabbing
        } else if cursor.is_over(bounds) {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::default()
        }
    }
}

#[derive(Debug)]
pub struct Primitive {
    model: Model,
    yaw: f32,
    pitch: f32,
    zoom: f32,
    glow: f32,
    warm: bool,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view: [f32; 4],
    fit: [f32; 4],
    base: [f32; 4],
    mat: [f32; 4],
    /// zoom, hover glow, exposure cut (dark themes), 0
    fx: [f32; 4],
}

struct GpuModel {
    vbuf: wgpu::Buffer,
    ibuf: wgpu::Buffer,
    count: u32,
    ubuf: wgpu::Buffer,
    bind: wgpu::BindGroup,
    mesh: Arc<Mesh>,
}

struct Target {
    size: (u32, u32),
    msaa: wgpu::TextureView,
    depth: wgpu::TextureView,
    resolve: wgpu::TextureView,
    blit: wgpu::BindGroup,
    shadow: wgpu::Buffer,
    drawn: Option<[u32; 5]>,
}

pub struct Pipeline {
    format: wgpu::TextureFormat,
    model_pipe: wgpu::RenderPipeline,
    blit_pipe: wgpu::RenderPipeline,
    model_bgl: wgpu::BindGroupLayout,
    blit_bgl: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    models: HashMap<Model, GpuModel>,
    targets: HashMap<Model, Target>,
    /// Which targets were used this frame (trim the rest).
    live: Vec<Model>,
}

/// 4× MSAA (guaranteed for these formats by WebGPU); `SHORASHIM_3D_MSAA=1`
/// turns it off for measurements.
fn samples() -> u32 {
    static S: OnceLock<u32> = OnceLock::new();
    *S.get_or_init(|| match crate::util::env_hook("3D_MSAA").as_deref() {
        Some("1") => 1,
        _ => 4,
    })
}
const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

fn shaders() -> (&'static str, &'static str) {
    const SRC: &str = include_str!("model.wgsl");
    SRC.split_once("// ---- blit").unwrap_or((SRC, ""))
}

impl shader::Pipeline for Pipeline {
    fn new(device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let (model_src, blit_src) = shaders();
        let model_mod = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("model3d"),
            source: wgpu::ShaderSource::Wgsl(model_src.into()),
        });
        let blit_mod = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("model3d blit"),
            source: wgpu::ShaderSource::Wgsl(format!("// ---- blit{blit_src}").into()),
        });
        let tex_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let uniform_entry = |binding, visibility| wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let sampler_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        };
        let model_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("model3d"),
            entries: &[
                uniform_entry(0, wgpu::ShaderStages::VERTEX_FRAGMENT),
                sampler_entry(1),
                tex_entry(2),
                tex_entry(3),
                tex_entry(4),
            ],
        });
        let blit_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("model3d blit"),
            entries: &[
                sampler_entry(0),
                tex_entry(1),
                uniform_entry(2, wgpu::ShaderStages::FRAGMENT),
            ],
        });
        let model_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("model3d"),
            bind_group_layouts: &[&model_bgl],
            push_constant_ranges: &[],
        });
        let blit_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("model3d blit"),
            bind_group_layouts: &[&blit_bgl],
            push_constant_ranges: &[],
        });
        let model_pipe = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("model3d"),
            layout: Some(&model_layout),
            vertex: wgpu::VertexState {
                module: &model_mod,
                entry_point: Some("vs_model"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<glb::Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &model_mod,
                entry_point: Some("fs_model"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: samples(),
                ..Default::default()
            },
            multiview: None,
            cache: None,
        });
        let blit_pipe = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("model3d blit"),
            layout: Some(&blit_layout),
            vertex: wgpu::VertexState {
                module: &blit_mod,
                entry_point: Some("vs_blit"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &blit_mod,
                entry_point: Some("fs_blit"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("model3d"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            anisotropy_clamp: 4,
            ..Default::default()
        });
        Pipeline {
            format,
            model_pipe,
            blit_pipe,
            model_bgl,
            blit_bgl,
            sampler,
            models: HashMap::new(),
            targets: HashMap::new(),
            live: Vec::new(),
        }
    }

    fn trim(&mut self) {
        let live = std::mem::take(&mut self.live);
        self.targets.retain(|m, _| live.contains(m));
        // Uploaded models stay cached for the whole run (the target PCs
        // have 16 GB+ RAM; twenty-one models, and up to eight on «Праздники»,
        // still fit in a few hundred MB on a real GPU).
    }
}

fn upload_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    tex: Option<&glb::Texture>,
    fallback: [u8; 4],
    srgb: bool,
) -> wgpu::TextureView {
    let one = glb::Texture {
        width: 1,
        height: 1,
        mips: vec![fallback.to_vec()],
    };
    let tex = tex.unwrap_or(&one);
    let t = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("model3d texture"),
        size: wgpu::Extent3d {
            width: tex.width,
            height: tex.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: tex.mips.len() as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: if srgb {
            wgpu::TextureFormat::Rgba8UnormSrgb
        } else {
            wgpu::TextureFormat::Rgba8Unorm
        },
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let (mut w, mut h) = (tex.width, tex.height);
    for (level, data) in tex.mips.iter().enumerate() {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &t,
                mip_level: level as u32,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * w),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        w = (w / 2).max(1);
        h = (h / 2).max(1);
    }
    t.create_view(&Default::default())
}

impl Pipeline {
    fn gpu_model(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, model: Model) -> bool {
        if self.models.contains_key(&model) {
            return true;
        }
        let Some(mesh) = model.mesh() else {
            return false;
        };
        use wgpu::util::DeviceExt;
        let vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("model3d vertices"),
            contents: bytemuck::cast_slice(&mesh.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("model3d indices"),
            contents: bytemuck::cast_slice(&mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let ubuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("model3d uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // Textures are taken on the first upload; a second pipeline (new
        // device) decodes them again from the embedded GLB.
        let tex = mesh
            .textures
            .lock()
            .ok()
            .and_then(|mut t| t.take())
            .or_else(|| glb::parse(model.glb()).ok()?.textures.into_inner().ok()?);
        let tex = tex.unwrap_or(glb::Textures {
            base_color: None,
            metal_rough: None,
            normal: None,
        });
        let base = upload_texture(device, queue, tex.base_color.as_ref(), [255; 4], true);
        let mr = upload_texture(device, queue, tex.metal_rough.as_ref(), [255; 4], false);
        let nrm = upload_texture(
            device,
            queue,
            tex.normal.as_ref(),
            [128, 128, 255, 255],
            false,
        );
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("model3d"),
            layout: &self.model_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ubuf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&base),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&mr),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&nrm),
                },
            ],
        });
        self.models.insert(
            model,
            GpuModel {
                vbuf,
                ibuf,
                count: mesh.indices.len() as u32,
                ubuf,
                bind,
                mesh,
            },
        );
        true
    }

    fn target(&mut self, device: &wgpu::Device, model: Model, size: (u32, u32)) -> &mut Target {
        if self
            .targets
            .get(&model)
            .map(|t| t.size != size)
            .unwrap_or(true)
        {
            let tex = |samples, format, usage| {
                device
                    .create_texture(&wgpu::TextureDescriptor {
                        label: Some("model3d target"),
                        size: wgpu::Extent3d {
                            width: size.0,
                            height: size.1,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: samples,
                        dimension: wgpu::TextureDimension::D2,
                        format,
                        usage,
                        view_formats: &[],
                    })
                    .create_view(&Default::default())
            };
            let msaa = tex(
                samples(),
                self.format,
                wgpu::TextureUsages::RENDER_ATTACHMENT,
            );
            let depth = tex(samples(), DEPTH, wgpu::TextureUsages::RENDER_ATTACHMENT);
            let resolve = tex(
                1,
                self.format,
                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            );
            let shadow = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("model3d shadow"),
                size: 16,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let blit = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("model3d blit"),
                layout: &self.blit_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&resolve),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: shadow.as_entire_binding(),
                    },
                ],
            });
            self.targets.insert(
                model,
                Target {
                    size,
                    msaa,
                    depth,
                    resolve,
                    blit,
                    shadow,
                    drawn: None,
                },
            );
        }
        self.targets.get_mut(&model).unwrap()
    }
}

impl shader::Primitive for Primitive {
    type Pipeline = Pipeline;

    fn prepare(
        &self,
        pipe: &mut Pipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: &Rectangle,
        viewport: &Viewport,
    ) {
        let t0 = Instant::now();
        WGPU_SEEN.store(true, Ordering::Relaxed);
        if !pipe.gpu_model(device, queue, self.model) {
            return;
        }
        let s = viewport.scale_factor();
        let size = (
            ((bounds.width * s).ceil() as u32).clamp(1, 4096),
            ((bounds.height * s).ceil() as u32).clamp(1, 4096),
        );
        let srgb = pipe.format.is_srgb();
        let (ubuf, mesh) = {
            let g = &pipe.models[&self.model];
            (g.ubuf.clone(), g.mesh.clone())
        };
        pipe.live.push(self.model);
        let target = pipe.target(device, self.model, size);
        let key = [
            self.yaw.to_bits(),
            self.pitch.to_bits(),
            self.zoom.to_bits(),
            (self.glow * 64.0).round() as u32,
            self.warm as u32,
        ];
        if target.drawn == Some(key) {
            return;
        }
        let inv_r = 1.0 / mesh.radius;
        let u = Uniforms {
            view: [
                self.yaw,
                self.pitch,
                size.0 as f32 / size.1 as f32,
                if srgb { 1.0 } else { 0.0 },
            ],
            fit: [mesh.center[0], mesh.center[1], mesh.center[2], inv_r],
            base: mesh.base_factor,
            mat: [
                mesh.metallic,
                mesh.roughness,
                if mesh.has_normal_map { 1.0 } else { 0.0 },
                if self.warm { 1.0 } else { 0.0 },
            ],
            fx: [
                self.zoom,
                self.glow,
                if self.warm {
                    self.model.dark_dim()
                } else {
                    0.0
                },
                0.0,
            ],
        };
        queue.write_buffer(&ubuf, 0, bytemuck::bytes_of(&u));
        // Contact shadow under the model's lowest point (projected roughly).
        let base_y = (mesh.min_y - mesh.center[1]) * inv_r * self.pitch.cos();
        let sy =
            0.5 - base_y * 3.6 * self.zoom / 4.2 * 0.5 * (size.0 as f32 / size.1 as f32).min(1.0);
        // Zoomed in so far that the foot leaves the frame: no shadow.
        let strength = if sy > 0.97 { 0.0 } else { 0.38 };
        queue.write_buffer(
            &target.shadow,
            0,
            bytemuck::cast_slice(&[sy, 0.30 * self.zoom, 0.06 * self.zoom, strength]),
        );
        let msaa = samples() > 1;
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("model3d"),
        });
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("model3d"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: if msaa { &target.msaa } else { &target.resolve },
                    depth_slice: None,
                    resolve_target: msaa.then_some(&target.resolve),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: if msaa {
                            wgpu::StoreOp::Discard
                        } else {
                            wgpu::StoreOp::Store
                        },
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &target.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            let g = &pipe.models[&self.model];
            pass.set_pipeline(&pipe.model_pipe);
            pass.set_bind_group(0, &g.bind, &[]);
            pass.set_vertex_buffer(0, g.vbuf.slice(..));
            pass.set_index_buffer(g.ibuf.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..g.count, 0, 0..1);
        }
        queue.submit([enc.finish()]);
        if let Some(t) = pipe.targets.get_mut(&self.model) {
            t.drawn = Some(key);
        }
        GPU_READY.fetch_or(self.model.bit(), Ordering::Relaxed);
        if stats_enabled() {
            // Stats mode waits for the GPU after every render, so the number
            // is the whole 3D pass (CPU + GPU), not just command recording.
            let _ = device.poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            });
            PREP_NS.fetch_add(t0.elapsed().as_nanos() as u64, Ordering::Relaxed);
            PREP_N.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn draw(&self, pipe: &Pipeline, pass: &mut wgpu::RenderPass<'_>) -> bool {
        let Some(t) = pipe.targets.get(&self.model) else {
            return true;
        };
        if t.drawn.is_none() {
            return true;
        }
        pass.set_pipeline(&pipe.blit_pipe);
        pass.set_bind_group(0, &t.blit, &[]);
        pass.draw(0..3, 0..1);
        true
    }
}

#[cfg(test)]
mod tests {
    use iced::wgpu::naga;

    /// Both WGSL halves parse and validate (catches shader errors in CI,
    /// which has no GPU).
    #[test]
    fn shaders_validate() {
        let (model, blit) = super::shaders();
        for (name, src) in [
            ("model", model.to_string()),
            ("blit", format!("// ---- blit{blit}")),
        ] {
            let module = naga::front::wgsl::parse_str(&src)
                .unwrap_or_else(|e| panic!("{name}: {}", e.emit_to_string(&src)));
            naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::empty(),
            )
            .validate(&module)
            .unwrap_or_else(|e| panic!("{name}: {e:?}"));
        }
    }
}
