//! Retirement: the final reckoning of a pirate's career.

use crate::art;
use crate::game::Action;
use crate::input::Input;
use crate::ui::{self, *};
use crate::world::World;
use macroquad::prelude::*;
#[allow(unused_imports)]
use crate::ui::{BLACK, BLUE, BROWN, GREEN, MAGENTA, RED, WHITE, YELLOW};

pub struct Ending {
    name: String,
    age: i32,
    year: i32,
    total: i32,
    parts: Vec<(String, i32)>,
    career: &'static str,
    t: f32,
}

const CAREERS: [(i32, &str); 7] = [
    (0, "You spend your last years as a penniless beggar, telling tall tales for a cup of rum."),
    (6, "You live out your days as a humble laborer, haunted by memories of the sea."),
    (12, "You buy a small tavern, where sailors gather to hear of your adventures."),
    (19, "You become a prosperous merchant, respected throughout the colonies."),
    (27, "You retire to a great plantation, a wealthy and respected landowner."),
    (36, "The King appoints you Governor of a colony in recognition of your deeds."),
    (46, "You become a trusted Advisor to the King, the most celebrated adventurer of your age!"),
];

impl Ending {
    pub fn new(w: &World) -> Ending {
        let (total, parts) = w.score();
        let career = CAREERS.iter().rev().find(|(min, _)| total >= *min).map(|c| c.1).unwrap_or(CAREERS[0].1);
        Ending { name: w.player.title(), age: w.player.age as i32, year: w.year(), total, parts, career, t: 0.0 }
    }

    pub fn update(&mut self, inp: &Input) -> Option<Action> {
        self.t += 1.0 / 60.0;
        if self.t > 1.0 && (inp.confirm() || inp.cancel()) {
            return Some(Action::ToTitle);
        }
        None
    }

    pub fn draw(&self) {
        let t = get_time() as f32;
        for i in 0..20 {
            let k = i as f32 / 19.0;
            rect(0.0, i as f32 * 8.0, W, 9.0, ui::lerp_color(Color::new(0.2, 0.3, 0.7, 1.0), Color::new(1.0, 0.7, 0.4, 1.0), k));
        }
        rect(0.0, 150.0, W, 250.0, Color::new(0.1, 0.25, 0.5, 1.0));
        art::ship_side(520.0 + (t * 0.5).sin() * 3.0, 160.0, 0.6, crate::data::ShipKind::Sloop, crate::data::Nation::England, false);
        ui::parchment(30.0, 20.0, 470.0, 360.0);
        ui::text("Retirement", 50.0, 32.0, 20.0, RED);
        ui::text(&format!("In {} {}, aged {}, retires from the sea.", self.year, self.name, self.age), 50.0, 58.0, 12.0, INK);
        let mut y = 84.0;
        for (label, pts) in &self.parts {
            ui::text(label, 54.0, y, 12.0, INK);
            ui::text_right(&format!("{pts}"), 480.0, y, 12.0, BLACK);
            y += 17.0;
        }
        draw_line(54.0, y + 2.0, 480.0, y + 2.0, 1.0, INK);
        ui::text("Total", 54.0, y + 8.0, 13.0, INK);
        ui::text_right(&format!("{}", self.total), 480.0, y + 8.0, 13.0, RED);
        let lines = ui::wrap(self.career, 14.0, 420.0);
        for (i, l) in lines.iter().enumerate() {
            ui::text(l, 54.0, y + 40.0 + i as f32 * 18.0, 14.0, RED);
        }
        ui::text("Press Enter to return to the title screen.", 54.0, 352.0, 11.0, BLACK);
    }
}
