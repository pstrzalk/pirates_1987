//! The sailing screen: steer the fleet across the Caribbean with the wind,
//! meet ships, reach towns, search for treasure.

use crate::art;
use crate::data::*;
use crate::game::Action;
use crate::geo::{self, Geo, Tile};
use crate::input::Input;
use crate::ui::{self, *};
use crate::world::*;
use macroquad::prelude::*;
#[allow(unused_imports)]
use crate::ui::{BLACK, BLUE, BROWN, GREEN, MAGENTA, RED, WHITE, YELLOW};
use std::f32::consts::{PI, TAU};

/// Pixels per grid cell in the sailing view.
const ZOOM: f32 = 8.0;
const VIEW_H: f32 = 356.0;
/// Days of game time per real second.
const DAYS_PER_SEC: f32 = 0.9;

pub struct Sail {
    first: bool,
    /// Town we're currently next to and shouldn't re-enter until we leave.
    near_town: Option<usize>,
    messages: Vec<String>,
    menu: Option<Menu>,
    full_map: bool,
    paused: bool,
    fast: bool,
    hint: Option<(String, f32)>,
    wave_t: f32,
}

impl Sail {
    pub fn new() -> Sail {
        Sail { first: true, near_town: None, messages: vec![], menu: None, full_map: false, paused: false, fast: false, hint: None, wave_t: 0.0 }
    }

    pub fn update(&mut self, inp: &Input, dt: f32, w: &mut World, geo: &Geo) -> Option<Action> {
        self.wave_t += dt;
        if self.first {
            self.first = false;
            self.near_town = (0..w.towns.len()).find(|&i| v(w.towns[i].pos).distance(v(w.player.pos)) < 0.6);
        }
        if !self.messages.is_empty() {
            if inp.any_key() {
                self.messages.remove(0);
            }
            return None;
        }
        if let Some(m) = self.menu.as_mut() {
            if inp.cancel() {
                self.menu = None;
                return None;
            }
            if let Some(i) = m.update(inp) {
                self.menu = None;
                return match i {
                    0 => None,
                    1 => Some(Action::SaveGame),
                    2 => Some(Action::LoadGame),
                    3 => Some(Action::InfoSail),
                    4 => Some(Action::ToTitle),
                    _ => None,
                };
            }
            return None;
        }
        if self.full_map {
            if inp.pressed(KeyCode::M) || inp.cancel() || inp.confirm() || inp.clicked {
                self.full_map = false;
            }
            return None;
        }
        if inp.cancel() {
            self.menu = Some(Menu::new(vec!["Resume voyage".into(), "Save game".into(), "Load game".into(), "Ship's status & information".into(), "Quit to title".into()]));
            return None;
        }
        if inp.pressed(KeyCode::M) {
            self.full_map = true;
            return None;
        }
        if inp.pressed(KeyCode::I) || inp.pressed(KeyCode::S) {
            return Some(Action::InfoSail);
        }
        if inp.pressed(KeyCode::P) || inp.pressed(KeyCode::Space) {
            self.paused = !self.paused;
        }
        if inp.pressed(KeyCode::F) {
            self.fast = !self.fast;
        }
        if inp.pressed(KeyCode::D) {
            self.dig(w, geo);
            return None;
        }
        if let Some((_, t)) = self.hint.as_mut() {
            *t -= dt;
            if *t <= 0.0 {
                self.hint = None;
            }
        }
        if self.paused {
            return None;
        }

        // Steering.
        let turn = if w.player.skill == Skill::Navigation { 2.4 } else { 2.0 };
        if inp.down(KeyCode::Left) {
            w.player.heading -= turn * dt;
        }
        if inp.down(KeyCode::Right) {
            w.player.heading += turn * dt;
        }
        w.player.heading = w.player.heading.rem_euclid(TAU);

        // Time and movement.
        let days = dt * DAYS_PER_SEC * if self.fast { 2.5 } else { 1.0 };
        let angle = angle_diff(w.player.heading, w.wind_dir);
        let speed = w.player.fleet_speed(angle, w.wind_strength);
        let dir = vec2(w.player.heading.cos(), w.player.heading.sin());
        // Screen +y is south: latitude decreases.
        let delta = vec2(dir.x, -dir.y) * speed * days;
        let pos = v(w.player.pos);
        let mut moved = false;
        for cand in [pos + delta, pos + vec2(delta.x, 0.0), pos + vec2(0.0, delta.y)] {
            if geo.is_water(cand.x, cand.y) && Geo::in_bounds(cand.x, cand.y) {
                w.player.pos = p(cand);
                moved = true;
                break;
            }
        }
        if !moved && self.hint.is_none() {
            self.hint = Some(("Land ho! Steer away from the coast.".into(), 1.5));
        }
        let msgs = w.advance(geo, days);
        self.messages.extend(msgs);
        if w.player.health <= 0 {
            return Some(Action::ToSail);
        }

        // Towns.
        let pp = v(w.player.pos);
        if let Some(t) = self.near_town {
            if v(w.towns[t].pos).distance(pp) > 0.7 {
                self.near_town = None;
            }
        }
        for i in 0..w.towns.len() {
            if w.towns[i].nation.is_none() || Some(i) == self.near_town {
                continue;
            }
            if v(w.towns[i].pos).distance(pp) < 0.35 {
                self.near_town = Some(i);
                return Some(Action::ArriveTown(i));
            }
        }
        // Ships.
        for s in &w.ships {
            if s.ignore > 0.0 {
                continue;
            }
            if v(s.pos).distance(pp) < 0.28 {
                return Some(Action::Encounter(s.id, s.chase > 0.0));
            }
        }
        None
    }

    fn dig(&mut self, w: &mut World, geo: &Geo) {
        let pp = v(w.player.pos);
        // Must be close to shore.
        let mut near_land = false;
        for dy in -3..=3 {
            for dx in -3..=3 {
                if geo.tile_at(pp.x + dx as f32 * 0.1, pp.y + dy as f32 * 0.1) == Tile::Land {
                    near_land = true;
                }
            }
        }
        if !near_land {
            self.messages.push("You must be close to the shore to search for buried treasure.".into());
            return;
        }
        w.day += 3.0;
        if !w.player.treasure_found && v(w.treasure_pos).distance(pp) < 0.55 {
            w.player.treasure_found = true;
            w.player.gold += w.treasure_gold;
            w.player.morale = (w.player.morale + 25).min(100);
            let g = w.treasure_gold;
            w.add_log(format!("Dug up buried treasure worth {g} gold!"));
            self.messages.push(format!("Your men go ashore with picks and shovels... and strike a buried chest! The lost treasure is yours: {g} pieces of eight!"));
        } else {
            self.messages.push("Your men search the beaches and dunes for three days, but find nothing.".into());
        }
    }

    pub fn draw(&self, w: &World, geo: &Geo) {
        if self.full_map {
            draw_full_map(w, geo, true);
            ui::text_center("Press M to return to your ship", W / 2.0, H - 16.0, 12.0, WHITE);
            return;
        }
        let (pgx, pgy) = geo::to_grid(w.player.pos.0, w.player.pos.1);
        let cells_w = W / ZOOM;
        let cells_h = VIEW_H / ZOOM;
        let gx0 = (pgx - cells_w / 2.0).clamp(0.0, geo::GW as f32 - cells_w);
        let gy0 = (pgy - cells_h / 2.0).clamp(0.0, geo::GH as f32 - cells_h);
        let tp = geo::TPX as f32;
        draw_texture_ex(
            geo.texture(),
            0.0,
            0.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(W, VIEW_H)),
                source: Some(Rect::new(gx0 * tp, gy0 * tp, cells_w * tp, cells_h * tp)),
                ..Default::default()
            },
        );
        let to_screen = |lon: f32, lat: f32| {
            let (gx, gy) = geo::to_grid(lon, lat);
            vec2((gx - gx0) * ZOOM, (gy - gy0) * ZOOM)
        };
        let visible = |s: Vec2| s.x > -60.0 && s.x < W + 60.0 && s.y > -30.0 && s.y < VIEW_H + 30.0;
        // Waves sparkle in the direction of the wind.
        let wd = vec2(w.wind_dir.cos(), w.wind_dir.sin());
        for i in 0..40 {
            let bx = ((i * 137) % 640) as f32;
            let by = ((i * 71) % 356) as f32;
            let off = wd * ((self.wave_t * 12.0 + i as f32 * 5.0) % 40.0);
            let pos = vec2((bx + off.x).rem_euclid(W), (by + off.y).rem_euclid(VIEW_H));
            let (lon, lat) = geo::from_grid(gx0 + pos.x / ZOOM, gy0 + pos.y / ZOOM);
            if geo.tile_at(lon, lat) == Tile::Sea {
                draw_line(pos.x, pos.y, pos.x + wd.x * 5.0, pos.y + wd.y * 5.0, 1.0, Color::new(0.6, 0.75, 1.0, 0.6));
            }
        }
        // Storm.
        if let Some(st) = &w.storm {
            let c = to_screen(st.pos.0, st.pos.1);
            let r = st.radius * 10.0 * ZOOM;
            for k in 0..5 {
                let a = self.wave_t * 0.8 + k as f32 * 1.3;
                draw_circle(c.x + a.cos() * r * 0.4, c.y + a.sin() * r * 0.4, r * (0.5 + 0.1 * k as f32), Color::new(0.3, 0.3, 0.35, 0.25));
            }
        }
        // Towns.
        for t in &w.towns {
            let Some(n) = t.nation else { continue };
            let s = to_screen(t.pos.0, t.pos.1);
            if !visible(s) {
                continue;
            }
            let stone = Color::new(0.75, 0.72, 0.65, 1.0);
            rect(s.x - 7.0, s.y - 6.0, 14.0, 10.0, stone);
            rect(s.x - 9.0, s.y - 9.0, 5.0, 5.0, stone);
            rect(s.x + 4.0, s.y - 9.0, 5.0, 5.0, stone);
            frame(s.x - 7.0, s.y - 6.0, 14.0, 10.0, BLACK, 1.0);
            draw_line(s.x, s.y - 6.0, s.x, s.y - 18.0, 1.0, BLACK);
            draw_flag(n, s.x, s.y - 18.0, 10.0, 7.0);
            ui::text_shadow(t.name(), s.x - ui::measure(t.name(), 10.0) / 2.0, s.y + 6.0, 10.0, WHITE);
        }
        // AI ships.
        for s in &w.ships {
            let sc = to_screen(s.pos.0, s.pos.1);
            if !visible(sc) {
                continue;
            }
            let hull = Color::new(0.4, 0.22, 0.08, 1.0);
            let sail = if s.nation == Nation::Pirate { DGRAY } else { WHITE };
            let scale = s.ship.kind.spec().size;
            art::ship_top(sc.x, sc.y, -heading_to_screen(s.heading), scale * 1.4, hull, sail);
            draw_flag(s.nation, sc.x - 5.0, sc.y - 20.0, 10.0, 6.0);
        }
        // Player fleet.
        let pc = to_screen(w.player.pos.0, w.player.pos.1);
        let n_ships = w.player.ships.len();
        for k in (1..n_ships.min(4)).rev() {
            let back = vec2(-w.player.heading.cos(), -w.player.heading.sin()) * 10.0 * k as f32;
            let side = vec2(-w.player.heading.sin(), w.player.heading.cos()) * if k % 2 == 0 { 6.0 } else { -6.0 };
            let q = pc + back + side;
            art::ship_top(q.x, q.y, w.player.heading, 1.0, Color::new(0.35, 0.2, 0.08, 1.0), Color::new(0.9, 0.9, 0.8, 1.0));
        }
        art::ship_top(pc.x, pc.y, w.player.heading, w.player.flagship().kind.spec().size * 1.6, Color::new(0.5, 0.3, 0.1, 1.0), YELLOW);
        // Wake.
        let back = vec2(-w.player.heading.cos(), -w.player.heading.sin());
        for k in 1..5 {
            let q = pc + back * (10.0 + k as f32 * 5.0);
            draw_circle(q.x, q.y, 1.2, Color::new(1.0, 1.0, 1.0, 0.5 - k as f32 * 0.1));
        }

        self.draw_compass(w);
        self.draw_status(w, geo);

        if let Some((h, _)) = &self.hint {
            ui::text_center(h, W / 2.0, 30.0, 14.0, YELLOW);
        }
        if self.paused {
            ui::text_center("PAUSED - press Space to continue", W / 2.0, VIEW_H / 2.0 - 40.0, 16.0, if ui::blink() { WHITE } else { YELLOW });
        }
        if let Some(m) = &self.messages.first() {
            let lines = ui::wrap(m, 13.0, 400.0);
            let h = lines.len() as f32 * 16.0 + 40.0;
            let y = VIEW_H / 2.0 - h / 2.0;
            ui::parchment(110.0, y, 420.0, h);
            for (i, l) in lines.iter().enumerate() {
                ui::text(l, 124.0, y + 12.0 + i as f32 * 16.0, 13.0, INK);
            }
            ui::text_right("(press any key)", 520.0, y + h - 18.0, 10.0, BROWN);
        }
        if let Some(m) = &self.menu {
            ui::panel(180.0, 100.0, 280.0, 170.0, BLUE, LCYAN);
            ui::text_center("Captain's Orders", W / 2.0, 110.0, 16.0, YELLOW);
            m.draw(195.0, 140.0, 250.0, 14.0, WHITE, LCYAN, BLACK);
        }
    }

    fn draw_compass(&self, w: &World) {
        let c = vec2(W - 36.0, 36.0);
        draw_circle(c.x, c.y, 28.0, Color::new(0.93, 0.85, 0.64, 0.85));
        draw_circle_lines(c.x, c.y, 28.0, 2.0, INK);
        ui::text_center("N", c.x, c.y - 27.0, 9.0, INK);
        let d = vec2(w.wind_dir.cos(), w.wind_dir.sin());
        let a = c - d * 20.0;
        let b = c + d * 20.0;
        draw_line(a.x, a.y, b.x, b.y, 3.0, BLUE);
        let side = vec2(-d.y, d.x);
        draw_triangle(b + d * 4.0, b - d * 6.0 + side * 6.0, b - d * 6.0 - side * 6.0, BLUE);
        ui::text_center("WIND", c.x, c.y + 30.0, 9.0, WHITE);
        // Heading tick.
        let hd = vec2(w.player.heading.cos(), w.player.heading.sin());
        draw_line(c.x, c.y, c.x + hd.x * 14.0, c.y + hd.y * 14.0, 2.0, RED);
    }

    fn draw_status(&self, w: &World, geo: &Geo) {
        let y = VIEW_H;
        ui::rect(0.0, y, W, H - y, Color::new(0.1, 0.06, 0.02, 1.0));
        ui::rect(0.0, y, W, 2.0, BROWN);
        let pl = &w.player;
        let angle = angle_diff(pl.heading, w.wind_dir);
        let speed = pl.fleet_speed(angle, w.wind_strength);
        let knots = (speed * 8.0).round() as i32;
        ui::text(&w.date_string(), 6.0, y + 6.0, 12.0, YELLOW);
        let near = w.nearest_town(pl.pos);
        let loc = format!("Near {}", w.towns[near].name());
        ui::text(&loc, 6.0, y + 24.0, 11.0, LGRAY);
        let col = |x: f32, label: &str, val: String, c: Color| {
            ui::text(label, x, y + 6.0, 10.0, LGRAY);
            ui::text(&val, x, y + 22.0, 12.0, c);
        };
        col(170.0, "Gold", format!("{}", pl.gold), YELLOW);
        col(225.0, "Crew", format!("{}", pl.crew), WHITE);
        col(270.0, "Mood", pl.morale_word().to_string(), if pl.morale < 25 { LRED } else if pl.morale < 50 { YELLOW } else { LGREEN });
        col(330.0, "Food", format!("{}", pl.food), if pl.food < pl.crew / 20 + 3 { LRED } else { WHITE });
        col(375.0, "Ships", format!("{}", pl.ships.len()), WHITE);
        col(420.0, "Hull", format!("{}%", pl.flagship().hull), if pl.flagship().hull < 40 { LRED } else { WHITE });
        col(465.0, "Speed", format!("{knots} kn"), WHITE);
        let shallow = geo.tile_at(pl.pos.0, pl.pos.1) == Tile::Shallow;
        let mut ix = 6.0;
        if shallow {
            ui::text_shadow("SHALLOWS", ix, 6.0, 11.0, LCYAN);
            ix += 70.0;
        }
        if self.fast {
            ui::text_shadow(">> FAST SAILING", ix, 6.0, 11.0, YELLOW);
        }
        ui::text_right("Arrows steer  M map  I info", W - 4.0, y + 6.0, 9.0, LGRAY);
        ui::text_right("D dig  F fast  Space pause  Esc menu", W - 4.0, y + 22.0, 9.0, LGRAY);
    }
}

/// Smallest absolute difference between two angles (0..PI).
pub fn angle_diff(a: f32, b: f32) -> f32 {
    let mut d = (a - b).rem_euclid(TAU);
    if d > PI {
        d = TAU - d;
    }
    d
}

/// AI headings are computed in lon/lat space (y up); convert for drawing.
fn heading_to_screen(h: f32) -> f32 {
    h
}

/// Draw the whole Caribbean with towns.
pub fn draw_full_map(w: &World, geo: &Geo, show_player: bool) {
    let mh = W * geo::GH as f32 / geo::GW as f32;
    rect(0.0, 0.0, W, H, BLACK);
    draw_texture_ex(geo.texture(), 0.0, 0.0, WHITE, DrawTextureParams { dest_size: Some(vec2(W, mh)), ..Default::default() });
    let sc = W / geo::GW as f32;
    for t in &w.towns {
        let Some(n) = t.nation else { continue };
        let (gx, gy) = geo::to_grid(t.pos.0, t.pos.1);
        let (x, y) = (gx * sc, gy * sc);
        draw_rectangle(x - 2.5, y - 2.5, 5.0, 5.0, n.color());
        draw_rectangle_lines(x - 2.5, y - 2.5, 5.0, 5.0, 1.0, BLACK);
        ui::text_shadow(t.name(), x + 4.0, y - 5.0, 8.0, WHITE);
    }
    if let Some(st) = &w.storm {
        let (gx, gy) = geo::to_grid(st.pos.0, st.pos.1);
        draw_circle(gx * sc, gy * sc, st.radius * 10.0 * sc, Color::new(0.3, 0.3, 0.3, 0.4));
    }
    if show_player {
        let (gx, gy) = geo::to_grid(w.player.pos.0, w.player.pos.1);
        if ui::blink() {
            draw_circle_lines(gx * sc, gy * sc, 6.0, 2.0, YELLOW);
        }
        draw_circle(gx * sc, gy * sc, 2.5, RED);
    }
    ui::panel(4.0, 4.0, 138.0, 52.0, Color::new(0.0, 0.0, 0.3, 0.85), LGRAY);
    let yy = 9.0;
    for (i, n) in [Nation::Spain, Nation::England, Nation::France, Nation::Holland, Nation::Pirate].iter().enumerate() {
        let (cx, cy) = (10.0 + (i % 2) as f32 * 66.0, yy + (i / 2) as f32 * 14.0);
        draw_rectangle(cx, cy + 2.0, 6.0, 6.0, n.color());
        ui::text(n.name(), cx + 9.0, cy, 10.0, WHITE);
    }
}
