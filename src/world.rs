//! The persistent game world: player, towns, AI ships, politics, economy and
//! the passage of time. Contains rules but no UI.

use crate::data::*;
use crate::geo::{Geo, Tile};
use macroquad::math::{Vec2, vec2};
use macroquad::rand::gen_range;
use serde::{Deserialize, Serialize};

pub type Pos = (f32, f32);

pub fn v(p: Pos) -> Vec2 {
    vec2(p.0, p.1)
}
pub fn p(v: Vec2) -> Pos {
    (v.x, v.y)
}

pub fn chance(prob: f32) -> bool {
    gen_range(0.0f32, 1.0) < prob
}

pub fn pick<T: Copy>(items: &[T]) -> T {
    items[gen_range(0, items.len())]
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ship {
    pub kind: ShipKind,
    pub name: String,
    pub hull: i32,
    pub guns: i32,
}

impl Ship {
    pub fn new(kind: ShipKind) -> Ship {
        let spec = kind.spec();
        Ship { kind, name: pick(SHIP_NAMES).to_string(), hull: 100, guns: (spec.max_guns * 3 / 4).max(2) }
    }
    pub fn value(&self) -> i32 {
        let spec = self.kind.spec();
        spec.value * self.hull.max(10) / 100 + self.guns * GUN_PRICE / 2
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Relation {
    War,
    Peace,
    Alliance,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub nation: Nation,
    pub difficulty: Difficulty,
    pub skill: Skill,
    pub age: f32,
    pub health: i32,
    pub ships: Vec<Ship>,
    pub crew: i32,
    pub morale: i32,
    pub food: i32,
    pub cargo: [i32; 4],
    /// The ship's purse: plunder not yet divided.
    pub gold: i32,
    /// Personal fortune, safely banked.
    pub wealth: i32,
    pub pos: Pos,
    pub heading: f32,
    /// Reputation with each nation (incl. pirates), roughly -100..100.
    pub standing: [i32; 5],
    /// Service points earned for each nation (towards promotion).
    pub points: [i32; 4],
    pub rank: [usize; 4],
    pub land: [i32; 4],
    pub marque: [bool; 4],
    pub romance: Vec<i32>,
    /// (town index, beauty 1..5)
    pub wife: Option<(usize, i32)>,
    pub family_found: [bool; 4],
    pub family_clue: [bool; 4],
    pub map_pieces: u8,
    pub treasure_found: bool,
    pub days_since_divide: f32,
    pub voyages: i32,
    pub ships_captured: i32,
    pub ships_sunk: i32,
    pub towns_sacked: i32,
    pub duels_won: i32,
    pub fame: i32,
}

impl Player {
    pub fn flagship(&self) -> &Ship {
        &self.ships[0]
    }
    pub fn crew_capacity(&self) -> i32 {
        self.ships.iter().map(|s| s.kind.spec().max_crew).sum()
    }
    pub fn cargo_capacity(&self) -> i32 {
        self.ships.iter().map(|s| s.kind.spec().cargo).sum()
    }
    pub fn cargo_used(&self) -> i32 {
        self.food + self.cargo.iter().sum::<i32>()
    }
    pub fn total_guns(&self) -> i32 {
        self.ships.iter().map(|s| s.guns).sum()
    }
    pub fn morale_word(&self) -> &'static str {
        match self.morale {
            75.. => "Happy",
            50..=74 => "Content",
            25..=49 => "Unhappy",
            _ => "Angry",
        }
    }
    pub fn health_word(&self) -> &'static str {
        match self.health {
            80.. => "Excellent",
            60..=79 => "Good",
            40..=59 => "Fair",
            20..=39 => "Poor",
            _ => "Failing",
        }
    }
    /// Fleet speed in degrees per day at the given wind angle.
    pub fn fleet_speed(&self, wind_angle: f32, wind_strength: f32) -> f32 {
        let mut best = f32::MAX;
        for s in &self.ships {
            let spec = s.kind.spec();
            let hull = 0.6 + 0.4 * s.hull as f32 / 100.0;
            let sp = spec.speed * wind_factor(spec.square, wind_angle) * hull;
            best = best.min(sp);
        }
        let mut sp = best * wind_strength;
        if self.skill == Skill::Navigation {
            sp *= 1.15;
        }
        // Undermanned ships are slow.
        let need = (self.ships.len() as i32 * 8).max(self.crew_capacity() / 8);
        if self.crew < need {
            sp *= 0.5 + 0.5 * self.crew as f32 / need as f32;
        }
        sp
    }
    pub fn fencing_skill(&self) -> f32 {
        let mut s = if self.skill == Skill::Fencing { 1.25 } else { 1.0 };
        if self.age > 35.0 {
            s -= (self.age - 35.0) * 0.025;
        }
        s *= 0.6 + 0.4 * self.health as f32 / 100.0;
        s.clamp(0.4, 1.5)
    }
    pub fn highest_rank(&self) -> (usize, Nation) {
        let mut best = (0, self.nation);
        for n in NATIONS {
            if self.rank[n.idx()] > best.0 {
                best = (self.rank[n.idx()], n);
            }
        }
        best
    }
    pub fn title(&self) -> String {
        let (r, _) = self.highest_rank();
        if r == 0 { format!("Captain {}", self.name) } else { format!("{} {}", RANKS[r], self.name) }
    }
    pub fn total_land(&self) -> i32 {
        self.land.iter().sum()
    }
    pub fn family_count(&self) -> usize {
        self.family_found.iter().filter(|&&b| b).count()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Town {
    pub def: usize,
    pub nation: Option<Nation>,
    pub pos: Pos,
    pub anchor: Pos,
    pub population: i32,
    pub soldiers: i32,
    pub wealth: i32,
    /// 0 none, 1 fort, 2 fortress
    pub fort: u8,
    /// Price multipliers for each good.
    pub price: [f32; 4],
    pub food_price: f32,
    pub recruits: i32,
    pub sacked_day: f64,
    /// Day until which a silver train's treasure sits in the town.
    pub silver_until: f64,
    pub gov_met: bool,
}

impl Town {
    pub fn name(&self) -> &'static str {
        TOWNS[self.def].name
    }
    pub fn size_word(&self) -> &'static str {
        match self.population {
            0..=999 => "Settlement",
            1000..=2999 => "Village",
            3000..=5999 => "Town",
            6000..=9999 => "City",
            _ => "Great City",
        }
    }
    pub fn wealth_word(&self) -> &'static str {
        match self.wealth {
            0..=999 => "Poor",
            1000..=2999 => "Modest",
            3000..=5999 => "Prosperous",
            6000..=11999 => "Wealthy",
            _ => "Rich",
        }
    }
    pub fn fort_word(&self) -> &'static str {
        ["no fort", "a fort", "a fortress"][self.fort as usize]
    }
    pub fn buy_price(&self, g: usize) -> i32 {
        ((GOOD_BASE_PRICE[g] as f32 * self.price[g]) * 1.1).round().max(1.0) as i32
    }
    pub fn sell_price(&self, g: usize) -> i32 {
        ((GOOD_BASE_PRICE[g] as f32 * self.price[g]) * 0.9).round().max(1.0) as i32
    }
    pub fn food_buy(&self) -> i32 {
        (FOOD_BASE_PRICE as f32 * self.food_price).round().max(1.0) as i32
    }
    pub fn is_haven(&self) -> bool {
        self.nation == Some(Nation::Pirate)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Merchant,
    Warship,
    Pirate,
    Hunter,
    TreasureFleet,
    Smuggler,
}

impl Role {
    pub fn desc(self) -> &'static str {
        match self {
            Role::Merchant => "merchant",
            Role::Warship => "warship",
            Role::Pirate => "pirate",
            Role::Hunter => "pirate hunter",
            Role::TreasureFleet => "treasure galleon",
            Role::Smuggler => "smuggler",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiShip {
    pub id: u32,
    pub pos: Pos,
    pub heading: f32,
    pub ship: Ship,
    pub nation: Nation,
    pub role: Role,
    pub crew: i32,
    pub gold: i32,
    pub cargo: [i32; 4],
    pub path: Vec<Pos>,
    pub dest: Option<usize>,
    pub captain: String,
    /// Days of remaining pursuit of the player.
    pub chase: f32,
    /// Cooldown after an encounter so it doesn't trigger immediately again.
    pub ignore: f32,
    pub has_clue: bool,
    pub has_map: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Storm {
    pub pos: Pos,
    pub vel: Pos,
    pub radius: f32,
    pub days: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct World {
    pub era: usize,
    pub start_year: i32,
    /// Days since 1 January of start_year (360-day calendar).
    pub day: f64,
    pub player: Player,
    pub towns: Vec<Town>,
    pub ships: Vec<AiShip>,
    pub relations: [[Relation; 4]; 4],
    /// Direction the wind blows toward (radians, screen space: 0 = east, PI/2 = south).
    pub wind_dir: f32,
    pub wind_strength: f32,
    pub family_towns: [usize; 4],
    pub treasure_pos: Pos,
    pub treasure_gold: i32,
    pub news: Vec<String>,
    pub log: Vec<String>,
    pub next_id: u32,
    pub last_month: i64,
    pub storm: Option<Storm>,
    pub fleet_month: i64,
    pub retired: bool,
    #[serde(default)]
    pub food_acc: f32,
}

pub const MONTHS: [&str; 12] = [
    "January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December",
];

pub struct NewGame {
    pub name: String,
    pub era: usize,
    pub year: i32,
    pub nation: Nation,
    pub difficulty: Difficulty,
    pub skill: Skill,
    pub expedition: Option<usize>,
}

impl World {
    pub fn new(geo: &Geo, ng: &NewGame) -> World {
        let era = ng.era;
        let mut towns = vec![];
        for (i, def) in TOWNS.iter().enumerate() {
            let (land, anchor) = geo.snap_town(def.lon, def.lat);
            let nation = owner_for_era(def, era);
            let spanish = nation == Some(Nation::Spain);
            let pop = def.size * 1500 + gen_range(0, 900) + if era >= 3 && !spanish { 600 } else { 0 };
            let wealth_base = if spanish { 1200 } else { 700 };
            let mut price = [1.0f32; 4];
            for (g, pr) in price.iter_mut().enumerate() {
                *pr = gen_range(0.85, 1.2);
                if def.produces == Some(g) {
                    *pr *= 0.55;
                }
                if g == 3 {
                    // European goods are cheaper in big ports.
                    *pr *= 1.3 - def.size as f32 * 0.1;
                }
            }
            towns.push(Town {
                def: i,
                nation,
                pos: p(land),
                anchor: p(anchor),
                population: pop,
                soldiers: def.size * 40 + gen_range(10, 60) + if spanish { 40 } else { 0 },
                wealth: def.size * wealth_base + gen_range(0, 1000),
                fort: if def.size >= 4 { 2 } else if def.size >= 2 { 1 } else { 0 },
                price,
                food_price: gen_range(0.8, 1.3),
                recruits: def.size * 15 + 10,
                sacked_day: -1000.0,
                silver_until: -1.0,
                gov_met: false,
            });
        }

        let relations = initial_relations(era);
        let nation = ng.nation;
        let mut standing = [0i32; 5];
        standing[nation.idx()] = 30;
        for n in NATIONS {
            if n != nation && relations[nation.idx()][n.idx()] == Relation::War {
                standing[n.idx()] = -15;
            }
        }
        standing[4] = -10;

        // Starting fleet and port.
        let (ships, crew, gold, start_town) = if let Some(e) = ng.expedition {
            let ex = &EXPEDITIONS[e];
            let ships: Vec<Ship> = ex.ships.iter().map(|k| {
                let mut s = Ship::new(*k);
                s.guns = k.spec().max_guns * 3 / 4;
                s
            }).collect();
            let st = towns.iter().position(|t| t.name() == ex.start_town);
            (ships, ex.crew, ex.gold, st)
        } else {
            let kind = match era {
                0 | 1 => ShipKind::Pinnace,
                2 | 3 => ShipKind::Barque,
                _ => ShipKind::Sloop,
            };
            let mut s = Ship::new(kind);
            s.guns = kind.spec().max_guns * 3 / 4;
            let home: Vec<usize> = towns.iter().enumerate().filter(|(_, t)| t.nation == Some(nation)).map(|(i, _)| i).collect();
            let st = if home.is_empty() {
                towns.iter().position(|t| t.nation == Some(Nation::Pirate))
            } else {
                // Prefer a sizable home port.
                home.iter().copied().max_by_key(|&i| TOWNS[i].size * 10 + gen_range(0, 15))
            };
            (vec![s], 40, 200, st)
        };
        let start_town = start_town.unwrap_or(0);
        let pos = towns[start_town].anchor;
        let n_towns = towns.len();

        let player = Player {
            name: ng.name.clone(),
            nation,
            difficulty: ng.difficulty,
            skill: ng.skill,
            age: 20.0 + gen_range(0, 4) as f32,
            health: 100,
            crew,
            morale: 80,
            food: (crew / 2 + 20).min(ships.iter().map(|s| s.kind.spec().cargo).sum::<i32>() / 2),
            ships,
            cargo: [0; 4],
            gold,
            wealth: 0,
            pos,
            heading: 0.0,
            standing,
            points: [0; 4],
            rank: [0; 4],
            land: [0; 4],
            marque: [false; 4],
            romance: vec![0; n_towns],
            wife: None,
            family_found: [false; 4],
            family_clue: [false; 4],
            map_pieces: 0,
            treasure_found: false,
            days_since_divide: 0.0,
            voyages: 1,
            ships_captured: 0,
            ships_sunk: 0,
            towns_sacked: 0,
            duels_won: 0,
            fame: 0,
        };

        // Lost family members are held in distant towns (mostly Spanish).
        let mut family_towns = [0usize; 4];
        for f in family_towns.iter_mut() {
            loop {
                let t = gen_range(0, n_towns);
                if towns[t].nation.is_some() && t != start_town {
                    *f = t;
                    break;
                }
            }
        }
        let treasure = geo.random_coast_land();
        let year = ng.year;
        let mut w = World {
            era,
            start_year: year,
            day: 0.0,
            player,
            towns,
            ships: vec![],
            relations,
            wind_dir: gen_range(0.0, std::f32::consts::TAU),
            wind_strength: 1.0,
            family_towns,
            treasure_pos: p(treasure),
            treasure_gold: 5000 + gen_range(0, 10) * 1000,
            news: vec![],
            log: vec![],
            next_id: 1,
            last_month: 0,
            storm: None,
            fleet_month: -1,
            retired: false,
            food_acc: 0.0,
        };
        // Start in a random month of the year.
        w.day = gen_range(0, 6) as f64 * 30.0 + gen_range(0, 20) as f64;
        w.last_month = w.month_index();
        for _ in 0..22 {
            w.spawn_ship(geo);
        }
        w.add_log(format!("Arrived at {} to seek fortune.", w.towns[start_town].name()));
        w
    }

    pub fn month_index(&self) -> i64 {
        (self.day / 30.0).floor() as i64
    }
    pub fn year(&self) -> i32 {
        self.start_year + (self.day / 360.0).floor() as i32
    }
    pub fn month(&self) -> usize {
        ((self.day / 30.0).floor() as i64).rem_euclid(12) as usize
    }
    pub fn date_string(&self) -> String {
        let d = (self.day.rem_euclid(30.0)).floor() as i32 + 1;
        format!("{} {} {}", d, MONTHS[self.month()], self.year())
    }
    pub fn short_date(&self) -> String {
        format!("{} {}", &MONTHS[self.month()][..3], self.year())
    }

    pub fn add_log(&mut self, s: String) {
        let entry = format!("{}: {}", self.short_date(), s);
        self.log.push(entry);
        if self.log.len() > 60 {
            self.log.remove(0);
        }
    }

    pub fn add_news(&mut self, s: String) {
        self.news.push(s);
        if self.news.len() > 12 {
            self.news.remove(0);
        }
    }

    pub fn relation(&self, a: Nation, b: Nation) -> Relation {
        if a == Nation::Pirate || b == Nation::Pirate {
            return Relation::War;
        }
        if a == b {
            return Relation::Alliance;
        }
        self.relations[a.idx()][b.idx()]
    }

    fn set_relation(&mut self, a: Nation, b: Nation, r: Relation) {
        self.relations[a.idx()][b.idx()] = r;
        self.relations[b.idx()][a.idx()] = r;
    }

    /// Is this nation hostile to the player (will its forts fire, its ships attack)?
    pub fn hostile(&self, n: Nation) -> bool {
        if n == Nation::Pirate {
            return self.player.standing[4] < -40;
        }
        self.player.standing[n.idx()] < -25
    }

    pub fn town_index(&self, name: &str) -> Option<usize> {
        self.towns.iter().position(|t| t.name() == name)
    }

    pub fn nearest_town(&self, pos: Pos) -> usize {
        let pv = v(pos);
        (0..self.towns.len())
            .filter(|&i| self.towns[i].nation.is_some())
            .min_by(|&a, &b| {
                v(self.towns[a].pos).distance(pv).partial_cmp(&v(self.towns[b].pos).distance(pv)).unwrap()
            })
            .unwrap_or(0)
    }

    /// Nearest town that will welcome the player.
    pub fn nearest_friendly_town(&self, pos: Pos) -> Option<usize> {
        let pv = v(pos);
        (0..self.towns.len())
            .filter(|&i| self.towns[i].nation.is_some_and(|n| !self.hostile(n)))
            .min_by(|&a, &b| {
                v(self.towns[a].pos).distance(pv).partial_cmp(&v(self.towns[b].pos).distance(pv)).unwrap()
            })
    }

    // ---------------------------------------------------------------- time

    /// Advance the world by `days` while the player is at sea.
    /// Returns messages that should be shown to the player.
    pub fn advance(&mut self, geo: &Geo, days: f32) -> Vec<String> {
        let mut msgs = vec![];
        let before = self.day;
        self.day += days as f64;
        let pl = &mut self.player;
        pl.days_since_divide += days;
        pl.age += days / 360.0;

        // Food: one ton feeds 100 men for ~3 days.
        self.food_acc += pl.crew as f32 * days / 300.0;
        let whole = self.food_acc.floor();
        self.food_acc -= whole;
        pl.food = (pl.food - whole as i32).max(0);

        // Daily events
        let crossed_day = before.floor() != self.day.floor();
        if crossed_day {
            if self.player.food == 0 && self.player.crew > 0 {
                self.player.morale -= 3;
                let lost = (self.player.crew / 30).max(1);
                self.player.crew = (self.player.crew - lost).max(0);
                if chance(0.15) {
                    msgs.push(format!("Starvation! {lost} men have died. The crew grows desperate."));
                }
            }
            // Wind shifts gradually.
            self.wind_dir += gen_range(-0.15, 0.15);
            if chance(0.04) {
                self.wind_dir += gen_range(-1.2, 1.2);
            }
            self.wind_strength = (self.wind_strength + gen_range(-0.06, 0.06)).clamp(0.6, 1.25);
            // Deep-draft ships in shallow water risk running aground.
            if geo.tile_at(self.player.pos.0, self.player.pos.1) == Tile::Shallow {
                let deep: Vec<usize> = (0..self.player.ships.len()).filter(|&i| self.player.ships[i].kind.spec().deep).collect();
                if !deep.is_empty() && chance(0.25) {
                    let i = pick(&deep);
                    self.player.ships[i].hull -= gen_range(5, 15);
                    msgs.push(format!("The {} scrapes bottom in the shallows! Hull damaged.", self.player.ships[i].name));
                    if let Some(m) = self.check_ships_lost() {
                        msgs.push(m);
                    }
                }
            }
            if let Some(m) = self.storm_tick(geo) {
                msgs.push(m);
            }
        }

        // Monthly events
        let m = self.month_index();
        while self.last_month < m {
            self.last_month += 1;
            msgs.extend(self.monthly(geo));
        }

        // AI ships
        self.update_ships(geo, days);
        msgs
    }

    fn check_ships_lost(&mut self) -> Option<String> {
        let mut msg = None;
        let mut i = 0;
        while i < self.player.ships.len() {
            if self.player.ships[i].hull <= 0 {
                let s = self.player.ships.remove(i);
                let lost = (self.player.crew as f32 * 0.2) as i32;
                self.player.crew -= lost;
                msg = Some(format!("The {} has sunk! {} men drowned.", s.name, lost));
            } else {
                i += 1;
            }
        }
        if self.player.ships.is_empty() {
            // Survivors cling to a captured longboat... give a battered sloop.
            let mut s = Ship::new(ShipKind::Sloop);
            s.hull = 40;
            s.guns = 2;
            s.name = "Longboat".into();
            self.player.ships.push(s);
        }
        self.trim_to_capacity();
        msg
    }

    /// Keep crew and cargo within the fleet's capacity.
    pub fn trim_to_capacity(&mut self) {
        let cap = self.player.crew_capacity();
        if self.player.crew > cap {
            self.player.crew = cap;
        }
        let ccap = self.player.cargo_capacity();
        while self.player.cargo_used() > ccap {
            if let Some(g) = (0..4).rev().find(|&g| self.player.cargo[g] > 0) {
                let over = self.player.cargo_used() - ccap;
                self.player.cargo[g] -= over.min(self.player.cargo[g]);
            } else {
                self.player.food = ccap;
                break;
            }
        }
        for s in self.player.ships.iter_mut() {
            s.guns = s.guns.min(s.kind.spec().max_guns).max(0);
        }
    }

    fn storm_tick(&mut self, geo: &Geo) -> Option<String> {
        let month = self.month();
        if self.storm.is_none() {
            let p_storm = if (6..=9).contains(&month) { 0.03 } else { 0.006 };
            if chance(p_storm) {
                let pos = geo.random_sea();
                self.storm = Some(Storm { pos: p(pos), vel: (gen_range(-0.6, -0.2), gen_range(-0.4, 0.1)), radius: gen_range(1.2, 2.5), days: gen_range(6.0, 14.0) });
                self.add_news("A great storm has been sighted at sea.".into());
            }
        }
        let mut msg = None;
        if let Some(st) = self.storm.as_mut() {
            st.pos.0 += st.vel.0;
            st.pos.1 += st.vel.1;
            st.days -= 1.0;
            let d = v(st.pos).distance(v(self.player.pos));
            if d < st.radius {
                // Blown off course and damaged.
                let push = vec2(gen_range(-1.0, 1.0), gen_range(-1.0, 1.0)).normalize_or_zero() * gen_range(0.3, 0.9);
                let np = v(self.player.pos) + push;
                if geo.is_water(np.x, np.y) && Geo::in_bounds(np.x, np.y) {
                    self.player.pos = p(np);
                }
                let i = gen_range(0, self.player.ships.len());
                let dmg = gen_range(3, 12);
                self.player.ships[i].hull -= dmg;
                let lost = gen_range(0, 3);
                self.player.crew = (self.player.crew - lost).max(0);
                if self.player.skill == Skill::Navigation && chance(0.5) {
                    self.player.ships[i].hull += dmg / 2;
                }
                msg = Some("A violent storm batters your fleet and blows you off course!".to_string());
            }
            if st.days <= 0.0 {
                self.storm = None;
            }
        }
        if msg.is_some() {
            if let Some(m) = self.check_ships_lost() {
                msg = Some(m);
            }
        }
        msg
    }

    fn monthly(&mut self, geo: &Geo) -> Vec<String> {
        let mut msgs = vec![];
        let at_sea_months = self.player.days_since_divide / 30.0;
        // Crew morale declines with time at sea, less so when the purse is fat.
        let per_man = self.player.gold as f32 / self.player.crew.max(1) as f32;
        let mut drop = 2.0 + at_sea_months * 0.8 - (per_man / 40.0).min(5.0);
        if drop < 0.5 {
            drop = 0.5;
        }
        self.player.morale = (self.player.morale - drop as i32).clamp(0, 100);
        if self.player.morale < 25 && self.player.crew > 5 && chance(0.5) {
            let lost = (self.player.crew as f32 * gen_range(0.08, 0.2)) as i32;
            self.player.crew -= lost;
            msgs.push(format!("The crew is angry! {lost} men desert in the night. Divide the plunder soon, Captain!"));
            if self.player.ships.len() > 1 && chance(0.4) {
                let s = self.player.ships.pop().unwrap();
                let mutineers = (self.player.crew / 5).min(s.kind.spec().max_crew);
                self.player.crew -= mutineers;
                msgs.push(format!("Mutiny! {mutineers} men seize the {} and sail off to go on the account.", s.name));
            }
        }
        // Aging and health.
        let age = self.player.age;
        let mut decline = if age < 30.0 { 0.0 } else if age < 40.0 { 0.4 } else if age < 50.0 { 0.9 } else { 1.6 };
        if self.player.skill == Skill::Medicine {
            decline *= 0.5;
        }
        let mut h = self.player.health as f32 - decline;
        if age < 40.0 && self.player.health < 80 {
            h += if self.player.skill == Skill::Medicine { 3.0 } else { 1.5 };
        }
        self.player.health = h.round().clamp(0.0, 100.0) as i32;
        if self.player.health < 15 && chance(0.3) {
            msgs.push("You feel the weight of your years. Perhaps it is time to retire...".into());
        }

        // Standing slowly drifts back toward neutral.
        for s in self.player.standing.iter_mut() {
            if *s < -10 {
                *s += 1;
            }
        }

        // Towns grow and recover.
        for t in self.towns.iter_mut() {
            if t.nation.is_none() {
                continue;
            }
            let def = &TOWNS[t.def];
            t.population = (t.population + gen_range(0, 30 + def.size * 15)).min(def.size * 3500 + 3000);
            let target_soldiers = def.size * 40 + 50 + if t.nation == Some(Nation::Spain) { 40 } else { 0 };
            if t.soldiers < target_soldiers {
                t.soldiers += gen_range(3, 10);
            }
            t.wealth += gen_range(0, 30 + t.population / 100);
            t.wealth = t.wealth.min(def.size * 4000 + 2000);
            for (g, pr) in t.price.iter_mut().enumerate() {
                let base = if def.produces == Some(g) { 0.55 } else { 1.0 };
                *pr += (base - *pr) * 0.15 + gen_range(-0.05, 0.05);
                *pr = pr.clamp(0.3, 2.2);
            }
            t.food_price = (t.food_price + gen_range(-0.1, 0.1)).clamp(0.6, 1.8);
            t.recruits = (t.recruits + def.size * 4 + 3).min(def.size * 25 + 20);
        }

        // Politics: random wars and treaties.
        if chance(0.12) {
            let a = gen_range(0, 4);
            let mut b = gen_range(0, 4);
            if a == b {
                b = (b + 1) % 4;
            }
            let (na, nb) = (Nation::from_idx(a), Nation::from_idx(b));
            let cur = self.relation(na, nb);
            let spain_bias = a == 0 || b == 0;
            let new = match cur {
                Relation::War => {
                    if chance(if spain_bias { 0.4 } else { 0.7 }) { Some(Relation::Peace) } else { None }
                }
                Relation::Peace => {
                    if chance(if spain_bias { 0.65 } else { 0.35 }) {
                        Some(Relation::War)
                    } else if chance(0.4) {
                        Some(Relation::Alliance)
                    } else {
                        None
                    }
                }
                Relation::Alliance => {
                    if chance(0.5) { Some(Relation::Peace) } else { None }
                }
            };
            if let Some(r) = new {
                self.set_relation(na, nb, r);
                let s = match r {
                    Relation::War => format!("{} has declared war on {}!", na.name(), nb.name()),
                    Relation::Peace => format!("{} and {} have signed a peace treaty.", na.name(), nb.name()),
                    Relation::Alliance => format!("{} and {} have formed an alliance.", na.name(), nb.name()),
                };
                self.add_news(s.clone());
                msgs.push(format!("News from Europe: {s}"));
                if r != Relation::War {
                    // Letters of marque against a former enemy lapse.
                    self.revoke_marques();
                }
            }
        }

        // Silver trains arrive at the treasure ports.
        if chance(0.3) {
            let ports: Vec<usize> = self.towns.iter().enumerate()
                .filter(|(_, t)| t.nation == Some(Nation::Spain) && ["Porto Bello", "Nombre de Dios", "Cartagena", "Vera Cruz", "Caracas"].contains(&t.name()))
                .map(|(i, _)| i)
                .collect();
            if !ports.is_empty() {
                let t = pick(&ports);
                self.towns[t].silver_until = self.day + 45.0;
                self.towns[t].wealth += 3000 + gen_range(0, 3000);
                let s = format!("A silver train has reached {}. Its vaults are full!", self.towns[t].name());
                self.add_news(s);
            }
        }
        // The annual treasure fleet.
        let month = self.month();
        if month == 4 && self.fleet_month != self.month_index() {
            self.fleet_month = self.month_index();
            self.spawn_treasure_fleet(geo);
        }
        if month == 2 {
            let s = "The Spanish treasure fleet will sail from Porto Bello in May, bound for Havana.".to_string();
            if !self.news.contains(&s) {
                self.add_news(s);
            }
        }
        msgs
    }

    fn revoke_marques(&mut self) {
        for a in NATIONS {
            if !self.player.marque[a.idx()] {
                continue;
            }
            let at_war = NATIONS.iter().any(|&b| b != a && self.relation(a, b) == Relation::War);
            if !at_war {
                self.player.marque[a.idx()] = false;
            }
        }
    }

    // ---------------------------------------------------------- AI shipping

    fn alloc_id(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id
    }

    pub fn spawn_ship(&mut self, geo: &Geo) {
        let roll = gen_range(0, 100);
        let role = match roll {
            0..=54 => Role::Merchant,
            55..=72 => Role::Warship,
            73..=87 => Role::Pirate,
            _ => Role::Smuggler,
        };
        let populated: Vec<usize> = (0..self.towns.len()).filter(|&i| self.towns[i].nation.is_some_and(|n| n != Nation::Pirate)).collect();
        if populated.is_empty() {
            return;
        }
        let (nation, start, dest) = match role {
            Role::Pirate => {
                let havens: Vec<usize> = (0..self.towns.len()).filter(|&i| self.towns[i].is_haven()).collect();
                let start = if !havens.is_empty() && chance(0.5) { v(self.towns[pick(&havens)].anchor) } else { geo.random_sea() };
                (Nation::Pirate, start, None)
            }
            _ => {
                let from = pick(&populated);
                let n = self.towns[from].nation.unwrap();
                let friends: Vec<usize> = populated.iter().copied().filter(|&i| {
                    i != from && self.towns[i].nation.is_some_and(|m| self.relation(n, m) != Relation::War)
                }).collect();
                let dest = if friends.is_empty() { None } else { Some(pick(&friends)) };
                let start = if chance(0.3) { geo.random_sea() } else { v(self.towns[from].anchor) };
                (if role == Role::Smuggler { pick(&[Nation::England, Nation::France, Nation::Holland]) } else { n }, start, dest)
            }
        };
        let kind = match role {
            Role::Merchant => pick(&[ShipKind::Fluyt, ShipKind::Merchantman, ShipKind::TradeGalleon, ShipKind::Barque, ShipKind::Pinnace, ShipKind::Merchantman]),
            Role::Warship => match self.era {
                0 | 1 => pick(&[ShipKind::FastGalleon, ShipKind::WarGalleon, ShipKind::Pinnace]),
                _ => pick(&[ShipKind::Frigate, ShipKind::FastGalleon, ShipKind::WarGalleon, ShipKind::Frigate]),
            },
            Role::Pirate => pick(&[ShipKind::Sloop, ShipKind::Pinnace, ShipKind::Barque, ShipKind::Sloop, ShipKind::Frigate]),
            Role::Smuggler => pick(&[ShipKind::Sloop, ShipKind::Pinnace, ShipKind::Fluyt]),
            _ => ShipKind::TradeGalleon,
        };
        self.add_ai_ship(geo, kind, nation, role, start, dest);
    }

    pub fn add_ai_ship(&mut self, geo: &Geo, kind: ShipKind, nation: Nation, role: Role, start: Vec2, dest: Option<usize>) -> Option<u32> {
        let spec = kind.spec();
        let target = match dest {
            Some(d) => v(self.towns[d].anchor),
            None => geo.random_sea(),
        };
        let path = geo.find_path(start, target)?;
        let mut ship = Ship::new(kind);
        let crew_frac = match role {
            Role::Warship | Role::Hunter | Role::Pirate => gen_range(0.7, 1.0),
            Role::TreasureFleet => gen_range(0.6, 0.9),
            _ => gen_range(0.3, 0.55),
        };
        let gun_frac = match role {
            Role::Warship | Role::Hunter | Role::TreasureFleet => gen_range(0.8, 1.0),
            Role::Pirate => gen_range(0.6, 1.0),
            _ => gen_range(0.3, 0.7),
        };
        ship.guns = ((spec.max_guns as f32 * gun_frac) as i32).max(2);
        let crew = ((spec.max_crew as f32 * crew_frac) as i32).max(8);
        let era_rich = if self.era <= 2 { 1.2 } else { 1.0 };
        let gold = (match role {
            Role::Merchant => gen_range(100, 800),
            Role::Warship => gen_range(50, 300),
            Role::Pirate => gen_range(200, 1500),
            Role::Hunter => gen_range(50, 300),
            Role::TreasureFleet => gen_range(3000, 8000),
            Role::Smuggler => gen_range(100, 600),
        } as f32
            * era_rich) as i32;
        let mut cargo = [0; 4];
        if matches!(role, Role::Merchant | Role::Smuggler | Role::TreasureFleet) {
            let cap = spec.cargo;
            for c in cargo.iter_mut() {
                if chance(0.5) {
                    *c = gen_range(0, cap / 3);
                }
            }
        }
        let id = self.alloc_id();
        let first = path.first().copied().unwrap_or(target);
        let heading = (first - start).y.atan2((first - start).x);
        self.ships.push(AiShip {
            id,
            pos: p(start),
            heading,
            ship,
            nation,
            role,
            crew,
            gold,
            cargo,
            path: path.into_iter().map(p).collect(),
            dest,
            captain: pick(CAPTAIN_NAMES).to_string(),
            chase: 0.0,
            ignore: 0.0,
            has_clue: nation == Nation::Spain && chance(0.12),
            has_map: role == Role::Pirate && chance(0.25),
        });
        Some(id)
    }

    fn spawn_treasure_fleet(&mut self, geo: &Geo) {
        let start = self
            .town_index("Porto Bello")
            .or_else(|| self.town_index("Nombre de Dios"))
            .filter(|&i| self.towns[i].nation == Some(Nation::Spain));
        let havana = self.town_index("Havana").filter(|&i| self.towns[i].nation == Some(Nation::Spain));
        let (Some(s), Some(h)) = (start, havana) else { return };
        let n = gen_range(2, 5);
        for k in 0..n {
            let kind = if k == 0 { ShipKind::WarGalleon } else { pick(&[ShipKind::TradeGalleon, ShipKind::FastGalleon, ShipKind::TradeGalleon]) };
            let off = vec2(gen_range(-0.3, 0.3), gen_range(-0.3, 0.0));
            let st = v(self.towns[s].anchor) + off;
            let st = if geo.is_water(st.x, st.y) { st } else { v(self.towns[s].anchor) };
            self.add_ai_ship(geo, kind, Nation::Spain, Role::TreasureFleet, st, Some(h));
        }
        self.add_news(format!("The treasure fleet has sailed from {} for Havana, laden with silver!", self.towns[s].name()));
    }

    /// Spawn warships hunting the player when a nation hates them.
    fn maybe_spawn_hunter(&mut self, geo: &Geo) {
        let hunters = self.ships.iter().filter(|s| s.role == Role::Hunter).count();
        if hunters >= 2 {
            return;
        }
        let worst = NATIONS.iter().copied().min_by_key(|n| self.player.standing[n.idx()]).unwrap();
        if self.player.standing[worst.idx()] > -40 || !chance(0.002) {
            return;
        }
        let pp = v(self.player.pos);
        for _ in 0..20 {
            let off = vec2(gen_range(-4.0, 4.0), gen_range(-3.0, 3.0));
            let st = pp + off;
            if off.length() > 2.5 && Geo::in_bounds(st.x, st.y) && geo.tile_at(st.x, st.y) == Tile::Sea {
                let kind = if self.era <= 1 { ShipKind::FastGalleon } else { pick(&[ShipKind::Frigate, ShipKind::FastGalleon]) };
                if let Some(id) = self.add_ai_ship(geo, kind, worst, Role::Hunter, st, None) {
                    if let Some(s) = self.ships.iter_mut().find(|s| s.id == id) {
                        s.chase = 25.0;
                    }
                }
                return;
            }
        }
    }

    fn update_ships(&mut self, geo: &Geo, days: f32) {
        let pp = v(self.player.pos);
        let wind = self.wind_dir;
        let ws = self.wind_strength;
        let player_hated: [bool; 5] = std::array::from_fn(|i| self.hostile(Nation::from_idx(i)));
        let mut arrived = vec![];
        for (idx, s) in self.ships.iter_mut().enumerate() {
            s.ignore = (s.ignore - days).max(0.0);
            let spec = s.ship.kind.spec();
            let pos = v(s.pos);
            // Aggressive ships near the player may give chase.
            let dist = pos.distance(pp);
            let aggressive = match s.role {
                Role::Pirate => s.ship.guns >= 6,
                Role::Hunter => true,
                Role::Warship => player_hated[s.nation.idx()],
                _ => false,
            };
            if aggressive && dist < 3.0 && s.ignore <= 0.0 && s.chase <= 0.0 && chance(0.05 * days * 30.0) {
                s.chase = 6.0;
            }
            let target = if s.chase > 0.0 && s.ignore <= 0.0 {
                s.chase -= days;
                Some(pp)
            } else {
                s.path.first().map(|&q| v(q))
            };
            let Some(target) = target else {
                arrived.push(idx);
                continue;
            };
            let to = target - pos;
            if to.length() < 0.15 {
                if s.chase <= 0.0 {
                    s.path.remove(0);
                    if s.path.is_empty() {
                        arrived.push(idx);
                    }
                }
                continue;
            }
            let desired = to.y.atan2(to.x);
            s.heading = desired;
            let mut angle = (-desired - wind).rem_euclid(std::f32::consts::TAU);
            if angle > std::f32::consts::PI {
                angle = std::f32::consts::TAU - angle;
            }
            // AI captains tack cleverly: never slower than 45% speed.
            let f = wind_factor(spec.square, angle).max(0.45);
            let sp = spec.speed * f * ws * 0.85 * days;
            let np = pos + to.normalize() * sp.min(to.length());
            if geo.is_water(np.x, np.y) {
                s.pos = p(np);
            } else if s.chase > 0.0 {
                // Give up the chase if land is in the way.
                s.chase = 0.0;
                if let Some(path) = s.path.first().map(|&q| v(q)).and_then(|q| geo.find_path(pos, q)) {
                    s.path = path.into_iter().map(p).collect();
                }
            } else {
                s.pos = p(np);
            }
        }
        // Ships that reached their destination deliver goods and vanish.
        for idx in arrived.into_iter().rev() {
            let s = self.ships.remove(idx);
            if let Some(d) = s.dest {
                let t = &mut self.towns[d];
                t.wealth += s.gold / 4;
                for g in 0..4 {
                    if s.cargo[g] > 20 {
                        t.price[g] = (t.price[g] - 0.05).max(0.3);
                    }
                }
            }
        }
        // Keep the seas busy.
        let wanted = 22;
        let n = self.ships.iter().filter(|s| s.role != Role::TreasureFleet).count();
        if n < wanted && chance(0.3 * days) {
            self.spawn_ship(geo);
        }
        self.maybe_spawn_hunter(geo);
    }

    // ---------------------------------------------------------- reputation

    /// Credit the player for an act against nation `victim` worth `value`.
    /// Returns a short description of who is pleased.
    pub fn act_against(&mut self, victim: Nation, value: i32) -> Vec<String> {
        let mut out = vec![];
        if victim == Nation::Pirate {
            for n in NATIONS {
                self.player.standing[n.idx()] = (self.player.standing[n.idx()] + value / 2 + 1).min(100);
                self.player.points[n.idx()] += value / 2;
            }
            self.player.standing[4] -= value * 2;
            out.push("All nations welcome the destruction of pirates.".into());
            return out;
        }
        self.player.standing[victim.idx()] = (self.player.standing[victim.idx()] - value * 3).max(-100);
        for n in NATIONS {
            if n == victim {
                continue;
            }
            let rel = self.relation(n, victim);
            if rel == Relation::War && (self.player.marque[n.idx()] || self.player.standing[n.idx()] >= 0) {
                let bonus = if self.player.marque[n.idx()] { value + 1 } else { value };
                self.player.points[n.idx()] += bonus;
                self.player.standing[n.idx()] = (self.player.standing[n.idx()] + bonus).min(100);
                out.push(format!("{} is pleased by your attack on its enemy.", n.name()));
            } else if rel == Relation::Alliance {
                self.player.standing[n.idx()] = (self.player.standing[n.idx()] - value).max(-100);
            }
        }
        // Attacks without a war are piracy.
        let any_war = NATIONS.iter().any(|&n| n != victim && self.relation(n, victim) == Relation::War && self.player.standing[n.idx()] >= 0);
        if !any_war {
            self.player.standing[4] = (self.player.standing[4] + value).min(100);
            out.push(format!("Without a war against {}, this is an act of piracy!", victim.name()));
        }
        self.player.fame += value;
        out
    }

    /// Pending promotion for the given nation, if any.
    pub fn pending_rank(&self, n: Nation) -> Option<usize> {
        let i = n.idx();
        let cur = self.player.rank[i];
        let mut r = cur;
        while r + 1 < RANKS.len() && self.player.points[i] >= RANK_POINTS[r + 1] {
            r += 1;
        }
        if r > cur && self.player.standing[i] >= 10 { Some(r) } else { None }
    }

    /// Player's notoriety drives how many recruits will sign on.
    pub fn reputation_word(&self) -> &'static str {
        match self.player.fame {
            0..=4 => "unknown",
            5..=14 => "known",
            15..=34 => "respected",
            35..=69 => "famous",
            _ => "legendary",
        }
    }

    pub fn era_year_desc(&self) -> String {
        format!("{} ({})", ERAS[self.era].title, self.start_year)
    }

    /// Final score for retirement.
    pub fn score(&self) -> (i32, Vec<(String, i32)>) {
        let pl = &self.player;
        let mut parts = vec![];
        let wealth_pts = (pl.wealth / 1000).min(40);
        parts.push((format!("Personal wealth: {} pieces of eight", pl.wealth), wealth_pts));
        let land_pts = pl.total_land() / 100;
        parts.push((format!("Land holdings: {} acres", pl.total_land()), land_pts));
        let (r, n) = pl.highest_rank();
        let rank_pts = r as i32 * 2;
        parts.push((
            if r > 0 { format!("Highest title: {} ({})", RANKS[r], n.adj()) } else { "No titles".into() },
            rank_pts,
        ));
        let wife_pts = pl.wife.map(|(_, b)| b * 2).unwrap_or(0);
        parts.push((
            match pl.wife {
                Some((t, b)) => format!("Married the {} daughter of the Governor of {}", ["", "plain", "pleasant", "attractive", "beautiful", "radiant"][b.clamp(1, 5) as usize], self.towns[t].name()),
                None => "Never married".into(),
            },
            wife_pts,
        ));
        let fam = pl.family_count() as i32;
        parts.push((format!("Family members rescued: {fam} of 4"), fam * 3));
        parts.push((if pl.treasure_found { "Found the lost treasure".into() } else { "Treasure never found".into() }, if pl.treasure_found { 4 } else { 0 }));
        let diff = pl.difficulty.idx() as i32;
        parts.push((format!("Difficulty: {}", pl.difficulty.name()), diff * 2));
        let total: i32 = parts.iter().map(|x| x.1).sum();
        (total, parts)
    }

    pub fn save(&self, path: &str) -> Result<(), String> {
        let s = serde_json::to_string(self).map_err(|e| e.to_string())?;
        std::fs::write(path, s).map_err(|e| e.to_string())
    }

    pub fn load(path: &str) -> Result<World, String> {
        let s = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&s).map_err(|e| e.to_string())
    }
}

fn initial_relations(era: usize) -> [[Relation; 4]; 4] {
    use Relation::*;
    // Order: Spain, England, France, Holland
    let table: [[Relation; 4]; 4] = match era {
        0 => [[Alliance, War, War, Peace], [War, Alliance, Peace, Peace], [War, Peace, Alliance, Peace], [Peace, Peace, Peace, Alliance]],
        1 => [[Alliance, War, Peace, War], [War, Alliance, Peace, Alliance], [Peace, Peace, Alliance, Peace], [War, Alliance, Peace, Alliance]],
        2 => [[Alliance, Peace, War, War], [Peace, Alliance, Peace, Peace], [War, Peace, Alliance, Peace], [War, Peace, Peace, Alliance]],
        3 => [[Alliance, War, War, War], [War, Alliance, Peace, Peace], [War, Peace, Alliance, Peace], [War, Peace, Peace, Alliance]],
        4 => [[Alliance, War, War, Peace], [War, Alliance, Peace, War], [War, Peace, Alliance, Peace], [Peace, War, Peace, Alliance]],
        _ => [[Alliance, Peace, War, Peace], [Peace, Alliance, War, Alliance], [War, War, Alliance, War], [Peace, Alliance, War, Alliance]],
    };
    table
}
