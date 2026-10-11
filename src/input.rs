//! Input abstraction. Real keyboard/mouse input is collected each frame, or a
//! script file (env `PIRATES_SCRIPT`) can drive the game for automated testing.

use macroquad::prelude::*;
use std::collections::HashSet;

#[derive(Default, Clone)]
pub struct Input {
    pub pressed: Vec<KeyCode>,
    pub down: HashSet<KeyCode>,
    pub chars: Vec<char>,
    /// Mouse position in logical (640x400) coordinates.
    pub mouse: Vec2,
    pub clicked: bool,
    pub mouse_moved: bool,
}

impl Input {
    pub fn pressed(&self, k: KeyCode) -> bool {
        self.pressed.contains(&k)
    }
    pub fn down(&self, k: KeyCode) -> bool {
        self.down.contains(&k)
    }
    pub fn confirm(&self) -> bool {
        self.pressed(KeyCode::Enter) || self.pressed(KeyCode::Space) || self.pressed(KeyCode::KpEnter)
    }
    pub fn cancel(&self) -> bool {
        self.pressed(KeyCode::Escape)
    }
    pub fn any_key(&self) -> bool {
        !self.pressed.is_empty() || self.clicked
    }
    /// Returns 0-based digit if a number key 1..9 was pressed.
    pub fn digit(&self) -> Option<usize> {
        let keys = [
            KeyCode::Key1,
            KeyCode::Key2,
            KeyCode::Key3,
            KeyCode::Key4,
            KeyCode::Key5,
            KeyCode::Key6,
            KeyCode::Key7,
            KeyCode::Key8,
            KeyCode::Key9,
        ];
        keys.iter().position(|k| self.pressed(*k))
    }
}

pub fn key_from_name(name: &str) -> Option<KeyCode> {
    use KeyCode::*;
    let k = match name.to_ascii_lowercase().as_str() {
        "enter" | "return" => Enter,
        "esc" | "escape" => Escape,
        "space" => Space,
        "left" => Left,
        "right" => Right,
        "up" => Up,
        "down" => Down,
        "tab" => Tab,
        "backspace" => Backspace,
        "a" => A,
        "b" => B,
        "c" => C,
        "d" => D,
        "e" => E,
        "f" => F,
        "g" => G,
        "h" => H,
        "i" => I,
        "j" => J,
        "k" => K,
        "l" => L,
        "m" => M,
        "n" => N,
        "o" => O,
        "p" => P,
        "q" => Q,
        "r" => R,
        "s" => S,
        "t" => T,
        "u" => U,
        "v" => V,
        "w" => W,
        "x" => X,
        "y" => Y,
        "z" => Z,
        "0" => Key0,
        "1" => Key1,
        "2" => Key2,
        "3" => Key3,
        "4" => Key4,
        "5" => Key5,
        "6" => Key6,
        "7" => Key7,
        "8" => Key8,
        "9" => Key9,
        _ => return None,
    };
    Some(k)
}

/// Collect live input for this frame.
pub fn gather(screen_to_logical: impl Fn(Vec2) -> Vec2, last_mouse: Vec2) -> Input {
    let mut inp = Input::default();
    inp.pressed = get_keys_pressed().into_iter().collect();
    inp.down = get_keys_down();
    while let Some(c) = get_char_pressed() {
        inp.chars.push(c);
    }
    let (mx, my) = mouse_position();
    inp.mouse = screen_to_logical(vec2(mx, my));
    inp.mouse_moved = inp.mouse.distance(last_mouse) > 0.5;
    inp.clicked = is_mouse_button_pressed(MouseButton::Left);
    inp
}

pub enum ScriptCmd {
    Wait(u32),
    Press(KeyCode),
    Hold(KeyCode, u32),
    Type(String),
    Shot(String),
    Quit,
}

pub struct Script {
    cmds: Vec<ScriptCmd>,
    idx: usize,
    wait: u32,
    held: Option<(KeyCode, u32)>,
    pub shot_request: Option<String>,
    pub quit: bool,
}

impl Script {
    pub fn load(path: &str) -> Option<Script> {
        let text = std::fs::read_to_string(path).ok()?;
        let mut cmds = vec![];
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (cmd, rest) = line.split_once(' ').unwrap_or((line, ""));
            let rest = rest.trim();
            match cmd {
                "wait" => cmds.push(ScriptCmd::Wait(rest.parse().unwrap_or(1))),
                "press" => {
                    for k in rest.split_whitespace() {
                        if let Some(k) = key_from_name(k) {
                            cmds.push(ScriptCmd::Press(k));
                            cmds.push(ScriptCmd::Wait(2));
                        }
                    }
                }
                "hold" => {
                    let mut it = rest.split_whitespace();
                    if let (Some(k), Some(n)) = (it.next().and_then(key_from_name), it.next()) {
                        cmds.push(ScriptCmd::Hold(k, n.parse().unwrap_or(1)));
                    }
                }
                "type" => cmds.push(ScriptCmd::Type(rest.to_string())),
                "shot" => cmds.push(ScriptCmd::Shot(rest.to_string())),
                "quit" => cmds.push(ScriptCmd::Quit),
                _ => eprintln!("script: unknown command {line}"),
            }
        }
        Some(Script { cmds, idx: 0, wait: 0, held: None, shot_request: None, quit: false })
    }

    /// Produce the scripted input for the next frame.
    pub fn next_frame(&mut self) -> Input {
        let mut inp = Input::default();
        if let Some((k, n)) = self.held.as_mut() {
            inp.down.insert(*k);
            *n -= 1;
            if *n == 0 {
                self.held = None;
            }
            return inp;
        }
        if self.wait > 0 {
            self.wait -= 1;
            return inp;
        }
        if self.idx < self.cmds.len() {
            let cmd = &self.cmds[self.idx];
            self.idx += 1;
            match cmd {
                ScriptCmd::Wait(n) => {
                    self.wait = *n;
                    return inp;
                }
                ScriptCmd::Press(k) => {
                    inp.pressed.push(*k);
                    inp.down.insert(*k);
                    return inp;
                }
                ScriptCmd::Hold(k, n) => {
                    inp.pressed.push(*k);
                    inp.down.insert(*k);
                    if *n > 1 {
                        self.held = Some((*k, n - 1));
                    }
                    return inp;
                }
                ScriptCmd::Type(s) => {
                    inp.chars = s.chars().collect();
                    return inp;
                }
                ScriptCmd::Shot(s) => {
                    self.shot_request = Some(s.clone());
                    return inp;
                }
                ScriptCmd::Quit => {
                    self.quit = true;
                    return inp;
                }
            }
        }
        self.quit = true;
        inp
    }
}
