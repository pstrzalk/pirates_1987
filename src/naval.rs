//! Naval battle minigame: top-down ship-to-ship (or ship-vs-fort) combat
//! with wind, broadsides and boarding.

use crate::data::{Nation, ShipKind};
use crate::input::Input;

#[derive(Clone, Debug)]
pub enum NavalEnemy {
    Ship { kind: ShipKind, name: String, nation: Nation, guns: i32, crew: i32, hull: i32 },
    /// Attacking a town from the sea: the player must sail past the fort's
    /// guns and reach the town's quay. Reaching it yields `Boarded` (landing).
    Fort { town: String, nation: Nation, guns: i32, soldiers: i32 },
}

#[derive(Clone, Debug)]
pub struct NavalSetup {
    pub player_kind: ShipKind,
    pub player_name: String,
    pub player_guns: i32,
    pub player_crew: i32,
    /// Hull integrity 0..=100
    pub player_hull: i32,
    pub enemy: NavalEnemy,
    /// Direction the wind blows TOWARD, radians, screen space (0 = +x east, PI/2 = +y down/south).
    pub wind_dir: f32,
    /// Player has the gunnery special skill.
    pub gunnery: bool,
    /// 0 = Apprentice .. 3 = Swashbuckler
    pub difficulty: usize,
    /// Enemy prefers to run away (merchants) rather than fight.
    pub enemy_flees: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavalOutcomeKind {
    /// Ships touched: boarding (or landing at the town quay for forts).
    Boarded,
    EnemySunk,
    PlayerSunk,
    PlayerEscaped,
    EnemyEscaped,
}

#[derive(Clone, Debug)]
pub struct NavalOutcome {
    pub kind: NavalOutcomeKind,
    pub player_guns: i32,
    pub player_crew: i32,
    pub player_hull: i32,
    pub enemy_guns: i32,
    pub enemy_crew: i32,
    pub enemy_hull: i32,
}

pub struct NavalBattle {
    pub setup: NavalSetup,
}

impl NavalBattle {
    pub fn new(setup: NavalSetup) -> Self {
        NavalBattle { setup }
    }

    /// A default setup used by `PIRATES_TEST=naval`.
    pub fn test_default() -> Self {
        Self::new(NavalSetup {
            player_kind: ShipKind::Barque,
            player_name: "Revenge".into(),
            player_guns: 12,
            player_crew: 80,
            player_hull: 100,
            enemy: NavalEnemy::Ship { kind: ShipKind::Merchantman, name: "San Felipe".into(), nation: Nation::Spain, guns: 14, crew: 90, hull: 100 },
            wind_dir: 0.6,
            gunnery: false,
            difficulty: 1,
            enemy_flees: false,
        })
    }

    pub fn update(&mut self, _inp: &Input, _dt: f32) -> Option<NavalOutcome> {
        None
    }

    pub fn draw(&self) {}
}
