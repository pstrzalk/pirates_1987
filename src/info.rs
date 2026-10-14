//! Information screens: ship status, personal status, politics, the
//! captain's log, the treasure map and the map of the Caribbean.

use crate::data::*;
use crate::game::Action;
use crate::geo::{self, Geo, Tile};
use crate::input::Input;
use crate::sail::draw_full_map;
use crate::ui::{self, *};
use crate::world::*;
use macroquad::prelude::*;
#[allow(unused_imports)]
use crate::ui::{BLACK, BLUE, BROWN, GREEN, MAGENTA, RED, WHITE, YELLOW};

const PAGES: [&str; 6] = ["Fleet", "Personal", "Politics", "Towns", "Log", "Treasure Map"];

pub struct Info {
    page: usize,
    back: Action,
    scroll: usize,
}

impl Info {
    pub fn new(back: Action) -> Info {
        Info { page: 0, back, scroll: 0 }
    }

    pub fn update(&mut self, inp: &Input) -> Option<Action> {
        if inp.cancel() || inp.confirm() {
            return Some(self.back.clone());
        }
        if inp.pressed(KeyCode::Right) || inp.pressed(KeyCode::Tab) {
            self.page = (self.page + 1) % (PAGES.len() + 1);
            self.scroll = 0;
        }
        if inp.pressed(KeyCode::Left) {
            self.page = (self.page + PAGES.len()) % (PAGES.len() + 1);
            self.scroll = 0;
        }
        if inp.pressed(KeyCode::Down) {
            self.scroll += 1;
        }
        if inp.pressed(KeyCode::Up) {
            self.scroll = self.scroll.saturating_sub(1);
        }
        if let Some(d) = inp.digit() {
            if d <= PAGES.len() {
                self.page = d;
            }
        }
        None
    }

    pub fn draw(&self, w: &World, geo: &Geo) {
        if self.page == PAGES.len() {
            draw_full_map(w, geo, true);
            ui::text_center("Left/Right: pages   Esc: return", W / 2.0, H - 14.0, 11.0, WHITE);
            return;
        }
        rect(0.0, 0.0, W, H, BLUE);
        ui::parchment(10.0, 10.0, W - 20.0, H - 20.0);
        // Tabs.
        let mut x = 22.0;
        for (i, p) in PAGES.iter().chain(std::iter::once(&"Map")).enumerate() {
            let tw = ui::measure(p, 11.0) + 12.0;
            rect(x, 18.0, tw, 16.0, if i == self.page { HILITE } else { WHITE });
            frame(x, 18.0, tw, 16.0, BLUE, 1.0);
            ui::text(p, x + 6.0, 20.0, 11.0, if i == self.page { WHITE } else { INK });
            x += tw + 4.0;
        }
        let y = 46.0;
        match self.page {
            0 => self.fleet(w, y),
            1 => self.personal(w, y),
            2 => self.politics(w, y),
            3 => self.towns(w, y),
            4 => self.log(w, y),
            _ => self.treasure(w, geo, y),
        }
        ui::text_center("Left/Right: pages   Up/Down: scroll   Esc: return", W / 2.0, H - 26.0, 10.0, BLACK);
    }

    fn fleet(&self, w: &World, y: f32) {
        let pl = &w.player;
        ui::text(&format!("{} -- {}", pl.title(), w.date_string()), 26.0, y, 15.0, RED);
        let mut yy = y + 24.0;
        ui::text("Ship", 30.0, yy, 11.0, BLACK);
        ui::text("Type", 170.0, yy, 11.0, BLACK);
        ui::text_right("Guns", 380.0, yy, 11.0, BLACK);
        ui::text_right("Hull", 440.0, yy, 11.0, BLACK);
        ui::text_right("Cargo", 510.0, yy, 11.0, BLACK);
        ui::text_right("Crew max", 600.0, yy, 11.0, BLACK);
        yy += 16.0;
        for (i, s) in pl.ships.iter().enumerate() {
            let spec = s.kind.spec();
            let c = if i == 0 { RED } else { INK };
            ui::text(&format!("{}{}", s.name, if i == 0 { " (flag)" } else { "" }), 30.0, yy, 12.0, c);
            ui::text(spec.name, 170.0, yy, 12.0, c);
            ui::text_right(&format!("{}/{}", s.guns, spec.max_guns), 380.0, yy, 12.0, c);
            ui::text_right(&format!("{}%", s.hull), 440.0, yy, 12.0, c);
            ui::text_right(&spec.cargo.to_string(), 510.0, yy, 12.0, c);
            ui::text_right(&spec.max_crew.to_string(), 600.0, yy, 12.0, c);
            yy += 15.0;
        }
        yy += 12.0;
        let rows = [
            format!("Crew: {} men (room for {}), morale: {}", pl.crew, pl.crew_capacity(), pl.morale_word()),
            format!("Ship's purse: {} pieces of eight ({} months since the last division)", pl.gold, (pl.days_since_divide / 30.0) as i32),
            format!("Food: {} tons, lasting about {} days", pl.food, if pl.crew > 0 { pl.food * 100 / pl.crew } else { 999 }),
            format!(
                "Cargo: {} sugar, {} tobacco, {} hides, {} goods ({} / {} tons used)",
                pl.cargo[0], pl.cargo[1], pl.cargo[2], pl.cargo[3], pl.cargo_used(), pl.cargo_capacity()
            ),
            format!("Ships captured: {}   Ships sunk: {}   Towns sacked: {}   Duels won: {}", pl.ships_captured, pl.ships_sunk, pl.towns_sacked, pl.duels_won),
        ];
        for r in rows {
            ui::text(&r, 30.0, yy, 12.0, INK);
            yy += 17.0;
        }
    }

    fn personal(&self, w: &World, y: f32) {
        let pl = &w.player;
        ui::text(&format!("{}, {} privateer", pl.title(), pl.nation.adj()), 26.0, y, 15.0, RED);
        let mut yy = y + 22.0;
        let rows = [
            format!("Age {} years. Health: {}. Special ability: {}.", pl.age as i32, pl.health_word(), pl.skill.name()),
            format!("Personal fortune: {} pieces of eight. Land: {} acres. Difficulty: {}.", pl.wealth, pl.total_land(), pl.difficulty.name()),
            format!("Your reputation: {}.", w.reputation_word()),
            match pl.wife {
                Some((t, _)) => format!("You are married to the daughter of the Governor of {}.", w.towns[t].name()),
                None => "You are not married.".into(),
            },
        ];
        for r in rows {
            ui::text(&r, 30.0, yy, 12.0, INK);
            yy += 16.0;
        }
        yy += 8.0;
        ui::text("Nation", 30.0, yy, 11.0, BLACK);
        ui::text("Title", 150.0, yy, 11.0, BLACK);
        ui::text("Land", 260.0, yy, 11.0, BLACK);
        ui::text("Attitude", 330.0, yy, 11.0, BLACK);
        ui::text("Commission", 460.0, yy, 11.0, BLACK);
        yy += 16.0;
        for n in [Nation::Spain, Nation::England, Nation::France, Nation::Holland, Nation::Pirate] {
            let i = n.idx();
            draw_flag(n, 30.0, yy + 1.0, 14.0, 9.0);
            ui::text(n.name(), 50.0, yy, 12.0, INK);
            if i < 4 {
                ui::text(if pl.rank[i] == 0 { "-" } else { RANKS[pl.rank[i]] }, 150.0, yy, 12.0, INK);
                ui::text(&format!("{}", pl.land[i]), 260.0, yy, 12.0, INK);
                ui::text(if pl.marque[i] { "Letter of Marque" } else { "" }, 460.0, yy, 12.0, INK);
            }
            let s = pl.standing[i];
            let att = match s {
                50.. => "Admiring",
                20..=49 => "Friendly",
                0..=19 => "Neutral",
                -25..=-1 => "Suspicious",
                -60..=-26 => "Hostile",
                _ => "Vengeful",
            };
            ui::text(att, 330.0, yy, 12.0, if s < -25 { RED } else { INK });
            yy += 15.0;
        }
        yy += 10.0;
        ui::text("Your lost family:", 30.0, yy, 12.0, BLACK);
        yy += 15.0;
        for k in 0..4 {
            let s = if pl.family_found[k] {
                format!("Your {} -- found!", RELATIVES[k])
            } else if pl.family_clue[k] {
                format!("Your {} -- held in {}", RELATIVES[k], w.towns[w.family_towns[k]].name())
            } else {
                format!("Your {} -- whereabouts unknown", RELATIVES[k])
            };
            ui::text(&s, 40.0 + (k % 2) as f32 * 290.0, yy + (k / 2) as f32 * 15.0, 12.0, INK);
        }
    }

    fn politics(&self, w: &World, y: f32) {
        ui::text("The Political Situation in the New World", 26.0, y, 15.0, RED);
        let x0 = 140.0;
        let cw = 110.0;
        let yy = y + 30.0;
        for (j, n) in NATIONS.iter().enumerate() {
            draw_flag(*n, x0 + j as f32 * cw + 10.0, yy, 18.0, 12.0);
            ui::text(n.name(), x0 + j as f32 * cw + 32.0, yy, 12.0, INK);
        }
        for (i, a) in NATIONS.iter().enumerate() {
            let ry = yy + 24.0 + i as f32 * 22.0;
            draw_flag(*a, 30.0, ry, 18.0, 12.0);
            ui::text(a.name(), 52.0, ry, 12.0, INK);
            for (j, b) in NATIONS.iter().enumerate() {
                if i == j {
                    continue;
                }
                let (s, c) = match w.relation(*a, *b) {
                    Relation::War => ("WAR", RED),
                    Relation::Peace => ("peace", INK),
                    Relation::Alliance => ("ALLIED", GREEN),
                };
                ui::text(s, x0 + j as f32 * cw + 20.0, ry, 12.0, c);
            }
        }
        let mut ny = yy + 24.0 + 4.0 * 22.0 + 16.0;
        ui::text("Recent news:", 30.0, ny, 12.0, BLACK);
        ny += 16.0;
        for n in w.news.iter().rev().take(8) {
            ui::text(&format!("- {n}"), 36.0, ny, 11.0, INK);
            ny += 14.0;
        }
    }

    fn towns(&self, w: &World, y: f32) {
        ui::text("Towns of the Caribbean", 26.0, y, 15.0, RED);
        let list: Vec<&Town> = w.towns.iter().filter(|t| t.nation.is_some()).collect();
        let per_col = 18;
        let start = (self.scroll * 2).min(list.len().saturating_sub(1));
        for (k, t) in list.iter().skip(start).take(per_col * 2).enumerate() {
            let col = k / per_col;
            let row = k % per_col;
            let x = 30.0 + col as f32 * 300.0;
            let yy = y + 22.0 + row as f32 * 15.0;
            let n = t.nation.unwrap();
            draw_flag(n, x, yy + 1.0, 14.0, 9.0);
            ui::text(t.name(), x + 20.0, yy, 11.0, INK);
            ui::text(&format!("{} {}", t.size_word(), t.wealth_word().to_lowercase()), x + 140.0, yy, 10.0, BLACK);
        }
    }

    fn log(&self, w: &World, y: f32) {
        ui::text("The Captain's Log", 26.0, y, 15.0, RED);
        let n = w.log.len();
        let start = n.saturating_sub(19 + self.scroll);
        let end = n.saturating_sub(self.scroll);
        for (i, l) in w.log[start..end].iter().enumerate() {
            ui::text(l, 30.0, y + 22.0 + i as f32 * 15.0, 11.0, INK);
        }
    }

    fn treasure(&self, w: &World, geo: &Geo, y: f32) {
        let pl = &w.player;
        ui::text("The Treasure Map", 26.0, y, 15.0, RED);
        if pl.treasure_found {
            ui::text("You have already found the lost treasure.", 30.0, y + 24.0, 12.0, INK);
            return;
        }
        if pl.map_pieces == 0 {
            ui::text("You have no pieces of the treasure map. Seek them in taverns and pirate sea chests.", 30.0, y + 24.0, 12.0, INK);
            return;
        }
        // Map region around the treasure, offset so it isn't centered.
        let (tgx, tgy) = geo::to_grid(w.treasure_pos.0, w.treasure_pos.1);
        let ox = ((w.treasure_gold / 1000) % 7) as f32 - 3.0;
        let oy = ((w.treasure_gold / 1000) % 5) as f32 - 2.0;
        let (cx, cy) = (tgx + ox, tgy + oy);
        let cells = 36.0;
        let cs = 7.0;
        let mx = W / 2.0 - cells * cs / 2.0;
        let my = y + 22.0;
        let size = cells * cs;
        rect(mx - 4.0, my - 4.0, size + 8.0, size + 8.0, Color::new(0.85, 0.75, 0.5, 1.0));
        for j in 0..cells as i32 {
            for i in 0..cells as i32 {
                let quad = (if i < cells as i32 / 2 { 0 } else { 1 }) + (if j < cells as i32 / 2 { 0 } else { 2 });
                if quad >= pl.map_pieces as i32 {
                    continue;
                }
                let gx = (cx - cells / 2.0 + i as f32) as i32;
                let gy = (cy - cells / 2.0 + j as f32) as i32;
                let land = gx >= 0 && gy >= 0 && geo.tile(gx as usize, gy as usize) == Tile::Land;
                let c = if land { Color::new(0.55, 0.4, 0.2, 1.0) } else { Color::new(0.93, 0.87, 0.68, 1.0) };
                rect(mx + i as f32 * cs, my + j as f32 * cs, cs, cs, c);
            }
        }
        // Torn edges of missing pieces.
        for q in pl.map_pieces..4 {
            let qx = mx + (q % 2) as f32 * size / 2.0;
            let qy = my + (q / 2) as f32 * size / 2.0;
            rect(qx, qy, size / 2.0, size / 2.0, PARCHMENT);
            ui::text_center("?", qx + size / 4.0, qy + size / 4.0 - 12.0, 24.0, BLACK);
        }
        frame(mx - 4.0, my - 4.0, size + 8.0, size + 8.0, INK, 2.0);
        if pl.map_pieces >= 3 {
            let xx = mx + (tgx - (cx - cells / 2.0)) * cs;
            let yy = my + (tgy - (cy - cells / 2.0)) * cs;
            draw_line(xx - 6.0, yy - 6.0, xx + 6.0, yy + 6.0, 3.0, RED);
            draw_line(xx - 6.0, yy + 6.0, xx + 6.0, yy - 6.0, 3.0, RED);
        }
        let msg = if pl.map_pieces >= 4 {
            let near = w.nearest_town(w.treasure_pos);
            let d = v(w.towns[near].pos).distance(v(w.treasure_pos));
            let leagues = (d * 20.0) as i32;
            let tp = v(w.treasure_pos) - v(w.towns[near].pos);
            let dir = match (tp.y > 0.15, tp.y < -0.15, tp.x > 0.15, tp.x < -0.15) {
                (true, _, true, _) => "northeast",
                (true, _, _, true) => "northwest",
                (true, _, _, _) => "north",
                (_, true, true, _) => "southeast",
                (_, true, _, true) => "southwest",
                (_, true, _, _) => "south",
                (_, _, true, _) => "east",
                _ => "west",
            };
            format!("The treasure lies about {leagues} leagues {dir} of {}. Sail there and press D to dig.", w.towns[near].name())
        } else {
            format!("You hold {} of 4 pieces. Find the rest to learn where the treasure lies.", pl.map_pieces)
        };
        ui::text_center(&msg, W / 2.0, my + size + 10.0, 11.0, INK);
    }
}
