//! Headless tests of the game flow (no window needed).

use super::*;
use crate::fencing::DuelOutcome;
use crate::landbattle::{LandOutcome, LandOutcomeKind};
use crate::naval::{NavalOutcome, NavalOutcomeKind};

fn app_with(era: usize, nation: Nation) -> App {
    macroquad::rand::srand(42);
    let mut a = App { geo: Geo::new(), world: None, screen: Screen::Title(Title::new()), pending: None, prize: None, sneaked: None, quit: false };
    a.handle(Action::NewGame(era, ERAS[era].year, nation, "Tester".into(), Difficulty::Journeyman, Skill::Fencing, None));
    a
}

fn app() -> App {
    app_with(4, Nation::England)
}

fn dialog_labels(a: &App) -> Vec<String> {
    match &a.screen {
        Screen::Dialog(d) => d.menu.items.clone(),
        _ => panic!("expected dialog"),
    }
}

fn dialog_actions(a: &App) -> Vec<Action> {
    match &a.screen {
        Screen::Dialog(d) => d.actions.clone(),
        _ => panic!("expected dialog"),
    }
}

fn spawn_enemy(a: &mut App, role: Role, nation: Nation) -> u32 {
    let w = a.world.as_mut().unwrap();
    let start = a.geo.random_sea();
    let id = w.add_ai_ship(&a.geo, ShipKind::Merchantman, nation, role, start, None).expect("path");
    let s = w.ships.iter_mut().find(|s| s.id == id).unwrap();
    s.gold = 777;
    s.cargo = [10, 0, 5, 0];
    id
}

fn naval(kind: NavalOutcomeKind, pc: i32, ec: i32) -> NavalOutcome {
    NavalOutcome { kind, player_guns: 6, player_crew: pc, player_hull: 80, enemy_guns: 8, enemy_crew: ec, enemy_hull: 50 }
}

#[test]
fn new_game_starts_in_town() {
    let a = app();
    let w = a.world.as_ref().unwrap();
    assert_eq!(w.player.ships.len(), 1);
    assert!(w.player.crew > 0);
    assert!(w.ships.len() >= 15, "AI ships spawned: {}", w.ships.len());
    assert!(matches!(a.screen, Screen::Dialog(_)));
}

#[test]
fn every_era_and_nation_can_start() {
    for era in 0..6 {
        for &n in ERAS[era].nations {
            let a = app_with(era, n);
            let w = a.world.as_ref().unwrap();
            let pos = v(w.player.pos);
            assert!(a.geo.is_water(pos.x, pos.y), "era {era} {n:?} starts on land");
        }
    }
}

#[test]
fn expeditions_start() {
    for e in 0..EXPEDITIONS.len() {
        macroquad::rand::srand(7);
        let mut a = App { geo: Geo::new(), world: None, screen: Screen::Title(Title::new()), pending: None, prize: None, sneaked: None, quit: false };
        let ex = &EXPEDITIONS[e];
        a.handle(Action::NewGame(ex.era, ex.year, ex.nation, "X".into(), Difficulty::Adventurer, Skill::Gunnery, Some(e)));
        let w = a.world.as_ref().unwrap();
        assert_eq!(w.player.ships.len(), ex.ships.len());
        assert_eq!(w.player.crew, ex.crew);
    }
}

#[test]
fn town_services() {
    let mut a = app();
    let t = a.world.as_ref().unwrap().nearest_town(a.world.as_ref().unwrap().player.pos);
    a.handle(Action::TownMenu(t));
    let labels = dialog_labels(&a);
    assert!(labels.iter().any(|l| l.contains("Governor")));
    a.handle(Action::Governor(t));
    a.handle(Action::Tavern(t));
    let crew_before = a.world.as_ref().unwrap().player.crew;
    a.handle(Action::Recruit(t));
    assert!(a.world.as_ref().unwrap().player.crew >= crew_before);
    a.handle(Action::Rumors(t));
    a.handle(Action::Merchant(t));
    assert!(matches!(a.screen, Screen::Market(_)));
    a.handle(Action::Shipwright(t));
    a.world.as_mut().unwrap().player.ships[0].hull = 50;
    a.world.as_mut().unwrap().player.gold = 5000;
    a.handle(Action::Repair(t));
    assert_eq!(a.world.as_ref().unwrap().player.ships[0].hull, 100);
    a.handle(Action::InfoScreen(t));
    assert!(matches!(a.screen, Screen::Info(_)));
}

#[test]
fn divide_and_retire() {
    let mut a = app();
    let t = 5;
    a.world.as_mut().unwrap().player.gold = 10000;
    a.handle(Action::Divide(t));
    a.handle(Action::DivideConfirm(t));
    let w = a.world.as_ref().unwrap();
    assert_eq!(w.player.gold, 0);
    assert_eq!(w.player.wealth, 1500);
    a.handle(Action::Retire);
    assert!(matches!(a.screen, Screen::Ending(_)));
}

#[test]
fn board_and_capture_ship() {
    let mut a = app();
    let id = spawn_enemy(&mut a, Role::Merchant, Nation::Spain);
    a.handle(Action::Encounter(id, false));
    assert!(dialog_labels(&a).iter().any(|l| l.contains("Attack")));
    a.handle(Action::AttackShip(id));
    assert!(matches!(a.screen, Screen::Naval(_)));
    let crew = a.world.as_ref().unwrap().player.crew;
    a.on_naval(naval(NavalOutcomeKind::Boarded, crew - 2, 30));
    assert!(matches!(a.screen, Screen::Duel(_)));
    a.on_duel(DuelOutcome { won: true, player_crew: crew - 5, enemy_crew: 10, wounded: false });
    let w = a.world.as_ref().unwrap();
    assert!(w.player.gold >= 777);
    assert!(w.ships.iter().all(|s| s.id != id));
    assert_eq!(w.player.ships_captured, 1);
    assert!(dialog_labels(&a).iter().any(|l| l.contains("Keep")));
    a.handle(Action::KeepPrize);
    assert_eq!(a.world.as_ref().unwrap().player.ships.len(), 2);
}

#[test]
fn lose_boarding_means_capture() {
    let mut a = app();
    let id = spawn_enemy(&mut a, Role::Warship, Nation::Spain);
    a.handle(Action::AttackShip(id));
    a.on_naval(naval(NavalOutcomeKind::Boarded, 20, 100));
    a.on_duel(DuelOutcome { won: false, player_crew: 3, enemy_crew: 80, wounded: true });
    let w = a.world.as_ref().unwrap();
    assert_eq!(w.player.gold, 0);
    assert_eq!(w.player.ships.len(), 1);
    assert!(w.player.crew > 0);
    let pos = v(w.player.pos);
    assert!(a.geo.is_water(pos.x, pos.y));
}

#[test]
fn sink_and_get_sunk() {
    let mut a = app();
    let id = spawn_enemy(&mut a, Role::Pirate, Nation::Pirate);
    a.handle(Action::AttackShip(id));
    a.on_naval(naval(NavalOutcomeKind::EnemySunk, 30, 0));
    assert_eq!(a.world.as_ref().unwrap().player.ships_sunk, 1);
    let id2 = spawn_enemy(&mut a, Role::Warship, Nation::Spain);
    a.world.as_mut().unwrap().player.ships.push(Ship::new(ShipKind::Pinnace));
    a.handle(Action::AttackShip(id2));
    a.on_naval(NavalOutcome { player_hull: 0, ..naval(NavalOutcomeKind::PlayerSunk, 10, 50) });
    let w = a.world.as_ref().unwrap();
    assert_eq!(w.player.ships.len(), 1);
    assert_eq!(w.player.ships[0].kind, ShipKind::Pinnace);
}

#[test]
fn assault_town_from_sea_and_install_governor() {
    let mut a = app();
    let t = a.world.as_ref().unwrap().town_index("Santiago").unwrap();
    {
        let w = a.world.as_mut().unwrap();
        w.player.standing[0] = -80;
        w.player.rank[1] = 2;
        w.player.crew = 200;
        w.relations[0][1] = Relation::War;
        w.relations[1][0] = Relation::War;
    }
    a.handle(Action::ArriveTown(t));
    assert!(dialog_labels(&a).iter().any(|l| l.contains("Attack")));
    a.handle(Action::AttackFort(t));
    assert!(matches!(a.screen, Screen::Naval(_)));
    a.on_naval(naval(NavalOutcomeKind::Boarded, 150, 60));
    assert!(matches!(a.screen, Screen::Duel(_)));
    let wealth = a.world.as_ref().unwrap().towns[t].wealth;
    a.on_duel(DuelOutcome { won: true, player_crew: 140, enemy_crew: 0, wounded: false });
    let w = a.world.as_ref().unwrap();
    assert!(w.player.gold > 200, "looted {}", w.player.gold);
    assert!(w.towns[t].wealth < wealth);
    let acts = dialog_actions(&a);
    let inst = acts.iter().find(|x| matches!(x, Action::InstallGovernor(_, Nation::England))).cloned().expect("install option");
    a.handle(inst);
    assert_eq!(a.world.as_ref().unwrap().towns[t].nation, Some(Nation::England));
}

#[test]
fn land_assault_outcomes() {
    let mut a = app();
    let t = a.world.as_ref().unwrap().town_index("Maracaibo").unwrap();
    a.world.as_mut().unwrap().player.crew = 150;
    a.handle(Action::March(t));
    assert!(matches!(a.screen, Screen::Land(_)));
    a.on_land(LandOutcome { kind: LandOutcomeKind::Victory, men_left: 120, defenders_left: 0 });
    assert_eq!(a.world.as_ref().unwrap().player.towns_sacked, 1);
    a.handle(Action::March(t));
    a.on_land(LandOutcome { kind: LandOutcomeKind::Defeat, men_left: 2, defenders_left: 50 });
    assert_eq!(a.world.as_ref().unwrap().player.crew, 12, "captured and released with a skeleton crew");
}

#[test]
fn sneak_in_eventually_fights_or_enters() {
    let mut a = app();
    let t = a.world.as_ref().unwrap().town_index("Havana").unwrap();
    a.world.as_mut().unwrap().player.standing[0] = -80;
    for _ in 0..10 {
        a.handle(Action::Sneak(t));
        match &a.screen {
            Screen::Duel(_) => {
                a.on_duel(DuelOutcome { won: true, player_crew: 0, enemy_crew: 0, wounded: false });
            }
            Screen::Dialog(_) => {
                assert_eq!(a.sneaked, Some(t));
                a.handle(Action::TownMenu(t));
                assert!(!dialog_labels(&a).iter().any(|l| l.contains("Governor")));
            }
            _ => panic!("unexpected screen"),
        }
    }
}

#[test]
fn promotion_marque_and_marriage() {
    let mut a = app();
    let t = a.world.as_ref().unwrap().town_index("Port Royal").unwrap();
    {
        let w = a.world.as_mut().unwrap();
        w.player.points[1] = 45;
        w.player.standing[1] = 50;
    }
    a.handle(Action::Governor(t));
    let w = a.world.as_ref().unwrap();
    assert_eq!(w.player.rank[1], 5, "promoted to Admiral");
    assert!(w.player.land[1] > 0);
    for _ in 0..30 {
        a.handle(Action::Court(t));
    }
    a.handle(Action::Governor(t));
    assert!(dialog_labels(&a).iter().any(|l| l.contains("hand")));
    a.handle(Action::Propose(t));
    assert!(a.world.as_ref().unwrap().player.wife.is_some());
}

#[test]
fn family_clue_then_reunion() {
    let mut a = app();
    let ft = a.world.as_ref().unwrap().family_towns[0];
    {
        let w = a.world.as_mut().unwrap();
        w.player.gold = 5000;
        w.player.family_clue[0] = true;
        let n = w.towns[ft].nation.unwrap();
        if n != Nation::Pirate {
            w.player.standing[n.idx()] = 50;
        }
    }
    a.handle(Action::ArriveTown(ft));
    assert!(a.world.as_ref().unwrap().player.family_found[0]);
}

#[test]
fn simulate_years_and_save_load() {
    let mut a = app();
    let geo = &a.geo;
    let w = a.world.as_mut().unwrap();
    for _ in 0..(360 * 5) {
        w.advance(geo, 1.0);
    }
    assert!(w.year() >= 1665);
    assert!(!w.ships.is_empty());
    let path = std::env::temp_dir().join("pirates_test.sav");
    let path = path.to_str().unwrap();
    w.save(path).unwrap();
    let w2 = World::load(path).unwrap();
    assert_eq!(w2.player.name, w.player.name);
    assert_eq!(w2.towns.len(), w.towns.len());
    assert_eq!(w2.ships.len(), w.ships.len());
}

#[test]
fn paths_exist_between_major_ports() {
    let a = app();
    let w = a.world.as_ref().unwrap();
    for name in ["Havana", "Cartagena", "Barbados", "Vera Cruz", "Tortuga", "St. Augustine", "Curacao", "Porto Bello"] {
        let t = w.town_index(name).unwrap();
        let from = v(w.towns[w.town_index("Port Royal").unwrap()].anchor);
        let to = v(w.towns[t].anchor);
        assert!(a.geo.find_path(from, to).is_some(), "no sea route to {name}");
    }
}

#[test]
fn treasure_dig() {
    let mut a = app();
    let mut sail = crate::sail::Sail::new();
    let w = a.world.as_mut().unwrap();
    // Put the player on water next to the treasure.
    let tp = v(w.treasure_pos);
    let mut spot = None;
    for dy in -4..=4 {
        for dx in -4..=4 {
            let q = tp + vec2(dx as f32 * 0.1, dy as f32 * 0.1);
            if a.geo.is_water(q.x, q.y) && spot.is_none() {
                spot = Some(q);
            }
        }
    }
    w.player.pos = p(spot.expect("water near treasure"));
    let gold = w.player.gold;
    let mut inp = Input::default();
    inp.pressed.push(KeyCode::D);
    sail.update(&inp, 0.016, w, &a.geo);
    assert!(w.player.treasure_found);
    assert!(w.player.gold > gold);
}
