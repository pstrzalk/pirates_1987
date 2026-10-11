//! The town merchant: buy and sell food, trade goods and cannon.

use crate::data::*;
use crate::game::Action;
use crate::input::Input;
use crate::ui::{self, *};
use crate::world::World;
use macroquad::prelude::*;
#[allow(unused_imports)]
use crate::ui::{BLACK, BLUE, BROWN, GREEN, MAGENTA, RED, WHITE, YELLOW};

pub struct Market {
    town: usize,
    sel: usize,
    msg: String,
}

const ROWS: usize = 6; // food, 4 goods, cannon

impl Market {
    pub fn new(town: usize) -> Market {
        Market { town, sel: 0, msg: "Right/B buys, Left/S sells (hold Shift for 10). Esc to leave.".into() }
    }

    fn prices(&self, w: &World, row: usize) -> (i32, i32) {
        let t = &w.towns[self.town];
        match row {
            0 => (t.food_buy(), (t.food_buy() * 2 / 3).max(1)),
            1..=4 => (t.buy_price(row - 1), t.sell_price(row - 1)),
            _ => {
                let p = GUN_PRICE + (5 - TOWNS[t.def].size) * 10;
                (p, p / 2)
            }
        }
    }

    fn held(&self, w: &World, row: usize) -> i32 {
        match row {
            0 => w.player.food,
            1..=4 => w.player.cargo[row - 1],
            _ => w.player.total_guns(),
        }
    }

    pub fn update(&mut self, inp: &Input, w: &mut World) -> Option<Action> {
        if inp.cancel() || inp.pressed(KeyCode::Enter) {
            return Some(Action::TownMenu(self.town));
        }
        if inp.pressed(KeyCode::Up) {
            self.sel = (self.sel + ROWS - 1) % ROWS;
        }
        if inp.pressed(KeyCode::Down) {
            self.sel = (self.sel + 1) % ROWS;
        }
        let qty = if inp.down(KeyCode::LeftShift) || inp.down(KeyCode::RightShift) { 10 } else { 1 };
        if inp.pressed(KeyCode::Right) || inp.pressed(KeyCode::B) {
            for _ in 0..qty {
                if !self.buy(w) {
                    break;
                }
            }
        }
        if inp.pressed(KeyCode::Left) || inp.pressed(KeyCode::S) {
            for _ in 0..qty {
                if !self.sell(w) {
                    break;
                }
            }
        }
        None
    }

    fn buy(&mut self, w: &mut World) -> bool {
        let (buy, _) = self.prices(w, self.sel);
        if w.player.gold < buy {
            self.msg = "You cannot afford that.".into();
            return false;
        }
        match self.sel {
            0..=4 => {
                if w.player.cargo_used() >= w.player.cargo_capacity() {
                    self.msg = "Your holds are full.".into();
                    return false;
                }
                if self.sel == 0 {
                    w.player.food += 1;
                    let t = &mut w.towns[self.town];
                    t.food_price = (t.food_price + 0.003).min(2.5);
                } else {
                    let g = self.sel - 1;
                    w.player.cargo[g] += 1;
                    let t = &mut w.towns[self.town];
                    t.price[g] = (t.price[g] + 0.004).min(2.5);
                }
            }
            _ => {
                // Mount on the first ship with a free gun deck.
                let Some(s) = w.player.ships.iter_mut().find(|s| s.guns < s.kind.spec().max_guns) else {
                    self.msg = "Your ships cannot mount more guns.".into();
                    return false;
                };
                s.guns += 1;
            }
        }
        w.player.gold -= buy;
        self.msg = String::new();
        true
    }

    fn sell(&mut self, w: &mut World) -> bool {
        let (_, sell) = self.prices(w, self.sel);
        match self.sel {
            0 => {
                if w.player.food == 0 {
                    return false;
                }
                w.player.food -= 1;
            }
            1..=4 => {
                let g = self.sel - 1;
                if w.player.cargo[g] == 0 {
                    return false;
                }
                w.player.cargo[g] -= 1;
                let t = &mut w.towns[self.town];
                t.price[g] = (t.price[g] - 0.004).max(0.25);
            }
            _ => {
                // Remove from the ship with the most guns, keeping the flagship armed.
                let Some(i) = (0..w.player.ships.len()).rev().max_by_key(|&i| w.player.ships[i].guns) else { return false };
                if w.player.ships[i].guns == 0 || w.player.total_guns() <= 1 {
                    return false;
                }
                w.player.ships[i].guns -= 1;
            }
        }
        w.player.gold += sell;
        self.msg = String::new();
        true
    }

    pub fn draw(&self, w: &World) {
        rect(0.0, 0.0, W, H, Color::new(0.25, 0.15, 0.06, 1.0));
        // Shelves with goods.
        for i in 0..4 {
            rect(0.0, 20.0 + i as f32 * 26.0, W, 4.0, Color::new(0.4, 0.25, 0.1, 1.0));
            for k in 0..14 {
                let c = [BROWN, Color::new(0.8, 0.7, 0.4, 1.0), Color::new(0.6, 0.4, 0.2, 1.0)][(i + k) % 3];
                rect(10.0 + k as f32 * 45.0, 4.0 + i as f32 * 26.0, 30.0, 16.0, c);
            }
        }
        let t = &w.towns[self.town];
        ui::parchment(40.0, 70.0, 560.0, 315.0);
        ui::text(&format!("The Merchant of {}", t.name()), 60.0, 82.0, 18.0, RED);
        let names = ["Food", GOODS[0], GOODS[1], GOODS[2], GOODS[3], "Cannon"];
        let y0 = 120.0;
        ui::text("Goods", 70.0, y0 - 16.0, 12.0, BROWN);
        ui::text_right("Buy", 300.0, y0 - 16.0, 12.0, BROWN);
        ui::text_right("Sell", 380.0, y0 - 16.0, 12.0, BROWN);
        ui::text_right("Aboard", 480.0, y0 - 16.0, 12.0, BROWN);
        for (i, name) in names.iter().enumerate() {
            let y = y0 + i as f32 * 24.0;
            let (buy, sell) = self.prices(w, i);
            let held = self.held(w, i);
            if i == self.sel {
                rect(60.0, y - 3.0, 520.0, 22.0, BROWN);
            }
            let c = if i == self.sel { WHITE } else { INK };
            ui::text(name, 70.0, y, 14.0, c);
            ui::text_right(&buy.to_string(), 300.0, y, 14.0, c);
            ui::text_right(&sell.to_string(), 380.0, y, 14.0, c);
            ui::text_right(&held.to_string(), 480.0, y, 14.0, c);
            if (1..=4).contains(&i) && TOWNS[t.def].produces == Some(i - 1) {
                ui::text("local", 500.0, y + 2.0, 10.0, if i == self.sel { YELLOW } else { GREEN });
            }
        }
        let pl = &w.player;
        let y = y0 + 6.0 * 24.0 + 10.0;
        ui::text(&format!("Gold: {}", pl.gold), 70.0, y, 14.0, INK);
        ui::text(&format!("Cargo: {} / {} tons", pl.cargo_used(), pl.cargo_capacity()), 200.0, y, 14.0, INK);
        let gun_cap: i32 = pl.ships.iter().map(|s| s.kind.spec().max_guns).sum();
        ui::text(&format!("Guns: {} / {}", pl.total_guns(), gun_cap), 420.0, y, 14.0, INK);
        let days = if pl.crew > 0 { pl.food * 300 / pl.crew } else { 999 };
        ui::text(&format!("Food lasts about {} days for {} men.", days, pl.crew), 70.0, y + 22.0, 12.0, BROWN);
        ui::text(&self.msg, 70.0, y + 42.0, 12.0, RED);
        ui::text("Up/Down select   Right/B buy   Left/S sell   Shift x10   Esc leave", 70.0, 362.0, 11.0, BROWN);
    }
}
