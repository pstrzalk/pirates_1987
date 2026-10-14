//! The town merchant: buy and sell food, trade goods and cannon.

use crate::art;
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
        let t = &w.towns[self.town];
        let pl = &w.player;
        // The merchant behind his counter, framed as in the original.
        rect(0.0, 0.0, W, art::STRIP_H, LBLUE);
        art::draw_framed(&art::Picture::Merchant, 14.0, 11.0, Some(w));
        let (bx, by, bw, bh) = (322.0, 12.0, 306.0, 166.0);
        art::info_box(bx, by, bw, bh, BLUE);
        ui::text(&format!("The Merchant of {}", t.name()), bx + 14.0, by + 12.0, 16.0, BLACK);
        let gun_cap: i32 = pl.ships.iter().map(|s| s.kind.spec().max_guns).sum();
        let days = if pl.crew > 0 { pl.food * 100 / pl.crew } else { 999 };
        let info = [
            format!("Gold: {}", pl.gold),
            format!("Cargo: {} / {} tons", pl.cargo_used(), pl.cargo_capacity()),
            format!("Guns: {} / {}", pl.total_guns(), gun_cap),
            format!("Crew: {} men", pl.crew),
            format!("Food lasts about {days} days."),
        ];
        for (i, l) in info.iter().enumerate() {
            ui::text(l, bx + 14.0, by + 36.0 + i as f32 * 16.0, 13.0, BLUE);
        }
        for (i, l) in ui::wrap(&self.msg, 13.0, bw - 30.0).iter().take(2).enumerate() {
            ui::text(l, bx + 14.0, by + 122.0 + i as f32 * 15.0, 13.0, RED);
        }
        // Price list.
        let (tx, ty) = (2.0, art::STRIP_H);
        ui::parchment(tx, ty, W - 4.0, H - ty - 2.0);
        let names = ["Food", GOODS[0], GOODS[1], GOODS[2], GOODS[3], "Cannon"];
        let y0 = ty + 32.0;
        ui::text("Goods", 40.0, y0 - 18.0, 12.0, BLACK);
        ui::text_right("Buy", 300.0, y0 - 18.0, 12.0, BLACK);
        ui::text_right("Sell", 380.0, y0 - 18.0, 12.0, BLACK);
        ui::text_right("Aboard", 480.0, y0 - 18.0, 12.0, BLACK);
        for (i, name) in names.iter().enumerate() {
            let y = y0 + i as f32 * 21.0;
            let (buy, sell) = self.prices(w, i);
            let held = self.held(w, i);
            if i == self.sel {
                rect(30.0, y - 3.0, 580.0, 20.0, HILITE);
            }
            let c = if i == self.sel { WHITE } else { INK };
            ui::text(name, 40.0, y, 14.0, c);
            ui::text_right(&buy.to_string(), 300.0, y, 14.0, c);
            ui::text_right(&sell.to_string(), 380.0, y, 14.0, c);
            ui::text_right(&held.to_string(), 480.0, y, 14.0, c);
            if (1..=4).contains(&i) && TOWNS[t.def].produces == Some(i - 1) {
                ui::text("local", 500.0, y + 2.0, 10.0, if i == self.sel { YELLOW } else { GREEN });
            }
        }
        ui::text("Up/Down select   Right/B buy   Left/S sell   Shift x10   Esc leave", 40.0, H - 26.0, 11.0, BLACK);
    }
}
