//! Sword duel minigame: side-view fencing between two captains whose crews
//! fight behind them.

use crate::input::Input;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DuelKind {
    /// Boarding an enemy ship: captain vs captain, crews fight in the background.
    Boarding,
    /// Storming a town: player vs the garrison commander, soldiers behind him.
    TownAssault,
    /// A personal duel with no crews (e.g. with a rival or the evil nobleman).
    Personal,
}

#[derive(Clone, Debug)]
pub struct DuelSetup {
    pub kind: DuelKind,
    pub player_name: String,
    /// e.g. "Don Alonso"
    pub opponent_name: String,
    /// e.g. "Captain of the Spanish Galleon" or "Commandant of Havana"
    pub opponent_title: String,
    pub player_crew: i32,
    pub enemy_crew: i32,
    /// Player fencing ability, roughly 0.5 (old, sick) .. 1.5 (young expert).
    pub player_skill: f32,
    /// Opponent ability, roughly 0.5 .. 1.5 (scaled by difficulty).
    pub enemy_skill: f32,
}

#[derive(Clone, Debug)]
pub struct DuelOutcome {
    pub won: bool,
    pub player_crew: i32,
    pub enemy_crew: i32,
    /// Player took a serious wound during the fight.
    pub wounded: bool,
}

pub struct Duel {
    pub setup: DuelSetup,
}

impl Duel {
    pub fn new(setup: DuelSetup) -> Self {
        Duel { setup }
    }

    /// A default setup used by `PIRATES_TEST=fencing`.
    pub fn test_default() -> Self {
        Self::new(DuelSetup {
            kind: DuelKind::Boarding,
            player_name: "Nicholas".into(),
            opponent_name: "Don Alonso".into(),
            opponent_title: "Captain of the Spanish Merchantman".into(),
            player_crew: 80,
            enemy_crew: 60,
            player_skill: 1.0,
            enemy_skill: 1.0,
        })
    }

    pub fn update(&mut self, _inp: &Input, _dt: f32) -> Option<DuelOutcome> {
        None
    }

    pub fn draw(&self) {}
}
