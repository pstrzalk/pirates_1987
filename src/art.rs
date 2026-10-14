//! Scene art. Dialog pictures are painted as 16-colour EGA pixel art in a
//! bevelled frame, in the manner of the 1987 original: close-up portraits of
//! governors, merchants and tavern folk, and small vignettes of towns and ships.

use crate::data::*;
use crate::geo::Geo;
use crate::pixel::{Canvas, Paint, Paint::*, *};
use crate::ui::{self, frame, rect, BLACK, BLUE, DGRAY, LBLUE, LGRAY, RED, WHITE, YELLOW};
use crate::world::World;
use macroquad::prelude::{draw_line, draw_triangle, draw_triangle_lines, get_time, vec2, Color};

#[derive(Clone, Debug, PartialEq)]
pub enum Picture {
    None,
    Sea,
    /// Street scene inside a town.
    Town(usize),
    /// The town seen from the sea.
    Harbor(usize),
    Governor(usize),
    Merchant,
    Tavern,
    Shipyard,
    Daughter(i32),
    Ship(ShipKind, Nation),
    Family,
    Prison,
    Treasure,
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


// ------------------------------------------------------------ pixel scenes

/// Picture size in EGA pixels; each is drawn 2x2 on the 640x400 screen.
pub const PIC_W: i32 = 140;
pub const PIC_H: i32 = 84;

fn nation_coat(n: Nation) -> (u8, u8) {
    // (coat colour, trim colour)
    match n {
        Nation::Spain => (K, Y),
        Nation::England => (R, Y),
        Nation::France => (B, Y),
        Nation::Holland => (DG, W),
        Nation::Pirate => (N, Y),
    }
}

fn flag_px(cv: &mut Canvas, n: Nation, x: i32, y: i32, w: i32, h: i32) {
    match n {
        Nation::Spain => {
            cv.rect(x, y, w, h, R);
            cv.rect(x, y + h / 4, w, h - 2 * (h / 4), Y);
        }
        Nation::England => {
            cv.rect(x, y, w, h, W);
            cv.rect(x + w / 2 - 1, y, 2, h, R);
            cv.rect(x, y + h / 2 - 1, w, 2, R);
        }
        Nation::France => {
            cv.rect(x, y, w, h, W);
            cv.rect(x + w / 2 - 1, y + h / 2 - 1, 2, 2, Y);
        }
        Nation::Holland => {
            cv.rect(x, y, w, h / 3, R);
            cv.rect(x, y + h / 3, w, h / 3, W);
            cv.rect(x, y + 2 * h / 3, w, h - 2 * (h / 3), B);
        }
        Nation::Pirate => {
            cv.rect(x, y, w, h, K);
            cv.rect(x + w / 2 - 1, y + h / 2 - 1, 2, 2, W);
        }
    }
}

fn clouds(cv: &mut Canvas, f: i32, y: i32) {
    for (i, &(cx, cw)) in [(20, 22), (72, 30), (118, 18)].iter().enumerate() {
        let x = ((cx + f / 6 + i as i32 * 3) % (PIC_W + 40)) - 20;
        let x = x as f32;
        let yy = (y + (i as i32 % 2) * 6) as f32;
        cv.ellipse(x, yy + 3.0, cw as f32 * 0.5, 3.5, Ck(W, LC));
        cv.ellipse(x - cw as f32 * 0.15, yy + 1.0, cw as f32 * 0.25, 3.0, W);
        cv.ellipse(x + cw as f32 * 0.15, yy, cw as f32 * 0.2, 3.5, W);
    }
}

/// Open sea from row `y` down, with animated wave crests.
fn sea_px(cv: &mut Canvas, y: i32, f: i32) {
    cv.rect(0, y, PIC_W, PIC_H - y, B);
    cv.rect(0, y, PIC_W, 2, Ck(B, LB));
    for r in 0..(PIC_H - y) / 3 {
        let yy = y + 3 + r * 3;
        let step = 9 + (r % 3) * 4;
        let off = (r * 7 + f * (1 + r % 2)) % step;
        let mut x = -step + off;
        while x < PIC_W {
            let len = 2 + (r % 2) + ((x / step) & 1);
            cv.hline(x, x + len, yy, LB);
            if (x / step + r) % 5 == 0 {
                cv.set(x + 1, yy - 1, W);
            }
            x += step;
        }
    }
}

/// Side view of a ship. `x, wl` is the centre of the waterline.
fn ship_px(cv: &mut Canvas, x: f32, wl: f32, s: f32, kind: ShipKind, nation: Nation, right: bool) {
    let spec = kind.spec();
    let len = 62.0 * s * spec.size;
    let hh = 7.0 * s * spec.size;
    let d = if right { 1.0 } else { -1.0 };
    let px = |t: f32| x + d * t * len;
    let galleon = spec.size >= 1.0;
    // Hull.
    let stern_top = if galleon { wl - hh * 2.4 } else { wl - hh * 1.6 };
    let hull = [
        (px(-0.5), stern_top),
        (px(-0.32), stern_top),
        (px(-0.3), wl - hh * 1.3),
        (px(0.4), wl - hh * 1.25),
        (px(0.52), wl - hh * 1.7),
        (px(0.42), wl + hh * 0.4),
        (px(-0.42), wl + hh * 0.4),
    ];
    cv.poly(&hull, N);
    cv.poly(&[(px(-0.45), wl - hh * 0.2), (px(0.45), wl - hh * 0.3), (px(0.42), wl + hh * 0.4), (px(-0.42), wl + hh * 0.4)], Ck(N, K));
    let deck = (wl - hh * 1.25) as i32;
    cv.hline(px(-0.3) as i32, px(0.42) as i32, deck, Y);
    cv.hline(px(-0.5) as i32, px(-0.32) as i32, stern_top as i32, Y);
    let ports = (spec.max_guns / 6).clamp(1, 7);
    for i in 0..ports {
        let t = -0.28 + 0.62 * (i as f32 + 0.5) / ports as f32;
        cv.set(px(t) as i32, deck + 2, K);
        cv.set(px(t) as i32 + 1, deck + 2, K);
    }
    if galleon {
        cv.vline(px(-0.4) as i32, stern_top as i32 + 2, deck, Y);
    }
    // Bowsprit and jib.
    let bow = (px(0.52), wl - hh * 1.7);
    let tip = (px(0.75), wl - hh * 2.8);
    cv.line(bow.0 as i32, bow.1 as i32, tip.0 as i32, tip.1 as i32, N);
    // Masts and sails.
    let masts = spec.masts as i32;
    let mast_h = 48.0 * s * spec.size;
    let mut main_top = (x, wl);
    for m in 0..masts {
        let t = if masts == 1 { 0.05 } else { -0.3 + 0.6 * m as f32 / (masts - 1) as f32 };
        let mx = px(t);
        let h = mast_h * if m == masts / 2 { 1.0 } else if m == 0 { 0.8 } else { 0.88 };
        let top = wl - hh * 1.25 - h;
        cv.vline(mx as i32, top as i32, deck, K);
        let last = m == masts - 1;
        let fore_aft = !spec.square || (masts >= 3 && m == 0);
        if fore_aft {
            // Lateen / gaff sail swept aft.
            let foot = deck as f32 - 3.0;
            let sail = [(mx + d * 1.0, top + 2.0), (mx + d * 1.0, foot), (mx - d * len * 0.3, foot)];
            cv.poly(&sail, W);
            cv.poly(&[(mx - d * len * 0.12, foot - h * 0.25), (mx - d * len * 0.3, foot), (mx - d * len * 0.12, foot)], Ck(W, LB));
            if !spec.square && last {
                // Headsail to the bowsprit.
                cv.poly(&[(mx + d * 2.0, top + 4.0), (tip.0, tip.1), (mx + d * 2.0, deck as f32 - 3.0)], W);
                cv.poly(&[(mx + d * 2.0, (top + deck as f32) * 0.5), (tip.0, tip.1), (mx + d * 2.0, deck as f32 - 3.0)], Ck(W, LB));
            }
        } else {
            let sw = len * 0.17 * if m == masts / 2 { 1.0 } else { 0.85 };
            let tiers = 3;
            for k in 0..tiers {
                let sh = h * 0.27;
                let ty = top + 3.0 + k as f32 * (sh + 1.5);
                let ww = sw * (0.7 + 0.15 * k as f32);
                let (l, r) = (mx - ww, mx + ww);
                cv.poly(&[(l, ty), (r, ty), (r + d * 1.5, ty + sh), (l + d * 1.5, ty + sh)], W);
                // Shade the belly of the sail on the lee side.
                let (sl, sr) = if right { (l, l + ww * 0.6) } else { (r - ww * 0.6, r) };
                cv.poly(&[(sl, ty + 1.0), (sr, ty + 1.0), (sr, ty + sh), (sl, ty + sh)], Ck(W, LB));
                cv.hline(l as i32 - 1, r as i32 + 1, ty as i32, N);
            }
            if m == masts - 1 {
                cv.poly(&[(mx + d * 2.0, top + h * 0.4), (tip.0, tip.1), (mx + d * 2.0, deck as f32 - 4.0)], W);
                cv.poly(&[(mx + d * 2.0, top + h * 0.7), (tip.0, tip.1), (mx + d * 2.0, deck as f32 - 4.0)], Ck(W, LB));
            }
        }
        if m == masts / 2 {
            main_top = (mx, top);
        }
    }
    let (fx, fy) = (main_top.0 as i32, main_top.1 as i32);
    let fw = (8.0 * s).max(5.0) as i32;
    let fh = (5.0 * s).max(3.0) as i32;
    cv.vline(fx, fy - fh - 1, fy, K);
    flag_px(cv, nation, if right { fx - fw } else { fx + 1 }, fy - fh - 1, fw, fh);
}

/// A bust portrait. All of the parameters that vary between characters.
#[derive(Clone, Copy)]
struct Bust {
    cx: f32,
    /// Centre of the face.
    hy: f32,
    /// Face scale (1.0 = governor size).
    s: f32,
    skin: u8,
    hair: Hair,
    hair_c: u8,
    coat: Paint,
    trim: u8,
    collar: Collar,
    smile: bool,
    mustache: bool,
    beard: bool,
    female: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Hair {
    Wig,
    Bald,
    Long,
    Short,
    Bun,
}

#[derive(Clone, Copy, PartialEq)]
enum Collar {
    Lace,
    Shirt,
    Apron,
    OffShoulder,
    Bodice,
}

fn bust(cv: &mut Canvas, b: Bust) {
    let (cx, hy, s) = (b.cx, b.hy, b.s);
    let fr = 9.0 * s; // face half-width
    let fh = 12.0 * s; // face half-height
    let shade = if b.skin == LR { R } else { N };
    // Hair behind the head.
    match b.hair {
        Hair::Wig => {
            cv.ellipse(cx, hy - 3.0 * s, fr + 5.0 * s, fh + 1.0 * s, W);
            for side in [-1.0, 1.0] {
                for k in 0..3 {
                    let yy = hy - 2.0 * s + k as f32 * 5.0 * s;
                    let xx = cx + side * (fr + 4.5 * s);
                    cv.ellipse(xx, yy, 6.0 * s, 3.0 * s, W);
                    // Shadow under each roll.
                    cv.hline((xx - 4.0 * s) as i32, (xx + 4.0 * s) as i32, (yy + 2.5 * s) as i32, LG);
                    cv.set((xx + side * 4.0 * s) as i32, (yy + 1.0 * s) as i32, LG);
                }
            }
            // Queue ribbon behind the neck.
            cv.rect((cx - 2.0 * s) as i32, (hy + fh) as i32, (4.0 * s) as i32, (3.0 * s) as i32, K);
        }
        Hair::Long => {
            cv.ellipse(cx, hy, fr + 5.0 * s, fh + 3.0 * s, b.hair_c);
            cv.poly(&[(cx - fr - 5.0 * s, hy), (cx + fr + 5.0 * s, hy), (cx + fr + 8.0 * s, hy + 26.0 * s), (cx - fr - 8.0 * s, hy + 26.0 * s)], b.hair_c);
            cv.poly(&[(cx - fr - 3.0 * s, hy + 6.0 * s), (cx - fr - 7.0 * s, hy + 24.0 * s), (cx - fr - 1.0 * s, hy + 22.0 * s)], Ck(b.hair_c, K));
        }
        Hair::Bun => {
            cv.ellipse(cx, hy - 2.0 * s, fr + 2.0 * s, fh + 1.0 * s, b.hair_c);
            cv.ellipse(cx, hy - fh - 1.0 * s, 5.0 * s, 4.0 * s, b.hair_c);
        }
        _ => {}
    }
    // Shoulders and coat.
    let sy = hy + fh + 3.0 * s;
    let bottom = PIC_H as f32;
    let sw = 30.0 * s;
    cv.poly(&[(cx - sw * 0.55, sy), (cx + sw * 0.55, sy), (cx + sw, sy + 6.0 * s), (cx + sw + 6.0 * s, bottom), (cx - sw - 6.0 * s, bottom), (cx - sw, sy + 6.0 * s)], b.coat);
    // Neck.
    cv.rect((cx - 4.0 * s) as i32, (hy + fh - 3.0 * s) as i32, (8.0 * s) as i32, (7.0 * s) as i32, b.skin);
    cv.rect((cx + 1.0 * s) as i32, (hy + fh - 3.0 * s) as i32, (3.0 * s) as i32, (7.0 * s) as i32, Ck(b.skin, shade));
    match b.collar {
        Collar::Lace => {
            // Lace jabot spilling down the chest, and the cravat band.
            cv.poly(&[(cx - 14.0 * s, sy - 1.0), (cx + 14.0 * s, sy - 1.0), (cx + 8.0 * s, sy + 9.0 * s), (cx, sy + 24.0 * s), (cx - 8.0 * s, sy + 9.0 * s)], W);
            cv.poly(&[(cx - 5.0 * s, sy + 8.0 * s), (cx + 5.0 * s, sy + 8.0 * s), (cx, sy + 24.0 * s)], Ck(W, LG));
            let band = (sy + 2.0 * s) as i32;
            cv.rect((cx - 13.0 * s) as i32, band, (26.0 * s) as i32, 2, Sp(W, B));
            for k in 0..((26.0 * s) as i32 / 3) {
                cv.set((cx - 13.0 * s) as i32 + k * 3 + 1, band + 2, B);
            }
            // Coat lapels with trim.
            cv.line((cx - 14.0 * s) as i32, sy as i32, (cx - 6.0 * s) as i32, (sy + 26.0 * s) as i32, b.trim);
            cv.line((cx + 14.0 * s) as i32, sy as i32, (cx + 6.0 * s) as i32, (sy + 26.0 * s) as i32, b.trim);
        }
        Collar::Shirt => {
            cv.poly(&[(cx - 7.0 * s, sy - 1.0), (cx + 7.0 * s, sy - 1.0), (cx + 4.0 * s, sy + 14.0 * s), (cx - 4.0 * s, sy + 14.0 * s)], W);
            cv.poly(&[(cx - 3.0 * s, sy - 1.0), (cx + 3.0 * s, sy - 1.0), (cx, sy + 6.0 * s)], b.skin);
        }
        Collar::Apron => {
            cv.poly(&[(cx - 9.0 * s, sy - 1.0), (cx + 9.0 * s, sy - 1.0), (cx + 8.0 * s, sy + 6.0 * s), (cx - 8.0 * s, sy + 6.0 * s)], W);
            cv.rect((cx - 13.0 * s) as i32, (sy + 6.0 * s) as i32, (26.0 * s) as i32, PIC_H, W);
            cv.rect((cx + 5.0 * s) as i32, (sy + 6.0 * s) as i32, (8.0 * s) as i32, PIC_H, Ck(W, LG));
            cv.line((cx - 9.0 * s) as i32, (sy - 1.0) as i32, (cx - 12.0 * s) as i32, (sy + 7.0 * s) as i32, LG);
            cv.line((cx + 9.0 * s) as i32, (sy - 1.0) as i32, (cx + 12.0 * s) as i32, (sy + 7.0 * s) as i32, LG);
        }
        Collar::OffShoulder => {
            // Bare shoulders above a white ruffled neckline.
            cv.ellipse(cx, sy + 9.0 * s, sw * 0.95, 9.0 * s, b.skin);
            cv.ellipse(cx + sw * 0.9, sy + 11.0 * s, sw * 0.12, 5.0 * s, Ck(b.skin, shade));
            // Ruffled white neckline, scalloped along the bottom.
            let (l, r) = ((cx - sw - 3.0 * s) as i32, (cx + sw + 3.0 * s) as i32);
            cv.rect(l, (sy + 10.0 * s) as i32, r - l, (5.0 * s) as i32, W);
            let by = (sy + 15.0 * s) as i32;
            let mut x = l;
            while x < r {
                cv.hline(x, x + 2, by, W);
                cv.set(x + 3, by - 1, LG);
                x += 4;
            }
            cv.hline(l, r, (sy + 10.0 * s) as i32, LG);
        }
        Collar::Bodice => {
            cv.poly(&[(cx - 10.0 * s, sy - 1.0), (cx + 10.0 * s, sy - 1.0), (cx + 6.0 * s, sy + 9.0 * s), (cx - 6.0 * s, sy + 9.0 * s)], b.skin);
            cv.hline((cx - 10.0 * s) as i32, (cx + 10.0 * s) as i32, (sy - 1.0) as i32, b.trim);
            cv.line((cx - 10.0 * s) as i32, sy as i32, (cx - 6.0 * s) as i32, (sy + 9.0 * s) as i32, b.trim);
            cv.line((cx + 10.0 * s) as i32, sy as i32, (cx + 6.0 * s) as i32, (sy + 9.0 * s) as i32, b.trim);
        }
    }
    // Face.
    cv.ellipse(cx, hy, fr, fh, b.skin);
    cv.poly(&[(cx + fr * 0.62, hy - fh * 0.45), (cx + fr * 1.01, hy - fh * 0.1), (cx + fr * 0.85, hy + fh * 0.6), (cx + fr * 0.35, hy + fh * 0.98), (cx + fr * 0.6, hy + fh * 0.4), (cx + fr * 0.7, hy)], Ck(b.skin, shade));
    // Rosy cheeks.
    cv.set((cx - fr * 0.55) as i32, (hy + 3.0 * s) as i32, R);
    cv.set((cx + fr * 0.55) as i32, (hy + 3.0 * s) as i32, R);
    cv.ellipse(cx, hy + fh * 0.85, fr * 0.45, fh * 0.18, b.skin);
    // Ears.
    if b.hair == Hair::Bald || b.hair == Hair::Short {
        cv.ellipse(cx - fr, hy + 1.0 * s, 2.0 * s, 3.0 * s, b.skin);
        cv.ellipse(cx + fr, hy + 1.0 * s, 2.0 * s, 3.0 * s, shade);
    }
    // Hair on top of the head.
    match b.hair {
        Hair::Wig => {
            cv.poly(&[(cx - fr - 2.0 * s, hy - 2.0 * s), (cx - fr, hy - fh * 0.75), (cx - 3.0 * s, hy - fh - 1.0 * s), (cx + 4.0 * s, hy - fh - 1.0 * s), (cx + fr, hy - fh * 0.75), (cx + fr + 2.0 * s, hy - 2.0 * s), (cx + fr * 0.6, hy - fh * 0.55), (cx - fr * 0.6, hy - fh * 0.55)], W);
            for k in 0..4 {
                cv.set((cx - fr * 0.5 + k as f32 * fr * 0.35) as i32, (hy - fh * 0.75) as i32, LG);
            }
        }
        Hair::Bald => {
            // A fringe of grey hair round the sides.
            for side in [-1.0, 1.0] {
                cv.ellipse(cx + side * (fr - 0.5 * s), hy - 2.0 * s, 2.5 * s, 4.0 * s, Ck(b.hair_c, LG));
            }
            cv.hline((cx - 3.0 * s) as i32, (cx - 1.0 * s) as i32, (hy - fh * 0.7) as i32, W);
        }
        Hair::Short => {
            cv.ellipse(cx, hy - fh * 0.62, fr + 1.0 * s, fh * 0.45, b.hair_c);
            cv.rect((cx - fr - 1.0 * s) as i32, (hy - fh * 0.6) as i32, (3.0 * s) as i32, (6.0 * s) as i32, b.hair_c);
            cv.rect((cx + fr - 2.0 * s) as i32, (hy - fh * 0.6) as i32, (3.0 * s) as i32, (6.0 * s) as i32, b.hair_c);
        }
        Hair::Long | Hair::Bun => {
            cv.poly(&[(cx - fr - 1.0 * s, hy + 2.0 * s), (cx - fr, hy - fh * 0.7), (cx, hy - fh - 1.0 * s), (cx + fr, hy - fh * 0.7), (cx + fr + 1.0 * s, hy + 2.0 * s), (cx + fr * 0.7, hy - fh * 0.4), (cx - fr * 0.3, hy - fh * 0.62), (cx - fr * 0.75, hy - fh * 0.2)], b.hair_c);
        }
    }
    // Eyes and brows.
    let ey = (hy - 1.0 * s) as i32;
    let brow = if b.female { b.hair_c } else if b.hair == Hair::Wig || b.hair == Hair::Bald { N } else { b.hair_c };
    for side in [-1.0f32, 1.0] {
        let ex = (cx + side * 4.0 * s) as i32;
        let w = (1.5 * s).round().max(1.0) as i32;
        cv.hline(ex - w, ex + w, ey - (2.0 * s) as i32, if brow == W { LG } else { brow });
        cv.hline(ex - w, ex + w, ey, W);
        cv.set(ex, ey, K);
        if s >= 0.9 {
            cv.set(ex + if side < 0.0 { 1 } else { -1 }, ey, if b.female { B } else { K });
            cv.hline(ex - w, ex + w, ey + 1, shade);
        }
        if b.female {
            cv.set(ex - w - 1, ey - 1, K);
            cv.hline(ex - w, ex + w, ey - 1, K);
        }
    }
    // Nose.
    let nx = (cx + 0.5 * s) as i32;
    cv.line(nx, ey + 1, nx + (1.0 * s) as i32, ey + (4.0 * s) as i32, shade);
    cv.hline(nx - 1, nx + (1.0 * s) as i32, ey + (5.0 * s) as i32, shade);
    // Mouth.
    let my = (hy + 6.5 * s) as i32;
    let mw = (2.5 * s) as i32;
    let lips = if b.female { R } else { shade };
    if b.smile {
        cv.hline((cx as i32) - mw + 1, (cx as i32) + mw - 1, my + 1, lips);
        cv.set(cx as i32 - mw, my, lips);
        cv.set(cx as i32 + mw, my, lips);
        if s >= 0.9 {
            cv.hline((cx as i32) - mw + 1, (cx as i32) + mw - 1, my, W);
        }
    } else {
        cv.hline((cx as i32) - mw, (cx as i32) + mw, my, lips);
    }
    if b.female {
        cv.hline((cx as i32) - mw + 1, (cx as i32) + mw - 1, my + 1, LR.max(lips));
        cv.set((cx - 6.0 * s) as i32, my - 2, LM);
        cv.set((cx + 6.0 * s) as i32, my - 2, LM);
    }
    if b.mustache {
        let c = if b.hair == Hair::Wig { N } else { b.hair_c };
        cv.hline((cx as i32) - mw - 2, (cx as i32) + mw + 2, my - 1, c);
        cv.set((cx as i32) - mw - 3, my, c);
        cv.set((cx as i32) + mw + 3, my, c);
    }
    if b.beard {
        let c = if b.hair == Hair::Wig { N } else { b.hair_c };
        cv.poly(&[(cx - fr, hy + 1.0 * s), (cx - fr * 0.6, hy + 5.0 * s), (cx + fr * 0.6, hy + 5.0 * s), (cx + fr, hy + 1.0 * s), (cx + fr * 0.6, hy + fh + 3.0 * s), (cx, hy + fh + 5.0 * s), (cx - fr * 0.6, hy + fh + 3.0 * s)], Sp(c, if c == LG { W } else { K }));
        cv.hline((cx as i32) - mw, (cx as i32) + mw, my, R);
    }
}

/// Clasped hands with a ring, in front of the chest.
fn hands(cv: &mut Canvas, cx: f32, y: f32, skin: u8, cuff: u8) {
    let shade = if skin == LR { R } else { N };
    cv.ellipse(cx - 9.0, y + 4.0, 7.0, 4.0, cuff);
    cv.ellipse(cx + 9.0, y + 4.0, 7.0, 4.0, cuff);
    cv.ellipse(cx - 3.0, y, 6.0, 4.0, skin);
    cv.ellipse(cx + 3.0, y + 1.0, 6.0, 4.0, skin);
    cv.line(cx as i32, y as i32 - 3, cx as i32 + 1, y as i32 + 4, shade);
    for k in 0..3 {
        cv.vline((cx - 6.0 + k as f32 * 2.0) as i32, (y - 2.0) as i32, (y - 1.0) as i32, shade);
        cv.vline((cx + 4.0 + k as f32 * 2.0) as i32, (y - 1.0) as i32, y as i32, shade);
    }
    cv.set(cx as i32 + 3, y as i32 - 1, Y);
    cv.set(cx as i32 + 4, y as i32 - 1, LN);
}

/// A small full-length figure; `feet` is the y of the feet.
fn figure(cv: &mut Canvas, x: i32, feet: i32, shirt: u8, hair: u8, breeches: u8) {
    let top = feet - 24;
    cv.ellipse(x as f32 + 0.5, top as f32 + 3.0, 3.0, 3.5, LR);
    cv.rect(x - 2, top - 1, 6, 2, hair);
    cv.set(x + 2, top + 3, K);
    cv.rect(x - 3, top + 7, 7, 8, shirt);
    cv.rect(x + 2, top + 7, 2, 8, Ck(shirt, LG));
    cv.line(x - 3, top + 8, x - 6, top + 14, shirt);
    cv.line(x + 3, top + 8, x - 3, top + 12, shirt);
    cv.set(x - 6, top + 15, LR);
    cv.rect(x - 3, top + 15, 7, 5, breeches);
    cv.rect(x - 3, top + 20, 2, 3, LR);
    cv.rect(x + 2, top + 20, 2, 3, LR);
    cv.hline(x - 4, x - 1, feet - 1, K);
    cv.hline(x + 2, x + 5, feet - 1, K);
}

fn governor_px(cv: &mut Canvas, nation: Nation, f: i32) {
    // Panelled wall with a damask pattern, curtain to one side.
    let (wall_a, wall_b) = match nation {
        Nation::Spain => (R, M),
        Nation::England => (G, K),
        Nation::France => (B, LB),
        Nation::Holland => (N, K),
        Nation::Pirate => (DG, K),
    };
    cv.fill(wall_a);
    for y in 0..PIC_H {
        for x in 0..PIC_W {
            let (u, v) = ((x + y) % 8, (x - y).rem_euclid(8));
            if u == 0 || v == 0 {
                cv.set(x, y, wall_b);
            } else if u == 4 && v == 4 {
                cv.set(x, y, Y);
            }
        }
    }
    // Curtain swag on the left and window on the right.
    cv.poly(&[(0.0, 0.0), (26.0, 0.0), (14.0, 30.0), (10.0, 84.0), (0.0, 84.0)], Ck(M, K));
    cv.poly(&[(0.0, 0.0), (20.0, 0.0), (8.0, 30.0), (5.0, 84.0), (0.0, 84.0)], M);
    cv.line(20, 0, 8, 30, LM);
    cv.rect(108, 8, 24, 34, Y);
    cv.rect(110, 10, 20, 30, LB);
    cv.rect(110, 28, 20, 12, B);
    for k in 0..3 {
        cv.hline(111 + (k * 7 + f) % 18, 113 + (k * 7 + f) % 18, 31 + k * 3, LB);
    }
    cv.vline(120, 10, 39, Y);
    cv.hline(110, 129, 24, Y);
    let (coat, trim) = nation_coat(nation);
    // The governor: powdered wig, lace, a ring on his clasped hands.
    let smile = (f / 12) % 4 == 0;
    bust(cv, Bust { cx: 64.0, hy: 25.0, s: 1.3, skin: LR, hair: Hair::Wig, hair_c: W, coat: Ck(coat, if coat == K { DG } else { coat }), trim, collar: Collar::Lace, smile, mustache: false, beard: false, female: false });
    // Desk in front.
    cv.rect(0, 74, PIC_W, 10, N);
    cv.rect(0, 74, PIC_W, 1, Y);
    cv.rect(0, 77, PIC_W, 7, Ck(N, K));
    cv.rect(96, 70, 16, 5, W);
    cv.rect(98, 69, 14, 1, LG);
    cv.line(112, 70, 120, 58, W);
    cv.line(113, 70, 121, 59, LG);
    hands(cv, 64.0, 72.0, LR, W);
}

fn daughter_px(cv: &mut Canvas, beauty: i32, f: i32) {
    // Stone alcove with an oval window on a starry night.
    cv.fill(Ck(LG, DG));
    cv.ellipse(70.0, 30.0, 52.0, 60.0, DG);
    cv.ellipse(70.0, 30.0, 48.0, 56.0, Sp(DG, K));
    cv.ellipse(112.0, 22.0, 13.0, 16.0, LG);
    cv.ellipse(112.0, 22.0, 11.0, 14.0, B);
    for (i, (sx, sy)) in [(106, 14), (115, 12), (110, 24), (118, 28), (104, 26)].iter().enumerate() {
        cv.set(*sx, *sy, if (f / 3 + i as i32) % 4 == 0 { LC } else { W });
    }
    cv.ellipse(116.0, 17.0, 2.0, 2.0, Y);
    cv.vline(112, 8, 36, LG);
    let (hair, dress, collar) = match beauty {
        1 => (N, Ck(N, K), Collar::Bodice),
        2 => (K, Paint::S(G), Collar::Bodice),
        3 => (Y, Paint::S(LB), Collar::OffShoulder),
        4 => (N, Paint::S(LN), Collar::Bodice),
        _ => (R, Paint::S(W), Collar::OffShoulder),
    };
    let trim = match dress {
        Paint::S(c) if c == W => LG,
        _ => W,
    };
    bust(cv, Bust { cx: 62.0, hy: 27.0, s: 1.25, skin: LR, hair: if beauty == 2 { Hair::Bun } else { Hair::Long }, hair_c: hair, coat: dress, trim, collar, smile: beauty >= 3, mustache: false, beard: false, female: true });
    if collar == Collar::OffShoulder {
        // A painted fan in her hand.
        cv.poly(&[(30.0, 84.0), (18.0, 68.0), (40.0, 66.0)], Ck(W, LM));
        for k in 0..5 {
            cv.line(30, 83, 20 + k * 5, 67, M);
        }
    } else {
        // Knitting needles and balls of wool.
        cv.line(48, 84, 70, 66, LG);
        cv.line(80, 84, 58, 66, LG);
        cv.ellipse(104.0, 78.0, 6.0, 5.0, Ck(B, LB));
        cv.ellipse(116.0, 79.0, 6.0, 5.0, Ck(R, LR));
        cv.ellipse(110.0, 82.0, 6.0, 4.0, Ck(G, LN));
    }
}

fn tavern_px(cv: &mut Canvas, f: i32) {
    // Back wall stacked with casks.
    cv.fill(Ck(N, K));
    cv.rect(0, 0, PIC_W, 34, K);
    for row in 0..2 {
        for k in 0..10 {
            let x = 4.0 + k as f32 * 14.0 + (row * 7) as f32;
            let y = 9.0 + row as f32 * 14.0;
            cv.ellipse(x, y, 7.0, 7.0, N);
            cv.ellipse(x, y, 5.0, 5.0, Ck(N, Y));
            cv.ellipse(x, y, 1.5, 1.5, K);
        }
    }
    cv.rect(0, 34, PIC_W, 2, Y);
    // Lantern.
    let glow = if (f / 2) % 3 == 0 { Y } else { W };
    cv.vline(122, 0, 3, K);
    cv.rect(119, 4, 7, 8, Ck(Y, N));
    cv.rect(121, 6, 3, 4, glow);
    // Barkeep behind the bar: bald, bearded, in a white apron.
    bust(cv, Bust { cx: 36.0, hy: 36.0, s: 1.0, skin: LR, hair: Hair::Bald, hair_c: DG, coat: Paint::S(B), trim: W, collar: Collar::Apron, smile: true, mustache: true, beard: false, female: false });
    // The barmaid with a tankard.
    bust(cv, Bust { cx: 98.0, hy: 36.0, s: 1.0, skin: LR, hair: Hair::Long, hair_c: K, coat: Paint::S(W), trim: R, collar: Collar::Bodice, smile: true, mustache: false, beard: false, female: true });
    cv.rect(108, 54, 6, 8, LG);
    cv.rect(109, 55, 4, 6, W);
    cv.rect(114, 56, 2, 4, LG);
    // Bar counter.
    cv.rect(0, 60, PIC_W, 4, N);
    cv.rect(0, 60, PIC_W, 1, Y);
    cv.rect(0, 64, PIC_W, 20, Ck(N, K));
    for k in 0..7 {
        cv.vline(10 + k * 20, 65, 83, K);
    }
    // Tankards on the bar and a blue jug in front.
    for x in [58, 70, 78] {
        cv.rect(x, 55, 5, 5, LG);
        cv.rect(x + 1, 55, 3, 1, W);
        cv.rect(x + 5, 56, 1, 3, LG);
    }
    cv.ellipse(18.0, 76.0, 7.0, 8.0, B);
    cv.ellipse(17.0, 74.0, 4.0, 4.0, LB);
    cv.rect(15, 66, 6, 4, B);
    cv.ellipse(40.0, 80.0, 10.0, 3.0, W);
    cv.ellipse(40.0, 79.0, 6.0, 2.0, Y);
}

fn merchant_px(cv: &mut Canvas, f: i32) {
    // Shelves of bottles and jars, like the merchant's shop of the original.
    cv.fill(K);
    for row in 0..4 {
        let y = 4 + row * 12;
        cv.rect(0, y + 9, PIC_W, 2, N);
        for k in 0..23 {
            let x = 2 + k * 6;
            let c = [LN, G, W, Y, LB, G][((k * 7 + row * 3) % 6) as usize];
            match (k + row) % 3 {
                0 => {
                    cv.rect(x, y + 3, 3, 6, c);
                    cv.set(x + 1, y + 2, c);
                    cv.set(x + 1, y + 1, LG);
                }
                1 => {
                    cv.rect(x, y + 4, 4, 5, c);
                    cv.hline(x, x + 3, y + 3, N);
                }
                _ => {
                    cv.rect(x, y + 2, 3, 7, c);
                    cv.set(x + 2, y + 3, W);
                }
            }
        }
    }
    for x in [0, 46, 92, 138] {
        cv.rect(x, 0, 2, 52, N);
    }
    // The merchant: bald, spectacles, sleeves rolled, white apron.
    let (mx, my, ms) = (84.0, 30.0, 1.1);
    bust(cv, Bust { cx: mx, hy: my, s: ms, skin: LR, hair: Hair::Bald, hair_c: W, coat: Paint::S(W), trim: LG, collar: Collar::Apron, smile: (f / 10) % 3 == 0, mustache: true, beard: false, female: false });
    // Spectacles.
    let ey = (my - 1.0 * ms) as i32;
    for side in [-1.0f32, 1.0] {
        let ex = (mx + side * 4.0 * ms) as i32;
        cv.hline(ex - 2, ex + 2, ey - 2, K);
        cv.hline(ex - 2, ex + 2, ey + 2, K);
        cv.vline(ex - 2, ey - 2, ey + 2, K);
        cv.vline(ex + 2, ey - 2, ey + 2, K);
    }
    cv.hline((mx - 1.0) as i32, (mx + 1.0) as i32, ey - 1, K);
    // Counter with scales and coins.
    cv.rect(0, 58, PIC_W, 3, Y);
    cv.rect(0, 61, PIC_W, 23, N);
    cv.rect(0, 64, PIC_W, 20, Ck(N, R));
    for k in 0..7 {
        cv.vline(8 + k * 20, 62, 83, K);
    }
    cv.vline(110, 44, 57, Y);
    cv.hline(102, 118, 44, Y);
    cv.line(102, 44, 100, 52, Y);
    cv.line(102, 44, 104, 52, Y);
    cv.line(118, 44, 116, 50, Y);
    cv.line(118, 44, 120, 50, Y);
    cv.hline(99, 105, 53, Y);
    cv.hline(115, 121, 51, Y);
    cv.hline(106, 114, 57, Y);
    for k in 0..4 {
        cv.hline(64 + k * 3, 66 + k * 3, 57 - k % 2, Y);
    }
    // Ledger, quill and sacks on the left.
    cv.rect(52, 53, 18, 5, W);
    cv.vline(61, 53, 57, LG);
    cv.line(70, 53, 75, 44, W);
    cv.ellipse(14.0, 74.0, 11.0, 10.0, Ck(Y, N));
    cv.ellipse(14.0, 64.0, 4.0, 2.0, N);
    cv.ellipse(34.0, 77.0, 10.0, 8.0, Ck(LG, N));
    cv.ellipse(34.0, 69.0, 3.0, 2.0, N);
}

fn town_px(cv: &mut Canvas, nation: Nation, fort: u8, f: i32) {
    // A street corner: brick house with a tiled roof, the church, a lamp.
    cv.fill(LC);
    cv.rect(0, 0, PIC_W, 14, Ck(LC, W));
    clouds(cv, f, 6);
    // Distant fort wall and flag.
    let flag_x = 116;
    if fort > 0 {
        let fw = if fort >= 2 { 58 } else { 40 };
        cv.rect(PIC_W - fw, 44, fw, 18, LG);
        cv.rect(PIC_W - fw, 44, fw, 18, Sp(LG, DG));
        for k in 0..fw / 6 {
            cv.rect(PIC_W - fw + k * 6, 41, 4, 3, LG);
        }
        cv.rect(flag_x - 6, 34, 12, 10, LG);
    }
    cv.vline(flag_x, 18, 44, K);
    let wave = if (f / 2) % 2 == 0 { 0 } else { 1 };
    flag_px(cv, nation, flag_x + 1, 18 + wave, 13, 8);
    // Church tower with a red roof.
    cv.rect(80, 26, 14, 40, W);
    cv.rect(90, 26, 4, 40, Ck(W, LG));
    cv.poly(&[(78.0, 27.0), (96.0, 27.0), (87.0, 12.0)], R);
    cv.poly(&[(87.0, 12.0), (96.0, 27.0), (89.0, 27.0)], Ck(R, K));
    cv.vline(87, 6, 12, K);
    cv.hline(85, 89, 8, K);
    cv.ellipse(87.0, 36.0, 3.0, 4.0, K);
    cv.rect(84, 50, 6, 16, N);
    // Row of houses behind the street.
    let wall = match nation {
        Nation::Spain => W,
        Nation::England => LG,
        Nation::France => W,
        Nation::Holland => LR,
        Nation::Pirate => LG,
    };
    for (i, &(x, w, h)) in [(46, 18, 20), (62, 16, 16), (96, 22, 22)].iter().enumerate() {
        let top = 66 - h;
        cv.rect(x, top, w, h, wall);
        cv.poly(&[(x as f32 - 2.0, top as f32), ((x + w) as f32 + 2.0, top as f32), ((x + w / 2) as f32, top as f32 - 8.0)], if i % 2 == 0 { R } else { N });
        for k in 0..(w / 6) {
            cv.rect(x + 2 + k * 6, top + 4, 3, 4, if (k + i as i32) % 2 == 0 { B } else { K });
        }
        cv.rect(x + w / 2 - 2, 60, 4, 6, N);
    }
    // Green and the street.
    cv.rect(0, 66, PIC_W, 4, G);
    cv.rect(0, 70, PIC_W, 14, Ck(LG, N));
    cv.rect(0, 70, PIC_W, 1, DG);
    // Big brick house in the foreground with a tiled roof.
    cv.poly(&[(0.0, 10.0), (38.0, 10.0), (50.0, 30.0), (0.0, 30.0)], Ck(N, R));
    for r in 0..5 {
        let y = 12 + r * 4;
        cv.hline(0, 38 + r * 2, y, K);
    }
    cv.rect(0, 30, 44, 54, R);
    for r in 0..14 {
        let y = 31 + r * 4;
        cv.rect(0, y, 44, 1, Ck(R, N));
        let off = if r % 2 == 0 { 0 } else { 4 };
        let mut x = off;
        while x < 44 {
            cv.set(x, y + 1, K);
            cv.set(x, y + 2, K);
            x += 8;
        }
    }
    cv.rect(8, 38, 12, 14, B);
    cv.rect(9, 39, 10, 12, LB);
    cv.vline(14, 39, 50, K);
    cv.hline(9, 18, 44, K);
    cv.rect(26, 54, 12, 30, N);
    cv.rect(27, 55, 10, 29, Ck(N, K));
    cv.set(35, 69, Y);
    // Hanging sign.
    cv.hline(44, 60, 34, K);
    cv.vline(48, 34, 37, K);
    cv.vline(58, 34, 37, K);
    cv.rect(46, 37, 14, 10, N);
    cv.rect(47, 38, 12, 8, Y);
    cv.ellipse(53.0, 42.0, 3.0, 3.0, K);
    // Street lamp.
    cv.rect(66, 46, 3, 38, B);
    cv.vline(67, 46, 83, LB);
    cv.rect(62, 38, 11, 8, B);
    cv.rect(63, 39, 9, 6, if (f / 3) % 4 == 0 { W } else { Y });
    cv.vline(67, 39, 44, B);
    cv.poly(&[(61.0, 38.0), (74.0, 38.0), (67.5, 33.0)], B);
    cv.rect(63, 80, 9, 4, B);
}

fn harbor_px(cv: &mut Canvas, nation: Nation, size: i32, fort: u8, f: i32) {
    cv.fill(LC);
    cv.rect(0, 0, PIC_W, 10, Ck(LC, LB));
    cv.rect(0, 0, PIC_W, 4, LB);
    clouds(cv, f, 8);
    // Island with green hills.
    cv.ellipse(30.0, 50.0, 46.0, 18.0, Ck(G, LN));
    cv.ellipse(92.0, 52.0, 56.0, 22.0, G);
    cv.ellipse(92.0, 52.0, 52.0, 18.0, Ck(G, LN));
    cv.ellipse(60.0, 54.0, 70.0, 8.0, G);
    // Houses with red roofs on the slope.
    let n = 5 + size * 3;
    for i in 0..n {
        let x = 22 + (i * 13) % 74;
        let y = 46 - ((i * 5) % 3) * 3 + (i / 6);
        cv.rect(x, y, 7, 5, if i % 3 == 0 { LG } else { W });
        cv.hline(x - 1, x + 7, y - 1, R);
        cv.hline(x, x + 6, y - 2, R);
        cv.set(x + 3, y + 3, K);
    }
    // Church.
    if size >= 2 {
        cv.rect(56, 34, 5, 12, W);
        cv.poly(&[(55.0, 34.0), (62.0, 34.0), (58.5, 28.0)], R);
    }
    // Fort on the point with the flag.
    let fx = 100;
    if fort > 0 {
        let fw = if fort >= 2 { 30 } else { 22 };
        cv.rect(fx, 46, fw, 10, LG);
        cv.rect(fx, 52, fw, 4, Sp(LG, DG));
        for k in 0..fw / 4 {
            cv.rect(fx + k * 4, 44, 2, 2, LG);
        }
        for k in 0..fw / 8 {
            cv.set(fx + 3 + k * 8, 49, K);
        }
    }
    cv.vline(fx + 10, 30, 46, K);
    flag_px(cv, nation, fx + 11, 30 + (f / 2) % 2, 10, 6);
    sea_px(cv, 56, f);
    ship_px(cv, 30.0, 70.0, 0.45, ShipKind::Merchantman, nation, true);
}

fn ship_scene_px(cv: &mut Canvas, kind: ShipKind, nation: Nation, f: i32, right: bool) {
    cv.fill(LB);
    cv.rect(0, 40, PIC_W, 14, Ck(LB, LC));
    cv.rect(0, 50, PIC_W, 4, LC);
    clouds(cv, f, 6);
    sea_px(cv, 54, f);
    let bob = if (f / 3) % 2 == 0 { 0.0 } else { 1.0 };
    let scale = 1.0 / kind.spec().size.max(0.9);
    ship_px(cv, 70.0, 64.0 + bob, scale, kind, nation, right);
    // Waves lapping at the hull.
    for k in 0..12 {
        let x = 34 + k * 6 + (f % 3);
        cv.hline(x, x + 2, 66 + (k % 2), LB);
    }
}

fn shipyard_px(cv: &mut Canvas, kind: ShipKind, nation: Nation, f: i32) {
    cv.fill(LB);
    cv.rect(0, 30, PIC_W, 22, Ck(LB, LC));
    clouds(cv, f, 4);
    sea_px(cv, 58, f);
    // Slipway and stocks.
    cv.poly(&[(0.0, 56.0), (92.0, 56.0), (104.0, 84.0), (0.0, 84.0)], Ck(N, K));
    cv.poly(&[(0.0, 56.0), (92.0, 56.0), (94.0, 60.0), (0.0, 60.0)], N);
    cv.hline(0, 92, 56, Y);
    ship_px(cv, 46.0, 56.0, 0.85, kind, nation, true);
    for k in 0..6 {
        cv.line(12 + k * 13, 56, 16 + k * 13, 49, N);
    }
    // Timber stack and the shipwright with his adze.
    for k in 0..4 {
        cv.rect(98 - k * 2, 62 + k * 4, 30 + k * 4, 3, N);
        cv.hline(98 - k * 2, 127 + k * 2, 62 + k * 4, Y);
    }
    let swing = if (f / 3) % 2 == 0 { 0 } else { 3 };
    figure(cv, 114, 60, W, N, LB);
    cv.line(110, 46, 103, 40 + swing, N);
    cv.rect(101, 38 + swing, 4, 2, LG);
}

fn family_px(cv: &mut Canvas, f: i32) {
    // A warm interior with your long-lost relative.
    cv.fill(Ck(N, R));
    cv.rect(0, 0, PIC_W, 40, Hl(N, K));
    cv.rect(98, 10, 28, 30, Y);
    cv.rect(100, 12, 24, 26, LB);
    cv.rect(100, 28, 24, 10, LN);
    cv.vline(112, 12, 37, Y);
    let flick = if (f / 2) % 2 == 0 { Y } else { W };
    cv.rect(20, 48, 3, 10, W);
    cv.set(21, 46, flick);
    cv.set(21, 47, Y);
    bust(cv, Bust { cx: 64.0, hy: 28.0, s: 1.2, skin: LR, hair: Hair::Short, hair_c: LG, coat: Paint::S(N), trim: Y, collar: Collar::Shirt, smile: true, mustache: true, beard: true, female: false });
}

fn prison_px(cv: &mut Canvas, f: i32) {
    // Dim cell, barred window, a hunched prisoner (after the original).
    cv.fill(Ck(DG, K));
    for r in 0..7 {
        let y = r * 12;
        cv.hline(0, PIC_W - 1, y, K);
        let off = if r % 2 == 0 { 0 } else { 12 };
        let mut x = off;
        while x < PIC_W {
            cv.vline(x, y, y + 11, K);
            cv.hline(x + 1, x + 10, y + 1, DG);
            x += 24;
        }
    }
    cv.rect(86, 10, 26, 32, LB);
    cv.rect(86, 10, 26, 32, if (f / 8) % 2 == 0 { Paint::S(LB) } else { Ck(LB, B) });
    for k in 0..5 {
        cv.rect(88 + k * 5, 10, 2, 32, K);
    }
    cv.rect(84, 8, 30, 2, DG);
    cv.rect(84, 42, 30, 2, DG);
    // Light falling on the floor.
    cv.poly(&[(86.0, 44.0), (112.0, 44.0), (96.0, 84.0), (60.0, 84.0)], Sp(DG, LG));
    // Bench and seated figure in silhouette.
    cv.rect(10, 60, 50, 4, N);
    cv.rect(14, 64, 3, 20, N);
    cv.rect(52, 64, 3, 20, N);
    cv.ellipse(32.0, 36.0, 6.0, 7.0, K);
    cv.poly(&[(22.0, 44.0), (40.0, 42.0), (44.0, 60.0), (20.0, 60.0)], K);
    cv.poly(&[(36.0, 56.0), (52.0, 56.0), (52.0, 80.0), (47.0, 80.0), (46.0, 62.0), (36.0, 62.0)], K);
    cv.line(40, 46, 48, 56, K);
    cv.line(41, 46, 49, 56, K);
    // Chain.
    for k in 0..6 {
        cv.set(4 + k * 2, 50 + k, LG);
    }
}

fn treasure_px(cv: &mut Canvas, f: i32) {
    cv.fill(LB);
    clouds(cv, f, 4);
    sea_px(cv, 24, f);
    cv.poly(&[(0.0, 38.0), (60.0, 34.0), (140.0, 40.0), (140.0, 84.0), (0.0, 84.0)], Ck(Y, N));
    cv.poly(&[(0.0, 38.0), (60.0, 34.0), (140.0, 40.0), (140.0, 44.0), (0.0, 42.0)], Y);
    // Palm tree.
    cv.poly(&[(18.0, 84.0), (24.0, 84.0), (30.0, 20.0), (27.0, 20.0)], N);
    for k in 0..16 {
        let y = 24 + k * 4;
        let x = 29 - (k * 4 * 9 / 64);
        cv.hline(x - 3, x, y, K);
    }
    for (dx, dy) in [(-20.0, 8.0), (-14.0, -6.0), (2.0, -10.0), (16.0, -4.0), (20.0, 10.0)] {
        cv.poly(&[(28.0, 20.0), (28.0 + dx, 20.0 + dy), (28.0 + dx * 0.6, 22.0 + dy * 0.8)], G);
        cv.line(28, 20, (28.0 + dx) as i32, (20.0 + dy) as i32, LN);
    }
    // Open chest spilling gold.
    let (cx, cy) = (60, 56);
    cv.rect(cx, cy, 40, 20, N);
    cv.rect(cx, cy + 6, 40, 2, Y);
    cv.rect(cx + 18, cy + 4, 4, 6, Y);
    cv.poly(&[(cx as f32, cy as f32), ((cx + 40) as f32, cy as f32), ((cx + 36) as f32, (cy - 14) as f32), ((cx + 4) as f32, (cy - 14) as f32)], Ck(N, K));
    cv.ellipse((cx + 20) as f32, cy as f32, 19.0, 4.0, Y);
    for k in 0..12 {
        let x = cx + 2 + (k * 13) % 38;
        let y = cy - 2 + (k % 3);
        cv.set(x, y, if (k + f / 2) % 5 == 0 { W } else { N });
    }
    for k in 0..6 {
        cv.ellipse((cx + 44 + k * 4) as f32, (cy + 18 - (k % 2) * 2) as f32, 2.0, 1.0, Y);
    }
    cv.vline(116, 52, 76, N);
    cv.rect(112, 50, 9, 3, LG);
    cv.ellipse(116.0, 78.0, 8.0, 3.0, Ck(N, K));
}

/// Paint a picture into a fresh canvas.
pub fn paint(pic: &Picture, world: Option<&World>, f: i32) -> Canvas {
    let mut cv = Canvas::new(PIC_W, PIC_H);
    let town = |t: usize| -> (Nation, i32, u8) {
        match world {
            Some(w) => (w.towns[t].nation.unwrap_or(Nation::Pirate), TOWNS[w.towns[t].def].size, w.towns[t].fort),
            None => (Nation::Spain, TOWNS[t].size, 1),
        }
    };
    let player_nation = world.map(|w| w.player.nation).unwrap_or(Nation::England);
    match pic {
        Picture::None => {}
        Picture::Sea => ship_scene_px(&mut cv, world.map(|w| w.player.flagship().kind).unwrap_or(ShipKind::Frigate), player_nation, f, true),
        Picture::Ship(k, n) => ship_scene_px(&mut cv, *k, *n, f, false),
        Picture::Town(t) => {
            let (n, _, fort) = town(*t);
            town_px(&mut cv, n, fort, f);
        }
        Picture::Harbor(t) => {
            let (n, size, fort) = town(*t);
            harbor_px(&mut cv, n, size, fort, f);
        }
        Picture::Governor(t) => {
            let (n, _, _) = town(*t);
            governor_px(&mut cv, n, f);
        }
        Picture::Merchant => merchant_px(&mut cv, f),
        Picture::Tavern => tavern_px(&mut cv, f),
        Picture::Shipyard => shipyard_px(&mut cv, world.map(|w| w.player.flagship().kind).unwrap_or(ShipKind::Barque), player_nation, f),
        Picture::Daughter(b) => daughter_px(&mut cv, *b, f),
        Picture::Family => family_px(&mut cv, f),
        Picture::Prison => prison_px(&mut cv, f),
        Picture::Treasure => treasure_px(&mut cv, f),
    }
    cv
}

/// The bevelled frame the original put round every picture.
pub fn picture_frame(x: f32, y: f32, w: f32, h: f32) {
    rect(x - 10.0, y - 10.0, w + 20.0, h + 20.0, BLUE);
    rect(x - 6.0, y - 6.0, w + 12.0, h + 12.0, LGRAY);
    rect(x - 6.0, y - 6.0, w + 12.0, 2.0, WHITE);
    rect(x - 6.0, y - 6.0, 2.0, h + 12.0, WHITE);
    rect(x - 6.0, y + h + 4.0, w + 12.0, 2.0, DGRAY);
    rect(x + w + 4.0, y - 6.0, 2.0, h + 12.0, DGRAY);
    rect(x - 2.0, y - 2.0, w + 4.0, h + 4.0, BLACK);
}

/// Draw a framed picture with its top-left pixel at (x, y), 2x scale.
pub fn draw_framed(pic: &Picture, x: f32, y: f32, world: Option<&World>) {
    let f = (get_time() * 6.0) as i32;
    let cv = paint(pic, world, f);
    let (w, h) = (PIC_W as f32 * 2.0, PIC_H as f32 * 2.0);
    picture_frame(x, y, w, h);
    cv.present(x, y, w, h);
}

/// Grey EGA info box with a coloured double border.
pub fn info_box(x: f32, y: f32, w: f32, h: f32, border: Color) {
    rect(x, y, w, h, BLACK);
    rect(x + 1.0, y + 1.0, w - 2.0, h - 2.0, border);
    rect(x + 4.0, y + 4.0, w - 8.0, h - 8.0, LGRAY);
    frame(x + 6.0, y + 6.0, w - 12.0, h - 12.0, border, 1.0);
}

/// The whole picture strip at the top of a dialog: light blue backdrop,
/// framed picture on the left and a grey caption box on the right.
pub fn draw_picture(pic: &Picture, title: &str, world: Option<&World>, geo: &Geo) {
    let _ = geo;
    rect(0.0, 0.0, ui::W, STRIP_H, LBLUE);
    draw_framed(pic, 14.0, 11.0, world);
    let (bx, by, bw) = (322.0, 12.0, 306.0);
    let mut lines: Vec<(String, Color)> = vec![];
    let mut flag = None;
    let town_info = |t: usize, lines: &mut Vec<(String, Color)>| {
        if let Some(w) = world {
            let town = &w.towns[t];
            let n = town.nation.unwrap_or(Nation::Pirate);
            lines.push((format!("( {} )", if town.is_haven() { "Pirate haven" } else { n.adj() }), RED));
            lines.push((format!("{} with {}", town.size_word(), town.fort_word()), BLUE));
            lines.push((format!("{} soldiers", town.soldiers), BLUE));
            lines.push((format!("{} citizens", town.population), BLUE));
            lines.push((town.wealth_word().to_string(), BLUE));
            Some(n)
        } else {
            None
        }
    };
    match pic {
        Picture::Town(t) | Picture::Harbor(t) => flag = town_info(*t, &mut lines),
        Picture::Governor(t) => {
            if let Some(w) = world {
                let n = w.towns[*t].nation.unwrap_or(Nation::Pirate);
                lines.push((format!("His Excellency the {} Governor of {}", n.adj(), w.towns[*t].name()), BLUE));
                flag = Some(n);
            }
        }
        Picture::Ship(k, n) => {
            lines.push((format!("{} {}", n.adj(), k.name()), BLUE));
            lines.push((format!("{} guns", k.spec().max_guns), BLUE));
            flag = Some(*n);
        }
        Picture::Sea => {
            if let Some(w) = world {
                let s = w.player.flagship();
                lines.push((format!("{} '{}'", s.kind.name(), s.name), BLUE));
                lines.push((w.date_string(), BLUE));
                flag = Some(w.player.nation);
            }
        }
        _ => {
            if let Some(w) = world {
                lines.push((w.date_string(), BLUE));
            }
        }
    }
    let title_lines = ui::wrap(title, 16.0, bw - 30.0 - if flag.is_some() { 44.0 } else { 0.0 });
    let mut body = vec![];
    for (l, c) in &lines {
        for wl in ui::wrap(l, 13.0, bw - 30.0 - if flag.is_some() { 44.0 } else { 0.0 }) {
            body.push((wl, *c));
        }
    }
    let bh = (title_lines.len() as f32 * 18.0 + body.len() as f32 * 15.0 + 26.0).min(STRIP_H - 20.0);
    info_box(bx, by, bw, bh, BLUE);
    let mut y = by + 12.0;
    for l in &title_lines {
        ui::text(l, bx + 14.0, y, 16.0, BLACK);
        y += 18.0;
    }
    for (l, c) in &body {
        if y + 13.0 > by + bh - 6.0 {
            break;
        }
        ui::text(l, bx + 14.0, y, 13.0, *c);
        y += 15.0;
    }
    if let Some(n) = flag {
        draw_flag(n, bx + bw - 50.0, by + 12.0, 36.0, 24.0);
    }
}

/// Height of the picture strip at the top of a dialog.
pub const STRIP_H: f32 = 190.0;

/// Every picture, for checking the art (PIRATES_TEST=pictures).
pub fn draw_gallery(page: usize) {
    use Nation::*;
    let all = [
        Picture::Governor(0),
        Picture::Merchant,
        Picture::Tavern,
        Picture::Town(0),
        Picture::Harbor(0),
        Picture::Shipyard,
        Picture::Daughter(1),
        Picture::Daughter(2),
        Picture::Daughter(3),
        Picture::Daughter(4),
        Picture::Daughter(5),
        Picture::Family,
        Picture::Prison,
        Picture::Treasure,
        Picture::Sea,
        Picture::Ship(ShipKind::Sloop, Pirate),
        Picture::Ship(ShipKind::Barque, France),
        Picture::Ship(ShipKind::Frigate, England),
        Picture::Ship(ShipKind::WarGalleon, Spain),
        Picture::Ship(ShipKind::Fluyt, Holland),
    ];
    let f = (get_time() * 6.0) as i32;
    let mut sheet = Canvas::new(2 * PIC_W + 4, 2 * PIC_H + 4);
    for (i, pic) in all.iter().skip((page * 4) % all.len()).take(4).enumerate() {
        let cv = paint(pic, None, f);
        sheet.blit(&cv, (i as i32 % 2) * (PIC_W + 4), (i as i32 / 2) * (PIC_H + 4));
    }
    rect(0.0, 0.0, ui::W, ui::H, LBLUE);
    sheet.present(20.0, 20.0, sheet.w as f32 * 2.0, sheet.h as f32 * 2.0);
}
