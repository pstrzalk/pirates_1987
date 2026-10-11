//! Procedural scene art drawn with shapes: towns, governors, taverns, ships.

use crate::data::*;
use crate::geo::Geo;
use crate::ui::{self, *};
use crate::world::World;
use macroquad::prelude::*;
#[allow(unused_imports)]
use crate::ui::{BLACK, BLUE, BROWN, GREEN, MAGENTA, RED, WHITE, YELLOW};

#[derive(Clone, Debug, PartialEq)]
pub enum Picture {
    None,
    Sea,
    Town(usize),
    Governor(usize),
    Tavern,
    Shipyard,
    Daughter(i32),
    Ship(ShipKind, Nation),
    Family,
    Prison,
    Treasure,
}

fn sky(x: f32, y: f32, w: f32, h: f32, top: Color, bottom: Color) {
    let bands = 12;
    for i in 0..bands {
        let t = i as f32 / (bands - 1) as f32;
        rect(x, y + h * i as f32 / bands as f32, w, h / bands as f32 + 1.0, lerp_color(top, bottom, t));
    }
}

fn sea_band(x: f32, y: f32, w: f32, h: f32, phase: f32) {
    rect(x, y, w, h, Color::new(0.05, 0.2, 0.55, 1.0));
    let rows = (h / 8.0) as i32;
    for r in 0..rows {
        let yy = y + 4.0 + r as f32 * 8.0;
        let off = ((r * 37) % 23) as f32 + phase * (1.0 + r as f32 * 0.2);
        let mut xx = x - 30.0 + off % 30.0;
        while xx < x + w {
            draw_line(xx, yy, xx + 8.0, yy, 1.0, Color::new(0.4, 0.6, 0.95, 0.7));
            xx += 30.0 + (r % 3) as f32 * 7.0;
        }
    }
}

/// Side view of a sailing ship. `x, y` is the waterline center.
pub fn ship_side(x: f32, y: f32, s: f32, kind: ShipKind, nation: Nation, facing_right: bool) {
    let spec = kind.spec();
    let len = 120.0 * s * spec.size;
    let dir = if facing_right { 1.0 } else { -1.0 };
    let hull_h = 18.0 * s * spec.size;
    let hull = Color::new(0.45, 0.25, 0.1, 1.0);
    // Hull: trapezoid with raised stern castle.
    let bow = vec2(x + dir * len * 0.55, y - hull_h * 0.9);
    let stern = vec2(x - dir * len * 0.5, y - hull_h * 1.3);
    let keel_f = vec2(x + dir * len * 0.4, y + hull_h * 0.2);
    let keel_b = vec2(x - dir * len * 0.42, y + hull_h * 0.2);
    draw_triangle(bow, keel_f, keel_b, hull);
    draw_triangle(bow, keel_b, stern, hull);
    // Stern castle for galleons / big ships.
    if spec.size >= 1.0 {
        let sx = x - dir * len * 0.5;
        let w = len * 0.22;
        let rx = if facing_right { sx } else { sx - w };
        rect(rx, y - hull_h * 1.9, w, hull_h * 0.9, hull);
        rect(rx, y - hull_h * 1.95, w, 2.0, YELLOW);
    }
    draw_line(keel_b.x, y - hull_h * 0.45, bow.x - dir * 6.0, y - hull_h * 0.55, 2.0, Color::new(0.85, 0.7, 0.3, 1.0));
    // Gun ports.
    let ports = (spec.max_guns / 4).clamp(1, 9);
    for i in 0..ports {
        let t = (i as f32 + 0.5) / ports as f32;
        let px = keel_b.x + (keel_f.x - keel_b.x) * t;
        rect(px - 2.0, y - hull_h * 0.3, 4.0, 3.5, BLACK);
    }
    // Masts and sails.
    let masts = spec.masts as i32;
    let mast_h = 95.0 * s * spec.size;
    for m in 0..masts {
        let t = if masts == 1 { 0.5 } else { 0.2 + 0.6 * m as f32 / (masts - 1) as f32 };
        let mx = x - dir * len * 0.4 + dir * len * 0.8 * t;
        let h = mast_h * if m == masts / 2 { 1.0 } else { 0.85 };
        draw_line(mx, y - hull_h, mx, y - hull_h - h, 2.0, Color::new(0.3, 0.18, 0.05, 1.0));
        let sail = Color::new(0.95, 0.93, 0.85, 1.0);
        if !spec.square && m == masts - 1 || (!spec.square && masts == 1) {
            // Fore-and-aft triangular/gaff sail.
            draw_triangle(vec2(mx, y - hull_h - h), vec2(mx, y - hull_h - 6.0), vec2(mx - dir * len * 0.35, y - hull_h - 6.0), sail);
            draw_triangle(vec2(mx, y - hull_h - h * 0.95), vec2(mx + dir * len * 0.3, y - hull_h - 4.0), vec2(mx, y - hull_h - 6.0), sail);
        } else {
            let sw = len * 0.22;
            for k in 0..3 {
                let top = y - hull_h - h + 8.0 + k as f32 * h * 0.3;
                let sh = h * 0.26;
                let ww = sw * (1.0 - k as f32 * 0.0) * (0.8 + k as f32 * 0.1);
                rect(mx - ww / 2.0, top, ww, sh, sail);
                draw_line(mx - ww / 2.0 - 2.0, top, mx + ww / 2.0 + 2.0, top, 1.5, Color::new(0.3, 0.18, 0.05, 1.0));
            }
        }
        if m == masts / 2 || masts == 1 {
            draw_flag(nation, mx, y - hull_h - h - 10.0 * s.max(0.6), 16.0 * s.max(0.6), 10.0 * s.max(0.6));
        }
    }
    // Bowsprit.
    draw_line(bow.x, bow.y, bow.x + dir * len * 0.18, bow.y - hull_h * 0.6, 2.0, Color::new(0.3, 0.18, 0.05, 1.0));
}

/// Top-down ship for the sailing map. `heading` in radians (screen space).
pub fn ship_top(x: f32, y: f32, heading: f32, scale: f32, hull: Color, sail: Color) {
    let f = vec2(heading.cos(), heading.sin());
    let r = vec2(-f.y, f.x);
    let c = vec2(x, y);
    let l = 9.0 * scale;
    let w = 3.5 * scale;
    let bow = c + f * l;
    let p1 = c + f * (l * 0.2) + r * w;
    let p2 = c - f * l * 0.9 + r * w * 0.8;
    let p3 = c - f * l * 0.9 - r * w * 0.8;
    let p4 = c + f * (l * 0.2) - r * w;
    draw_triangle(bow, p1, p4, hull);
    draw_triangle(p1, p2, p3, hull);
    draw_triangle(p1, p3, p4, hull);
    // Sails as bars across the hull.
    for k in [-0.35f32, 0.25] {
        let m = c + f * (l * k);
        draw_line((m + r * w * 1.6).x, (m + r * w * 1.6).y, (m - r * w * 1.6).x, (m - r * w * 1.6).y, 2.2 * scale.max(0.7), sail);
    }
    draw_triangle_lines(bow, p1, p4, 1.0, BLACK);
}

/// A simple standing figure; `y` is the feet position.
pub fn person(x: f32, y: f32, h: f32, coat: Color, wig: bool, hat: bool, dress: bool) {
    let skin = Color::new(0.95, 0.78, 0.6, 1.0);
    let head_r = h * 0.09;
    let head_y = y - h + head_r;
    if dress {
        draw_triangle(vec2(x, y - h * 0.62), vec2(x - h * 0.28, y), vec2(x + h * 0.28, y), coat);
        rect(x - h * 0.08, y - h * 0.78, h * 0.16, h * 0.2, coat);
    } else {
        // legs
        draw_line(x - h * 0.06, y - h * 0.45, x - h * 0.08, y, h * 0.06, Color::new(0.2, 0.15, 0.1, 1.0));
        draw_line(x + h * 0.06, y - h * 0.45, x + h * 0.08, y, h * 0.06, Color::new(0.2, 0.15, 0.1, 1.0));
        rect(x - h * 0.13, y - h * 0.8, h * 0.26, h * 0.4, coat);
    }
    // arms
    draw_line(x - h * 0.13, y - h * 0.76, x - h * 0.2, y - h * 0.5, h * 0.05, coat);
    draw_line(x + h * 0.13, y - h * 0.76, x + h * 0.2, y - h * 0.5, h * 0.05, coat);
    if wig {
        draw_circle(x, head_y + head_r * 0.2, head_r * 1.5, WHITE);
    }
    draw_circle(x, head_y, head_r, skin);
    if dress {
        draw_circle(x, head_y - head_r * 0.6, head_r * 1.0, Color::new(0.5, 0.3, 0.1, 1.0));
        draw_circle(x, head_y, head_r * 0.85, skin);
    }
    if hat {
        rect(x - head_r * 1.8, head_y - head_r * 0.9, head_r * 3.6, head_r * 0.5, BLACK);
        draw_triangle(vec2(x - head_r, head_y - head_r * 0.8), vec2(x + head_r, head_y - head_r * 0.8), vec2(x, head_y - head_r * 2.2), BLACK);
    }
}

fn house(x: f32, y: f32, w: f32, h: f32, wall: Color, roof: Color) {
    rect(x, y - h, w, h, wall);
    draw_triangle(vec2(x - 2.0, y - h), vec2(x + w + 2.0, y - h), vec2(x + w / 2.0, y - h - h * 0.6), roof);
    rect(x + w * 0.4, y - h * 0.45, w * 0.2, h * 0.45, Color::new(0.3, 0.2, 0.1, 1.0));
}

fn picture_frame(x: f32, y: f32, w: f32, h: f32) {
    frame(x, y, w, h, BLACK, 2.0);
}

pub fn draw_picture(pic: &Picture, x: f32, y: f32, w: f32, h: f32, world: Option<&World>, geo: &Geo) {
    let _ = geo;
    let t = get_time() as f32;
    match pic {
        Picture::None => {}
        Picture::Sea => {
            sky(x, y, w, h * 0.6, Color::new(0.35, 0.55, 0.95, 1.0), Color::new(0.75, 0.85, 1.0, 1.0));
            sea_band(x, y + h * 0.6, w, h * 0.4, t * 8.0);
            ship_side(x + w * 0.5 + (t * 5.0).sin() * 3.0, y + h * 0.78, 1.0, ShipKind::Frigate, world.map(|w| w.player.nation).unwrap_or(Nation::England), true);
        }
        Picture::Ship(kind, nation) => {
            sky(x, y, w, h * 0.55, Color::new(0.3, 0.5, 0.9, 1.0), Color::new(0.8, 0.85, 0.95, 1.0));
            sea_band(x, y + h * 0.55, w, h * 0.45, t * 8.0);
            let bob = (t * 2.0).sin() * 2.0;
            ship_side(x + w * 0.5, y + h * 0.8 + bob, 1.15, *kind, *nation, false);
            ui::text_shadow(&format!("{} {}", nation.adj(), kind.name()), x + 10.0, y + 8.0, 16.0, WHITE);
        }
        Picture::Town(ti) => {
            let (nation, size, name) = match world {
                Some(w) => (w.towns[*ti].nation.unwrap_or(Nation::Pirate), TOWNS[w.towns[*ti].def].size, w.towns[*ti].name()),
                None => (Nation::Spain, 3, TOWNS[*ti].name),
            };
            sky(x, y, w, h * 0.5, Color::new(0.4, 0.6, 1.0, 1.0), Color::new(0.85, 0.9, 1.0, 1.0));
            // Hills behind.
            draw_circle(x + w * 0.2, y + h * 0.62, w * 0.2, Color::new(0.15, 0.5, 0.15, 1.0));
            draw_circle(x + w * 0.65, y + h * 0.66, w * 0.28, Color::new(0.12, 0.45, 0.12, 1.0));
            rect(x, y + h * 0.5, w, h * 0.2, Color::new(0.2, 0.55, 0.2, 1.0));
            // Houses.
            let n_houses = 4 + size * 3;
            let wall = match nation {
                Nation::Spain => Color::new(0.95, 0.9, 0.75, 1.0),
                Nation::England => Color::new(0.85, 0.75, 0.6, 1.0),
                Nation::France => Color::new(0.95, 0.95, 0.9, 1.0),
                Nation::Holland => Color::new(0.75, 0.4, 0.3, 1.0),
                Nation::Pirate => Color::new(0.6, 0.5, 0.35, 1.0),
            };
            for i in 0..n_houses {
                let hx = x + 30.0 + (i as f32 * 41.0) % (w * 0.62);
                let hy = y + h * 0.7 - ((i * 7) % 3) as f32 * 9.0;
                house(hx, hy, 22.0, 16.0 + (i % 3) as f32 * 3.0, wall, if i % 2 == 0 { RED } else { BROWN });
            }
            // Church.
            if size >= 2 {
                let cx = x + w * 0.38;
                rect(cx, y + h * 0.7 - 50.0, 16.0, 50.0, wall);
                draw_triangle(vec2(cx - 2.0, y + h * 0.7 - 50.0), vec2(cx + 18.0, y + h * 0.7 - 50.0), vec2(cx + 8.0, y + h * 0.7 - 70.0), RED);
                rect(cx + 7.0, y + h * 0.7 - 80.0, 2.0, 10.0, BLACK);
                rect(cx + 4.0, y + h * 0.7 - 77.0, 8.0, 2.0, BLACK);
            }
            // Fort on the right.
            let fx = x + w * 0.74;
            let fy = y + h * 0.72;
            let stone = Color::new(0.6, 0.58, 0.52, 1.0);
            rect(fx, fy - 40.0, 120.0, 40.0, stone);
            for k in 0..8 {
                rect(fx + k as f32 * 15.0, fy - 46.0, 9.0, 6.0, stone);
            }
            rect(fx + 40.0, fy - 62.0, 30.0, 22.0, stone);
            draw_line(fx + 55.0, fy - 62.0, fx + 55.0, fy - 85.0, 2.0, BLACK);
            draw_flag(nation, fx + 56.0, fy - 85.0, 22.0, 14.0);
            for k in 0..4 {
                rect(fx + 10.0 + k as f32 * 28.0, fy - 25.0, 8.0, 5.0, BLACK);
            }
            sea_band(x, y + h * 0.72, w, h * 0.28, t * 6.0);
            ship_side(x + w * 0.18, y + h * 0.93, 0.5, ShipKind::Merchantman, nation, true);
            ui::text_shadow(name, x + 10.0, y + 8.0, 20.0, WHITE);
        }
        Picture::Governor(ti) => {
            let nation = world.and_then(|w| w.towns[*ti].nation).unwrap_or(Nation::Spain);
            rect(x, y, w, h, Color::new(0.45, 0.15, 0.12, 1.0));
            for i in 0..12 {
                rect(x + i as f32 * 56.0 + 20.0, y, 3.0, h * 0.8, Color::new(0.55, 0.22, 0.16, 1.0));
            }
            rect(x, y + h * 0.8, w, h * 0.2, Color::new(0.35, 0.22, 0.1, 1.0));
            // Window with sea view.
            rect(x + 40.0, y + 25.0, 100.0, 110.0, Color::new(0.5, 0.7, 1.0, 1.0));
            rect(x + 40.0, y + 95.0, 100.0, 40.0, Color::new(0.1, 0.3, 0.7, 1.0));
            frame(x + 40.0, y + 25.0, 100.0, 110.0, Color::new(0.85, 0.7, 0.3, 1.0), 4.0);
            draw_line(x + 90.0, y + 25.0, x + 90.0, y + 135.0, 3.0, Color::new(0.85, 0.7, 0.3, 1.0));
            // Flag on wall.
            draw_flag(nation, x + w - 170.0, y + 25.0, 90.0, 56.0);
            // Desk.
            rect(x + w * 0.35, y + h * 0.62, w * 0.35, h * 0.3, Color::new(0.4, 0.22, 0.08, 1.0));
            rect(x + w * 0.35, y + h * 0.6, w * 0.35, 8.0, Color::new(0.55, 0.32, 0.12, 1.0));
            // Governor.
            let coat = match nation {
                Nation::Spain => Color::new(0.1, 0.1, 0.1, 1.0),
                Nation::England => RED,
                Nation::France => BLUE,
                Nation::Holland => Color::new(0.2, 0.2, 0.3, 1.0),
                Nation::Pirate => BROWN,
            };
            person(x + w * 0.52, y + h * 0.66, 110.0, coat, true, false, false);
            // Quill and papers.
            rect(x + w * 0.42, y + h * 0.57, 30.0, 6.0, WHITE);
            draw_line(x + w * 0.6, y + h * 0.6, x + w * 0.62, y + h * 0.5, 1.5, WHITE);
        }
        Picture::Tavern => {
            rect(x, y, w, h, Color::new(0.3, 0.17, 0.07, 1.0));
            for i in 0..8 {
                rect(x, y + i as f32 * 24.0, w, 2.0, Color::new(0.22, 0.12, 0.05, 1.0));
            }
            rect(x, y + h * 0.75, w, h * 0.25, Color::new(0.2, 0.1, 0.04, 1.0));
            // Barrels.
            for i in 0..3 {
                let bx = x + 20.0 + i as f32 * 45.0;
                rect(bx, y + h * 0.5, 38.0, 50.0, Color::new(0.5, 0.3, 0.12, 1.0));
                rect(bx, y + h * 0.55, 38.0, 3.0, DGRAY);
                rect(bx, y + h * 0.7, 38.0, 3.0, DGRAY);
            }
            // Tables with patrons.
            for (i, c) in [RED, BLUE, GREEN, Color::new(0.6, 0.5, 0.2, 1.0), MAGENTA].iter().enumerate() {
                let px = x + 220.0 + i as f32 * 80.0;
                person(px, y + h * 0.88, 70.0 + (i % 2) as f32 * 8.0, *c, false, i % 2 == 0, false);
            }
            for k in 0..2 {
                let tx = x + 250.0 + k as f32 * 170.0;
                rect(tx, y + h * 0.7, 70.0, 8.0, Color::new(0.55, 0.32, 0.12, 1.0));
                rect(tx + 30.0, y + h * 0.7, 8.0, 40.0, Color::new(0.45, 0.25, 0.1, 1.0));
                rect(tx + 10.0, y + h * 0.66, 6.0, 9.0, YELLOW);
            }
            // Candle glow.
            let flicker = 0.8 + (t * 13.0).sin() * 0.1;
            draw_circle(x + w * 0.9, y + 40.0, 30.0, Color::new(1.0, 0.8, 0.3, 0.15 * flicker));
            draw_circle(x + w * 0.9, y + 40.0, 4.0, YELLOW);
        }
        Picture::Shipyard => {
            sky(x, y, w, h * 0.6, Color::new(0.4, 0.6, 1.0, 1.0), Color::new(0.85, 0.9, 1.0, 1.0));
            sea_band(x, y + h * 0.65, w, h * 0.35, t * 6.0);
            rect(x, y + h * 0.6, w * 0.6, h * 0.1, Color::new(0.7, 0.6, 0.4, 1.0));
            // Ship on the slipway, tilted.
            ship_side(x + w * 0.3, y + h * 0.62, 0.9, world.map(|w| w.player.ships[0].kind).unwrap_or(ShipKind::Barque), world.map(|w| w.player.nation).unwrap_or(Nation::England), true);
            // Scaffolding and timber.
            for i in 0..5 {
                let sx = x + w * 0.12 + i as f32 * 40.0;
                draw_line(sx, y + h * 0.6, sx, y + h * 0.3, 2.0, BROWN);
            }
            for i in 0..4 {
                rect(x + w * 0.7, y + h * 0.55 - i as f32 * 8.0, 120.0, 7.0, Color::new(0.6, 0.4, 0.2, 1.0));
            }
            person(x + w * 0.85, y + h * 0.62, 50.0, Color::new(0.5, 0.4, 0.3, 1.0), false, false, false);
        }
        Picture::Daughter(beauty) => {
            sky(x, y, w, h, Color::new(0.1, 0.1, 0.35, 1.0), Color::new(0.5, 0.3, 0.5, 1.0));
            for i in 0..30 {
                let sx = x + ((i * 97) % 640) as f32;
                let sy = y + ((i * 53) % 90) as f32;
                draw_circle(sx, sy, 1.0, WHITE);
            }
            draw_circle(x + w * 0.85, y + 35.0, 18.0, Color::new(1.0, 1.0, 0.85, 1.0));
            // Balcony.
            rect(x, y + h * 0.75, w, h * 0.25, Color::new(0.75, 0.7, 0.6, 1.0));
            for i in 0..20 {
                rect(x + i as f32 * 32.0 + 8.0, y + h * 0.6, 6.0, h * 0.15, Color::new(0.85, 0.8, 0.7, 1.0));
            }
            rect(x, y + h * 0.58, w, 6.0, Color::new(0.85, 0.8, 0.7, 1.0));
            let dress = match beauty {
                1 => BROWN,
                2 => Color::new(0.4, 0.5, 0.3, 1.0),
                3 => BLUE,
                4 => MAGENTA,
                _ => Color::new(0.9, 0.2, 0.4, 1.0),
            };
            person(x + w * 0.45, y + h * 0.9, 120.0, dress, false, false, true);
            person(x + w * 0.6, y + h * 0.9, 125.0, RED, false, true, false);
            // Flowers.
            for i in 0..6 {
                draw_circle(x + 40.0 + i as f32 * 14.0, y + h * 0.56, 4.0, if i % 2 == 0 { LRED } else { YELLOW });
            }
        }
        Picture::Family => {
            sky(x, y, w, h * 0.7, Color::new(0.5, 0.7, 1.0, 1.0), Color::new(1.0, 0.9, 0.7, 1.0));
            rect(x, y + h * 0.7, w, h * 0.3, Color::new(0.7, 0.6, 0.4, 1.0));
            person(x + w * 0.45, y + h * 0.92, 110.0, RED, false, true, false);
            person(x + w * 0.55, y + h * 0.92, 100.0, Color::new(0.5, 0.3, 0.6, 1.0), false, false, true);
        }
        Picture::Prison => {
            rect(x, y, w, h, Color::new(0.15, 0.15, 0.17, 1.0));
            for i in 0..10 {
                for j in 0..6 {
                    frame(x + i as f32 * 64.0 + (j % 2) as f32 * 32.0, y + j as f32 * 32.0, 64.0, 32.0, Color::new(0.1, 0.1, 0.1, 1.0), 1.0);
                }
            }
            rect(x + w * 0.4, y + 30.0, 120.0, 80.0, Color::new(0.3, 0.4, 0.6, 1.0));
            for i in 0..6 {
                rect(x + w * 0.4 + 8.0 + i as f32 * 20.0, y + 30.0, 4.0, 80.0, DGRAY);
            }
            person(x + w * 0.3, y + h * 0.95, 80.0, Color::new(0.4, 0.35, 0.3, 1.0), false, false, false);
        }
        Picture::Treasure => {
            rect(x, y, w, h, Color::new(0.85, 0.75, 0.5, 1.0));
            sea_band(x, y, w, h * 0.3, t * 6.0);
            // Chest
            let cx = x + w * 0.5 - 60.0;
            let cy = y + h * 0.55;
            rect(cx, cy, 120.0, 60.0, Color::new(0.5, 0.28, 0.1, 1.0));
            rect(cx, cy - 25.0, 120.0, 25.0, Color::new(0.6, 0.35, 0.12, 1.0));
            rect(cx + 54.0, cy - 5.0, 12.0, 14.0, YELLOW);
            for i in 0..14 {
                draw_circle(cx + 10.0 + (i * 17 % 100) as f32, cy - 25.0 - (i % 4) as f32 * 4.0, 5.0, YELLOW);
            }
            // Palm tree
            draw_line(x + 80.0, y + h, x + 95.0, y + h * 0.35, 6.0, BROWN);
            for k in 0..5 {
                let a = -2.6 + k as f32 * 0.55;
                draw_line(x + 95.0, y + h * 0.35, x + 95.0 + a.cos() * 50.0, y + h * 0.35 + a.sin() * 25.0 + 15.0, 4.0, GREEN);
            }
        }
    }
    picture_frame(x, y, w, h);
}
