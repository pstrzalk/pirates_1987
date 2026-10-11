//! Static game data: nations, ships, goods, eras, towns, ranks, expeditions.

use crate::ui;
use macroquad::prelude::Color;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Serialize, Deserialize)]
pub enum Nation {
    Spain,
    England,
    France,
    Holland,
    Pirate,
}

pub const NATIONS: [Nation; 4] = [Nation::Spain, Nation::England, Nation::France, Nation::Holland];

impl Nation {
    pub fn idx(self) -> usize {
        match self {
            Nation::Spain => 0,
            Nation::England => 1,
            Nation::France => 2,
            Nation::Holland => 3,
            Nation::Pirate => 4,
        }
    }
    pub fn from_idx(i: usize) -> Nation {
        [Nation::Spain, Nation::England, Nation::France, Nation::Holland, Nation::Pirate][i.min(4)]
    }
    pub fn name(self) -> &'static str {
        match self {
            Nation::Spain => "Spain",
            Nation::England => "England",
            Nation::France => "France",
            Nation::Holland => "Holland",
            Nation::Pirate => "Pirates",
        }
    }
    pub fn adj(self) -> &'static str {
        match self {
            Nation::Spain => "Spanish",
            Nation::England => "English",
            Nation::France => "French",
            Nation::Holland => "Dutch",
            Nation::Pirate => "Pirate",
        }
    }
    pub fn color(self) -> Color {
        match self {
            Nation::Spain => ui::YELLOW,
            Nation::England => ui::LRED,
            Nation::France => ui::WHITE,
            Nation::Holland => Color::new(1.0, 0.6, 0.1, 1.0),
            Nation::Pirate => ui::DGRAY,
        }
    }
}

/// Draw a small flag for the nation.
pub fn draw_flag(n: Nation, x: f32, y: f32, w: f32, h: f32) {
    use ui::*;
    match n {
        Nation::Spain => {
            rect(x, y, w, h, RED);
            rect(x, y + h * 0.25, w, h * 0.5, YELLOW);
        }
        Nation::England => {
            rect(x, y, w, h, WHITE);
            rect(x + w * 0.4, y, w * 0.2, h, RED);
            rect(x, y + h * 0.38, w, h * 0.24, RED);
        }
        Nation::France => {
            rect(x, y, w, h, WHITE);
            rect(x + w * 0.42, y + h * 0.3, w * 0.16, h * 0.4, YELLOW);
        }
        Nation::Holland => {
            let o = Color::new(1.0, 0.5, 0.0, 1.0);
            rect(x, y, w, h / 3.0, o);
            rect(x, y + h / 3.0, w, h / 3.0, WHITE);
            rect(x, y + 2.0 * h / 3.0, w, h / 3.0, BLUE);
        }
        Nation::Pirate => {
            rect(x, y, w, h, BLACK);
            macroquad::prelude::draw_circle(x + w * 0.5, y + h * 0.45, h * 0.22, WHITE);
        }
    }
    frame(x, y, w, h, BLACK, 1.0);
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Serialize, Deserialize)]
pub enum ShipKind {
    Sloop,
    Pinnace,
    Barque,
    Fluyt,
    Merchantman,
    Frigate,
    TradeGalleon,
    FastGalleon,
    WarGalleon,
}


pub struct ShipSpec {
    pub name: &'static str,
    pub max_guns: i32,
    pub max_crew: i32,
    pub cargo: i32,
    /// Best speed in map degrees per day.
    pub speed: f32,
    /// Square rigged ships sail poorly into the wind.
    pub square: bool,
    /// Deep draft ships run aground in shallows.
    pub deep: bool,
    /// Turning rate multiplier (battle).
    pub turn: f32,
    /// Base value in pieces of eight.
    pub value: i32,
    /// Visual size multiplier.
    pub size: f32,
    pub masts: u8,
}

impl ShipKind {
    pub fn spec(self) -> &'static ShipSpec {
        match self {
            ShipKind::Sloop => &ShipSpec { name: "Sloop", max_guns: 8, max_crew: 75, cargo: 40, speed: 1.15, square: false, deep: false, turn: 1.0, value: 400, size: 0.75, masts: 1 },
            ShipKind::Pinnace => &ShipSpec { name: "Pinnace", max_guns: 10, max_crew: 80, cargo: 80, speed: 1.05, square: false, deep: false, turn: 0.9, value: 700, size: 0.8, masts: 2 },
            ShipKind::Barque => &ShipSpec { name: "Barque", max_guns: 12, max_crew: 100, cargo: 100, speed: 0.95, square: false, deep: false, turn: 0.8, value: 900, size: 0.85, masts: 2 },
            ShipKind::Fluyt => &ShipSpec { name: "Fluyt", max_guns: 10, max_crew: 60, cargo: 170, speed: 0.85, square: true, deep: false, turn: 0.65, value: 1100, size: 0.95, masts: 3 },
            ShipKind::Merchantman => &ShipSpec { name: "Merchantman", max_guns: 16, max_crew: 120, cargo: 200, speed: 0.8, square: true, deep: true, turn: 0.6, value: 1400, size: 1.0, masts: 3 },
            ShipKind::Frigate => &ShipSpec { name: "Frigate", max_guns: 32, max_crew: 200, cargo: 160, speed: 1.0, square: true, deep: true, turn: 0.75, value: 3000, size: 1.1, masts: 3 },
            ShipKind::TradeGalleon => &ShipSpec { name: "Trade Galleon", max_guns: 20, max_crew: 150, cargo: 320, speed: 0.7, square: true, deep: true, turn: 0.45, value: 2400, size: 1.2, masts: 3 },
            ShipKind::FastGalleon => &ShipSpec { name: "Fast Galleon", max_guns: 28, max_crew: 200, cargo: 220, speed: 0.88, square: true, deep: true, turn: 0.6, value: 3400, size: 1.2, masts: 3 },
            ShipKind::WarGalleon => &ShipSpec { name: "War Galleon", max_guns: 40, max_crew: 300, cargo: 260, speed: 0.75, square: true, deep: true, turn: 0.45, value: 4500, size: 1.3, masts: 4 },
        }
    }
    pub fn name(self) -> &'static str {
        self.spec().name
    }
}

/// Speed multiplier for sailing at `angle` radians off the downwind direction
/// (0 = wind directly astern, PI = straight into the wind).
pub fn wind_factor(square: bool, angle: f32) -> f32 {
    let a = angle.abs().min(std::f32::consts::PI) / std::f32::consts::PI * 180.0;
    // (angle in degrees, factor)
    let table: &[(f32, f32)] = if square {
        &[(0.0, 1.0), (45.0, 1.0), (90.0, 0.8), (120.0, 0.5), (135.0, 0.3), (150.0, 0.15), (180.0, 0.1)]
    } else {
        &[(0.0, 0.8), (45.0, 0.9), (90.0, 1.0), (120.0, 0.85), (135.0, 0.65), (150.0, 0.35), (180.0, 0.15)]
    };
    for w in table.windows(2) {
        let (a0, f0) = w[0];
        let (a1, f1) = w[1];
        if a <= a1 {
            return f0 + (f1 - f0) * (a - a0) / (a1 - a0);
        }
    }
    table.last().unwrap().1
}

pub const GOODS: [&str; 4] = ["Sugar", "Tobacco", "Hides", "Goods"];
pub const GOOD_BASE_PRICE: [i32; 4] = [40, 55, 30, 70];
pub const FOOD_BASE_PRICE: i32 = 12;
pub const GUN_PRICE: i32 = 80;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Difficulty {
    Apprentice,
    Journeyman,
    Adventurer,
    Swashbuckler,
}

impl Difficulty {
    pub fn idx(self) -> usize {
        self as usize
    }
    pub fn from_idx(i: usize) -> Self {
        [Difficulty::Apprentice, Difficulty::Journeyman, Difficulty::Adventurer, Difficulty::Swashbuckler][i.min(3)]
    }
    pub fn name(self) -> &'static str {
        ["Apprentice", "Journeyman", "Adventurer", "Swashbuckler"][self.idx()]
    }
    /// Captain's share of the plunder when it is divided.
    pub fn share(self) -> f32 {
        [0.10, 0.15, 0.20, 0.25][self.idx()]
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Skill {
    Fencing,
    Gunnery,
    Navigation,
    Medicine,
    WitCharm,
}

impl Skill {
    pub fn from_idx(i: usize) -> Self {
        [Skill::Fencing, Skill::Gunnery, Skill::Navigation, Skill::Medicine, Skill::WitCharm][i.min(4)]
    }
    pub fn name(self) -> &'static str {
        match self {
            Skill::Fencing => "Skill at Fencing",
            Skill::Gunnery => "Skill at Gunnery",
            Skill::Navigation => "Skill at Navigation",
            Skill::Medicine => "Skill in Medicine",
            Skill::WitCharm => "Wit and Charm",
        }
    }
    pub fn desc(self) -> &'static str {
        match self {
            Skill::Fencing => "Your blade is quicker and surer in a duel.",
            Skill::Gunnery => "Your gun crews reload faster and aim truer.",
            Skill::Navigation => "Your ships sail faster on every point of sail.",
            Skill::Medicine => "You stay healthy longer and recover from wounds.",
            Skill::WitCharm => "Governors and their daughters find you agreeable.",
        }
    }
}

pub struct Era {
    pub year: i32,
    pub title: &'static str,
    pub desc: &'static str,
    /// Which nations a player may serve in this era.
    pub nations: &'static [Nation],
}

pub const ERAS: [Era; 6] = [
    Era {
        year: 1560,
        title: "The Silver Empire",
        desc: "Spain rules the Main and its treasure fleets carry silver home. Other nations are mere interlopers and smugglers.",
        nations: &[Nation::Spain, Nation::England, Nation::France],
    },
    Era {
        year: 1600,
        title: "Merchants and Smugglers",
        desc: "Spain is weakened by war in Europe. English, French and Dutch traders slip into Spanish waters.",
        nations: &[Nation::Spain, Nation::England, Nation::France, Nation::Holland],
    },
    Era {
        year: 1620,
        title: "The New Colonists",
        desc: "Foreign colonies spring up in the Lesser Antilles. The Dutch prowl for Spanish silver.",
        nations: &[Nation::Spain, Nation::England, Nation::France, Nation::Holland],
    },
    Era {
        year: 1640,
        title: "War for Profit",
        desc: "Colonies prosper and nations wage war for trade. Buccaneers gather at Tortuga.",
        nations: &[Nation::Spain, Nation::England, Nation::France, Nation::Holland],
    },
    Era {
        year: 1660,
        title: "The Buccaneer Heroes",
        desc: "The golden age of the buccaneers. Port Royal and Tortuga launch great raids on the Spanish Main.",
        nations: &[Nation::Spain, Nation::England, Nation::France, Nation::Holland],
    },
    Era {
        year: 1680,
        title: "Pirates' Sunset",
        desc: "Governments turn against the pirates, yet wars still make privateering profitable.",
        nations: &[Nation::Spain, Nation::England, Nation::France, Nation::Holland],
    },
];

pub struct TownDef {
    pub name: &'static str,
    pub lon: f32,
    pub lat: f32,
    /// Owner per era: S/E/F/D/P or '-' if the town does not exist.
    pub owners: &'static str,
    /// Base size 1..5
    pub size: i32,
    /// What the area produces cheaply: index into GOODS, or None.
    pub produces: Option<usize>,
}

pub const TOWNS: &[TownDef] = &[
    TownDef { name: "St. Augustine", lon: -81.31, lat: 29.9, owners: "SSSSSS", size: 2, produces: Some(2) },
    TownDef { name: "Havana", lon: -82.36, lat: 23.14, owners: "SSSSSS", size: 5, produces: Some(1) },
    TownDef { name: "Puerto Principe", lon: -77.25, lat: 21.65, owners: "SSSSSS", size: 2, produces: Some(2) },
    TownDef { name: "Santiago", lon: -75.83, lat: 19.98, owners: "SSSSSS", size: 3, produces: Some(1) },
    TownDef { name: "Trinidad de Cuba", lon: -79.98, lat: 21.76, owners: "SSSSSS", size: 2, produces: Some(0) },
    TownDef { name: "Port Royal", lon: -76.84, lat: 17.94, owners: "SSSEEE", size: 3, produces: Some(0) },
    TownDef { name: "Tortuga", lon: -72.78, lat: 20.02, owners: "PPPPFF", size: 1, produces: Some(2) },
    TownDef { name: "Petit Goave", lon: -72.86, lat: 18.43, owners: "---FFF", size: 2, produces: Some(1) },
    TownDef { name: "Santo Domingo", lon: -69.9, lat: 18.47, owners: "SSSSSS", size: 4, produces: Some(0) },
    TownDef { name: "San Juan", lon: -66.1, lat: 18.47, owners: "SSSSSS", size: 4, produces: Some(0) },
    TownDef { name: "St. Thomas", lon: -64.93, lat: 18.34, owners: "--PDDP", size: 1, produces: None },
    TownDef { name: "St. Eustatius", lon: -62.98, lat: 17.49, owners: "--DDDD", size: 1, produces: None },
    TownDef { name: "St. Kitts", lon: -62.73, lat: 17.3, owners: "-EEEEE", size: 2, produces: Some(1) },
    TownDef { name: "Nevis", lon: -62.6, lat: 17.13, owners: "--EEEE", size: 1, produces: Some(0) },
    TownDef { name: "Antigua", lon: -61.85, lat: 17.13, owners: "---EEE", size: 2, produces: Some(0) },
    TownDef { name: "Guadeloupe", lon: -61.72, lat: 16.0, owners: "-FFFFF", size: 2, produces: Some(0) },
    TownDef { name: "Martinique", lon: -61.08, lat: 14.6, owners: "--FFFF", size: 3, produces: Some(0) },
    TownDef { name: "St. Lucia", lon: -60.99, lat: 14.0, owners: "---FEF", size: 1, produces: Some(1) },
    TownDef { name: "Barbados", lon: -59.62, lat: 13.1, owners: "EEEEEE", size: 3, produces: Some(0) },
    TownDef { name: "Grenada", lon: -61.76, lat: 12.06, owners: "----FF", size: 1, produces: Some(1) },
    TownDef { name: "Trinidad", lon: -61.52, lat: 10.65, owners: "SSSSSS", size: 2, produces: Some(1) },
    TownDef { name: "Margarita", lon: -63.88, lat: 11.0, owners: "SSSSSS", size: 2, produces: None },
    TownDef { name: "Cumana", lon: -64.18, lat: 10.46, owners: "SSSSSS", size: 2, produces: Some(2) },
    TownDef { name: "Caracas", lon: -66.93, lat: 10.6, owners: "SSSSSS", size: 4, produces: Some(1) },
    TownDef { name: "Curacao", lon: -68.93, lat: 12.11, owners: "SSDDDD", size: 2, produces: None },
    TownDef { name: "Coro", lon: -69.67, lat: 11.42, owners: "SSSSSS", size: 1, produces: Some(2) },
    TownDef { name: "Maracaibo", lon: -71.6, lat: 10.98, owners: "SSSSSS", size: 3, produces: Some(1) },
    TownDef { name: "Rio de la Hacha", lon: -72.9, lat: 11.55, owners: "SSSSSS", size: 2, produces: Some(2) },
    TownDef { name: "Santa Marta", lon: -74.2, lat: 11.25, owners: "SSSSSS", size: 2, produces: Some(2) },
    TownDef { name: "Cartagena", lon: -75.55, lat: 10.42, owners: "SSSSSS", size: 5, produces: None },
    TownDef { name: "Nombre de Dios", lon: -79.47, lat: 9.58, owners: "SS----", size: 3, produces: None },
    TownDef { name: "Porto Bello", lon: -79.66, lat: 9.56, owners: "-SSSSS", size: 4, produces: None },
    TownDef { name: "San Juan del Norte", lon: -83.7, lat: 10.92, owners: "SSSSSS", size: 2, produces: Some(1) },
    TownDef { name: "Trujillo", lon: -86.0, lat: 15.92, owners: "SSSSSS", size: 2, produces: Some(2) },
    TownDef { name: "Belize", lon: -88.19, lat: 17.5, owners: "----EP", size: 1, produces: Some(2) },
    TownDef { name: "Campeche", lon: -90.53, lat: 19.84, owners: "SSSSSS", size: 3, produces: Some(2) },
    TownDef { name: "Villa Hermosa", lon: -92.7, lat: 18.62, owners: "SSSSSS", size: 2, produces: Some(1) },
    TownDef { name: "Vera Cruz", lon: -96.13, lat: 19.2, owners: "SSSSSS", size: 5, produces: None },
    TownDef { name: "Tampico", lon: -97.85, lat: 22.25, owners: "SSSSSS", size: 2, produces: Some(2) },
    TownDef { name: "Pensacola", lon: -87.2, lat: 30.36, owners: "-----S", size: 1, produces: Some(2) },
    TownDef { name: "Nassau", lon: -77.35, lat: 25.07, owners: "----EP", size: 1, produces: None },
    TownDef { name: "Isla Vaca", lon: -73.65, lat: 18.07, owners: "PP--P-", size: 1, produces: Some(2) },
];

pub fn owner_for_era(def: &TownDef, era: usize) -> Option<Nation> {
    match def.owners.as_bytes().get(era).copied().unwrap_or(b'-') {
        b'S' => Some(Nation::Spain),
        b'E' => Some(Nation::England),
        b'F' => Some(Nation::France),
        b'D' => Some(Nation::Holland),
        b'P' => Some(Nation::Pirate),
        _ => None,
    }
}

pub const RANKS: [&str; 10] = [
    "none", "Ensign", "Captain", "Major", "Colonel", "Admiral", "Baron", "Count", "Marquis", "Duke",
];
/// Service points required for each rank.
pub const RANK_POINTS: [i32; 10] = [0, 4, 10, 18, 28, 40, 55, 72, 92, 115];
/// Acres of land granted on reaching each rank.
pub const RANK_LAND: [i32; 10] = [0, 0, 0, 0, 100, 200, 400, 600, 800, 1000];

pub const RELATIVES: [&str; 4] = ["sister", "uncle", "aunt", "grandfather"];

pub const SHIP_NAMES: &[&str] = &[
    "Santa Maria", "Esperanza", "Dolphin", "Swallow", "Pelican", "Golden Hind", "Revenge", "Neptune", "Concepcion",
    "San Felipe", "Trinidad", "Fortune", "Hope", "Mermaid", "Le Cerf", "La Fleur", "Gouden Leeuw", "Zeeland",
    "Adventure", "Delight", "Black Swan", "Royal Fortune", "Ranger", "Phoenix", "Santo Cristo", "Rosario",
    "Bonaventure", "Unity", "Seahorse", "Merlin", "Saint Pierre", "Hollandia", "Gracia", "Cygnet", "Falcon",
];

pub const CAPTAIN_NAMES: &[&str] = &[
    "Don Alonso", "Don Pedro", "Capitan Vargas", "Captain Hawkins", "Captain Blythe", "Capitaine Lefevre",
    "Capitaine Moreau", "Kapitein de Vries", "Kapitein Jansz", "Black Bart", "Calico Jack", "Long Ben",
    "Don Rodrigo", "Captain Morgan", "Capitaine Dulac",
];

pub struct Expedition {
    pub name: &'static str,
    pub leader: &'static str,
    pub year: i32,
    pub era: usize,
    pub nation: Nation,
    pub start_town: &'static str,
    pub ships: &'static [ShipKind],
    pub crew: i32,
    pub gold: i32,
    pub desc: &'static str,
}

pub const EXPEDITIONS: [Expedition; 6] = [
    Expedition {
        name: "Francis Drake's raid (1573)",
        leader: "Francis Drake",
        year: 1573,
        era: 0,
        nation: Nation::England,
        start_town: "Barbados",
        ships: &[ShipKind::Pinnace, ShipKind::Sloop],
        crew: 73,
        gold: 200,
        desc: "With two small ships and seventy men, Drake set out to seize the silver of Nombre de Dios.",
    },
    Expedition {
        name: "Piet Heyn and the Silver Fleet (1628)",
        leader: "Piet Heyn",
        year: 1628,
        era: 2,
        nation: Nation::Holland,
        start_town: "Curacao",
        ships: &[ShipKind::Frigate, ShipKind::Frigate, ShipKind::Fluyt, ShipKind::Pinnace],
        crew: 420,
        gold: 1000,
        desc: "The Dutch West India Company sends Piet Heyn to capture the Spanish treasure fleet off Cuba.",
    },
    Expedition {
        name: "L'Olonnais sacks Maracaibo (1666)",
        leader: "Francois L'Olonnais",
        year: 1666,
        era: 4,
        nation: Nation::France,
        start_town: "Tortuga",
        ships: &[ShipKind::Barque, ShipKind::Pinnace, ShipKind::Sloop, ShipKind::Sloop],
        crew: 300,
        gold: 400,
        desc: "The cruel Francois L'Olonnais gathers the buccaneers of Tortuga to plunder Maracaibo.",
    },
    Expedition {
        name: "Henry Morgan at Porto Bello (1668)",
        leader: "Henry Morgan",
        year: 1668,
        era: 4,
        nation: Nation::England,
        start_town: "Port Royal",
        ships: &[ShipKind::Barque, ShipKind::Pinnace, ShipKind::Pinnace, ShipKind::Sloop],
        crew: 350,
        gold: 500,
        desc: "Henry Morgan leads the buccaneers of Port Royal against the treasure port of Porto Bello.",
    },
    Expedition {
        name: "Baron de Pointis at Cartagena (1697)",
        leader: "Baron de Pointis",
        year: 1697,
        era: 5,
        nation: Nation::France,
        start_town: "Petit Goave",
        ships: &[ShipKind::WarGalleon, ShipKind::Frigate, ShipKind::Frigate, ShipKind::Barque],
        crew: 700,
        gold: 1500,
        desc: "A royal French squadron joins the buccaneers of Saint-Domingue to storm mighty Cartagena.",
    },
    Expedition {
        name: "De Graaf and Van Hoorn at Vera Cruz (1683)",
        leader: "Laurens de Graaf",
        year: 1683,
        era: 5,
        nation: Nation::France,
        start_town: "Petit Goave",
        ships: &[ShipKind::Frigate, ShipKind::Barque, ShipKind::Pinnace, ShipKind::Sloop],
        crew: 500,
        gold: 600,
        desc: "Laurens de Graaf and Nicholas van Hoorn lead the freebooters against rich Vera Cruz.",
    },
];
