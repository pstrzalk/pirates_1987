//! Game flow: the screen state machine, dialogs, and the rules glue that
//! connects towns, encounters, battles and plunder.

use crate::art::{self, Picture};
use crate::data::*;
use crate::fencing::{Duel, DuelKind, DuelOutcome, DuelSetup};
use crate::geo::Geo;
use crate::info::Info;
use crate::input::Input;
use crate::landbattle::{LandBattle, LandOutcome, LandOutcomeKind, LandSetup};
use crate::market::Market;
use crate::naval::{NavalBattle, NavalEnemy, NavalOutcome, NavalOutcomeKind, NavalSetup};
use crate::sail::Sail;
use crate::title::Title;
use crate::ending::Ending;
use crate::ui::{self, Menu};
use crate::world::*;
use macroquad::prelude::*;
use macroquad::rand::gen_range;

pub const SAVE_FILE: &str = "pirates.sav";

#[derive(Clone, Debug)]
pub enum Action {
    Nothing,
    ToSail,
    ToTitle,
    QuitGame,
    NewGame(usize, i32, Nation, String, Difficulty, Skill, Option<usize>),
    LoadGame,
    SaveGame,
    // Town
    ArriveTown(usize),
    TownMenu(usize),
    Governor(usize),
    Tavern(usize),
    Merchant(usize),
    Shipwright(usize),
    Divide(usize),
    DivideConfirm(usize),
    Retire,
    InfoScreen(usize),
    InfoSail,
    Encounter(u32, bool),
    AcceptMarque(usize, Nation),
    Court(usize),
    Propose(usize),
    Recruit(usize),
    Rumors(usize),
    BuyClue(usize, i32),
    BuyMap(usize, i32),
    Repair(usize),
    SellShipMenu(usize),
    SellShip(usize, usize),
    ChangeFlagship(usize, usize),
    // Hostile town
    AttackFort(usize),
    March(usize),
    Sneak(usize),
    InstallGovernor(usize, Nation),
    // Encounters
    AttackShip(u32),
    HailShip(u32),
    IgnoreShip(u32),
    EscapeShip(u32),
    KeepPrize,
    SinkPrize,
    FreePrize,
}

/// A text screen with an optional picture and a menu of choices.
pub struct Dialog {
    pub title: String,
    pub body: String,
    pub picture: Picture,
    pub menu: Menu,
    pub actions: Vec<Action>,
    pub cancel: Option<Action>,
}

impl Dialog {
    pub fn new(title: impl Into<String>, body: impl Into<String>, picture: Picture, opts: Vec<(String, Action)>) -> Dialog {
        let (labels, actions): (Vec<String>, Vec<Action>) = opts.into_iter().unzip();
        Dialog { title: title.into(), body: body.into(), picture, menu: Menu::new(labels), actions, cancel: None }
    }
    pub fn msg(title: impl Into<String>, body: impl Into<String>, picture: Picture, next: Action) -> Dialog {
        let mut d = Dialog::new(title, body, picture, vec![("Continue".into(), next.clone())]);
        d.cancel = Some(next);
        d
    }
    pub fn with_cancel(mut self, a: Action) -> Dialog {
        self.cancel = Some(a);
        self
    }

    fn update(&mut self, inp: &Input) -> Option<Action> {
        if let Some(i) = self.menu.update(inp) {
            return Some(self.actions[i].clone());
        }
        if inp.cancel() {
            return self.cancel.clone();
        }
        None
    }

    fn draw(&self, world: Option<&World>, geo: &Geo) {
        let has_pic = self.picture != Picture::None;
        let (px, py, pw, ph) = (0.0, 0.0, ui::W, 190.0);
        if has_pic {
            art::draw_picture(&self.picture, px, py, pw, ph, world, geo);
        } else {
            ui::rect(0.0, 0.0, ui::W, ui::H, ui::BLUE);
        }
        let ty = if has_pic { 192.0 } else { 30.0 };
        let th = ui::H - ty - if has_pic { 2.0 } else { 30.0 };
        let (tx, tw) = if has_pic { (2.0, ui::W - 4.0) } else { (60.0, ui::W - 120.0) };
        ui::parchment(tx, ty, tw, th);
        ui::text(&self.title, tx + 14.0, ty + 10.0, 16.0, ui::RED);
        let menu_h = self.menu.height(14.0);
        let lines = ui::wrap(&self.body, 13.0, tw - 28.0);
        let max_lines = ((th - 40.0 - menu_h - 10.0) / 15.0).max(1.0) as usize;
        for (i, l) in lines.iter().take(max_lines).enumerate() {
            ui::text(l, tx + 14.0, ty + 32.0 + i as f32 * 15.0, 13.0, ui::INK);
        }
        let my = ty + th - menu_h - 10.0;
        self.menu.draw(tx + 10.0, my, tw - 20.0, 14.0, ui::INK, ui::BROWN, ui::WHITE);
    }
}

pub enum Screen {
    Title(Title),
    Sail(Sail),
    Dialog(Dialog),
    Market(Market),
    Naval(NavalBattle),
    Duel(Duel),
    Land(LandBattle),
    Info(Info),
    Ending(Ending),
}

/// What a battle in progress is about, so its outcome can be applied.
#[derive(Clone, Debug)]
pub enum Pending {
    ShipFight(Box<AiShip>),
    Boarding(Box<AiShip>, NavalOutcome),
    FortAssault(usize),
    TownDuel(usize),
    LandAssault(usize),
    SneakFight(usize),
}

pub struct App {
    pub geo: Geo,
    pub world: Option<World>,
    pub screen: Screen,
    pub pending: Option<Pending>,
    /// Prize ship waiting for the keep/sink decision.
    pub prize: Option<AiShip>,
    /// Town the player has sneaked into (restricted access).
    pub sneaked: Option<usize>,
    pub quit: bool,
}

impl App {
    pub fn new() -> App {
        App { geo: Geo::new(), world: None, screen: Screen::Title(Title::new()), pending: None, prize: None, sneaked: None, quit: false }
    }

    fn w(&mut self) -> &mut World {
        self.world.as_mut().expect("world")
    }

    pub fn update(&mut self, inp: &Input, dt: f32) {
        let act = match &mut self.screen {
            Screen::Title(t) => t.update(inp),
            Screen::Dialog(d) => d.update(inp),
            Screen::Market(m) => {
                let w = self.world.as_mut().unwrap();
                m.update(inp, w)
            }
            Screen::Info(i) => i.update(inp),
            Screen::Ending(e) => e.update(inp),
            Screen::Sail(s) => {
                let w = self.world.as_mut().unwrap();
                s.update(inp, dt, w, &self.geo)
            }
            Screen::Naval(b) => {
                if let Some(o) = b.update(inp, dt) {
                    self.on_naval(o);
                }
                None
            }
            Screen::Duel(d) => {
                if let Some(o) = d.update(inp, dt) {
                    self.on_duel(o);
                }
                None
            }
            Screen::Land(l) => {
                if let Some(o) = l.update(inp, dt) {
                    self.on_land(o);
                }
                None
            }
        };
        if let Some(a) = act {
            self.handle(a);
        }
    }

    pub fn draw(&self) {
        match &self.screen {
            Screen::Title(t) => t.draw(&self.geo),
            Screen::Dialog(d) => d.draw(self.world.as_ref(), &self.geo),
            Screen::Market(m) => m.draw(self.world.as_ref().unwrap()),
            Screen::Info(i) => i.draw(self.world.as_ref().unwrap(), &self.geo),
            Screen::Ending(e) => e.draw(),
            Screen::Sail(s) => s.draw(self.world.as_ref().unwrap(), &self.geo),
            Screen::Naval(b) => b.draw(),
            Screen::Duel(d) => d.draw(),
            Screen::Land(l) => l.draw(),
        }
    }

    fn dialog(&mut self, d: Dialog) {
        self.screen = Screen::Dialog(d);
    }

    fn message(&mut self, title: impl Into<String>, body: impl Into<String>, pic: Picture, next: Action) {
        self.dialog(Dialog::msg(title, body, pic, next));
    }

    pub fn handle(&mut self, a: Action) {
        match a {
            Action::Nothing => {}
            Action::ToSail => {
                self.sneaked = None;
                self.screen = Screen::Sail(Sail::new());
                self.check_forced_retirement();
            }
            Action::ToTitle => {
                self.world = None;
                self.screen = Screen::Title(Title::new());
            }
            Action::QuitGame => self.quit = true,
            Action::NewGame(era, year, nation, name, diff, skill, exp) => self.new_game(era, year, nation, name, diff, skill, exp),
            Action::LoadGame => match World::load(SAVE_FILE) {
                Ok(w) => {
                    self.world = Some(w);
                    self.message("Game Loaded", "Your saved career has been restored. Fair winds, Captain!", Picture::Sea, Action::ToSail);
                }
                Err(e) => {
                    let back = if self.world.is_some() { Action::ToSail } else { Action::ToTitle };
                    self.message("Cannot Load", format!("No saved game could be loaded ({e})."), Picture::None, back);
                }
            },
            Action::SaveGame => {
                let r = self.w().save(SAVE_FILE);
                let body = match r {
                    Ok(()) => format!("Your career has been saved to {SAVE_FILE}."),
                    Err(e) => format!("Saving failed: {e}"),
                };
                self.message("Save Game", body, Picture::None, Action::ToSail);
            }
            Action::ArriveTown(t) => self.arrive_town(t),
            Action::TownMenu(t) => self.town_menu(t),
            Action::Governor(t) => self.governor(t),
            Action::Tavern(t) => self.tavern(t),
            Action::Merchant(t) => self.screen = Screen::Market(Market::new(t)),
            Action::Shipwright(t) => self.shipwright(t),
            Action::Divide(t) => self.divide(t),
            Action::DivideConfirm(t) => self.divide_confirm(t),
            Action::Retire => self.retire(),
            Action::InfoScreen(t) => self.screen = Screen::Info(Info::new(Action::TownMenu(t))),
            Action::InfoSail => self.screen = Screen::Info(Info::new(Action::ToSail)),
            Action::Encounter(id, attacking) => self.encounter(id, attacking),
            Action::AcceptMarque(t, enemy) => {
                let n = self.w().towns[t].nation.unwrap();
                let w = self.w();
                w.player.marque[n.idx()] = true;
                w.player.standing[n.idx()] += 5;
                w.add_log(format!("Accepted a Letter of Marque from {} against {}.", n.name(), enemy.name()));
                self.message(
                    "Letter of Marque",
                    format!("The governor hands you a commission sealed with the arms of {}. You may now attack {} ships and towns in the King's name, and you will be rewarded for it.", n.name(), enemy.adj()),
                    Picture::Governor(t),
                    Action::Governor(t),
                );
            }
            Action::Court(t) => self.court(t),
            Action::Propose(t) => self.propose(t),
            Action::Recruit(t) => self.recruit(t),
            Action::Rumors(t) => self.rumors(t),
            Action::BuyClue(t, price) => self.buy_clue(t, price),
            Action::BuyMap(t, price) => self.buy_map(t, price),
            Action::Repair(t) => {
                let cost = self.repair_cost();
                let w = self.w();
                if w.player.gold >= cost {
                    w.player.gold -= cost;
                    for s in w.player.ships.iter_mut() {
                        s.hull = 100;
                    }
                    self.message("Shipwright", format!("Your ships are careened, caulked and repaired for {cost} pieces of eight."), Picture::Shipyard, Action::Shipwright(t));
                } else {
                    self.message("Shipwright", "\"You haven't the gold for such work, Captain.\"", Picture::Shipyard, Action::Shipwright(t));
                }
            }
            Action::SellShipMenu(t) => self.sell_ship_menu(t),
            Action::SellShip(t, i) => {
                let w = self.w();
                if w.player.ships.len() > 1 && i < w.player.ships.len() {
                    let s = w.player.ships.remove(i);
                    let price = s.value() / 2;
                    w.player.gold += price;
                    w.trim_to_capacity();
                    self.message("Shipwright", format!("The {} {} is sold for {price} pieces of eight.", s.kind.name(), s.name), Picture::Shipyard, Action::Shipwright(t));
                } else {
                    self.handle(Action::Shipwright(t));
                }
            }
            Action::ChangeFlagship(t, i) => {
                let w = self.w();
                if i < w.player.ships.len() {
                    w.player.ships.swap(0, i);
                }
                self.handle(Action::Shipwright(t));
            }
            Action::AttackFort(t) => self.attack_fort(t),
            Action::March(t) => self.march(t),
            Action::Sneak(t) => self.sneak(t),
            Action::InstallGovernor(t, n) => {
                let w = self.w();
                let old = w.towns[t].nation;
                w.towns[t].nation = Some(n);
                w.towns[t].soldiers = 40 + gen_range(0, 30);
                w.player.points[n.idx()] += 5;
                w.player.standing[n.idx()] = (w.player.standing[n.idx()] + 10).min(100);
                let name = w.towns[t].name();
                w.add_log(format!("Installed a {} governor in {}.", n.adj(), name));
                w.add_news(format!("{} has fallen to {}!", name, n.name()));
                let _ = old;
                self.message("A New Governor", format!("You install a new governor loyal to {}. {} now flies the {} flag, and the grateful crown will remember your service.", n.name(), name, n.adj()), Picture::Town(t), Action::ToSail);
            }
            Action::AttackShip(id) => self.attack_ship(id),
            Action::HailShip(id) => self.hail_ship(id),
            Action::IgnoreShip(id) => {
                if let Some(s) = self.w().ships.iter_mut().find(|s| s.id == id) {
                    s.ignore = 4.0;
                    s.chase = 0.0;
                }
                self.handle(Action::ToSail);
            }
            Action::EscapeShip(id) => self.escape_ship(id),
            Action::KeepPrize => self.keep_prize(),
            Action::SinkPrize | Action::FreePrize => {
                let sink = matches!(a, Action::SinkPrize);
                if let Some(pz) = self.prize.take() {
                    let w = self.w();
                    if sink {
                        w.player.ships_sunk += 1;
                    }
                    let body = if sink {
                        format!("The {} is put to the torch and sinks beneath the waves.", pz.ship.kind.name())
                    } else {
                        format!("The {} is left to drift with her surviving crew.", pz.ship.kind.name())
                    };
                    self.message("The Prize", body, Picture::Ship(pz.ship.kind, pz.nation), Action::ToSail);
                } else {
                    self.handle(Action::ToSail);
                }
            }
        }
    }

    // ------------------------------------------------------------ new game

    #[allow(clippy::too_many_arguments)]
    fn new_game(&mut self, era: usize, year: i32, nation: Nation, name: String, diff: Difficulty, skill: Skill, exp: Option<usize>) {
        let ng = NewGame { name, era, year, nation, difficulty: diff, skill, expedition: exp };
        let w = World::new(&self.geo, &ng);
        let start = w.nearest_town(w.player.pos);
        let story = if let Some(e) = exp {
            let ex = &EXPEDITIONS[e];
            format!("{}\n\nYou command {} ships and {} men. Your fleet lies at anchor off {}.", ex.desc, w.player.ships.len(), w.player.crew, w.towns[start].name())
        } else {
            format!(
                "The year is {}. Years ago the evil Marquis de la Montalban betrayed your family and scattered them across the Spanish Main: your {}, your {}, your {} and your {} vanished into his prisons.\n\nNow a grown man, you have come to the New World in the service of {} with a small {} and {} men, determined to win fame, fortune and a title -- and to find your lost family.",
                year, RELATIVES[0], RELATIVES[1], RELATIVES[2], RELATIVES[3], nation.name(), w.player.ships[0].kind.name(), w.player.crew
            )
        };
        self.world = Some(w);
        self.message("A New Career", story, Picture::Town(start), Action::ArriveTown(start));
    }

    // -------------------------------------------------------------- towns

    fn arrive_town(&mut self, t: usize) {
        let w = self.world.as_ref().unwrap();
        let town = &w.towns[t];
        let Some(n) = town.nation else {
            self.handle(Action::ToSail);
            return;
        };
        let name = town.name();
        if w.hostile(n) {
            let body = format!(
                "{} is a {} town and its people are hostile to you. The fort's guns are trained on your ships. {} soldiers guard the town, which has {}.",
                name,
                n.adj(),
                town.soldiers,
                town.fort_word()
            );
            let opts = vec![
                ("Attack the town from the sea".to_string(), Action::AttackFort(t)),
                ("Land your men and march on the town".to_string(), Action::March(t)),
                ("Sneak into town by night".to_string(), Action::Sneak(t)),
                ("Sail away".to_string(), Action::ToSail),
            ];
            self.dialog(Dialog::new(format!("{name} ({})", n.adj()), body, Picture::Town(t), opts).with_cancel(Action::ToSail));
            return;
        }
        // Friendly arrival: reunite with family if they are here.
        let mut extra = String::new();
        let w = self.w();
        for k in 0..4 {
            if w.player.family_clue[k] && !w.player.family_found[k] && w.family_towns[k] == t {
                w.player.family_found[k] = true;
                w.add_log(format!("Found my long-lost {} in {}.", RELATIVES[k], name));
                extra = format!("Joy! Following the clues you have gathered, you find your long-lost {} here in {}. After a tearful reunion, your {} tells you what is known of the rest of the family.", RELATIVES[k], name, RELATIVES[k]);
                if let Some(next) = (0..4).find(|&j| !w.player.family_found[j] && !w.player.family_clue[j]) {
                    w.player.family_clue[next] = true;
                    extra.push_str(&format!(" Your {} is said to be held in {}.", RELATIVES[next], w.towns[w.family_towns[next]].name()));
                }
                if w.player.map_pieces < 4 && chance(0.5) {
                    w.player.map_pieces += 1;
                    extra.push_str(" Your relative also gives you a torn piece of an old treasure map!");
                }
                break;
            }
        }
        if !extra.is_empty() {
            self.message("A Family Reunion", extra, Picture::Family, Action::TownMenu(t));
        } else {
            self.town_menu(t);
        }
    }

    fn town_menu(&mut self, t: usize) {
        let w = self.world.as_ref().unwrap();
        let town = &w.towns[t];
        let n = town.nation.unwrap_or(Nation::Pirate);
        let sneaked = self.sneaked == Some(t);
        let body = format!(
            "{}. {} {}, population {}. It is {} and defended by {} soldiers and {}.{}",
            w.date_string(),
            if town.is_haven() { "A pirate haven" } else { n.adj() },
            town.size_word().to_lowercase(),
            town.population,
            town.wealth_word().to_lowercase(),
            town.soldiers,
            town.fort_word(),
            if sneaked { " You move through the streets in disguise." } else { "" }
        );
        let mut opts = vec![];
        if !town.is_haven() && !sneaked {
            opts.push(("Visit the Governor".to_string(), Action::Governor(t)));
        }
        opts.push(("Visit the Tavern".to_string(), Action::Tavern(t)));
        opts.push(("Visit the Merchant".to_string(), Action::Merchant(t)));
        if !sneaked {
            opts.push(("Visit the Shipwright".to_string(), Action::Shipwright(t)));
            opts.push(("Divide the Plunder".to_string(), Action::Divide(t)));
        }
        opts.push(("Check Information".to_string(), Action::InfoScreen(t)));
        opts.push(("Leave Town".to_string(), Action::ToSail));
        let title = format!("{} ({})", town.name(), if town.is_haven() { "Pirates" } else { n.adj() });
        self.dialog(Dialog::new(title, body, Picture::Town(t), opts).with_cancel(Action::ToSail));
    }

    fn governor(&mut self, t: usize) {
        let w = self.world.as_mut().unwrap();
        let n = w.towns[t].nation.unwrap();
        let name = w.towns[t].name();
        let mut body = String::new();
        let mut opts = vec![];
        let st = w.player.standing[n.idx()];
        if !w.towns[t].gov_met {
            w.towns[t].gov_met = true;
        }
        if st < -10 {
            body.push_str(&format!("The Governor of {name} receives you coldly. \"Your deeds against {} are well known here, Captain. Be glad I do not have you arrested.\"", n.name()));
        } else if let Some(r) = w.pending_rank(n) {
            let old = w.player.rank[n.idx()];
            w.player.rank[n.idx()] = r;
            let mut land = 0;
            for k in old + 1..=r {
                land += RANK_LAND[k];
            }
            w.player.land[n.idx()] += land;
            body.push_str(&format!("The Governor greets you warmly. \"In recognition of your valiant service, His Majesty is pleased to grant you the title of {}.\"", RANKS[r]));
            if land > 0 {
                body.push_str(&format!(" You are also granted {land} acres of land."));
            }
            w.add_log(format!("Promoted to {} by {}.", RANKS[r], n.name()));
        } else {
            let r = w.player.rank[n.idx()];
            let greet = if r > 0 { format!("\"Welcome, {} {}.\"", RANKS[r], w.player.name) } else { format!("\"Welcome to {name}, Captain.\"") };
            body.push_str(&format!("The Governor receives you. {greet}"));
        }
        // Political news.
        let wars: Vec<&str> = NATIONS.iter().filter(|&&m| m != n && w.relation(n, m) == Relation::War).map(|m| m.name()).collect();
        let allies: Vec<&str> = NATIONS.iter().filter(|&&m| m != n && w.relation(n, m) == Relation::Alliance).map(|m| m.name()).collect();
        if wars.is_empty() {
            body.push_str(&format!(" \"{} is at peace with all nations.\"", n.name()));
        } else {
            body.push_str(&format!(" \"We are at war with {}.\"", wars.join(" and ")));
        }
        if !allies.is_empty() {
            body.push_str(&format!(" \"We are allied with {}.\"", allies.join(" and ")));
        }
        if st >= -10 && !w.player.marque[n.idx()] {
            if let Some(enemy) = NATIONS.iter().copied().find(|&m| m != n && w.relation(n, m) == Relation::War) {
                body.push_str(&format!(" \"Will you carry a Letter of Marque against {}?\"", enemy.name()));
                opts.push((format!("Accept a Letter of Marque against {}", enemy.name()), Action::AcceptMarque(t, enemy)));
            }
        }
        // Points needed for next rank.
        let r = w.player.rank[n.idx()];
        if st >= -10 && r + 1 < RANKS.len() {
            let need = RANK_POINTS[r + 1] - w.player.points[n.idx()];
            if need > 0 {
                let hint = match need {
                    1..=3 => "a little more",
                    4..=10 => "further",
                    _ => "much greater",
                };
                body.push_str(&format!(" \"Perform {hint} service against our enemies and you shall be rewarded.\""));
            }
        }
        // The governor's daughter.
        if st >= 0 && w.player.wife.is_none() && w.player.rank[n.idx()] >= 1 {
            let rom = w.player.romance[t];
            if rom >= 3 && w.player.rank[n.idx()] >= 3 {
                opts.push(("Ask for the hand of the Governor's daughter".to_string(), Action::Propose(t)));
            } else {
                opts.push(("Pay your respects to the Governor's daughter".to_string(), Action::Court(t)));
            }
        }
        opts.push(("Take your leave".to_string(), Action::TownMenu(t)));
        self.dialog(Dialog::new(format!("The Governor of {name}"), body, Picture::Governor(t), opts).with_cancel(Action::TownMenu(t)));
    }

    fn daughter_beauty(&self, t: usize) -> i32 {
        let w = self.world.as_ref().unwrap();
        let n = w.towns[t].nation.unwrap_or(Nation::Spain);
        (TOWNS[w.towns[t].def].size + w.player.rank[n.idx()] as i32 / 3).clamp(1, 5)
    }

    fn court(&mut self, t: usize) {
        let beauty = self.daughter_beauty(t);
        let w = self.w();
        let n = w.towns[t].nation.unwrap();
        let mut p_ok = 0.45 + 0.07 * w.player.rank[n.idx()] as f32 - 0.05 * beauty as f32;
        if w.player.skill == Skill::WitCharm {
            p_ok += 0.25;
        }
        if w.player.age > 40.0 {
            p_ok -= 0.15;
        }
        let ok = chance(p_ok);
        let looks = ["", "plain", "pleasant", "attractive", "beautiful", "radiant"][beauty as usize];
        let body = if ok {
            w.player.romance[t] += 1;
            match w.player.romance[t] {
                1 => format!("The Governor's {looks} daughter accepts your invitation to dance. She seems quite taken with your tales of adventure."),
                2 => "You stroll with her through the governor's gardens. She blushes when you take her hand.".to_string(),
                _ => "She confides that she thinks of you often while you are at sea. Perhaps, with a higher title, her father would consent to a marriage...".to_string(),
            }
        } else {
            format!("The Governor's {looks} daughter is polite but distant today. Perhaps a more distinguished suitor would impress her more.")
        };
        self.message("The Governor's Daughter", body, Picture::Daughter(beauty), Action::Governor(t));
    }

    fn propose(&mut self, t: usize) {
        let beauty = self.daughter_beauty(t);
        let w = self.w();
        let dowry = beauty * 500 + gen_range(0, 500);
        w.player.wife = Some((t, beauty));
        w.player.wealth += dowry;
        let name = w.towns[t].name();
        w.add_log(format!("Married the Governor's daughter in {name}."));
        self.message(
            "A Wedding!",
            format!("The Governor gives his blessing, and you are married in the great church of {name} amid much celebration. Her dowry of {dowry} pieces of eight is added to your fortune."),
            Picture::Daughter(beauty),
            Action::TownMenu(t),
        );
    }

    fn tavern(&mut self, t: usize) {
        let w = self.world.as_ref().unwrap();
        let town = &w.towns[t];
        let mut opts = vec![];
        let willing = self.recruits_willing(t);
        let space = w.player.crew_capacity() - w.player.crew;
        let mut body = format!("The tavern is crowded with sailors, adventurers and rogues. You are {} here.", w.reputation_word());
        if willing > 0 && space > 0 {
            body.push_str(&format!(" {} men would join your crew.", willing.min(space)));
            opts.push((format!("Recruit {} men", willing.min(space)), Action::Recruit(t)));
        } else if space <= 0 {
            body.push_str(" Your ships cannot hold any more men.");
        } else {
            body.push_str(" No one wishes to join your crew.");
        }
        opts.push(("Listen to the gossip".to_string(), Action::Rumors(t)));
        // A mysterious stranger, determined by town and month so it is stable.
        let seed = (t as i64 * 7919 + w.month_index() * 31) % 100;
        let next_clue = (0..4).find(|&k| !w.player.family_found[k] && !w.player.family_clue[k]);
        if seed < 40 {
            if let Some(k) = next_clue {
                let price = 100 + (seed as i32) * 10;
                body.push_str(&format!(" A mysterious stranger beckons from a dark corner. He claims to know where your {} is held.", RELATIVES[k]));
                opts.push((format!("Pay the stranger {price} gold for his information"), Action::BuyClue(t, price)));
            }
        } else if seed < 70 && w.player.map_pieces < 4 && !w.player.treasure_found {
            let price = 200 + (seed as i32 - 40) * 25;
            body.push_str(" A grizzled old sailor offers to sell you a piece of a treasure map.");
            opts.push((format!("Buy the map piece for {price} gold"), Action::BuyMap(t, price)));
        }
        let _ = town;
        opts.push(("Leave the tavern".to_string(), Action::TownMenu(t)));
        self.dialog(Dialog::new("The Tavern", body, Picture::Tavern, opts).with_cancel(Action::TownMenu(t)));
    }

    fn recruits_willing(&self, t: usize) -> i32 {
        let w = self.world.as_ref().unwrap();
        let town = &w.towns[t];
        let mut base = town.recruits as f32;
        base *= match w.player.fame {
            0..=4 => 0.5,
            5..=14 => 0.7,
            15..=34 => 0.9,
            _ => 1.1,
        };
        if town.is_haven() {
            base *= 1.6;
        }
        if let Some(n) = town.nation {
            if n != Nation::Pirate && w.player.standing[n.idx()] < 0 {
                base *= 0.6;
            }
        }
        // Rich captains attract men.
        base *= 0.8 + (w.player.gold as f32 / 5000.0).min(0.6);
        base as i32
    }

    fn recruit(&mut self, t: usize) {
        let willing = self.recruits_willing(t);
        let w = self.w();
        let space = w.player.crew_capacity() - w.player.crew;
        let n = willing.min(space).max(0);
        w.player.crew += n;
        let rec = &mut w.towns[t].recruits;
        *rec = (*rec - n).max(0);
        // New men are eager.
        w.player.morale = (w.player.morale + 3).min(100);
        let crew = w.player.crew;
        self.message("The Tavern", format!("{n} men sign the articles and join your crew. You now command {crew} men."), Picture::Tavern, Action::Tavern(t));
    }

    fn rumors(&mut self, t: usize) {
        let w = self.world.as_ref().unwrap();
        let mut lines: Vec<String> = vec![];
        let n_news = w.news.len();
        for i in 0..n_news.min(2) {
            lines.push(w.news[n_news - 1 - i].clone());
        }
        // Trade tip.
        let tt = gen_range(0, w.towns.len());
        if w.towns[tt].nation.is_some() {
            let g = gen_range(0, 4);
            let pr = w.towns[tt].price[g];
            if pr > 1.2 {
                lines.push(format!("They say {} fetches a high price in {}.", GOODS[g].to_lowercase(), w.towns[tt].name()));
            } else if pr < 0.7 {
                lines.push(format!("{} is cheap in {} these days.", GOODS[g], w.towns[tt].name()));
            }
        }
        // A sighting of a rich ship or pirate.
        let here = v(w.towns[t].pos);
        if let Some(s) = w.ships.iter().filter(|s| matches!(s.role, Role::Pirate | Role::TreasureFleet) || s.gold > 600).min_by(|a, b| {
            v(a.pos).distance(here).partial_cmp(&v(b.pos).distance(here)).unwrap()
        }) {
            let near = w.nearest_town(s.pos);
            let who = match s.role {
                Role::Pirate => format!("the pirate {} in a {}", s.captain, s.ship.kind.name()),
                Role::TreasureFleet => "a Spanish treasure galleon".to_string(),
                _ => format!("a richly laden {} {}", s.nation.adj(), s.ship.kind.name()),
            };
            lines.push(format!("A sailor swears he saw {} near {}.", who, w.towns[near].name()));
        }
        for (i, town) in w.towns.iter().enumerate() {
            if town.silver_until > w.day && i != t {
                lines.push(format!("The vaults of {} are bursting with silver.", town.name()));
                break;
            }
        }
        if lines.is_empty() {
            lines.push("The sailors grumble about the weather and the price of rum.".into());
        }
        let body = lines.iter().map(|l| format!("\"{l}\"")).collect::<Vec<_>>().join("\n");
        self.message("Tavern Gossip", body, Picture::Tavern, Action::Tavern(t));
    }

    fn buy_clue(&mut self, t: usize, price: i32) {
        let w = self.w();
        if w.player.gold < price {
            self.message("The Stranger", "\"Come back when your purse is heavier, Captain.\"", Picture::Tavern, Action::Tavern(t));
            return;
        }
        let Some(k) = (0..4).find(|&k| !w.player.family_found[k] && !w.player.family_clue[k]) else {
            self.handle(Action::Tavern(t));
            return;
        };
        w.player.gold -= price;
        w.player.family_clue[k] = true;
        let ft = w.family_towns[k];
        let tn = w.towns[ft].name();
        w.add_log(format!("Learned my {} is held in {tn}.", RELATIVES[k]));
        let body = format!("The stranger leans close. \"Your {} lives, held by the Marquis' men in {tn}. Go there, and you may yet find {}.\"", RELATIVES[k], if k == 0 || k == 2 { "her" } else { "him" });
        self.message("The Stranger", body, Picture::Tavern, Action::Tavern(t));
    }

    fn buy_map(&mut self, t: usize, price: i32) {
        let w = self.w();
        if w.player.gold < price {
            self.message("The Old Sailor", "\"No gold, no map.\"", Picture::Tavern, Action::Tavern(t));
            return;
        }
        w.player.gold -= price;
        w.player.map_pieces = (w.player.map_pieces + 1).min(4);
        let pc = w.player.map_pieces;
        w.add_log(format!("Bought treasure map piece {pc} of 4."));
        // Bump the month seed so the offer isn't repeated.
        w.towns[t].recruits = w.towns[t].recruits.max(0);
        self.message("The Old Sailor", format!("You now hold {pc} of 4 pieces of the treasure map. Study it in your Information screens, then search the coast it shows (press D while sailing)."), Picture::Treasure, Action::TownMenu(t));
    }

    fn repair_cost(&self) -> i32 {
        let w = self.world.as_ref().unwrap();
        w.player.ships.iter().map(|s| (100 - s.hull).max(0) * s.kind.spec().value / 250).sum()
    }

    fn shipwright(&mut self, t: usize) {
        let w = self.world.as_ref().unwrap();
        let cost = self.repair_cost();
        let mut body = String::from("Your fleet:\n");
        for s in &w.player.ships {
            body.push_str(&format!("  {} '{}': {} guns, hull {}%\n", s.kind.name(), s.name, s.guns, s.hull));
        }
        let mut opts = vec![];
        if cost > 0 {
            opts.push((format!("Repair all ships ({cost} gold)"), Action::Repair(t)));
        }
        if w.player.ships.len() > 1 {
            opts.push(("Sell a ship".to_string(), Action::SellShipMenu(t)));
        }
        opts.push(("Leave the shipyard".to_string(), Action::TownMenu(t)));
        self.dialog(Dialog::new("The Shipwright", body, Picture::Shipyard, opts).with_cancel(Action::TownMenu(t)));
    }

    fn sell_ship_menu(&mut self, t: usize) {
        let w = self.world.as_ref().unwrap();
        let mut opts = vec![];
        for (i, s) in w.player.ships.iter().enumerate().skip(1).take(6) {
            opts.push((format!("Sell {} '{}' for {}", s.kind.name(), s.name, s.value() / 2), Action::SellShip(t, i)));
        }
        if w.player.ships.len() > 1 {
            opts.push(("Make another ship your flagship".to_string(), Action::ChangeFlagship(t, 1)));
        }
        opts.push(("Cancel".to_string(), Action::Shipwright(t)));
        self.dialog(Dialog::new("Sell a Ship", "Which ship will you sell? (Your flagship cannot be sold.)", Picture::Shipyard, opts).with_cancel(Action::Shipwright(t)));
    }

    fn divide(&mut self, t: usize) {
        let w = self.world.as_ref().unwrap();
        let share = w.player.difficulty.share();
        let gold = w.player.gold;
        let mine = (gold as f32 * share) as i32;
        let per_man = if w.player.crew > 0 { (gold - mine) / w.player.crew } else { 0 };
        let body = format!(
            "The ship's purse holds {gold} pieces of eight. As captain your share is {}% ({mine}). Each of your {} men would receive {per_man}.\n\nOnce the plunder is divided, most of the crew will take their money and leave. You may then retire, or begin a new voyage.",
            (share * 100.0) as i32,
            w.player.crew
        );
        let opts = vec![
            ("Divide the plunder".to_string(), Action::DivideConfirm(t)),
            ("Not yet".to_string(), Action::TownMenu(t)),
        ];
        self.dialog(Dialog::new("Divide the Plunder", body, Picture::Tavern, opts).with_cancel(Action::TownMenu(t)));
    }

    fn divide_confirm(&mut self, t: usize) {
        let w = self.w();
        let share = w.player.difficulty.share();
        let gold = w.player.gold;
        let mine = (gold as f32 * share) as i32;
        let per_man = if w.player.crew > 0 { (gold - mine) / w.player.crew } else { 0 };
        w.player.wealth += mine;
        w.player.gold = 0;
        let stay_frac = if per_man > 150 { 0.25 } else if per_man > 50 { 0.12 } else { 0.05 };
        let stay = ((w.player.crew as f32 * stay_frac) as i32).max(if w.player.crew > 0 { 4.min(w.player.crew) } else { 0 });
        w.player.crew = stay;
        w.player.morale = 85;
        w.player.days_since_divide = 0.0;
        w.player.voyages += 1;
        w.add_log(format!("Divided the plunder: my share {mine}. Personal fortune {}.", w.player.wealth));
        let body = format!(
            "The plunder is shared out and the crew scatters to the taverns and brothels. Your share of {mine} brings your personal fortune to {} pieces of eight. {stay} loyal men remain aboard.\n\nYou are {} years old and your health is {}.",
            w.player.wealth,
            w.player.age as i32,
            w.player.health_word().to_lowercase()
        );
        let opts = vec![
            ("Begin a new adventure".to_string(), Action::TownMenu(t)),
            ("Retire from the sea".to_string(), Action::Retire),
        ];
        self.dialog(Dialog::new("The Plunder is Divided", body, Picture::Tavern, opts));
    }

    fn retire(&mut self) {
        let w = self.w();
        w.retired = true;
        self.screen = Screen::Ending(Ending::new(self.world.as_ref().unwrap()));
    }

    fn check_forced_retirement(&mut self) {
        let Some(w) = self.world.as_ref() else { return };
        if w.player.health <= 0 || w.player.age >= 70.0 {
            let w = self.w();
            w.player.wealth += (w.player.gold as f32 * w.player.difficulty.share()) as i32;
            w.player.gold = 0;
            self.message(
                "Failing Health",
                "Your old wounds ache and your strength is gone. You can no longer command at sea. It is time to retire and live out your days ashore.",
                Picture::Sea,
                Action::Retire,
            );
        }
    }

    // ------------------------------------------------------ town assaults

    fn attack_fort(&mut self, t: usize) {
        let w = self.world.as_ref().unwrap();
        let town = &w.towns[t];
        let pl = &w.player;
        let fs = pl.flagship();
        let setup = NavalSetup {
            player_kind: fs.kind,
            player_name: fs.name.clone(),
            player_guns: fs.guns,
            player_crew: pl.crew.min(fs.kind.spec().max_crew),
            player_hull: fs.hull,
            enemy: NavalEnemy::Fort { town: town.name().to_string(), nation: town.nation.unwrap(), guns: town.fort as i32 * 10 + 4, soldiers: town.soldiers },
            wind_dir: w.wind_dir,
            gunnery: pl.skill == Skill::Gunnery,
            difficulty: pl.difficulty.idx(),
            enemy_flees: false,
        };
        self.pending = Some(Pending::FortAssault(t));
        self.screen = Screen::Naval(NavalBattle::new(setup));
    }

    fn march(&mut self, t: usize) {
        let w = self.world.as_ref().unwrap();
        let town = &w.towns[t];
        if w.player.crew < 10 {
            self.message("Too Few Men", "You have too few men to attack the town.", Picture::Town(t), Action::ArriveTown(t));
            return;
        }
        let setup = LandSetup {
            town: town.name().to_string(),
            nation: town.nation.unwrap(),
            player_men: w.player.crew,
            defenders: town.soldiers,
            fort_level: town.fort,
            difficulty: w.player.difficulty.idx(),
        };
        self.pending = Some(Pending::LandAssault(t));
        self.screen = Screen::Land(LandBattle::new(setup));
    }

    fn sneak(&mut self, t: usize) {
        let w = self.world.as_ref().unwrap();
        let mut p_ok = 0.6 - w.towns[t].fort as f32 * 0.08;
        if w.player.skill == Skill::WitCharm {
            p_ok += 0.15;
        }
        if chance(p_ok) {
            self.sneaked = Some(t);
            self.message("Into the Town", "Under cover of darkness you slip past the guards and into the town unseen. You must avoid the governor's palace.", Picture::Town(t), Action::TownMenu(t));
        } else {
            let w = self.world.as_ref().unwrap();
            let pl = &w.player;
            let setup = DuelSetup {
                kind: DuelKind::Personal,
                player_name: pl.name.clone(),
                opponent_name: "The Captain of the Guard".into(),
                opponent_title: format!("of {}", w.towns[t].name()),
                player_crew: 0,
                enemy_crew: 0,
                player_skill: pl.fencing_skill(),
                enemy_skill: self.enemy_skill(0.1),
            };
            self.pending = Some(Pending::SneakFight(t));
            self.dialog(Dialog::msg("Discovered!", "\"Halt! Who goes there?\" A patrol has spotted you! The captain of the guard draws his sword. You must fight your way out!", Picture::Town(t), Action::Nothing));
            // Replace Continue's action by starting the duel.
            self.screen = Screen::Duel(Duel::new(setup));
        }
    }

    fn enemy_skill(&self, bonus: f32) -> f32 {
        let w = self.world.as_ref().unwrap();
        (0.65 + 0.15 * w.player.difficulty.idx() as f32 + bonus + gen_range(-0.1, 0.1)).clamp(0.4, 1.5)
    }

    fn sack_town(&mut self, t: usize, men_left: i32) {
        let w = self.w();
        w.player.crew = men_left.max(0);
        let town = &mut w.towns[t];
        let victim = town.nation.unwrap();
        let silver = town.silver_until > w.day;
        let frac = gen_range(0.45, 0.7);
        let loot = (town.wealth as f32 * frac) as i32;
        town.wealth -= loot;
        town.soldiers = (town.soldiers as f32 * 0.3) as i32;
        town.population = (town.population as f32 * 0.9) as i32;
        town.silver_until = -1.0;
        town.sacked_day = w.day;
        let name = town.name();
        let size = TOWNS[town.def].size;
        w.player.gold += loot;
        w.player.towns_sacked += 1;
        w.player.morale = (w.player.morale + 20).min(100);
        let reactions = w.act_against(victim, 4 + size * 2);
        w.add_log(format!("Sacked {name} and took {loot} gold."));
        w.add_news(format!("{name} was sacked by {}!", w.player.title()));
        let mut body = format!("{name} is yours! Your men loot the town, seizing {loot} pieces of eight{}.", if silver { ", including the silver from the treasure train" } else { "" });
        // Rescue a family member held here.
        for k in 0..4 {
            if w.family_towns[k] == t && !w.player.family_found[k] {
                w.player.family_found[k] = true;
                w.player.family_clue[k] = true;
                body.push_str(&format!(" In the town's dungeon you discover your long-lost {}!", RELATIVES[k]));
                w.add_log(format!("Rescued my {} from {name}.", RELATIVES[k]));
            }
        }
        if !reactions.is_empty() {
            body.push(' ');
            body.push_str(&reactions.join(" "));
        }
        let mut opts = vec![];
        for n in NATIONS {
            if n != victim && w.player.rank[n.idx()] >= 1 && w.relation(n, victim) == Relation::War {
                opts.push((format!("Install a new {} governor", n.adj()), Action::InstallGovernor(t, n)));
            }
        }
        opts.push(("Return to your ships".to_string(), Action::ToSail));
        self.dialog(Dialog::new(format!("{name} Captured!"), body, Picture::Town(t), opts));
    }

    // --------------------------------------------------------- encounters

    pub fn encounter(&mut self, id: u32, attacking: bool) {
        let w = self.world.as_ref().unwrap();
        let Some(s) = w.ships.iter().find(|s| s.id == id) else {
            self.handle(Action::ToSail);
            return;
        };
        let flag = if s.nation == Nation::Pirate { "the black flag of a pirate".to_string() } else { format!("{} colors", s.nation.adj()) };
        let kind = s.ship.kind.name();
        let mut opts = vec![];
        let body;
        if attacking {
            body = format!("A {kind} flying {flag} bears down on you, guns run out! She means to attack!");
            opts.push(("Clear for action!".to_string(), Action::AttackShip(id)));
            opts.push(("Try to escape".to_string(), Action::EscapeShip(id)));
        } else {
            let what = match s.role {
                Role::TreasureFleet => " She rides low in the water -- a treasure galleon of the silver fleet!",
                Role::Merchant | Role::Smuggler => " She appears to be a merchant vessel.",
                Role::Warship | Role::Hunter => " Her gunports mark her as a ship of war.",
                Role::Pirate => " A pirate! Beware.",
            };
            body = format!("Sail ho! A {kind} flying {flag} is sighted.{what}");
            opts.push(("Attack her".to_string(), Action::AttackShip(id)));
            if s.nation != Nation::Pirate && !w.hostile(s.nation) {
                opts.push(("Hail her for news".to_string(), Action::HailShip(id)));
            }
            opts.push(("Ignore her and sail on".to_string(), Action::IgnoreShip(id)));
        }
        let (k, n) = (s.ship.kind, s.nation);
        let cancel = if attacking { Action::AttackShip(id) } else { Action::IgnoreShip(id) };
        self.dialog(Dialog::new("Sail Ho!", body, Picture::Ship(k, n), opts).with_cancel(cancel));
    }

    fn hail_ship(&mut self, id: u32) {
        let w = self.w();
        let Some(i) = w.ships.iter().position(|s| s.id == id) else {
            self.handle(Action::ToSail);
            return;
        };
        w.ships[i].ignore = 5.0;
        let s = w.ships[i].clone();
        let mut news = w.news.last().cloned().unwrap_or_else(|| "All is quiet on the Main.".into());
        if let Some(d) = s.dest {
            news = format!("{} We are bound for {}.", news, w.towns[d].name());
        }
        self.message(
            "Hailing",
            format!("You exchange greetings with {} of the {} {} '{}'. \"{}\"", s.captain, s.nation.adj(), s.ship.kind.name(), s.ship.name, news),
            Picture::Ship(s.ship.kind, s.nation),
            Action::ToSail,
        );
    }

    fn escape_ship(&mut self, id: u32) {
        let w = self.world.as_ref().unwrap();
        let Some(s) = w.ships.iter().find(|s| s.id == id) else {
            self.handle(Action::ToSail);
            return;
        };
        let mine = w.player.fleet_speed(1.0, 1.0);
        let theirs = s.ship.kind.spec().speed;
        let p_ok = (0.5 * mine / theirs).clamp(0.15, 0.85);
        if chance(p_ok) {
            let w = self.w();
            if let Some(s) = w.ships.iter_mut().find(|s| s.id == id) {
                s.ignore = 6.0;
                s.chase = 0.0;
            }
            self.message("Escape!", "You crowd on all sail and slip away from your pursuer.", Picture::Sea, Action::ToSail);
        } else {
            self.message("No Escape", "She is too fast for you. You must fight!", Picture::Sea, Action::AttackShip(id));
        }
    }

    fn attack_ship(&mut self, id: u32) {
        let w = self.world.as_ref().unwrap();
        let Some(s) = w.ships.iter().find(|s| s.id == id).cloned() else {
            self.handle(Action::ToSail);
            return;
        };
        let pl = &w.player;
        let fs = pl.flagship();
        let setup = NavalSetup {
            player_kind: fs.kind,
            player_name: fs.name.clone(),
            player_guns: fs.guns,
            player_crew: pl.crew.min(fs.kind.spec().max_crew + pl.crew / 4),
            player_hull: fs.hull,
            enemy: NavalEnemy::Ship { kind: s.ship.kind, name: s.ship.name.clone(), nation: s.nation, guns: s.ship.guns, crew: s.crew, hull: s.ship.hull },
            wind_dir: w.wind_dir,
            gunnery: pl.skill == Skill::Gunnery,
            difficulty: pl.difficulty.idx(),
            enemy_flees: matches!(s.role, Role::Merchant | Role::Smuggler | Role::TreasureFleet) && s.crew < pl.crew * 2,
        };
        self.pending = Some(Pending::ShipFight(Box::new(s)));
        self.screen = Screen::Naval(NavalBattle::new(setup));
    }

    /// Apply naval losses to the player's fleet. Returns crew lost.
    fn apply_naval_to_player(&mut self, o: &NavalOutcome, crew_in_battle: i32) {
        let w = self.w();
        let lost = (crew_in_battle - o.player_crew).max(0);
        w.player.crew = (w.player.crew - lost).max(0);
        let fs = &mut w.player.ships[0];
        fs.guns = o.player_guns.max(0);
        fs.hull = o.player_hull;
    }

    fn lose_flagship(&mut self) -> String {
        let w = self.w();
        let s = w.player.ships.remove(0);
        let drowned = (w.player.crew as f32 * 0.3) as i32;
        w.player.crew -= drowned;
        if w.player.ships.is_empty() {
            return String::new();
        }
        w.trim_to_capacity();
        format!("Your flagship, the {} '{}', goes down! {drowned} men are lost. The survivors are picked up by the {}, your new flagship.", s.kind.name(), s.name, w.player.ships[0].name)
    }

    fn on_naval(&mut self, o: NavalOutcome) {
        let Some(pending) = self.pending.take() else {
            self.handle(Action::ToSail);
            return;
        };
        let crew_in_battle = {
            let w = self.world.as_ref().unwrap();
            let fs = w.player.flagship();
            match &pending {
                Pending::ShipFight(_) => w.player.crew.min(fs.kind.spec().max_crew + w.player.crew / 4),
                _ => w.player.crew.min(fs.kind.spec().max_crew),
            }
        };
        self.apply_naval_to_player(&o, crew_in_battle);
        match pending {
            Pending::ShipFight(ai) => {
                let id = ai.id;
                let mut ai = *ai;
                ai.crew = o.enemy_crew;
                ai.ship.guns = o.enemy_guns;
                ai.ship.hull = o.enemy_hull;
                if let Some(s) = self.w().ships.iter_mut().find(|s| s.id == id) {
                    s.crew = ai.crew;
                    s.ship.guns = ai.ship.guns;
                    s.ship.hull = ai.ship.hull;
                    s.ignore = 5.0;
                    s.chase = 0.0;
                }
                match o.kind {
                    NavalOutcomeKind::Boarded => {
                        let w = self.world.as_ref().unwrap();
                        let pl = &w.player;
                        let bonus = match ai.role {
                            Role::Pirate | Role::Hunter => 0.2,
                            Role::Warship | Role::TreasureFleet => 0.1,
                            _ => 0.0,
                        };
                        let setup = DuelSetup {
                            kind: DuelKind::Boarding,
                            player_name: pl.name.clone(),
                            opponent_name: ai.captain.clone(),
                            opponent_title: format!("Captain of the {} {}", ai.nation.adj(), ai.ship.kind.name()),
                            player_crew: pl.crew,
                            enemy_crew: ai.crew,
                            player_skill: pl.fencing_skill(),
                            enemy_skill: self.enemy_skill(bonus),
                        };
                        self.pending = Some(Pending::Boarding(Box::new(ai), o));
                        self.screen = Screen::Duel(Duel::new(setup));
                    }
                    NavalOutcomeKind::EnemySunk => {
                        let w = self.w();
                        w.ships.retain(|s| s.id != id);
                        w.player.ships_sunk += 1;
                        w.player.morale = (w.player.morale + 5).min(100);
                        let value = role_value(ai.role);
                        let r = w.act_against(ai.nation, value);
                        w.add_log(format!("Sank the {} {} '{}'.", ai.nation.adj(), ai.ship.kind.name(), ai.ship.name));
                        self.message("Victory!", format!("The {} {} '{}' founders and sinks, taking her cargo with her. {}", ai.nation.adj(), ai.ship.kind.name(), ai.ship.name, r.join(" ")), Picture::Sea, Action::ToSail);
                    }
                    NavalOutcomeKind::PlayerSunk => self.player_defeated_at_sea(Some(ai.nation)),
                    NavalOutcomeKind::PlayerEscaped => self.message("Escaped", "You break off the action and escape.", Picture::Sea, Action::ToSail),
                    NavalOutcomeKind::EnemyEscaped => self.message("She Escapes", format!("The {} slips away over the horizon.", ai.ship.kind.name()), Picture::Sea, Action::ToSail),
                }
            }
            Pending::FortAssault(t) => match o.kind {
                NavalOutcomeKind::Boarded => {
                    let w = self.w();
                    let town = &mut w.towns[t];
                    town.soldiers = o.enemy_crew.max(5);
                    let setup = DuelSetup {
                        kind: DuelKind::TownAssault,
                        player_name: w.player.name.clone(),
                        opponent_name: "The Commandant".into(),
                        opponent_title: format!("Commandant of {}", w.towns[t].name()),
                        player_crew: w.player.crew,
                        enemy_crew: w.towns[t].soldiers,
                        player_skill: w.player.fencing_skill(),
                        enemy_skill: 0.0,
                    };
                    let mut setup = setup;
                    setup.enemy_skill = self.enemy_skill(0.15);
                    self.pending = Some(Pending::TownDuel(t));
                    self.screen = Screen::Duel(Duel::new(setup));
                }
                NavalOutcomeKind::PlayerSunk => {
                    let n = self.world.as_ref().unwrap().towns[t].nation;
                    self.player_defeated_at_sea(n);
                }
                _ => self.message("Withdrawal", "You withdraw out of range of the fort's guns.", Picture::Sea, Action::ToSail),
            },
            _ => self.handle(Action::ToSail),
        }
    }

    fn player_defeated_at_sea(&mut self, captor: Option<Nation>) {
        let msg = self.lose_flagship();
        if msg.is_empty() {
            self.captured(captor, "Your ship is shot to pieces and sinks beneath you. You are dragged from the water by your enemies.");
        } else {
            self.message("Ship Lost!", msg, Picture::Sea, Action::ToSail);
        }
    }

    /// The player is captured: lose the purse, spend time in prison, and
    /// start again from the nearest friendly port.
    fn captured(&mut self, captor: Option<Nation>, how: &str) {
        let geo_free;
        let w = self.w();
        let months = gen_range(2, 8);
        w.day += months as f64 * 30.0;
        w.player.age += months as f32 / 12.0;
        w.player.health = (w.player.health - 10).max(1);
        w.player.gold = 0;
        w.player.cargo = [0; 4];
        let lost_ships = w.player.ships.len();
        w.player.ships.clear();
        let mut s = Ship::new(ShipKind::Sloop);
        s.guns = 4;
        s.name = "Phoenix".into();
        w.player.ships.push(s);
        w.player.crew = 12;
        w.player.food = 15;
        w.player.morale = 60;
        w.player.days_since_divide = 0.0;
        w.last_month = w.month_index();
        let town = w.nearest_friendly_town(w.player.pos).unwrap_or(0);
        w.player.pos = w.towns[town].anchor;
        geo_free = w.towns[town].name();
        let who = captor.map(|n| n.adj()).unwrap_or("enemy");
        w.add_log(format!("Captured and imprisoned for {months} months."));
        let _ = lost_ships;
        let body = format!("{how}\n\nYou spend {months} long months in a {who} prison before you escape. With a handful of loyal men you steal a small sloop and make your way to {geo_free}.");
        self.message("Captured!", body, Picture::Prison, Action::ArriveTown(town));
    }

    fn on_duel(&mut self, o: DuelOutcome) {
        let Some(pending) = self.pending.take() else {
            self.handle(Action::ToSail);
            return;
        };
        if o.wounded {
            let w = self.w();
            let dmg = if w.player.skill == Skill::Medicine { 4 } else { 8 };
            w.player.health = (w.player.health - dmg).max(1);
        }
        match pending {
            Pending::Boarding(ai, _naval) => {
                let w = self.w();
                w.player.crew = o.player_crew.max(if o.won { 1 } else { 0 });
                if o.won {
                    w.player.duels_won += 1;
                    self.capture_prize(*ai, o.enemy_crew);
                } else {
                    self.captured(Some(ai.nation), "Your crew is overwhelmed and you are forced to surrender your sword.");
                }
            }
            Pending::TownDuel(t) => {
                if o.won {
                    self.w().player.duels_won += 1;
                    self.sack_town(t, o.player_crew);
                } else {
                    let n = self.world.as_ref().unwrap().towns[t].nation;
                    self.w().player.crew = o.player_crew;
                    self.captured(n, "The commandant's men drive you back and you are taken prisoner.");
                }
            }
            Pending::SneakFight(t) => {
                if o.won {
                    self.message("Escape!", "You cut down the guard captain and flee through the dark alleys back to your ship.", Picture::Town(t), Action::ToSail);
                } else {
                    let n = self.world.as_ref().unwrap().towns[t].nation;
                    self.captured(n, "You are overpowered by the guards and thrown into the dungeon.");
                }
            }
            _ => self.handle(Action::ToSail),
        }
    }

    fn capture_prize(&mut self, ai: AiShip, enemy_crew_left: i32) {
        let geo_pirate = ai.nation == Nation::Pirate;
        let w = self.w();
        w.ships.retain(|s| s.id != ai.id);
        w.player.ships_captured += 1;
        w.player.gold += ai.gold;
        w.player.morale = (w.player.morale + 10).min(100);
        let mut body = format!("You have captured the {} {} '{}'! In her hold you find {} pieces of eight", ai.nation.adj(), ai.ship.kind.name(), ai.ship.name, ai.gold);
        // Take cargo up to capacity.
        let mut taken = vec![];
        for g in 0..4 {
            let space = w.player.cargo_capacity() - w.player.cargo_used();
            let n = ai.cargo[g].min(space.max(0));
            if n > 0 {
                w.player.cargo[g] += n;
                taken.push(format!("{n} tons of {}", GOODS[g].to_lowercase()));
            }
        }
        let food = (ai.crew / 4 + gen_range(0, 15)).min((w.player.cargo_capacity() - w.player.cargo_used()).max(0));
        w.player.food += food;
        if !taken.is_empty() {
            body.push_str(&format!(" and {}", taken.join(", ")));
        }
        body.push('.');
        if food > 0 {
            body.push_str(&format!(" You also take {food} tons of provisions."));
        }
        // Some of the defeated crew join.
        let join_frac = if geo_pirate { 0.6 } else { 0.25 };
        let joiners = ((enemy_crew_left.max(0) as f32) * join_frac) as i32;
        let space = (w.player.crew_capacity() + ai.ship.kind.spec().max_crew - w.player.crew).max(0);
        let joiners = joiners.min(space);
        if joiners > 0 {
            w.player.crew += joiners;
            body.push_str(&format!(" {joiners} of her crew agree to join you."));
        }
        if ai.has_clue {
            if let Some(k) = (0..4).find(|&k| !w.player.family_found[k] && !w.player.family_clue[k]) {
                w.player.family_clue[k] = true;
                body.push_str(&format!(" Among the captain's papers you find a letter revealing that your {} is held in {}!", RELATIVES[k], w.towns[w.family_towns[k]].name()));
            }
        }
        if ai.has_map && w.player.map_pieces < 4 && !w.player.treasure_found {
            w.player.map_pieces += 1;
            body.push_str(" In the pirate's sea chest you discover a piece of a treasure map!");
        }
        let value = role_value(ai.role);
        let r = w.act_against(ai.nation, value);
        if !r.is_empty() {
            body.push(' ');
            body.push_str(&r.join(" "));
        }
        w.add_log(format!("Captured the {} {} '{}' with {} gold.", ai.nation.adj(), ai.ship.kind.name(), ai.ship.name, ai.gold));
        let can_keep = w.player.ships.len() < 8;
        let mut opts = vec![];
        if can_keep {
            opts.push((format!("Keep the {} as a prize", ai.ship.kind.name()), Action::KeepPrize));
        }
        opts.push(("Sink her".to_string(), Action::SinkPrize));
        opts.push(("Let her go".to_string(), Action::FreePrize));
        let (k, n) = (ai.ship.kind, ai.nation);
        self.prize = Some(ai);
        self.dialog(Dialog::new("A Prize!", body, Picture::Ship(k, n), opts));
    }

    fn keep_prize(&mut self) {
        let Some(pz) = self.prize.take() else {
            self.handle(Action::ToSail);
            return;
        };
        let w = self.w();
        let mut s = pz.ship.clone();
        s.hull = s.hull.max(25);
        let (kind, name) = (s.kind, s.name.clone());
        w.player.ships.push(s);
        self.message("The Prize", format!("A prize crew takes command of the {} '{}'. She joins your fleet.", kind.name(), name), Picture::Ship(kind, pz.nation), Action::ToSail);
    }

    fn on_land(&mut self, o: LandOutcome) {
        let Some(Pending::LandAssault(t)) = self.pending.take() else {
            self.handle(Action::ToSail);
            return;
        };
        match o.kind {
            LandOutcomeKind::Victory => self.sack_town(t, o.men_left),
            LandOutcomeKind::Retreat => {
                let w = self.w();
                w.player.crew = o.men_left;
                w.towns[t].soldiers = o.defenders_left.max(5);
                w.player.morale = (w.player.morale - 10).max(0);
                self.message("Retreat", "Your men fall back to the boats and row back to the ships.", Picture::Town(t), Action::ToSail);
            }
            LandOutcomeKind::Defeat => {
                let w = self.w();
                w.towns[t].soldiers = o.defenders_left.max(5);
                if o.men_left >= 5 {
                    w.player.crew = o.men_left;
                    w.player.morale = (w.player.morale - 20).max(0);
                    self.message("Defeat", "Your attack is broken. The survivors flee back to the ships in disarray.", Picture::Town(t), Action::ToSail);
                } else {
                    let n = w.towns[t].nation;
                    self.captured(n, "Your landing party is destroyed and you are taken prisoner.");
                }
            }
        }
    }
}

pub fn role_value(r: Role) -> i32 {
    match r {
        Role::Merchant | Role::Smuggler => 1,
        Role::Warship => 3,
        Role::Hunter => 3,
        Role::Pirate => 2,
        Role::TreasureFleet => 4,
    }
}
