//! Drawing helpers, EGA-style palette, text, panels and a menu widget.
//! All coordinates are logical (640x400).

use crate::input::Input;
use macroquad::prelude::*;
use std::cell::Cell;

pub const W: f32 = 640.0;
pub const H: f32 = 400.0;

// EGA palette
pub const BLACK: Color = Color::new(0.0, 0.0, 0.0, 1.0);
pub const BLUE: Color = Color::new(0.0, 0.0, 0.667, 1.0);
pub const GREEN: Color = Color::new(0.0, 0.667, 0.0, 1.0);
pub const CYAN: Color = Color::new(0.0, 0.667, 0.667, 1.0);
pub const RED: Color = Color::new(0.667, 0.0, 0.0, 1.0);
pub const MAGENTA: Color = Color::new(0.667, 0.0, 0.667, 1.0);
pub const BROWN: Color = Color::new(0.667, 0.333, 0.0, 1.0);
pub const LGRAY: Color = Color::new(0.667, 0.667, 0.667, 1.0);
pub const DGRAY: Color = Color::new(0.333, 0.333, 0.333, 1.0);
pub const LBLUE: Color = Color::new(0.333, 0.333, 1.0, 1.0);
pub const LGREEN: Color = Color::new(0.333, 1.0, 0.333, 1.0);
pub const LCYAN: Color = Color::new(0.333, 1.0, 1.0, 1.0);
pub const LRED: Color = Color::new(1.0, 0.333, 0.333, 1.0);
pub const LMAGENTA: Color = Color::new(1.0, 0.333, 1.0, 1.0);
pub const YELLOW: Color = Color::new(1.0, 1.0, 0.333, 1.0);
pub const WHITE: Color = Color::new(1.0, 1.0, 1.0, 1.0);
pub const PARCHMENT: Color = Color::new(0.93, 0.85, 0.64, 1.0);
pub const INK: Color = Color::new(0.25, 0.13, 0.03, 1.0);
pub const SEA: Color = Color::new(0.05, 0.15, 0.55, 1.0);
pub const SHALLOW: Color = Color::new(0.2, 0.4, 0.8, 1.0);

thread_local! {
    static SCALE: Cell<f32> = const { Cell::new(1.0) };
}

pub fn set_scale(s: f32) {
    SCALE.with(|c| c.set(s.max(0.25)));
}

fn scale() -> f32 {
    SCALE.with(|c| c.get())
}

fn params(size: f32, color: Color) -> TextParams<'static> {
    let k = scale();
    TextParams {
        font: None,
        font_size: (size * k).round().max(1.0) as u16,
        font_scale: 1.0 / k,
        font_scale_aspect: 1.0,
        rotation: 0.0,
        color,
    }
}

/// Width of text in logical units.
pub fn measure(s: &str, size: f32) -> f32 {
    let k = scale();
    let d = measure_text(s, None, (size * k).round().max(1.0) as u16, 1.0 / k);
    d.width
}

/// Draw text with (x, y) being the top-left corner.
pub fn text(s: &str, x: f32, y: f32, size: f32, color: Color) {
    draw_text_ex(s, x.round(), (y + size * 0.78).round(), params(size, color));
}

pub fn text_shadow(s: &str, x: f32, y: f32, size: f32, color: Color) {
    text(s, x + 1.0, y + 1.0, size, BLACK);
    text(s, x, y, size, color);
}

pub fn text_center(s: &str, cx: f32, y: f32, size: f32, color: Color) {
    let w = measure(s, size);
    text(s, cx - w / 2.0, y, size, color);
}

pub fn text_right(s: &str, rx: f32, y: f32, size: f32, color: Color) {
    let w = measure(s, size);
    text(s, rx - w, y, size, color);
}

/// Word-wrap text to a maximum width.
pub fn wrap(s: &str, size: f32, max_w: f32) -> Vec<String> {
    let mut out = vec![];
    for para in s.split('\n') {
        let mut line = String::new();
        for word in para.split_whitespace() {
            let cand = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
            if measure(&cand, size) > max_w && !line.is_empty() {
                out.push(std::mem::take(&mut line));
                line = word.to_string();
            } else {
                line = cand;
            }
        }
        out.push(line);
    }
    out
}

pub fn rect(x: f32, y: f32, w: f32, h: f32, c: Color) {
    draw_rectangle(x, y, w, h, c);
}

pub fn frame(x: f32, y: f32, w: f32, h: f32, c: Color, t: f32) {
    draw_rectangle_lines(x, y, w, h, t, c);
}

/// A bordered panel in the classic style.
pub fn panel(x: f32, y: f32, w: f32, h: f32, bg: Color, border: Color) {
    rect(x, y, w, h, bg);
    frame(x + 2.0, y + 2.0, w - 4.0, h - 4.0, border, 2.0);
    frame(x, y, w, h, BLACK, 1.0);
}

pub fn parchment(x: f32, y: f32, w: f32, h: f32) {
    rect(x, y, w, h, PARCHMENT);
    frame(x + 3.0, y + 3.0, w - 6.0, h - 6.0, BROWN, 2.0);
    frame(x, y, w, h, INK, 1.0);
}

pub fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

/// Generic vertical menu.
#[derive(Clone, Debug)]
pub struct Menu {
    pub items: Vec<String>,
    pub enabled: Vec<bool>,
    pub sel: usize,
    /// Layout cache from the last draw: (x, y, w, line height)
    layout: Cell<(f32, f32, f32, f32)>,
}

impl Menu {
    pub fn new(items: Vec<String>) -> Menu {
        let n = items.len();
        Menu { items, enabled: vec![true; n], sel: 0, layout: Cell::new((0.0, 0.0, 0.0, 0.0)) }
    }

    pub fn with_enabled(items: Vec<String>, enabled: Vec<bool>) -> Menu {
        let mut m = Menu::new(items);
        m.enabled = enabled;
        if !m.enabled.get(m.sel).copied().unwrap_or(true) {
            m.move_sel(1);
        }
        m
    }

    fn move_sel(&mut self, d: i32) {
        let n = self.items.len() as i32;
        if n == 0 {
            return;
        }
        let mut s = self.sel as i32;
        for _ in 0..n {
            s = (s + d).rem_euclid(n);
            if self.enabled[s as usize] {
                break;
            }
        }
        self.sel = s as usize;
    }

    /// Returns the chosen index when confirmed.
    pub fn update(&mut self, inp: &Input) -> Option<usize> {
        if self.items.is_empty() {
            return None;
        }
        if inp.pressed(KeyCode::Up) {
            self.move_sel(-1);
        }
        if inp.pressed(KeyCode::Down) {
            self.move_sel(1);
        }
        if let Some(d) = inp.digit() {
            if d < self.items.len() && self.enabled[d] {
                self.sel = d;
                return Some(d);
            }
        }
        let (x, y, w, lh) = self.layout.get();
        if lh > 0.0 {
            let m = inp.mouse;
            if m.x >= x && m.x <= x + w && m.y >= y {
                let i = ((m.y - y) / lh) as usize;
                if i < self.items.len() && self.enabled[i] {
                    if inp.mouse_moved {
                        self.sel = i;
                    }
                    if inp.clicked {
                        self.sel = i;
                        return Some(i);
                    }
                }
            }
        }
        if inp.confirm() && self.enabled[self.sel] {
            return Some(self.sel);
        }
        None
    }

    pub fn draw(&self, x: f32, y: f32, w: f32, size: f32, fg: Color, hi_bg: Color, hi_fg: Color) {
        let lh = size + 6.0;
        self.layout.set((x, y, w, lh));
        for (i, it) in self.items.iter().enumerate() {
            let yy = y + i as f32 * lh;
            let en = self.enabled[i];
            if i == self.sel {
                rect(x, yy, w, lh, hi_bg);
                text(it, x + 6.0, yy + 3.0, size, hi_fg);
            } else {
                text(it, x + 6.0, yy + 3.0, size, if en { fg } else { DGRAY });
            }
        }
    }

    pub fn height(&self, size: f32) -> f32 {
        self.items.len() as f32 * (size + 6.0)
    }
}

/// Simple single-line text entry.
#[derive(Clone, Debug, Default)]
pub struct TextEntry {
    pub value: String,
    pub max: usize,
}

impl TextEntry {
    pub fn new(initial: &str, max: usize) -> Self {
        TextEntry { value: initial.to_string(), max }
    }
    /// Returns true when Enter is pressed.
    pub fn update(&mut self, inp: &Input) -> bool {
        for &c in &inp.chars {
            if (c.is_alphanumeric() || c == ' ' || c == '\'' || c == '-') && self.value.chars().count() < self.max {
                self.value.push(c);
            }
        }
        if inp.pressed(KeyCode::Backspace) {
            self.value.pop();
        }
        inp.pressed(KeyCode::Enter) || inp.pressed(KeyCode::KpEnter)
    }
}

pub fn blink() -> bool {
    (get_time() * 2.0) as i64 % 2 == 0
}
