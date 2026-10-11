//! Title screen and the new career wizard.

use crate::art;
use crate::data::*;
use crate::game::{Action, SAVE_FILE};
use crate::geo::Geo;
use crate::input::Input;
use crate::ui::{self, *};
use macroquad::prelude::*;
#[allow(unused_imports)]
use crate::ui::{BLACK, BLUE, BROWN, GREEN, MAGENTA, RED, WHITE, YELLOW};

enum Step {
    Main,
    Expedition,
    Era,
    Nation,
    Name,
    Difficulty,
    Skill,
}

pub struct Title {
    step: Step,
    menu: Menu,
    era: usize,
    nation: Nation,
    name: TextEntry,
    difficulty: Difficulty,
    t: f32,
}

impl Title {
    pub fn new() -> Title {
        let mut t = Title {
            step: Step::Main,
            menu: Menu::new(vec![]),
            era: 4,
            nation: Nation::England,
            name: TextEntry::new("Nicholas Craig", 20),
            difficulty: Difficulty::Journeyman,
            t: 0.0,
        };
        t.goto(Step::Main);
        t
    }

    fn goto(&mut self, step: Step) {
        self.menu = match step {
            Step::Main => {
                let has_save = std::path::Path::new(SAVE_FILE).exists();
                Menu::with_enabled(
                    vec!["Start a New Career".into(), "Command a Famous Expedition".into(), "Continue a Saved Game".into(), "Quit".into()],
                    vec![true, true, has_save, true],
                )
            }
            Step::Expedition => {
                let mut items: Vec<String> = EXPEDITIONS.iter().map(|e| e.name.to_string()).collect();
                items.push("Back".into());
                Menu::new(items)
            }
            Step::Era => {
                let mut m = Menu::new(ERAS.iter().map(|e| format!("{}: {}", e.year, e.title)).collect());
                m.sel = self.era;
                m
            }
            Step::Nation => Menu::new(ERAS[self.era].nations.iter().map(|n| format!("{} privateer", n.adj())).collect()),
            Step::Name => Menu::new(vec![]),
            Step::Difficulty => {
                let mut m = Menu::new(
                    [Difficulty::Apprentice, Difficulty::Journeyman, Difficulty::Adventurer, Difficulty::Swashbuckler]
                        .iter()
                        .map(|d| format!("{} (captain's share {}%)", d.name(), (d.share() * 100.0) as i32))
                        .collect(),
                );
                m.sel = 1;
                m
            }
            Step::Skill => Menu::new((0..5).map(|i| Skill::from_idx(i).name().to_string()).collect()),
        };
        self.step = step;
    }

    pub fn update(&mut self, inp: &Input) -> Option<Action> {
        self.t += 1.0 / 60.0;
        if let Step::Name = self.step {
            if inp.cancel() {
                self.goto(Step::Nation);
                return None;
            }
            if self.name.update(inp) && !self.name.value.trim().is_empty() {
                self.goto(Step::Difficulty);
            }
            return None;
        }
        if inp.cancel() {
            match self.step {
                Step::Main => {}
                Step::Expedition | Step::Era => self.goto(Step::Main),
                Step::Nation => self.goto(Step::Era),
                Step::Difficulty => self.goto(Step::Name),
                Step::Skill => self.goto(Step::Difficulty),
                Step::Name => {}
            }
            return None;
        }
        let choice = self.menu.update(inp)?;
        match self.step {
            Step::Main => match choice {
                0 => self.goto(Step::Era),
                1 => self.goto(Step::Expedition),
                2 => return Some(Action::LoadGame),
                _ => return Some(Action::QuitGame),
            },
            Step::Expedition => {
                if choice >= EXPEDITIONS.len() {
                    self.goto(Step::Main);
                } else {
                    let e = &EXPEDITIONS[choice];
                    let leader = e.name.split(['\'', ' ']).take(2).collect::<Vec<_>>().join(" ");
                    return Some(Action::NewGame(e.era, e.year, e.nation, leader, Difficulty::Adventurer, Skill::Gunnery, Some(choice)));
                }
            }
            Step::Era => {
                self.era = choice;
                self.goto(Step::Nation);
            }
            Step::Nation => {
                self.nation = ERAS[self.era].nations[choice];
                self.goto(Step::Name);
            }
            Step::Difficulty => {
                self.difficulty = Difficulty::from_idx(choice);
                self.goto(Step::Skill);
            }
            Step::Skill => {
                return Some(Action::NewGame(
                    self.era,
                    ERAS[self.era].year,
                    self.nation,
                    self.name.value.trim().to_string(),
                    self.difficulty,
                    Skill::from_idx(choice),
                    None,
                ));
            }
            Step::Name => {}
        }
        None
    }

    pub fn draw(&self, _geo: &Geo) {
        let t = get_time() as f32;
        // Sunset seascape.
        for i in 0..20 {
            let k = i as f32 / 19.0;
            rect(0.0, i as f32 * 12.0, W, 13.0, ui::lerp_color(Color::new(0.1, 0.05, 0.3, 1.0), Color::new(1.0, 0.55, 0.2, 1.0), k));
        }
        draw_circle(W * 0.75, 215.0, 40.0, Color::new(1.0, 0.85, 0.4, 1.0));
        rect(0.0, 230.0, W, 170.0, Color::new(0.05, 0.12, 0.35, 1.0));
        for r in 0..10 {
            let y = 236.0 + r as f32 * 16.0;
            let off = (t * 10.0 + r as f32 * 13.0) % 40.0;
            let mut x = -40.0 + off;
            while x < W {
                draw_line(x, y, x + 12.0, y, 1.0, Color::new(1.0, 0.7, 0.4, 0.4));
                x += 40.0;
            }
        }
        art::ship_side(150.0 + (t * 0.7).sin() * 4.0, 300.0 + (t * 1.5).sin() * 2.0, 1.1, ShipKind::FastGalleon, Nation::Pirate, true);

        ui::text_center("PIRATES!", W / 2.0 + 3.0, 20.0, 64.0, BLACK);
        ui::text_center("PIRATES!", W / 2.0, 17.0, 64.0, YELLOW);
        ui::text_center("A tribute to the 1987 classic of adventure on the Spanish Main", W / 2.0, 82.0, 12.0, WHITE);

        let (px, py, pw) = (300.0, 110.0, 320.0);
        match self.step {
            Step::Name => {
                ui::parchment(px, py, pw, 120.0);
                ui::text("What is your name, Captain?", px + 16.0, py + 16.0, 14.0, INK);
                rect(px + 16.0, py + 46.0, pw - 32.0, 24.0, WHITE);
                let cursor = if ui::blink() { "_" } else { "" };
                ui::text(&format!("{}{}", self.name.value, cursor), px + 22.0, py + 50.0, 16.0, BLACK);
                ui::text("Type a name and press Enter.", px + 16.0, py + 86.0, 11.0, BROWN);
            }
            _ => {
                let (title, desc) = match self.step {
                    Step::Main => ("", String::new()),
                    Step::Expedition => ("Famous Expeditions", EXPEDITIONS.get(self.menu.sel).map(|e| e.desc.to_string()).unwrap_or_default()),
                    Step::Era => ("Choose your era", ERAS[self.menu.sel].desc.to_string()),
                    Step::Nation => ("Sail under which flag?", format!("You will begin in a {} port with a letter of introduction to its governor.", ERAS[self.era].nations[self.menu.sel].adj())),
                    Step::Difficulty => ("Difficulty level", ["Gentle foes, but a small share of the plunder.", "A fair challenge.", "Skilled enemies; a larger share.", "Deadly enemies; the captain's share is greatest."][self.menu.sel].to_string()),
                    Step::Skill => ("Special ability", Skill::from_idx(self.menu.sel).desc().to_string()),
                    Step::Name => unreachable!(),
                };
                let mh = self.menu.height(13.0);
                let desc_lines = if desc.is_empty() { vec![] } else { ui::wrap(&desc, 11.0, pw - 30.0) };
                let h = mh + 40.0 + desc_lines.len() as f32 * 13.0 + if title.is_empty() { 0.0 } else { 18.0 };
                let (px, pw) = if matches!(self.step, Step::Expedition) { (250.0, 380.0) } else { (px, pw) };
                ui::parchment(px, py, pw, h);
                let mut y = py + 12.0;
                if !title.is_empty() {
                    ui::text(title, px + 14.0, y, 14.0, RED);
                    y += 20.0;
                }
                self.menu.draw(px + 8.0, y, pw - 16.0, 13.0, INK, BROWN, WHITE);
                y += mh + 8.0;
                for l in desc_lines {
                    ui::text(&l, px + 14.0, y, 11.0, BROWN);
                    y += 13.0;
                }
            }
        }
        ui::text_center("Arrows + Enter to choose, Esc to go back", W / 2.0, H - 16.0, 10.0, LGRAY);
    }
}
