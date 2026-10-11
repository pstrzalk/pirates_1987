//! Procedurally synthesized sound: PC-speaker style square waves and noise,
//! rendered into in-memory WAV files at startup.

use macroquad::audio::{PlaySoundParams, Sound, load_sound_from_bytes, play_sound, stop_sound};
use std::cell::RefCell;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sfx {
    Cannon,
    Splash,
    Hit,
    Clang,
    Slash,
    Musket,
    Fanfare,
    Defeat,
    Theme,
}

const ALL: [Sfx; 9] = [Sfx::Cannon, Sfx::Splash, Sfx::Hit, Sfx::Clang, Sfx::Slash, Sfx::Musket, Sfx::Fanfare, Sfx::Defeat, Sfx::Theme];

thread_local! {
    static SOUNDS: RefCell<Vec<(Sfx, Sound)>> = const { RefCell::new(Vec::new()) };
}

const RATE: u32 = 22050;

/// Tiny deterministic noise generator.
struct Noise(u32);
impl Noise {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

fn wav(samples: &[f32]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut v = Vec::with_capacity(44 + data_len as usize);
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&(36 + data_len).to_le_bytes());
    v.extend_from_slice(b"WAVEfmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes()); // PCM
    v.extend_from_slice(&1u16.to_le_bytes()); // mono
    v.extend_from_slice(&RATE.to_le_bytes());
    v.extend_from_slice(&(RATE * 2).to_le_bytes());
    v.extend_from_slice(&2u16.to_le_bytes());
    v.extend_from_slice(&16u16.to_le_bytes());
    v.extend_from_slice(b"data");
    v.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        let i = (s.clamp(-1.0, 1.0) * 32000.0) as i16;
        v.extend_from_slice(&i.to_le_bytes());
    }
    v
}

fn square(freq: f32, t: f32) -> f32 {
    if (t * freq).fract() < 0.5 { 1.0 } else { -1.0 }
}

/// Square-wave melody: (frequency Hz or 0 for rest, beats).
fn melody(notes: &[(f32, f32)], beat: f32, vol: f32) -> Vec<f32> {
    let mut out = vec![];
    for &(f, beats) in notes {
        let n = (beats * beat * RATE as f32) as usize;
        for i in 0..n {
            let t = i as f32 / RATE as f32;
            let env = if i + 300 > n { ((n - i) as f32 / 300.0).max(0.0) } else { 1.0 };
            out.push(if f > 0.0 { square(f, t) * vol * env } else { 0.0 });
        }
    }
    out
}

fn note(name: &str) -> f32 {
    let (base, oct) = name.split_at(name.len() - 1);
    let oct: i32 = oct.parse().unwrap_or(4);
    let semis = match base {
        "C" => -9,
        "C#" => -8,
        "D" => -7,
        "D#" => -6,
        "E" => -5,
        "F" => -4,
        "F#" => -3,
        "G" => -2,
        "G#" => -1,
        "A" => 0,
        "A#" => 1,
        "B" => 2,
        _ => return 0.0,
    };
    440.0 * 2f32.powf((semis + (oct - 4) * 12) as f32 / 12.0)
}

fn tune(spec: &str) -> Vec<(f32, f32)> {
    spec.split_whitespace()
        .map(|tok| {
            let (n, d) = tok.split_once(':').unwrap_or((tok, "1"));
            (if n == "R" { 0.0 } else { note(n) }, d.parse().unwrap_or(1.0))
        })
        .collect()
}

fn synth(s: Sfx) -> Vec<f32> {
    let mut rng = Noise(0x1234_5678);
    let len = |secs: f32| (secs * RATE as f32) as usize;
    match s {
        Sfx::Cannon => (0..len(0.7))
            .map(|i| {
                let t = i as f32 / RATE as f32;
                let env = (-t * 6.0).exp();
                let boom = (t * 55.0 * std::f32::consts::TAU * (1.0 - t)).sin();
                (rng.next() * 0.7 + boom * 0.6) * env * 0.8
            })
            .collect(),
        Sfx::Splash => {
            let mut lp = 0.0;
            (0..len(0.5))
                .map(|i| {
                    let t = i as f32 / RATE as f32;
                    lp += (rng.next() - lp) * 0.3;
                    lp * (-t * 5.0).exp() * 0.5
                })
                .collect()
        }
        Sfx::Hit => (0..len(0.35))
            .map(|i| {
                let t = i as f32 / RATE as f32;
                (rng.next() * 0.8 + square(90.0, t) * 0.4) * (-t * 10.0).exp() * 0.8
            })
            .collect(),
        Sfx::Clang => (0..len(0.3))
            .map(|i| {
                let t = i as f32 / RATE as f32;
                (square(1250.0, t) * 0.5 + square(1870.0, t) * 0.3) * (-t * 14.0).exp() * 0.35
            })
            .collect(),
        Sfx::Slash => (0..len(0.15))
            .map(|i| {
                let t = i as f32 / RATE as f32;
                rng.next() * (t / 0.15 * std::f32::consts::PI).sin() * 0.35
            })
            .collect(),
        Sfx::Musket => (0..len(0.25))
            .map(|i| {
                let t = i as f32 / RATE as f32;
                rng.next() * (-t * 22.0).exp() * 0.7
            })
            .collect(),
        Sfx::Fanfare => melody(&tune("G4:1 C5:1 E5:1 G5:2 E5:1 G5:4"), 0.11, 0.18),
        Sfx::Defeat => melody(&tune("E4:2 D#4:2 D4:2 C#4:6"), 0.16, 0.18),
        // "What Shall We Do with the Drunken Sailor" (traditional).
        Sfx::Theme => melody(
            &tune(
                "A4:2 A4 A4 A4:2 A4 A4 A4:2 D4:2 F4:2 A4:2 \
                 G4:2 G4 G4 G4:2 G4 G4 G4:2 C4:2 E4:2 G4:2 \
                 A4:2 A4 A4 A4:2 A4 A4 A4:2 B4:2 C#5:2 D5:2 \
                 C5:2 A4:2 G4:2 E4:2 D4:4 D4:4 \
                 A4:4 A4:2 A4:2 D4:2 F4:2 A4:4 \
                 G4:4 G4:2 G4:2 C4:2 E4:2 G4:4 \
                 A4:4 A4:2 A4:2 B4:2 C#5:2 D5:2 C5:2 \
                 A4:2 G4:2 E4:2 F#4:2 D4:4 R:4",
            ),
            0.14,
            0.12,
        ),
    }
}

fn enabled() -> bool {
    std::env::var("PIRATES_MUTE").is_err() && std::env::var("PIRATES_SCRIPT").is_err()
}

/// Synthesize and load every sound. Call once at startup.
pub async fn init() {
    if !enabled() {
        return;
    }
    for s in ALL {
        if let Ok(snd) = load_sound_from_bytes(&wav(&synth(s))).await {
            SOUNDS.with(|v| v.borrow_mut().push((s, snd)));
        }
    }
}

pub fn play(s: Sfx) {
    SOUNDS.with(|v| {
        if let Some((_, snd)) = v.borrow().iter().find(|(k, _)| *k == s) {
            play_sound(snd, PlaySoundParams { looped: false, volume: 0.6 });
        }
    });
}

pub fn stop(s: Sfx) {
    SOUNDS.with(|v| {
        if let Some((_, snd)) = v.borrow().iter().find(|(k, _)| *k == s) {
            stop_sound(snd);
        }
    });
}
