//! A tiny software canvas in the 16-colour EGA palette, used to paint scene
//! pictures as chunky, dithered pixel art like the 1987 original.

use macroquad::prelude::*;
use std::cell::RefCell;

pub const K: u8 = 0; // black
pub const B: u8 = 1; // blue
pub const G: u8 = 2; // green
pub const R: u8 = 4; // red
pub const M: u8 = 5; // magenta
pub const N: u8 = 6; // brown
pub const LG: u8 = 7; // light gray
pub const DG: u8 = 8; // dark gray
pub const LB: u8 = 9; // light blue
pub const LN: u8 = 10; // light green
pub const LC: u8 = 11; // light cyan
pub const LR: u8 = 12; // light red
pub const LM: u8 = 13; // light magenta
pub const Y: u8 = 14; // yellow
pub const W: u8 = 15; // white

const EGA: [[u8; 3]; 16] = [
    [0x00, 0x00, 0x00],
    [0x00, 0x00, 0xAA],
    [0x00, 0xAA, 0x00],
    [0x00, 0xAA, 0xAA],
    [0xAA, 0x00, 0x00],
    [0xAA, 0x00, 0xAA],
    [0xAA, 0x55, 0x00],
    [0xAA, 0xAA, 0xAA],
    [0x55, 0x55, 0x55],
    [0x55, 0x55, 0xFF],
    [0x55, 0xFF, 0x55],
    [0x55, 0xFF, 0xFF],
    [0xFF, 0x55, 0x55],
    [0xFF, 0x55, 0xFF],
    [0xFF, 0xFF, 0x55],
    [0xFF, 0xFF, 0xFF],
];

/// A fill pattern: solid colour or one of the classic ordered dithers.
#[derive(Clone, Copy, Debug)]
pub enum Paint {
    S(u8),
    /// 50% checkerboard.
    Ck(u8, u8),
    /// Mostly the first colour, every fourth pixel the second.
    Sp(u8, u8),
    /// Alternating horizontal lines.
    Hl(u8, u8),
    /// Alternating vertical lines.
    Vl(u8, u8),
}

impl Paint {
    fn at(self, x: i32, y: i32) -> u8 {
        match self {
            Paint::S(c) => c,
            Paint::Ck(a, b) => if (x + y) & 1 == 0 { a } else { b },
            Paint::Sp(a, b) => if x & 1 == 0 && (y & 1 == 0) == (x & 2 == 0) { b } else { a },
            Paint::Hl(a, b) => if y & 1 == 0 { a } else { b },
            Paint::Vl(a, b) => if x & 1 == 0 { a } else { b },
        }
    }
}

impl From<u8> for Paint {
    fn from(c: u8) -> Paint {
        Paint::S(c)
    }
}

pub struct Canvas {
    pub w: i32,
    pub h: i32,
    px: Vec<u8>,
}

impl Canvas {
    pub fn new(w: i32, h: i32) -> Canvas {
        Canvas { w, h, px: vec![0; (w * h) as usize] }
    }

    pub fn set(&mut self, x: i32, y: i32, c: u8) {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            self.px[(y * self.w + x) as usize] = c;
        }
    }

    fn span(&mut self, y: i32, x0: i32, x1: i32, p: Paint) {
        for x in x0.max(0)..=x1.min(self.w - 1) {
            self.set(x, y, p.at(x, y));
        }
    }

    pub fn fill(&mut self, p: impl Into<Paint>) {
        let p = p.into();
        self.rect(0, 0, self.w, self.h, p);
    }

    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, p: impl Into<Paint>) {
        let p = p.into();
        for yy in y.max(0)..(y + h).min(self.h) {
            self.span(yy, x, x + w - 1, p);
        }
    }

    pub fn hline(&mut self, x0: i32, x1: i32, y: i32, c: u8) {
        self.span(y, x0.min(x1), x0.max(x1), Paint::S(c));
    }

    pub fn vline(&mut self, x: i32, y0: i32, y1: i32, c: u8) {
        for y in y0.min(y1)..=y0.max(y1) {
            self.set(x, y, c);
        }
    }

    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: u8) {
        let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
        let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
        let (mut x, mut y, mut err) = (x0, y0, dx + dy);
        loop {
            self.set(x, y, c);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// Filled ellipse centred on (cx, cy).
    pub fn ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, p: impl Into<Paint>) {
        let p = p.into();
        let (y0, y1) = ((cy - ry).floor() as i32, (cy + ry).ceil() as i32);
        for y in y0..=y1 {
            let dy = (y as f32 + 0.5 - cy) / ry;
            if dy.abs() > 1.0 {
                continue;
            }
            let hw = rx * (1.0 - dy * dy).sqrt();
            let (a, b) = ((cx - hw).round() as i32, (cx + hw).round() as i32 - 1);
            if b >= a {
                self.span(y, a, b, p);
            }
        }
    }

    /// Filled polygon (even-odd scanline fill).
    pub fn poly(&mut self, pts: &[(f32, f32)], p: impl Into<Paint>) {
        let p = p.into();
        if pts.len() < 3 {
            return;
        }
        let ymin = pts.iter().map(|q| q.1).fold(f32::MAX, f32::min).floor() as i32;
        let ymax = pts.iter().map(|q| q.1).fold(f32::MIN, f32::max).ceil() as i32;
        let mut xs = Vec::with_capacity(8);
        for y in ymin.max(0)..=ymax.min(self.h - 1) {
            let sy = y as f32 + 0.5;
            xs.clear();
            for i in 0..pts.len() {
                let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
                if (a.1 <= sy && b.1 > sy) || (b.1 <= sy && a.1 > sy) {
                    xs.push(a.0 + (sy - a.1) / (b.1 - a.1) * (b.0 - a.0));
                }
            }
            xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
            for pair in xs.chunks(2) {
                if let [a, b] = pair {
                    let (a, b) = (a.round() as i32, b.round() as i32 - 1);
                    if b >= a {
                        self.span(y, a, b, p);
                    }
                }
            }
        }
    }

    /// Copy another canvas into this one at (x, y).
    pub fn blit(&mut self, o: &Canvas, x: i32, y: i32) {
        for yy in 0..o.h {
            for xx in 0..o.w {
                self.set(x + xx, y + yy, o.px[(yy * o.w + xx) as usize]);
            }
        }
    }

    fn rgba(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.px.len() * 4);
        for &c in &self.px {
            let [r, g, b] = EGA[c as usize & 15];
            out.extend_from_slice(&[r, g, b, 255]);
        }
        out
    }

    /// Draw the canvas scaled to fill the given screen rectangle.
    pub fn present(&self, x: f32, y: f32, w: f32, h: f32) {
        thread_local! {
            static TEX: RefCell<Option<Texture2D>> = const { RefCell::new(None) };
        }
        TEX.with(|t| {
            let mut t = t.borrow_mut();
            let fits = t.as_ref().is_some_and(|tex| tex.width() as i32 == self.w && tex.height() as i32 == self.h);
            let bytes = self.rgba();
            if fits {
                t.as_ref().unwrap().update_from_bytes(self.w as u32, self.h as u32, &bytes);
            } else {
                let tex = Texture2D::from_rgba8(self.w as u16, self.h as u16, &bytes);
                tex.set_filter(FilterMode::Nearest);
                *t = Some(tex);
            }
            draw_texture_ex(t.as_ref().unwrap(), x, y, WHITE, DrawTextureParams { dest_size: Some(vec2(w, h)), ..Default::default() });
        });
    }
}
