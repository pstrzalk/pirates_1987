//! Land battle minigame: the player's landing party marches on a town
//! through jungle and open ground against the garrison.

use crate::data::Nation;
use crate::input::Input;

#[derive(Clone, Debug)]
pub struct LandSetup {
    pub town: String,
    pub nation: Nation,
    pub player_men: i32,
    pub defenders: i32,
    /// 0 = no fort, 1 = fort, 2 = fortress
    pub fort_level: u8,
    /// 0 = Apprentice .. 3 = Swashbuckler
    pub difficulty: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LandOutcomeKind {
    /// The party reached and took the town.
    Victory,
    /// The party was destroyed or broken.
    Defeat,
    /// The player retreated back to the ships.
    Retreat,
}

#[derive(Clone, Debug)]
pub struct LandOutcome {
    pub kind: LandOutcomeKind,
    pub men_left: i32,
    pub defenders_left: i32,
}

pub struct LandBattle {
    pub setup: LandSetup,
}

impl LandBattle {
    pub fn new(setup: LandSetup) -> Self {
        LandBattle { setup }
    }

    /// A default setup used by `PIRATES_TEST=land`.
    pub fn test_default() -> Self {
        Self::new(LandSetup { town: "Maracaibo".into(), nation: Nation::Spain, player_men: 150, defenders: 120, fort_level: 1, difficulty: 1 })
    }

    pub fn update(&mut self, _inp: &Input, _dt: f32) -> Option<LandOutcome> {
        None
    }

    pub fn draw(&self) {}
}
