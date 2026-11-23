//! Eretz Israel map data: «Израиль сегодня», Torah-border overlays, tribal
//! allotments and place cards.
//!
//! The base view follows official Israeli maps (Survey of Israel, the MFA):
//! united Jerusalem and the Golan inside Israel; Judea and Samaria labelled
//! «Иудея и Шомрон» with the 1949 armistice line as a thin dashed line, the
//! way Israeli historical and MFA maps mark it; Aza outside Israel's borders
//! since the 2005 disengagement. Neighbouring land is drawn in a neutral
//! colour so the coast and the Gulf read like a real map. The «Границы по
//! Торе» layer adds the promised extent (Bereshit 15:18, Devarim 11:24) and the
//! Canaan bounds of Bamidbar 34. Tribal allotments follow Yehoshua 13–19 as a
//! schematic teaching overlay, not a surveyed cadastre. Sources:
//! docs/RABBI-REVIEW.md, «Карта: Израиль сегодня».

/// A geographic point, `lon` degrees east, `lat` degrees north.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coord {
    pub lon: f32,
    pub lat: f32,
}

impl Coord {
    pub const fn new(lon: f32, lat: f32) -> Self {
        Self { lon, lat }
    }
}

/// A named closed ring (first point is not repeated).
#[derive(Debug, Clone, Copy)]
pub struct Ring {
    pub name: &'static str,
    pub pts: &'static [Coord],
}

/// Visible map extent in lon/lat, used by the projector.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Extent {
    pub west: f32,
    pub east: f32,
    pub south: f32,
    pub north: f32,
}

impl Extent {
    /// Tight view of the land itself (with a little sea and Sinai margin).
    pub const LAND: Self = Self {
        west: 34.00,
        east: 36.05,
        south: 29.35,
        north: 33.48,
    };

    /// Wider view that fits the covenant borders (Brook of Egypt to the Euphrates).
    pub const TORAH: Self = Self {
        west: 32.6,
        east: 42.0,
        south: 28.7,
        north: 37.3,
    };

    /// Mid view for tribal allotments: the land plus Transjordan, without the
    /// full stretch to the Euphrates, so the tribes stay readable.
    pub const TRIBES: Self = Self {
        west: 33.80,
        east: 36.90,
        south: 29.25,
        north: 33.90,
    };

    pub fn mid_lat(self) -> f32 {
        (self.south + self.north) * 0.5
    }

    /// Width in «projected» units (lon scaled by cos of mid-lat).
    pub fn proj_width(self) -> f32 {
        (self.east - self.west) * self.mid_lat().to_radians().cos()
    }

    pub fn proj_height(self) -> f32 {
        self.north - self.south
    }
}

/// Israel today as one polygon, clockwise from Kerem Shalom. Judea and
/// Samaria, the Golan and a united Jerusalem are inside the ring; Aza is not
/// (see [`AZA`]).
pub const LAND: &[Coord] = &[
    // Along Aza, Kerem Shalom → Zikim
    Coord::new(34.267, 31.220),
    Coord::new(34.300, 31.260),
    Coord::new(34.380, 31.310),
    Coord::new(34.450, 31.370),
    Coord::new(34.495, 31.420),
    Coord::new(34.540, 31.490),
    Coord::new(34.567, 31.540),
    Coord::new(34.525, 31.595),
    // Mediterranean coast, Zikim → Rosh HaNikra
    Coord::new(34.545, 31.660),
    Coord::new(34.570, 31.700),
    Coord::new(34.620, 31.800),
    Coord::new(34.655, 31.880),
    Coord::new(34.695, 31.960),
    Coord::new(34.743, 32.033),
    Coord::new(34.762, 32.085),
    Coord::new(34.785, 32.160),
    Coord::new(34.820, 32.240),
    Coord::new(34.855, 32.320),
    Coord::new(34.870, 32.400),
    Coord::new(34.890, 32.500),
    Coord::new(34.920, 32.620),
    Coord::new(34.945, 32.720),
    Coord::new(34.965, 32.800),
    Coord::new(34.985, 32.830),
    Coord::new(35.000, 32.845),
    Coord::new(35.030, 32.860),
    Coord::new(35.070, 32.920),
    Coord::new(35.085, 32.980),
    Coord::new(35.095, 33.020),
    Coord::new(35.104, 33.086),
    // Northern border to Hermon and the Golan
    Coord::new(35.150, 33.090),
    Coord::new(35.220, 33.100),
    Coord::new(35.300, 33.115),
    Coord::new(35.380, 33.130),
    Coord::new(35.450, 33.155),
    Coord::new(35.520, 33.200),
    Coord::new(35.544, 33.270),
    Coord::new(35.580, 33.248),
    Coord::new(35.650, 33.250),
    Coord::new(35.720, 33.270),
    Coord::new(35.785, 33.300),
    Coord::new(35.820, 33.285),
    Coord::new(35.850, 33.240),
    // Eastern Golan, then south to the Kinneret and the Jordan
    Coord::new(35.860, 33.180),
    Coord::new(35.855, 33.120),
    Coord::new(35.850, 33.050),
    Coord::new(35.840, 32.980),
    Coord::new(35.820, 32.900),
    Coord::new(35.790, 32.820),
    Coord::new(35.750, 32.750),
    Coord::new(35.700, 32.710),
    Coord::new(35.650, 32.700),
    Coord::new(35.620, 32.695),
    Coord::new(35.595, 32.640),
    Coord::new(35.585, 32.560),
    Coord::new(35.575, 32.480),
    Coord::new(35.560, 32.400),
    Coord::new(35.555, 32.320),
    Coord::new(35.548, 32.240),
    Coord::new(35.550, 32.160),
    Coord::new(35.555, 32.080),
    Coord::new(35.560, 32.000),
    Coord::new(35.557, 31.920),
    Coord::new(35.555, 31.850),
    Coord::new(35.550, 31.780),
    // Western shore of the Salt Sea, then the Arava to Eilat
    Coord::new(35.505, 31.760),
    Coord::new(35.490, 31.700),
    Coord::new(35.450, 31.600),
    Coord::new(35.420, 31.500),
    Coord::new(35.400, 31.400),
    Coord::new(35.385, 31.300),
    Coord::new(35.375, 31.200),
    Coord::new(35.370, 31.130),
    Coord::new(35.360, 31.080),
    Coord::new(35.300, 30.980),
    Coord::new(35.200, 30.850),
    Coord::new(35.170, 30.700),
    Coord::new(35.150, 30.550),
    Coord::new(35.130, 30.400),
    Coord::new(35.100, 30.250),
    Coord::new(35.050, 30.050),
    Coord::new(35.000, 29.800),
    Coord::new(34.980, 29.650),
    Coord::new(34.951, 29.550),
    // Egyptian border, Eilat → Rafah
    Coord::new(34.900, 29.500),
    Coord::new(34.850, 29.550),
    Coord::new(34.780, 29.650),
    Coord::new(34.700, 29.800),
    Coord::new(34.620, 29.950),
    Coord::new(34.550, 30.150),
    Coord::new(34.500, 30.300),
    Coord::new(34.460, 30.500),
    Coord::new(34.420, 30.700),
    Coord::new(34.380, 30.850),
    Coord::new(34.330, 31.000),
    Coord::new(34.270, 31.120),
];

/// Aza, outside Israel's borders since 2005: Rafah → Zikim along the coast,
/// back along the border to Kerem Shalom.
pub const AZA: &[Coord] = &[
    Coord::new(34.219, 31.217),
    Coord::new(34.230, 31.250),
    Coord::new(34.248, 31.296),
    Coord::new(34.270, 31.330),
    Coord::new(34.305, 31.380),
    Coord::new(34.380, 31.470),
    Coord::new(34.430, 31.520),
    Coord::new(34.490, 31.580),
    Coord::new(34.525, 31.595),
    Coord::new(34.567, 31.540),
    Coord::new(34.540, 31.490),
    Coord::new(34.495, 31.420),
    Coord::new(34.450, 31.370),
    Coord::new(34.380, 31.310),
    Coord::new(34.300, 31.260),
    Coord::new(34.267, 31.220),
];

/// The 1949 armistice line around Judea and Samaria, north part (Jordan →
/// the Latrun area → the edge of Jerusalem). Drawn thin and dashed; it stops
/// at Jerusalem's municipal edge, because the city is one.
pub const ARMISTICE_1949_NORTH: &[Coord] = &[
    Coord::new(35.555, 32.400),
    Coord::new(35.480, 32.410),
    Coord::new(35.400, 32.500),
    Coord::new(35.300, 32.530),
    Coord::new(35.210, 32.550),
    Coord::new(35.100, 32.500),
    Coord::new(35.070, 32.430),
    Coord::new(35.020, 32.360),
    Coord::new(35.030, 32.280),
    Coord::new(35.000, 32.220),
    Coord::new(34.960, 32.170),
    Coord::new(34.990, 32.100),
    Coord::new(35.000, 32.000),
    Coord::new(34.980, 31.930),
    Coord::new(34.970, 31.860),
    Coord::new(35.020, 31.830),
    Coord::new(35.080, 31.850),
    Coord::new(35.130, 31.840),
    Coord::new(35.165, 31.815),
];

/// South part: from the edge of Jerusalem around the Hebron hills to the
/// Salt Sea.
pub const ARMISTICE_1949_SOUTH: &[Coord] = &[
    Coord::new(35.150, 31.740),
    Coord::new(35.100, 31.720),
    Coord::new(35.030, 31.690),
    Coord::new(34.970, 31.620),
    Coord::new(34.920, 31.530),
    Coord::new(34.880, 31.420),
    Coord::new(34.920, 31.360),
    Coord::new(35.020, 31.350),
    Coord::new(35.150, 31.350),
    Coord::new(35.280, 31.360),
    Coord::new(35.400, 31.400),
    Coord::new(35.450, 31.490),
];

/// Neighbouring land, drawn in a neutral colour under Israel. Its western
/// edge is the real coast north of Rosh HaNikra and west of Rafah; along
/// Israel it runs inland, where Israel's own fill covers it.
pub const NEIGHBOURS: &[Coord] = &[
    Coord::new(34.219, 31.217),
    Coord::new(34.400, 31.300),
    Coord::new(34.800, 31.400),
    Coord::new(35.000, 32.000),
    Coord::new(35.100, 32.700),
    Coord::new(35.150, 33.000),
    Coord::new(35.104, 33.086),
    // Lebanon and Syria coast
    Coord::new(35.200, 33.270),
    Coord::new(35.370, 33.560),
    Coord::new(35.480, 33.900),
    Coord::new(35.650, 34.250),
    Coord::new(35.850, 34.450),
    Coord::new(35.980, 34.800),
    Coord::new(35.880, 35.200),
    Coord::new(35.780, 35.550),
    Coord::new(35.950, 35.900),
    Coord::new(35.850, 36.300),
    Coord::new(36.150, 36.600),
    // Anatolian coast, westward
    Coord::new(35.800, 36.800),
    Coord::new(35.300, 36.600),
    Coord::new(34.600, 36.800),
    Coord::new(33.900, 36.250),
    Coord::new(33.200, 36.150),
    Coord::new(32.000, 36.100),
    Coord::new(31.000, 36.600),
    Coord::new(30.600, 36.850),
    Coord::new(30.000, 36.300),
    Coord::new(29.000, 36.600),
    Coord::new(28.000, 36.800),
    Coord::new(27.300, 37.200),
    // Far frame, then Africa and Sinai coast back to Rafah
    Coord::new(27.300, 45.000),
    Coord::new(70.000, 45.000),
    Coord::new(70.000, 5.000),
    Coord::new(10.000, 5.000),
    Coord::new(10.000, 33.000),
    Coord::new(20.000, 31.400),
    Coord::new(30.000, 31.400),
    Coord::new(31.900, 31.500),
    Coord::new(32.300, 31.250),
    Coord::new(32.600, 31.070),
    Coord::new(33.100, 31.080),
    Coord::new(33.500, 31.120),
    Coord::new(33.800, 31.140),
    Coord::new(34.000, 31.170),
];

/// Cyprus, a rough outline for the wide Torah view.
pub const CYPRUS: &[Coord] = &[
    Coord::new(32.300, 35.050),
    Coord::new(32.600, 35.200),
    Coord::new(33.000, 35.350),
    Coord::new(33.600, 35.350),
    Coord::new(34.100, 35.500),
    Coord::new(34.600, 35.700),
    Coord::new(34.000, 35.300),
    Coord::new(33.900, 35.000),
    Coord::new(33.600, 34.950),
    Coord::new(33.000, 34.650),
    Coord::new(32.400, 34.750),
];

/// The Gulf of Eilat (Aqaba), from Eilat south to the Straits of Tiran.
pub const GULF_EILAT: &[Coord] = &[
    Coord::new(34.880, 29.500),
    Coord::new(34.951, 29.550),
    Coord::new(35.010, 29.520),
    Coord::new(34.920, 29.100),
    Coord::new(34.830, 28.700),
    Coord::new(34.680, 28.200),
    Coord::new(34.450, 27.900),
    Coord::new(34.300, 27.950),
    Coord::new(34.450, 28.400),
    Coord::new(34.620, 28.900),
    Coord::new(34.750, 29.300),
];

/// The Gulf of Suez, at the western edge of the Torah view.
pub const GULF_SUEZ: &[Coord] = &[
    Coord::new(32.550, 29.970),
    Coord::new(32.750, 29.650),
    Coord::new(33.200, 28.900),
    Coord::new(33.600, 28.200),
    Coord::new(33.900, 27.900),
    Coord::new(33.550, 27.800),
    Coord::new(32.900, 28.800),
    Coord::new(32.400, 29.600),
    Coord::new(32.400, 29.970),
];

/// Yam Kinneret (the Sea of Galilee), clockwise.
pub const KINNERET: &[Coord] = &[
    Coord::new(35.580, 32.905),
    Coord::new(35.630, 32.880),
    Coord::new(35.650, 32.830),
    Coord::new(35.640, 32.775),
    Coord::new(35.600, 32.720),
    Coord::new(35.560, 32.705),
    Coord::new(35.525, 32.735),
    Coord::new(35.515, 32.790),
    Coord::new(35.530, 32.855),
    Coord::new(35.555, 32.895),
];

/// Yam ha-Melach (the Salt Sea / Dead Sea), clockwise, with the Lisan pinch.
pub const SALT_SEA: &[Coord] = &[
    Coord::new(35.510, 31.765),
    Coord::new(35.575, 31.720),
    Coord::new(35.595, 31.600),
    Coord::new(35.590, 31.450),
    Coord::new(35.560, 31.320),
    Coord::new(35.505, 31.230),
    Coord::new(35.470, 31.200),
    Coord::new(35.415, 31.220),
    Coord::new(35.400, 31.280),
    Coord::new(35.410, 31.350),
    Coord::new(35.430, 31.450),
    Coord::new(35.455, 31.580),
    Coord::new(35.485, 31.700),
];

/// The Jordan, from the southern tip of the Kinneret to the Salt Sea.
pub const JORDAN: &[Coord] = &[
    Coord::new(35.585, 32.705),
    Coord::new(35.595, 32.640),
    Coord::new(35.585, 32.560),
    Coord::new(35.575, 32.480),
    Coord::new(35.560, 32.400),
    Coord::new(35.555, 32.320),
    Coord::new(35.548, 32.240),
    Coord::new(35.550, 32.160),
    Coord::new(35.555, 32.080),
    Coord::new(35.560, 32.000),
    Coord::new(35.557, 31.920),
    Coord::new(35.555, 31.850),
    Coord::new(35.548, 31.775),
];

/// Covenant border of Bereshit 15:18 and Devarim 11:24: from the Brook of
/// Egypt (Nachal Mitzrayim, Wadi El-Arish) to the Euphrates. The
/// identification of «река Египта» with Nachal Mitzrayim rather than the
/// Nile is the usual rabbinic reading; it is flagged in `docs/RABBI-REVIEW.md`.
pub const COVENANT: &[Coord] = &[
    // Brook of Egypt, then the Great Sea north through the Levant
    Coord::new(33.85, 31.13),
    Coord::new(34.05, 31.22),
    Coord::new(34.40, 31.50),
    Coord::new(34.75, 32.08),
    Coord::new(34.98, 32.83),
    Coord::new(35.10, 33.09),
    Coord::new(35.20, 33.50),
    Coord::new(35.37, 33.90),
    Coord::new(35.50, 34.20),
    Coord::new(35.53, 34.45),
    Coord::new(35.70, 34.80),
    Coord::new(35.84, 35.10),
    Coord::new(35.90, 35.40),
    Coord::new(35.92, 35.65),
    Coord::new(36.05, 35.95),
    // Across to the Euphrates near Carchemish, then down the river
    Coord::new(36.50, 36.40),
    Coord::new(37.20, 36.70),
    Coord::new(37.80, 36.82),
    Coord::new(38.02, 36.83),
    Coord::new(38.30, 36.20),
    Coord::new(38.80, 36.00),
    Coord::new(39.15, 35.95),
    Coord::new(39.60, 35.85),
    Coord::new(40.15, 35.34),
    Coord::new(40.50, 34.90),
    Coord::new(40.90, 34.45),
    // South-west across the wilderness, including Gilad and Moav, to Yam Suf
    Coord::new(40.20, 33.80),
    Coord::new(39.00, 33.20),
    Coord::new(38.00, 32.50),
    Coord::new(37.20, 31.80),
    Coord::new(36.80, 31.40),
    Coord::new(36.50, 30.80),
    Coord::new(36.20, 30.20),
    Coord::new(35.80, 29.80),
    Coord::new(35.20, 29.45),
    Coord::new(34.90, 29.50),
    Coord::new(34.50, 29.80),
    Coord::new(34.10, 30.20),
    Coord::new(33.80, 30.60),
    Coord::new(33.70, 30.90),
];

/// Settlement bounds of Canaan in Bamidbar 34: the Brook of Egypt, the
/// Great Sea, a northern line toward the entrance of Hamath, and the
/// Jordan with the Salt Sea on the east. Transjordan (the 2½ tribes) is
/// not inside this ring; it appears on the tribal overlay.
pub const CANAAN: &[Coord] = &[
    Coord::new(33.85, 31.13),
    Coord::new(34.10, 31.22),
    Coord::new(34.40, 31.50),
    Coord::new(34.75, 32.08),
    Coord::new(34.98, 32.83),
    Coord::new(35.10, 33.09),
    Coord::new(35.20, 33.50),
    Coord::new(35.37, 33.90),
    Coord::new(35.55, 34.30),
    Coord::new(35.85, 34.55),
    Coord::new(36.15, 34.50),
    Coord::new(36.40, 34.30),
    Coord::new(36.55, 34.10),
    Coord::new(36.50, 33.80),
    Coord::new(36.15, 33.40),
    Coord::new(35.95, 33.15),
    Coord::new(35.75, 32.90),
    Coord::new(35.68, 32.80),
    Coord::new(35.62, 32.70),
    Coord::new(35.59, 32.50),
    Coord::new(35.56, 32.20),
    Coord::new(35.55, 31.90),
    Coord::new(35.55, 31.76),
    Coord::new(35.50, 31.50),
    Coord::new(35.40, 31.20),
    Coord::new(35.37, 31.10),
    Coord::new(35.14, 30.92),
    Coord::new(34.90, 30.80),
    Coord::new(34.60, 30.70),
    Coord::new(34.43, 30.64),
    Coord::new(34.10, 30.70),
    Coord::new(33.90, 30.90),
];

/// A tribal allotment (Yehoshua 13–19). Shapes are schematic.
#[derive(Debug, Clone, Copy)]
pub struct Tribe {
    pub id: &'static str,
    pub name: &'static str,
    pub hebrew: &'static str,
    /// East of the Jordan (Reuven, Gad, half-Menashshe).
    pub transjordan: bool,
    pub pts: &'static [Coord],
}

pub const TRIBES: &[Tribe] = &[
    Tribe {
        id: "yehuda",
        name: "Йеhуда",
        hebrew: "יהודה",
        transjordan: false,
        pts: &[
            Coord::new(34.25, 31.22),
            Coord::new(34.55, 31.68),
            Coord::new(34.80, 31.85),
            Coord::new(35.10, 31.75),
            Coord::new(35.40, 31.72),
            Coord::new(35.50, 31.40),
            Coord::new(35.37, 31.10),
            Coord::new(35.00, 30.60),
            Coord::new(34.50, 30.50),
            Coord::new(34.30, 30.90),
        ],
    },
    Tribe {
        id: "shimon",
        name: "Шимон",
        hebrew: "שמעון",
        transjordan: false,
        pts: &[
            Coord::new(34.55, 31.08),
            Coord::new(34.95, 31.08),
            Coord::new(34.95, 31.35),
            Coord::new(34.55, 31.35),
        ],
    },
    Tribe {
        id: "binyamin",
        name: "Биньямин",
        hebrew: "בנימין",
        transjordan: false,
        pts: &[
            Coord::new(35.05, 31.75),
            Coord::new(35.50, 31.75),
            Coord::new(35.55, 31.92),
            Coord::new(35.30, 32.02),
            Coord::new(35.08, 31.95),
        ],
    },
    Tribe {
        id: "efrayim",
        name: "Эфраим",
        hebrew: "אפרים",
        transjordan: false,
        pts: &[
            Coord::new(35.00, 32.02),
            Coord::new(35.52, 32.02),
            Coord::new(35.52, 32.28),
            Coord::new(35.05, 32.30),
            Coord::new(34.92, 32.18),
        ],
    },
    Tribe {
        id: "menashshe-west",
        name: "Менаше (запад)",
        hebrew: "מנשה",
        transjordan: false,
        pts: &[
            Coord::new(34.90, 32.30),
            Coord::new(35.52, 32.30),
            Coord::new(35.55, 32.55),
            Coord::new(35.15, 32.62),
            Coord::new(34.88, 32.52),
        ],
    },
    Tribe {
        id: "dan",
        name: "Дан",
        hebrew: "דן",
        transjordan: false,
        pts: &[
            Coord::new(34.75, 31.88),
            Coord::new(34.95, 31.88),
            Coord::new(34.96, 32.16),
            Coord::new(34.76, 32.16),
        ],
    },
    Tribe {
        id: "yissachar",
        name: "Иссахар",
        hebrew: "יששכר",
        transjordan: false,
        pts: &[
            Coord::new(35.18, 32.52),
            Coord::new(35.55, 32.52),
            Coord::new(35.52, 32.70),
            Coord::new(35.20, 32.70),
        ],
    },
    Tribe {
        id: "zevulun",
        name: "Звулун",
        hebrew: "זבולון",
        transjordan: false,
        pts: &[
            Coord::new(35.05, 32.68),
            Coord::new(35.35, 32.68),
            Coord::new(35.32, 32.84),
            Coord::new(35.05, 32.82),
        ],
    },
    Tribe {
        id: "asher",
        name: "Ашер",
        hebrew: "אשר",
        transjordan: false,
        pts: &[
            Coord::new(35.05, 32.85),
            Coord::new(35.28, 32.90),
            Coord::new(35.30, 33.15),
            Coord::new(35.10, 33.09),
        ],
    },
    Tribe {
        id: "naftali",
        name: "Нафтали",
        hebrew: "נפתלי",
        transjordan: false,
        pts: &[
            Coord::new(35.30, 32.85),
            Coord::new(35.60, 32.85),
            Coord::new(35.62, 33.22),
            Coord::new(35.35, 33.22),
        ],
    },
    Tribe {
        id: "reuven",
        name: "Реувен",
        hebrew: "ראובן",
        transjordan: true,
        pts: &[
            Coord::new(35.58, 31.35),
            Coord::new(36.20, 31.35),
            Coord::new(36.20, 31.80),
            Coord::new(35.58, 31.80),
        ],
    },
    Tribe {
        id: "gad",
        name: "Гад",
        hebrew: "גד",
        transjordan: true,
        pts: &[
            Coord::new(35.58, 31.80),
            Coord::new(36.30, 31.80),
            Coord::new(36.30, 32.40),
            Coord::new(35.58, 32.40),
        ],
    },
    Tribe {
        id: "menashshe-east",
        name: "Менаше (восток)",
        hebrew: "מנשה",
        transjordan: true,
        pts: &[
            Coord::new(35.65, 32.50),
            Coord::new(36.40, 32.50),
            Coord::new(36.40, 33.20),
            Coord::new(35.72, 33.25),
            Coord::new(35.65, 32.70),
        ],
    },
];

/// A region label drawn on the land (not a clickable card).
#[derive(Debug, Clone, Copy)]
pub struct Region {
    pub name: &'static str,
    pub at: Coord,
    /// Label offset in degrees (east, north) so names do not sit on each other.
    pub dx: f32,
    pub dy: f32,
}

pub const REGIONS: &[Region] = &[
    Region {
        name: "Иудея и Шомрон",
        at: Coord::new(35.28, 32.47),
        dx: 0.0,
        dy: 0.0,
    },
    Region {
        name: "Голан",
        at: Coord::new(35.80, 33.14),
        dx: 0.0,
        dy: 0.0,
    },
    Region {
        name: "Галилея",
        at: Coord::new(35.25, 32.76),
        dx: 0.0,
        dy: 0.0,
    },
    Region {
        name: "Негев",
        at: Coord::new(34.90, 30.70),
        dx: 0.0,
        dy: 0.0,
    },
];

/// Labels that appear only with the Torah-borders layer (biblical names).
pub const TORAH_LABELS: &[Region] = &[
    Region {
        name: "Евфрат",
        at: Coord::new(40.15, 35.50),
        dx: 0.0,
        dy: 0.0,
    },
    Region {
        name: "Нахаль Мицраим",
        at: Coord::new(33.70, 30.85),
        dx: 0.0,
        dy: 0.0,
    },
    Region {
        name: "Леванон",
        at: Coord::new(35.85, 34.20),
        dx: 0.0,
        dy: 0.0,
    },
    Region {
        name: "Дамесек",
        at: Coord::new(36.30, 33.51),
        dx: 0.0,
        dy: 0.0,
    },
    Region {
        name: "Гилад",
        at: Coord::new(35.90, 32.30),
        dx: 0.0,
        dy: 0.0,
    },
    Region {
        name: "Башан",
        at: Coord::new(36.10, 32.90),
        dx: 0.0,
        dy: 0.0,
    },
    Region {
        name: "Моав",
        at: Coord::new(35.90, 31.50),
        dx: 0.0,
        dy: 0.0,
    },
    Region {
        name: "Ям ха-Гадоль",
        at: Coord::new(33.70, 32.40),
        dx: 0.0,
        dy: 0.0,
    },
];

/// A clickable city or site.
#[derive(Debug, Clone, Copy)]
pub struct Place {
    pub id: &'static str,
    pub name: &'static str,
    pub hebrew: &'static str,
    pub at: Coord,
    /// Slightly larger marker (the capital).
    pub capital: bool,
    /// Label offset in degrees (east, north).
    pub dx: f32,
    pub dy: f32,
    pub text: &'static str,
    /// Optional lesson opened from the card.
    pub lesson: Option<(&'static str, &'static str)>,
}

pub const PLACES: &[Place] = &[
    Place {
        id: "jerusalem",
        name: "Иерусалим",
        hebrew: "ירושלים",
        at: Coord::new(35.217, 31.778),
        capital: true,
        dx: 0.0,
        dy: 0.03,
        text: "Столица Государства Израиль и вечная столица еврейского народа. По Основному закону 1980 года «Иерусалим, единый и целостный, — столица Израиля». Здесь стояли Первый и Второй Храм, здесь Западная стена — Котель, остаток ограды Храмовой горы.",
        lesson: Some(("prazdniki-6", "Посты и памятные дни")),
    },
    Place {
        id: "hebron",
        name: "Хеврон",
        hebrew: "חברון",
        at: Coord::new(35.099, 31.533),
        capital: false,
        dx: 0.1,
        dy: -0.02,
        text: "Город праотцев в Иудее. Авраам купил здесь пещеру Махпела, и в ней похоронены Авраам и Сара, Ицхак и Ривка, Яаков и Лея. Здесь Давид царствовал семь лет, прежде чем перенёс столицу в Иерусалим.",
        lesson: Some(("osnovy-5", "Краткая история еврейского народа")),
    },
    Place {
        id: "shechem",
        name: "Шхем",
        hebrew: "שכם",
        at: Coord::new(35.261, 32.221),
        capital: false,
        dx: 0.0,
        dy: 0.05,
        text: "Древний Шхем в Шомроне. Здесь Авраам построил первый жертвенник в Земле; здесь покоятся кости Йосефа; между горами Гризим и Эйвал народ принял благословения и проклятия Торы.",
        lesson: Some(("osnovy-5", "Краткая история еврейского народа")),
    },
    Place {
        id: "beit-el",
        name: "Бейт-Эль",
        hebrew: "בית אל",
        at: Coord::new(35.227, 31.940),
        capital: false,
        dx: -0.12,
        dy: 0.02,
        text: "Дом Б-га: здесь Яакову во сне открылась лестница в небо, и он поставил камень и дал месту это имя. Бейт-Эль стоит в горах Эфраима, к северу от Иерусалима.",
        lesson: Some(("osnovy-5", "Краткая история еврейского народа")),
    },
    Place {
        id: "shiloh",
        name: "Шило",
        hebrew: "שילה",
        at: Coord::new(35.290, 32.056),
        capital: false,
        dx: 0.1,
        dy: 0.03,
        text: "Здесь, в наделе Эфраима, много лет стоял Мишкан — переносное Святилище, пока Шломо не построил Храм в Иерусалиме. В Шило служили Эли и рос Шмуэль.",
        lesson: Some(("osnovy-5", "Краткая история еврейского народа")),
    },
    Place {
        id: "jericho",
        name: "Иерихон",
        hebrew: "יריחו",
        at: Coord::new(35.456, 31.857),
        capital: false,
        dx: 0.14,
        dy: 0.0,
        text: "Город у Иордана, первый, который народ вошёл в Землю при Йеhошуа. Это один из древнейших городов мира, лежащий в Иорданской долине у края Иудеи.",
        lesson: Some(("osnovy-5", "Краткая история еврейского народа")),
    },
    Place {
        id: "beit-lechem",
        name: "Бейт-Лехем",
        hebrew: "בית לחם",
        at: Coord::new(35.204, 31.705),
        capital: false,
        dx: 0.0,
        dy: -0.07,
        text: "Дом хлеба, город в Иудее. У дороги стоит гробница Рахели; отсюда род Давида.",
        lesson: Some(("osnovy-5", "Краткая история еврейского народа")),
    },
    Place {
        id: "tzfat",
        name: "Цфат",
        hebrew: "צפת",
        at: Coord::new(35.494, 32.965),
        capital: false,
        dx: 0.02,
        dy: 0.06,
        text: "Один из четырёх святых городов, в горах Галилеи. Здесь в XVI веке расцвела каббала Аризаля, и отсюда расходится мелодия Леха Доди.",
        lesson: Some(("shabbat-2", "Подготовка и встреча Шаббата")),
    },
    Place {
        id: "tveriah",
        name: "Тверия",
        hebrew: "טבריה",
        at: Coord::new(35.542, 32.795),
        capital: false,
        dx: 0.12,
        dy: 0.0,
        text: "Святой город на берегу Кинерета. Здесь заседал Синедрион, здесь завершалась Иерусалимская Гемара, здесь могила Рамбама.",
        lesson: Some(("osnovy-2", "Тора Письменная и Устная")),
    },
    Place {
        id: "yafo",
        name: "Яффо",
        hebrew: "יפו",
        at: Coord::new(34.753, 32.052),
        capital: false,
        dx: -0.1,
        dy: 0.0,
        text: "Древний порт на берегу Великого моря. Отсюда Йона сел на корабль; через Яффо в Землю приходили поколения евреев.",
        lesson: None,
    },
    Place {
        id: "akko",
        name: "Акко",
        hebrew: "עכו",
        at: Coord::new(35.071, 32.928),
        capital: false,
        dx: -0.1,
        dy: 0.04,
        text: "Древний приморский город в наделе Ашера, у подножия Галилеи. Рамбам называет Акко границей Земли для некоторых заповедей, связанных с её святостью.",
        lesson: None,
    },
    Place {
        id: "haifa",
        name: "Хайфа",
        hebrew: "חיפה",
        at: Coord::new(34.989, 32.794),
        capital: false,
        dx: -0.1,
        dy: 0.02,
        text: "Город у горы Кармель, где пророк Элияhу противостоял пророкам Бааля. Кармель смотрит на Великое море.",
        lesson: Some(("osnovy-5", "Краткая история еврейского народа")),
    },
    Place {
        id: "beersheva",
        name: "Беэр-Шева",
        hebrew: "באר שבע",
        at: Coord::new(34.791, 31.253),
        capital: false,
        dx: -0.16,
        dy: 0.0,
        text: "Колодец клятвы: Авраам и Ицхак заключили здесь союзы и копали колодцы. Беэр-Шева — врата Негева, юга Земли.",
        lesson: Some(("osnovy-5", "Краткая история еврейского народа")),
    },
    Place {
        id: "eilat",
        name: "Эйлат",
        hebrew: "אילת",
        at: Coord::new(34.951, 29.557),
        capital: false,
        dx: 0.12,
        dy: 0.0,
        text: "Южные ворота Земли, у Ям Суф. В Танахе это край Эцион-Гевера, откуда Шломо отправлял корабли.",
        lesson: None,
    },
    Place {
        id: "aza",
        name: "Аза",
        hebrew: "עזה",
        at: Coord::new(34.466, 31.506),
        capital: false,
        dx: -0.12,
        dy: 0.02,
        text: "Древний город южного приморья. В Торе и у пророков Аза — в пределах Земли: в наделе Йеhуды названы «Аза, её города и селения, до ручья Египта» (Йеhошуа 15:47). С 2005 года, после ухода Израиля, Аза находится за пределами его границ.",
        lesson: Some(("osnovy-5", "Краткая история еврейского народа")),
    },
    Place {
        id: "katzrin",
        name: "Кацрин",
        hebrew: "קצרין",
        at: Coord::new(35.689, 32.992),
        capital: false,
        dx: 0.12,
        dy: 0.03,
        text: "Город на Голане — плато, которое с древности связано с Башаном и наделом восточного Менаше. С 1981 года на Голане действует израильский закон, и на израильских картах Голан — часть страны.",
        lesson: None,
    },
    Place {
        id: "hermon",
        name: "Хермон",
        hebrew: "חרמון",
        at: Coord::new(35.850, 33.310),
        capital: false,
        dx: 0.0,
        dy: 0.05,
        text: "Северная гора Земли, «глаза народа». С Хермона берут начало потоки, питающие Иордан.",
        lesson: None,
    },
];

/// Short caption under the Torah-borders layer, citing the three sources.
pub const TORAH_CAPTION: &str = "Границы по Торе. Берешит 15:18 — от реки Египта до великой реки, реки Евфрат. Бемидбар 34 — пределы земли Кнаан для поселения: юг до ручья Египта (Нахаль Мицраим), запад — Великое море, север — до входа в Хамат, восток — Иордан и Ям ха-Мелах. Дварим 11:24 — всякое место, куда ступит стопа ваша, от пустыни и Леванона до реки Евфрат и западного моря. Широкий контур — завет с Авраамом; внутренний — пределы Бемидбар 34. Наделы двух с половиной колен к востоку от Иордана показаны слоем «Наделы колен» (Йеhошуа 13–19).";

/// Even-odd ray test: is `p` inside the closed ring `ring`?
pub fn inside(p: Coord, ring: &[Coord]) -> bool {
    if ring.len() < 3 {
        return false;
    }
    let mut odd = false;
    let mut j = ring.len() - 1;
    for i in 0..ring.len() {
        let a = ring[i];
        let b = ring[j];
        let crosses = (a.lat > p.lat) != (b.lat > p.lat);
        if crosses {
            let x = (b.lon - a.lon) * (p.lat - a.lat) / (b.lat - a.lat) + a.lon;
            if p.lon < x {
                odd = !odd;
            }
        }
        j = i;
    }
    odd
}

/// All Russian strings drawn on the map or in place cards, for consistency checks.
pub fn all_visible_texts() -> impl Iterator<Item = &'static str> {
    REGIONS
        .iter()
        .map(|r| r.name)
        .chain(TORAH_LABELS.iter().map(|r| r.name))
        .chain(PLACES.iter().flat_map(|p| [p.name, p.text]))
        .chain(TRIBES.iter().map(|t| t.name))
        .chain(std::iter::once(TORAH_CAPTION))
        .chain(std::iter::once("Столица Израиля, вечная столица народа"))
        .chain(std::iter::once("Эрец-Исраэль"))
        .chain(std::iter::once("Израиль сегодня"))
        .chain(std::iter::once("Границы по Торе"))
        .chain(std::iter::once("Наделы колен"))
}

/// Words and phrases that must not appear on the map or in its cards.
pub const FORBIDDEN: &[&str] = &[
    "палестин",
    "palestine",
    "green line",
    "зелёная линия",
    "зеленая линия",
    "оккупир",
    "occupied",
    "disputed",
    "спорн",
    "западный берег",
    "west bank",
    "сектор газ",
    "gaza strip",
    "наблус",
    "nablus",
    "восточный иерусалим",
    "восточного иерусалим",
    "east jerusalem",
    "autonomy",
    "неделим",
    "вырезан",
    "автономия палест",
];

pub fn place(id: &str) -> Option<&'static Place> {
    PLACES.iter().find(|p| p.id == id)
}

/// Nearest place within `max_deg` (rough degrees) of `p`, if any.
pub fn nearest_place(p: Coord, max_deg: f32) -> Option<&'static Place> {
    let mut best: Option<(&'static Place, f32)> = None;
    for place in PLACES {
        let dlon = (place.at.lon - p.lon) * p.lat.to_radians().cos();
        let dlat = place.at.lat - p.lat;
        let d = (dlon * dlon + dlat * dlat).sqrt();
        if d <= max_deg && best.is_none_or(|(_, bd)| d < bd) {
            best = Some((place, d));
        }
    }
    best.map(|(pl, _)| pl)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(lon: f32, lat: f32) -> Coord {
        Coord::new(lon, lat)
    }

    #[test]
    fn land_is_a_single_ring_without_holes() {
        assert!(LAND.len() > 40);
        // The ring is open (first ≠ last); the drawer closes it.
        assert_ne!(LAND[0], *LAND.last().unwrap());
    }

    #[test]
    fn land_contains_the_whole_country() {
        let inside_pts = [
            ("jerusalem", c(35.217, 31.778)),
            ("east-jerusalem", c(35.24, 31.78)),
            ("hebron", c(35.099, 31.533)),
            ("shechem", c(35.261, 32.221)),
            ("beit-el", c(35.227, 31.940)),
            ("shiloh", c(35.290, 32.056)),
            ("jericho", c(35.456, 31.857)),
            ("katzrin-golan", c(35.689, 32.992)),
            ("eilat", c(34.951, 29.557)),
            ("tzfat", c(35.494, 32.965)),
            ("beersheva", c(34.791, 31.253)),
            ("yafo", c(34.753, 32.052)),
            ("haifa", c(34.989, 32.794)),
            ("beit-lechem", c(35.204, 31.705)),
        ];
        for (name, p) in inside_pts {
            assert!(inside(p, LAND), "{name} must be inside the land");
        }
        // East of the Jordan (Amman) and Damascus are not on the base map.
        assert!(
            !inside(c(35.93, 31.95), LAND),
            "Amman is east of the Jordan"
        );
        assert!(
            !inside(c(36.30, 33.51), LAND),
            "Damascus is outside the land"
        );
        assert!(
            !inside(c(33.80, 31.13), LAND),
            "El-Arish is outside the base land"
        );
        assert!(
            !inside(c(34.466, 31.506), LAND),
            "Aza is outside Israel's borders since 2005"
        );
        assert!(inside(c(34.466, 31.506), AZA));
        assert!(inside(c(34.60, 31.52), LAND), "Sderot is in Israel");
        assert!(!inside(c(34.60, 31.52), AZA));
    }

    #[test]
    fn neighbours_cover_sinai_and_jordan_but_not_the_sea() {
        assert!(inside(c(33.80, 30.50), NEIGHBOURS), "Sinai");
        assert!(inside(c(35.93, 31.95), NEIGHBOURS), "Amman");
        assert!(inside(c(36.30, 33.51), NEIGHBOURS), "Damascus");
        assert!(
            !inside(c(34.30, 32.50), NEIGHBOURS),
            "off the coast of Israel"
        );
        assert!(!inside(c(34.00, 34.00), NEIGHBOURS), "the open sea");
    }

    #[test]
    fn armistice_line_stays_out_of_jerusalem() {
        let jlm = c(35.217, 31.778);
        for p in ARMISTICE_1949_NORTH.iter().chain(ARMISTICE_1949_SOUTH) {
            let d = ((p.lon - jlm.lon).powi(2) + (p.lat - jlm.lat).powi(2)).sqrt();
            assert!(d > 0.05, "the line must not cross the city: {p:?}");
        }
    }

    #[test]
    fn covenant_reaches_the_euphrates_and_the_brook_of_egypt() {
        assert!(inside(c(35.217, 31.778), COVENANT));
        assert!(inside(c(35.93, 31.95), COVENANT), "Amman in the covenant");
        assert!(
            inside(c(36.30, 33.51), COVENANT),
            "Damascus in the covenant"
        );
        assert!(inside(c(35.50, 33.90), COVENANT), "Sidon / Lebanон");
        // A point on the mid-Euphrates near Raqqa.
        assert!(
            inside(c(39.00, 35.90), COVENANT),
            "the mid-Euphrates is inside the covenant"
        );
        // The brook mouth itself is on the ring; a point just inside Sinai of the ring.
        assert!(inside(c(34.20, 30.80), COVENANT));
    }

    #[test]
    fn canaan_is_west_of_the_jordan() {
        assert!(inside(c(35.217, 31.778), CANAAN));
        assert!(inside(c(34.466, 31.506), CANAAN), "Aza is in Yehuda");
        assert!(
            !inside(c(35.93, 31.95), CANAAN),
            "Amman is east of Bamidbar 34"
        );
        assert!(
            !inside(c(36.30, 33.51), CANAAN),
            "Damascus is outside Canaan of Bamidbar 34"
        );
    }

    #[test]
    fn required_jewish_place_names_are_present() {
        let names: Vec<&str> = PLACES.iter().map(|p| p.name).collect();
        for need in [
            "Иерусалим",
            "Хеврон",
            "Шхем",
            "Бейт-Эль",
            "Шило",
            "Иерихон",
            "Аза",
        ] {
            assert!(names.contains(&need), "missing place {need}");
        }
        let regions: Vec<&str> = REGIONS.iter().map(|r| r.name).collect();
        for need in ["Иудея и Шомрон", "Голан"] {
            assert!(regions.contains(&need), "missing region {need}");
        }
        // «Аза» is a city card (and would double the region label).
        assert!(PLACES.iter().any(|p| p.capital && p.name == "Иерусалим"));
    }

    #[test]
    fn no_forbidden_political_language() {
        for text in all_visible_texts() {
            let lower = text.to_lowercase();
            for word in FORBIDDEN {
                assert!(!lower.contains(word), "forbidden {word:?} in {text:?}");
            }
        }
    }

    #[test]
    fn caption_cites_the_three_sources() {
        for cite in ["Берешит 15:18", "Бемидбар 34", "Дварим 11:24"] {
            assert!(TORAH_CAPTION.contains(cite), "caption missing {cite}");
        }
    }

    #[test]
    fn every_place_has_hebrew_and_text() {
        for p in PLACES {
            assert!(!p.hebrew.is_empty(), "{}", p.id);
            assert!(p.text.chars().count() > 40, "{}", p.id);
        }
    }

    #[test]
    fn tribes_include_transjordan_and_levi_has_no_allotment() {
        assert!(TRIBES.iter().any(|t| t.id == "reuven" && t.transjordan));
        assert!(TRIBES.iter().any(|t| t.id == "gad" && t.transjordan));
        assert!(TRIBES
            .iter()
            .any(|t| t.id == "menashshe-east" && t.transjordan));
        assert!(TRIBES.iter().all(|t| t.id != "levi"));
        assert!(
            inside(
                c(35.20, 32.06),
                TRIBES.iter().find(|t| t.id == "efrayim").unwrap().pts
            ),
            "Shiloh is in Efrayim"
        );
    }

    #[test]
    fn nearest_place_finds_jerusalem() {
        let p = nearest_place(c(35.22, 31.78), 0.05).expect("jerusalem");
        assert_eq!(p.id, "jerusalem");
        assert!(nearest_place(c(33.0, 31.0), 0.05).is_none());
    }
}
