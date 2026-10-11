//! Naval battle minigame: top-down ship-to-ship (or ship-vs-fort) combat
//! with wind, broadsides and boarding.

use crate::data::{self, Nation, ShipKind};
use crate::input::Input;
use crate::ui;
use macroquad::prelude::*;
use macroquad::rand::gen_range;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

#[derive(Clone, Debug)]
pub enum NavalEnemy {
    Ship { kind: ShipKind, name: String, nation: Nation, guns: i32, crew: i32, hull: i32 },
    /// Attacking a town from the sea: the player must sail past the fort's
    /// guns and reach the town's quay. Reaching it yields `Boarded` (landing).
    Fort { town: String, nation: Nation, guns: i32, soldiers: i32 },
}

#[derive(Clone, Debug)]
pub struct NavalSetup {
    pub player_kind: ShipKind,
    pub player_name: String,
    pub player_guns: i32,
    pub player_crew: i32,
    /// Hull integrity 0..=100
    pub player_hull: i32,
    pub enemy: NavalEnemy,
    /// Direction the wind blows TOWARD, radians, screen space (0 = +x east, PI/2 = +y down/south).
    pub wind_dir: f32,
    /// Player has the gunnery special skill.
    pub gunnery: bool,
    /// 0 = Apprentice .. 3 = Swashbuckler
    pub difficulty: usize,
    /// Enemy prefers to run away (merchants) rather than fight.
    pub enemy_flees: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavalOutcomeKind {
    /// Ships touched: boarding (or landing at the town quay for forts).
    Boarded,
    EnemySunk,
    PlayerSunk,
    PlayerEscaped,
    EnemyEscaped,
}

#[derive(Clone, Debug)]
pub struct NavalOutcome {
    pub kind: NavalOutcomeKind,
    pub player_guns: i32,
    pub player_crew: i32,
    pub player_hull: i32,
    pub enemy_guns: i32,
    pub enemy_crew: i32,
    pub enemy_hull: i32,
}

// ---------------------------------------------------------------------------
// Tuning

/// Pixels per second per unit of ShipSpec::speed.
const SPEED_SCALE: f32 = 42.0;
const BALL_SPEED: f32 = 220.0;
const GUN_RANGE: f32 = 250.0;
const FORT_RANGE: f32 = 280.0;
const ESCAPE_DIST: f32 = 450.0;
const ESCAPE_TIME: f32 = 3.0;
const VIEW_CX: f32 = 320.0;
const VIEW_CY: f32 = 183.0;
const BAR_Y: f32 = 366.0;

// Fort map geometry (world coordinates).
const FORT_POS: Vec2 = Vec2::new(-100.0, -232.0);
const FORT_RADIUS: f32 = 22.0;
const QUAY_POS: Vec2 = Vec2::new(0.0, -320.0);
/// Land rectangles: (x, y, w, h)
const LAND: [(f32, f32, f32, f32); 3] = [
    (-3000.0, -3000.0, 2940.0, 2800.0), // west headland, coast at y=-200, bay edge x=-60
    (60.0, -3000.0, 2940.0, 2800.0),    // east shore
    (-60.0, -3000.0, 120.0, 2670.0),    // bay head, coast at y=-330
];

fn wrap(a: f32) -> f32 {
    let mut a = a % TAU;
    if a > PI {
        a -= TAU;
    }
    if a < -PI {
        a += TAU;
    }
    a
}

fn dir(a: f32) -> Vec2 {
    vec2(a.cos(), a.sin())
}

fn rot(v: Vec2, a: f32) -> Vec2 {
    let (s, c) = a.sin_cos();
    vec2(v.x * c - v.y * s, v.x * s + v.y * c)
}

fn in_land(p: Vec2, margin: f32) -> bool {
    LAND.iter().any(|&(x, y, w, h)| p.x > x - margin && p.x < x + w + margin && p.y > y - margin && p.y < y + h + margin)
}

fn diff_factor(d: usize) -> f32 {
    [1.45, 1.2, 1.0, 0.85][d.min(3)]
}

struct Ship {
    kind: ShipKind,
    name: String,
    nation: Nation,
    pos: Vec2,
    heading: f32,
    speed: f32,
    full_sail: bool,
    guns: i32,
    crew: i32,
    hull: f32,
    reload: f32,
    sinking: f32,
    wake_t: f32,
}

impl Ship {
    fn radius(&self) -> f32 {
        18.0 * self.kind.spec().size
    }

    fn length(&self) -> f32 {
        42.0 * self.kind.spec().size
    }

    fn max_speed(&self, wind: f32, navigation_bonus: f32) -> f32 {
        let spec = self.kind.spec();
        let wf = data::wind_factor(spec.square, wrap(self.heading - wind));
        let sail = if self.full_sail { 1.0 } else { 0.6 };
        spec.speed * SPEED_SCALE * wf * sail * (0.5 + 0.5 * (self.hull / 100.0).clamp(0.0, 1.0)) * navigation_bonus
    }

    fn turn_rate(&self) -> f32 {
        self.kind.spec().turn * 1.15 * if self.full_sail { 1.0 } else { 1.45 }
    }

    fn reload_time(&self, gunnery: bool, diff: f32) -> f32 {
        let mpg = if self.guns > 0 { self.crew as f32 / self.guns as f32 } else { 4.0 };
        let crew_factor = if mpg < 4.0 { (4.0 / mpg.max(0.5)).min(3.0) } else { 1.0 };
        let skill = if gunnery { 0.7 } else { 1.0 };
        3.6 * crew_factor * skill * diff
    }

    fn velocity(&self) -> Vec2 {
        dir(self.heading) * self.speed
    }

    fn steer_toward(&mut self, desired: f32, dt: f32) {
        let d = wrap(desired - self.heading);
        let r = self.turn_rate() * dt;
        self.heading = wrap(self.heading + d.clamp(-r, r));
    }
}

struct Ball {
    pos: Vec2,
    vel: Vec2,
    life: f32,
    /// 0 = player, 1 = enemy/fort
    owner: u8,
    weight: f32,
}

#[derive(Clone, Copy, PartialEq)]
enum PKind {
    Smoke,
    Splash,
    Flash,
    Wake,
}

struct Particle {
    pos: Vec2,
    vel: Vec2,
    life: f32,
    max: f32,
    kind: PKind,
}

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Intro,
    Fight,
    Sinking,
    End,
}

struct FortState {
    town: String,
    nation: Nation,
    guns: i32,
    soldiers: i32,
    reload: f32,
}

pub struct NavalBattle {
    pub setup: NavalSetup,
    player: Ship,
    enemy: Option<Ship>,
    fort: Option<FortState>,
    balls: Vec<Ball>,
    parts: Vec<Particle>,
    phase: Phase,
    timer: f32,
    time: f32,
    escape_t: f32,
    cam: Vec2,
    intro_msg: String,
    end_msg: String,
    result: Option<NavalOutcomeKind>,
    event_msg: String,
    event_t: f32,
    ai_think: f32,
    ai_heading: f32,
}

impl NavalBattle {
    pub fn new(setup: NavalSetup) -> Self {
        let mut player = Ship {
            kind: setup.player_kind,
            name: setup.player_name.clone(),
            nation: Nation::England,
            pos: Vec2::ZERO,
            heading: -FRAC_PI_2,
            speed: 0.0,
            full_sail: true,
            guns: setup.player_guns.max(0),
            crew: setup.player_crew.max(0),
            hull: setup.player_hull.clamp(1, 100) as f32,
            reload: 0.0,
            sinking: 0.0,
            wake_t: 0.0,
        };
        let mut enemy = None;
        let mut fort = None;
        let intro_msg;
        match &setup.enemy {
            NavalEnemy::Ship { kind, name, nation, guns, crew, hull } => {
                let a = gen_range(0.0, TAU);
                let pos = dir(a) * 270.0;
                // Enemy sails roughly across the player's bow.
                let heading = wrap(a + PI + if gen_range(0, 2) == 0 { 1.2 } else { -1.2 });
                player.heading = wrap(a + gen_range(-0.6, 0.6));
                enemy = Some(Ship {
                    kind: *kind,
                    name: name.clone(),
                    nation: *nation,
                    pos,
                    heading,
                    speed: 0.0,
                    full_sail: true,
                    guns: (*guns).max(0),
                    crew: (*crew).max(0),
                    hull: (*hull).clamp(1, 100) as f32,
                    reload: 1.5,
                    sinking: 0.0,
                    wake_t: 0.0,
                });
                intro_msg = format!("A {} {}! Clear for action!", nation.adj(), kind.name());
            }
            NavalEnemy::Fort { town, nation, guns, soldiers } => {
                player.pos = vec2(170.0, 60.0);
                player.heading = -FRAC_PI_2 - 0.4;
                fort = Some(FortState { town: town.clone(), nation: *nation, guns: (*guns).max(0), soldiers: (*soldiers).max(0), reload: 1.0 });
                intro_msg = format!("The guns of {town} open fire! Make for the town quay!");
            }
        }
        let mut setup = setup;
        if fort.is_some() {
            // Harbor breezes: never let the wind blow straight out of the harbor,
            // so the quay can always be reached by tacking.
            let d = wrap(-FRAC_PI_2 - setup.wind_dir);
            if d.abs() > 1.9 {
                setup.wind_dir = wrap(-FRAC_PI_2 - d.signum() * 1.9);
            }
        }
        let cam = player.pos;
        let ai_heading = enemy.as_ref().map(|e| e.heading).unwrap_or(0.0);
        NavalBattle {
            setup,
            player,
            enemy,
            fort,
            balls: vec![],
            parts: vec![],
            phase: Phase::Intro,
            timer: 2.5,
            time: 0.0,
            escape_t: 0.0,
            cam,
            intro_msg,
            end_msg: String::new(),
            result: None,
            event_msg: String::new(),
            event_t: 0.0,
            ai_think: 0.0,
            ai_heading,
        }
    }

    /// A default setup used by `PIRATES_TEST=naval`.
    pub fn test_default() -> Self {
        // PIRATES_NAVAL=fort/flee/board/sink selects a test variant.
        let variant = std::env::var("PIRATES_NAVAL").unwrap_or_default();
        let enemy = match variant.as_str() {
            "fort" => NavalEnemy::Fort { town: "Havana".into(), nation: Nation::Spain, guns: 24, soldiers: 200 },
            "sink" => NavalEnemy::Ship { kind: ShipKind::Sloop, name: "Swallow".into(), nation: Nation::Holland, guns: 4, crew: 30, hull: 6 },
            "board" => NavalEnemy::Ship { kind: ShipKind::WarGalleon, name: "Concepcion".into(), nation: Nation::Spain, guns: 40, crew: 300, hull: 100 },
            _ => NavalEnemy::Ship { kind: ShipKind::Merchantman, name: "San Felipe".into(), nation: Nation::Spain, guns: 14, crew: 90, hull: 100 },
        };
        let mut b = Self::new(NavalSetup {
            player_kind: ShipKind::Barque,
            player_name: "Revenge".into(),
            player_guns: 12,
            player_crew: 80,
            player_hull: 100,
            enemy,
            wind_dir: 0.6,
            gunnery: false,
            difficulty: 1,
            enemy_flees: variant == "flee",
        });
        if variant == "sink" {
            // Start abeam so a single broadside can finish the target.
            let h = b.player.heading;
            if let Some(e) = b.enemy.as_mut() {
                e.pos = b.player.pos + dir(h + FRAC_PI_2) * 100.0;
                e.heading = h;
            }
        }
        b
    }

    fn outcome(&self, kind: NavalOutcomeKind) -> NavalOutcome {
        let (eg, ec, eh) = if let Some(e) = &self.enemy {
            (e.guns, e.crew, e.hull.max(0.0).round() as i32)
        } else if let Some(f) = &self.fort {
            (f.guns, f.soldiers, 100)
        } else {
            (0, 0, 0)
        };
        NavalOutcome {
            kind,
            player_guns: self.player.guns.max(0),
            player_crew: self.player.crew.max(0),
            player_hull: self.player.hull.max(0.0).round() as i32,
            enemy_guns: eg.max(0),
            enemy_crew: ec.max(0),
            enemy_hull: eh,
        }
    }

    fn finish(&mut self, kind: NavalOutcomeKind) {
        self.result = Some(kind);
        let enemy_desc = match &self.enemy {
            Some(e) => format!("the {} {}", e.nation.adj(), e.kind.name()),
            None => "the enemy".to_string(),
        };
        self.end_msg = match kind {
            NavalOutcomeKind::Boarded => {
                if self.fort.is_some() {
                    "Your men leap ashore at the quay! Storm the town!".to_string()
                } else {
                    format!("Grappling hooks away! Prepare to board {enemy_desc}!")
                }
            }
            NavalOutcomeKind::EnemySunk => format!("{} goes down beneath the waves!", capitalize(&enemy_desc)),
            NavalOutcomeKind::PlayerSunk => "Your ship is sinking! Abandon ship!".to_string(),
            NavalOutcomeKind::PlayerEscaped => "You have escaped!".to_string(),
            NavalOutcomeKind::EnemyEscaped => format!("{} has escaped!", capitalize(&enemy_desc)),
        };
        let sinking = matches!(kind, NavalOutcomeKind::EnemySunk | NavalOutcomeKind::PlayerSunk);
        if sinking {
            self.phase = Phase::Sinking;
            self.timer = 1.6;
        } else {
            self.phase = Phase::End;
            self.timer = 2.0;
        }
    }

    fn event(&mut self, s: String) {
        self.event_msg = s;
        self.event_t = 2.5;
    }

    pub fn update(&mut self, inp: &Input, dt: f32) -> Option<NavalOutcome> {
        self.time += dt;
        self.event_t -= dt;
        match self.phase {
            Phase::Intro => {
                self.timer -= dt;
                self.update_particles(dt);
                if self.timer <= 0.0 || (self.timer < 2.2 && inp.any_key()) {
                    self.phase = Phase::Fight;
                }
                self.update_camera(dt);
                return None;
            }
            Phase::Sinking => {
                self.timer -= dt;
                if let Some(k) = self.result {
                    if k == NavalOutcomeKind::PlayerSunk {
                        self.player.sinking += dt;
                    } else if let Some(e) = self.enemy.as_mut() {
                        e.sinking += dt;
                    }
                }
                self.update_balls(dt, false);
                self.update_particles(dt);
                if self.timer <= 0.0 {
                    self.phase = Phase::End;
                    self.timer = 2.0;
                }
                return None;
            }
            Phase::End => {
                self.timer -= dt;
                self.update_particles(dt);
                if self.timer <= 0.0 || (self.timer < 1.6 && inp.any_key()) {
                    return self.result.map(|k| self.outcome(k));
                }
                return None;
            }
            Phase::Fight => {}
        }

        // --- Player controls
        if inp.pressed(KeyCode::Up) {
            self.player.full_sail = true;
        }
        if inp.pressed(KeyCode::Down) {
            self.player.full_sail = false;
        }
        let rate = self.player.turn_rate() * dt;
        if inp.down(KeyCode::Left) {
            self.player.heading = wrap(self.player.heading - rate);
        }
        if inp.down(KeyCode::Right) {
            self.player.heading = wrap(self.player.heading + rate);
        }
        self.player.reload -= dt;
        if (inp.pressed(KeyCode::Space) || inp.pressed(KeyCode::Enter)) && self.player.reload <= 0.0 && self.player.guns > 0 {
            let target = self.enemy.as_ref().map(|e| e.pos).unwrap_or(FORT_POS);
            let rel = wrap((target - self.player.pos).to_angle() - self.player.heading);
            let side = if rel >= 0.0 { 1.0 } else { -1.0 };
            let spread = if self.setup.gunnery { 0.09 } else { 0.16 };
            fire_broadside(&self.player, side, spread, 0, &mut self.balls, &mut self.parts);
            self.player.reload = self.player.reload_time(self.setup.gunnery, 1.0);
        }

        // --- Move player
        let nav = 1.0;
        move_ship(&mut self.player, self.setup.wind_dir, nav, dt, self.fort.is_some());

        // --- Enemy AI / fort
        if self.enemy.is_some() {
            self.enemy_ai(dt);
        }
        if self.fort.is_some() {
            self.fort_ai(dt);
        }

        // Wakes
        spawn_wake(&mut self.player, dt, &mut self.parts);
        if let Some(e) = self.enemy.as_mut() {
            spawn_wake(e, dt, &mut self.parts);
        }

        self.update_balls(dt, true);
        self.update_particles(dt);
        self.update_camera(dt);

        if self.result.is_some() {
            return None;
        }

        // --- Check end conditions
        if self.player.hull <= 0.0 {
            self.finish(NavalOutcomeKind::PlayerSunk);
            return None;
        }
        if let Some(e) = &self.enemy {
            if e.hull <= 0.0 {
                self.finish(NavalOutcomeKind::EnemySunk);
                return None;
            }
            let d = e.pos.distance(self.player.pos);
            if d < e.radius() + self.player.radius() {
                self.finish(NavalOutcomeKind::Boarded);
                return None;
            }
            if d > ESCAPE_DIST {
                self.escape_t += dt;
                if self.escape_t > ESCAPE_TIME {
                    self.finish(if self.setup.enemy_flees { NavalOutcomeKind::EnemyEscaped } else { NavalOutcomeKind::PlayerEscaped });
                }
            } else {
                self.escape_t = 0.0;
            }
        } else if self.fort.is_some() {
            let p = self.player.pos;
            if p.y < -300.0 && p.x.abs() < 52.0 {
                self.finish(NavalOutcomeKind::Boarded);
                return None;
            }
            if p.distance(FORT_POS) > 600.0 || p.y > 520.0 || p.x.abs() > 560.0 {
                self.escape_t += dt;
                if self.escape_t > ESCAPE_TIME {
                    self.finish(NavalOutcomeKind::PlayerEscaped);
                }
            } else {
                self.escape_t = 0.0;
            }
        }
        None
    }

    fn enemy_ai(&mut self, dt: f32) {
        let wind = self.setup.wind_dir;
        let diff = diff_factor(self.setup.difficulty);
        let ppos = self.player.pos;
        let pcrew = self.player.crew;
        let pvel = self.player.velocity();
        let flees = self.setup.enemy_flees;
        let e = self.enemy.as_mut().unwrap();
        let to_p = ppos - e.pos;
        let dist = to_p.length();
        let theta = to_p.to_angle();

        self.ai_think -= dt;
        if self.ai_think <= 0.0 {
            self.ai_think = 0.4;
            let spec = e.kind.spec();
            let score_heading = |h: f32, cur: f32| -> f32 {
                data::wind_factor(spec.square, wrap(h - wind)) - 0.25 * wrap(h - cur).abs() / PI
            };
            let wants_board = !flees && e.crew as f32 > pcrew as f32 * 1.5 && e.crew > 40;
            if flees {
                // Pick the heading that best combines speed and distance gained.
                let mut best = (f32::MIN, e.heading);
                for i in 0..24 {
                    let h = i as f32 / 24.0 * TAU;
                    let away = dir(h).dot(-to_p.normalize_or_zero());
                    let s = data::wind_factor(spec.square, wrap(h - wind)) * (away + 0.35) - 0.1 * wrap(h - e.heading).abs() / PI;
                    if s > best.0 {
                        best = (s, h);
                    }
                }
                self.ai_heading = best.1;
                e.full_sail = true;
            } else if wants_board {
                // Intercept.
                let lead = ppos + pvel * (dist / 50.0).min(3.0);
                let h = (lead - e.pos).to_angle();
                self.ai_heading = if data::wind_factor(spec.square, wrap(h - wind)) < 0.3 {
                    // Tack: pick a reachable heading close to the desired one.
                    let a = wrap(h + 0.8);
                    let b = wrap(h - 0.8);
                    if score_heading(a, e.heading) > score_heading(b, e.heading) { a } else { b }
                } else {
                    h
                };
                e.full_sail = true;
            } else {
                let mut h1 = theta + FRAC_PI_2;
                let mut h2 = theta - FRAC_PI_2;
                if dist > 210.0 {
                    h1 -= 0.7;
                    h2 += 0.7;
                } else if dist < 110.0 {
                    h1 += 0.5;
                    h2 -= 0.5;
                }
                let (h1, h2) = (wrap(h1), wrap(h2));
                self.ai_heading = if score_heading(h1, e.heading) >= score_heading(h2, e.heading) { h1 } else { h2 };
                e.full_sail = dist > 180.0;
            }
        }
        let desired = self.ai_heading;
        e.steer_toward(desired, dt);
        move_ship(e, wind, 1.0, dt, false);

        e.reload -= dt;
        let rel = wrap(theta - e.heading);
        if e.reload <= 0.0 && e.guns > 0 && dist < GUN_RANGE * 0.95 && (rel.abs() - FRAC_PI_2).abs() < 0.4 {
            let side = if rel >= 0.0 { 1.0 } else { -1.0 };
            let spread = 0.15 * diff;
            fire_broadside(e, side, spread, 1, &mut self.balls, &mut self.parts);
            e.reload = e.reload_time(false, diff);
        }
    }

    fn fort_ai(&mut self, dt: f32) {
        let diff = diff_factor(self.setup.difficulty);
        let f = self.fort.as_mut().unwrap();
        f.reload -= dt;
        let to_p = self.player.pos - FORT_POS;
        let dist = to_p.length();
        if f.reload <= 0.0 && f.guns > 0 && dist < FORT_RANGE {
            let aim = self.player.pos + self.player.velocity() * (dist / BALL_SPEED);
            let base = (aim - FORT_POS).to_angle();
            let dist_aim = aim.distance(FORT_POS);
            let n = (f.guns / 3 + 1).min(8);
            let weight = (f.guns as f32 / 2.0 / n as f32).max(0.5);
            for _ in 0..n {
                let a = base + gen_range(-0.11, 0.11) * diff;
                let origin = FORT_POS + vec2(gen_range(-10.0, 10.0), gen_range(-10.0, 10.0));
                let life = (dist_aim / BALL_SPEED * gen_range(0.88, 1.15)).min(FORT_RANGE * 1.1 / BALL_SPEED);
                self.balls.push(Ball { pos: origin, vel: dir(a) * BALL_SPEED, life, owner: 1, weight });
                self.parts.push(Particle { pos: origin, vel: dir(a) * 15.0, life: 1.0, max: 1.0, kind: PKind::Smoke });
            }
            f.reload = 3.2 * diff * gen_range(0.85, 1.15);
        }
    }

    fn update_balls(&mut self, dt: f32, allow_hits: bool) {
        let mut events: Vec<String> = vec![];
        let mut i = 0;
        while i < self.balls.len() {
            let b = &mut self.balls[i];
            b.pos += b.vel * dt;
            b.life -= dt;
            let mut remove = false;
            if allow_hits {
                if b.owner == 0 {
                    if let Some(e) = self.enemy.as_mut() {
                        if b.pos.distance(e.pos) < e.radius() * 0.95 && e.hull > 0.0 {
                            let (k, g) = apply_hit(e, b.weight);
                            if k > 0 || g {
                                events.push(hit_text("Enemy", k, g));
                            }
                            self.parts.push(Particle { pos: b.pos, vel: Vec2::ZERO, life: 0.35, max: 0.35, kind: PKind::Flash });
                            remove = true;
                        }
                    } else if let Some(f) = self.fort.as_mut() {
                        if b.pos.distance(FORT_POS) < FORT_RADIUS {
                            let mut g = false;
                            if gen_range(0.0, 1.0) < 0.3 && f.guns > 0 {
                                f.guns -= 1;
                                g = true;
                            }
                            let k = gen_range(0, 3);
                            f.soldiers = (f.soldiers - k).max(0);
                            if g {
                                events.push(format!("A fort gun is silenced! ({} left)", f.guns));
                            }
                            self.parts.push(Particle { pos: b.pos, vel: Vec2::ZERO, life: 0.35, max: 0.35, kind: PKind::Flash });
                            remove = true;
                        }
                    }
                } else if b.pos.distance(self.player.pos) < self.player.radius() * 0.95 && self.player.hull > 0.0 {
                    let (k, g) = apply_hit(&mut self.player, b.weight);
                    if k > 0 || g {
                        events.push(hit_text("We are hit", k, g));
                    }
                    self.parts.push(Particle { pos: b.pos, vel: Vec2::ZERO, life: 0.35, max: 0.35, kind: PKind::Flash });
                    remove = true;
                }
            }
            let b = &self.balls[i];
            if !remove && b.life <= 0.0 {
                if !(self.fort.is_some() && in_land(b.pos, 0.0)) {
                    self.parts.push(Particle { pos: b.pos, vel: Vec2::ZERO, life: 0.6, max: 0.6, kind: PKind::Splash });
                }
                remove = true;
            }
            if remove {
                self.balls.swap_remove(i);
            } else {
                i += 1;
            }
        }
        if let Some(s) = events.pop() {
            self.event(s);
        }
    }

    fn update_particles(&mut self, dt: f32) {
        for p in &mut self.parts {
            p.pos += p.vel * dt;
            p.vel *= 1.0 - 1.5 * dt;
            p.life -= dt;
        }
        self.parts.retain(|p| p.life > 0.0);
    }

    fn update_camera(&mut self, dt: f32) {
        let target = match &self.enemy {
            Some(e) => {
                let d = e.pos - self.player.pos;
                self.player.pos + (d * 0.5).clamp_length_max(140.0)
            }
            None => self.player.pos + ((QUAY_POS - self.player.pos) * 0.5).clamp_length_max(150.0),
        };
        let k = (4.0 * dt).min(1.0);
        self.cam = self.cam.lerp(target, k);
        if self.phase == Phase::Intro && self.time < 0.05 {
            self.cam = target;
        }
    }

    fn to_screen(&self, p: Vec2) -> Vec2 {
        p - self.cam + vec2(VIEW_CX, VIEW_CY)
    }

    pub fn draw(&self) {
        // Sea
        ui::rect(0.0, 0.0, ui::W, BAR_Y, ui::SEA);
        let cell = 32.0;
        let x0 = ((self.cam.x - VIEW_CX) / cell).floor() as i32 - 1;
        let y0 = ((self.cam.y - VIEW_CY) / cell).floor() as i32 - 1;
        for gy in y0..y0 + (BAR_Y / cell) as i32 + 3 {
            for gx in x0..x0 + (ui::W / cell) as i32 + 3 {
                let h = hash2(gx, gy);
                if h < 0.55 {
                    continue;
                }
                let ph = (self.time * 0.8 + h * 10.0).sin();
                let wp = vec2(gx as f32 * cell + h * 23.0 % cell, gy as f32 * cell + (h * 71.0) % cell);
                let s = self.to_screen(wp);
                if s.y > BAR_Y - 2.0 {
                    continue;
                }
                let w = 3.0 + ph * 1.5;
                draw_line(s.x - w, s.y, s.x + w, s.y, 1.0, Color::new(0.25, 0.4, 0.85, 1.0));
            }
        }

        if self.fort.is_some() {
            self.draw_coast();
        }

        // Wakes under ships
        for p in self.parts.iter().filter(|p| p.kind == PKind::Wake) {
            let s = self.to_screen(p.pos);
            let t = p.life / p.max;
            draw_circle(s.x, s.y, 1.0 + (1.0 - t) * 2.0, Color::new(0.7, 0.8, 1.0, 0.6 * t));
        }

        if let Some(e) = &self.enemy {
            self.draw_ship(e, false);
        }
        self.draw_ship(&self.player, true);

        for b in &self.balls {
            let s = self.to_screen(b.pos);
            draw_circle(s.x, s.y, 1.6, ui::BLACK);
        }
        for p in self.parts.iter().filter(|p| p.kind != PKind::Wake) {
            let s = self.to_screen(p.pos);
            let t = p.life / p.max;
            match p.kind {
                PKind::Smoke => draw_circle(s.x, s.y, 3.0 + (1.0 - t) * 6.0, Color::new(0.85, 0.85, 0.85, 0.55 * t)),
                PKind::Splash => {
                    draw_circle_lines(s.x, s.y, 2.0 + (1.0 - t) * 6.0, 1.0, Color::new(1.0, 1.0, 1.0, t));
                    draw_circle(s.x, s.y, 1.5 * t + 0.5, Color::new(1.0, 1.0, 1.0, t));
                }
                PKind::Flash => {
                    draw_circle(s.x, s.y, 7.0 * t + 2.0, Color::new(1.0, 0.8, 0.2, t));
                    draw_circle(s.x, s.y, 3.5 * t + 1.0, Color::new(1.0, 1.0, 0.8, t));
                }
                PKind::Wake => {}
            }
        }

        // Off-screen enemy indicator
        if let Some(e) = &self.enemy {
            let s = self.to_screen(e.pos);
            if s.x < 0.0 || s.x > ui::W || s.y < 0.0 || s.y > BAR_Y {
                let c = vec2(VIEW_CX, VIEW_CY);
                let d = (s - c).normalize_or_zero();
                let tx = if d.x.abs() > 1e-3 { (VIEW_CX - 14.0) / d.x.abs() } else { f32::MAX };
                let ty = if d.y.abs() > 1e-3 { (VIEW_CY - 14.0) / d.y.abs() } else { f32::MAX };
                let p = c + d * tx.min(ty);
                let a = d.to_angle();
                draw_triangle(p + dir(a) * 8.0, p + dir(a + 2.4) * 7.0, p + dir(a - 2.4) * 7.0, e.nation.color());
                let dist = e.pos.distance(self.player.pos) as i32;
                ui::text_center(&format!("{dist}"), p.x - d.x * 16.0, p.y - d.y * 16.0 - 4.0, 8.0, ui::WHITE);
            }
        }

        self.draw_wind();
        self.draw_status();

        if self.event_t > 0.0 && self.phase == Phase::Fight {
            ui::text_shadow(&self.event_msg, 8.0, 6.0, 10.0, ui::YELLOW);
        }
        if self.phase == Phase::Fight && self.escape_t > 0.0 {
            let who = if self.enemy.is_some() && self.setup.enemy_flees { "The enemy is getting away..." } else { "Escaping..." };
            ui::text_center(who, VIEW_CX, 330.0, 12.0, ui::LCYAN);
        } else if self.phase == Phase::Fight {
            if let Some(e) = &self.enemy {
                if e.pos.distance(self.player.pos) > 340.0 && !self.setup.enemy_flees {
                    ui::text_center("Keep sailing away to escape", VIEW_CX, 330.0, 10.0, ui::LGRAY);
                }
            }
        }

        match self.phase {
            Phase::Intro => self.draw_banner(&self.intro_msg),
            Phase::End => self.draw_banner(&self.end_msg),
            _ => {}
        }
    }

    fn draw_banner(&self, msg: &str) {
        let lines = ui::wrap(msg, 14.0, 440.0);
        let h = lines.len() as f32 * 18.0 + 20.0;
        let y = 140.0 - h / 2.0;
        ui::panel(90.0, y, 460.0, h, ui::BLUE, ui::LCYAN);
        for (i, l) in lines.iter().enumerate() {
            ui::text_center(l, 320.0, y + 10.0 + i as f32 * 18.0, 14.0, ui::WHITE);
        }
    }

    fn draw_coast(&self) {
        let sand = Color::new(0.85, 0.75, 0.45, 1.0);
        let green = Color::new(0.1, 0.55, 0.12, 1.0);
        let off = self.to_screen(Vec2::ZERO);
        for &(x, y, w, h) in &LAND {
            ui::rect(off.x + x - 4.0, off.y + y - 4.0, w + 8.0, h + 8.0, sand);
        }
        for &(x, y, w, h) in &LAND {
            ui::rect(off.x + x, off.y + y, w, h, green);
        }
        // Jungle tufts
        for i in 0..60 {
            let h = hash2(i, 7);
            let wx = -560.0 + h * 1120.0;
            let wy = -250.0 - hash2(i, 13) * 300.0;
            let p = vec2(wx, wy);
            if in_land(p, -6.0) {
                let s = self.to_screen(p);
                draw_circle(s.x, s.y, 4.0, Color::new(0.05, 0.42, 0.08, 1.0));
            }
        }
        // Town houses at the bay head
        for i in 0..7 {
            let x = -52.0 + i as f32 * 15.0;
            let y = -352.0 - (i % 2) as f32 * 12.0;
            let s = self.to_screen(vec2(x, y));
            ui::rect(s.x, s.y, 11.0, 8.0, ui::LGRAY);
            draw_triangle(vec2(s.x - 1.0, s.y), vec2(s.x + 12.0, s.y), vec2(s.x + 5.5, s.y - 5.0), ui::RED);
        }
        // Church
        let c = self.to_screen(vec2(28.0, -385.0));
        ui::rect(c.x, c.y, 10.0, 16.0, ui::WHITE);
        ui::rect(c.x + 4.0, c.y - 6.0, 2.0, 6.0, ui::BROWN);
        ui::rect(c.x + 2.0, c.y - 4.0, 6.0, 2.0, ui::BROWN);
        // Quay
        let q = self.to_screen(vec2(-40.0, -334.0));
        ui::rect(q.x, q.y, 80.0, 7.0, ui::BROWN);
        for i in 0..6 {
            ui::rect(q.x + 3.0 + i as f32 * 14.0, q.y + 6.0, 3.0, 4.0, Color::new(0.4, 0.2, 0.0, 1.0));
        }
        if ui::blink() && self.phase == Phase::Fight {
            ui::text_center("QUAY", q.x + 40.0, q.y + 12.0, 8.0, ui::YELLOW);
        }
        if let Some(f) = &self.fort {
            ui::text_center(&f.town, c.x - 30.0, c.y - 22.0, 10.0, ui::WHITE);
            // Fort: square with corner bastions
            let s = self.to_screen(FORT_POS);
            let grey = Color::new(0.55, 0.55, 0.5, 1.0);
            let dark = Color::new(0.35, 0.35, 0.32, 1.0);
            ui::rect(s.x - 15.0, s.y - 15.0, 30.0, 30.0, dark);
            ui::rect(s.x - 13.0, s.y - 13.0, 26.0, 26.0, grey);
            for (dx, dy) in [(-15.0, -15.0), (15.0, -15.0), (-15.0, 15.0), (15.0, 15.0)] {
                draw_poly(s.x + dx, s.y + dy, 4, 6.0, 45.0, dark);
                draw_poly(s.x + dx, s.y + dy, 4, 4.5, 45.0, grey);
            }
            ui::rect(s.x - 6.0, s.y - 6.0, 12.0, 12.0, dark);
            ui::rect(s.x - 0.5, s.y - 24.0, 1.5, 18.0, ui::BLACK);
            data::draw_flag(f.nation, s.x + 1.0, s.y - 24.0, 12.0, 8.0);
            if self.phase == Phase::Fight && self.player.pos.distance(FORT_POS) < FORT_RANGE + 60.0 {
                draw_circle_lines(s.x, s.y, FORT_RANGE, 1.0, Color::new(1.0, 0.3, 0.3, 0.25));
            }
        }
    }

    fn draw_ship(&self, ship: &Ship, is_player: bool) {
        let s = self.to_screen(ship.pos);
        let sink = (ship.sinking / 1.5).clamp(0.0, 1.0);
        if sink >= 1.0 {
            return;
        }
        let size = ship.kind.spec().size * (1.0 - 0.35 * sink);
        let alpha = 1.0 - sink;
        let l = 42.0 * size;
        let w = 14.0 * size;
        let h = ship.heading + sink * 0.6;
        let pts = [vec2(l * 0.5, 0.0), vec2(l * 0.22, w * 0.5), vec2(-l * 0.5, w * 0.42), vec2(-l * 0.5, -w * 0.42), vec2(l * 0.22, -w * 0.5)];
        let pts: Vec<Vec2> = pts.iter().map(|p| s + rot(*p, h)).collect();
        let hull_col = Color::new(0.45 * (1.0 - 0.5 * sink), 0.25 * (1.0 - 0.5 * sink), 0.08, alpha);
        let deck_col = Color::new(0.7, 0.5, 0.25, alpha);
        let center = s + rot(vec2(-l * 0.1, 0.0), h);
        for i in 0..pts.len() {
            draw_triangle(center, pts[i], pts[(i + 1) % pts.len()], hull_col);
        }
        // Deck
        let deck: Vec<Vec2> = [vec2(l * 0.38, 0.0), vec2(l * 0.18, w * 0.32), vec2(-l * 0.44, w * 0.28), vec2(-l * 0.44, -w * 0.28), vec2(l * 0.18, -w * 0.32)]
            .iter()
            .map(|p| s + rot(*p, h))
            .collect();
        for i in 0..deck.len() {
            draw_triangle(center, deck[i], deck[(i + 1) % deck.len()], deck_col);
        }
        // Sails: perpendicular bars, trimmed toward the wind.
        let masts = ship.kind.spec().masts as usize;
        let square = ship.kind.spec().square;
        let trim = {
            let rel = wrap(self.setup.wind_dir - h);
            rel.clamp(-1.0, 1.0) * 0.5
        };
        let sail_col = Color::new(0.95, 0.95, 0.9, alpha);
        let sail_len = if ship.full_sail { w * 1.6 } else { w * 1.1 };
        for m in 0..masts {
            let fx = if masts == 1 { 0.05 } else { 0.3 - 0.6 * m as f32 / (masts - 1) as f32 };
            let mp = s + rot(vec2(l * fx, 0.0), h);
            if square || m == 0 && masts > 1 {
                let a = h + FRAC_PI_2 + trim;
                let d = dir(a) * sail_len * 0.5;
                draw_line(mp.x - d.x, mp.y - d.y, mp.x + d.x, mp.y + d.y, 3.0 * size.max(0.8), sail_col);
            } else {
                // Fore-and-aft sail: a triangle swung to leeward.
                let boom = h + PI + trim * 1.6;
                let tip = mp + dir(boom) * l * 0.4;
                draw_triangle(mp, tip, mp + dir(boom + 0.35 * trim.signum()) * l * 0.25, sail_col);
                draw_line(mp.x, mp.y, tip.x, tip.y, 1.5, sail_col);
            }
            draw_circle(mp.x, mp.y, 1.4, Color::new(0.3, 0.15, 0.0, alpha));
        }
        // Pennant at the main mast
        let fx = if masts == 1 { 0.05 } else { 0.3 - 0.6 * (masts / 2) as f32 / (masts - 1).max(1) as f32 };
        let mp = s + rot(vec2(l * fx, 0.0), h);
        let pc = if is_player { ui::WHITE } else { ship.nation.color() };
        let pc = Color::new(pc.r, pc.g, pc.b, alpha);
        let flutter = (self.time * 9.0).sin() * 0.15;
        let pa = self.setup.wind_dir + flutter;
        draw_triangle(mp, mp + dir(pa + 0.25) * 9.0 * size + dir(pa) * 2.0, mp + dir(pa - 0.25) * 9.0 * size, pc);
        if is_player {
            draw_triangle(mp, mp + dir(pa + 0.12) * 6.0 * size, mp + dir(pa - 0.12) * 6.0 * size, ui::LBLUE);
        }
    }

    fn draw_wind(&self) {
        let c = vec2(604.0, 34.0);
        draw_circle(c.x, c.y, 22.0, Color::new(0.0, 0.0, 0.3, 0.7));
        draw_circle_lines(c.x, c.y, 22.0, 1.0, ui::LCYAN);
        let d = dir(self.setup.wind_dir);
        let tail = c - d * 15.0;
        let head = c + d * 15.0;
        draw_line(tail.x, tail.y, head.x, head.y, 2.0, ui::WHITE);
        draw_triangle(head + d * 3.0, head + rot(d, 2.5) * 7.0, head + rot(d, -2.5) * 7.0, ui::WHITE);
        ui::text_center("Wind", c.x, c.y + 24.0, 9.0, ui::LCYAN);
    }

    fn draw_status(&self) {
        ui::panel(0.0, BAR_Y, ui::W, ui::H - BAR_Y, ui::BLACK, ui::BROWN);
        let p = &self.player;
        ui::text(&format!("{} ({})", p.name, p.kind.name()), 8.0, BAR_Y + 5.0, 10.0, ui::LCYAN);
        ui::text(&format!("Crew {}  Guns {}  Hull {}%", p.crew, p.guns, p.hull.max(0.0).round() as i32), 8.0, BAR_Y + 19.0, 10.0, ui::WHITE);
        let (rs, rc) = if p.guns == 0 {
            ("No guns", ui::DGRAY)
        } else if p.reload <= 0.0 {
            ("Guns ready", ui::LGREEN)
        } else {
            ("Reloading", ui::LRED)
        };
        ui::text(rs, 230.0, BAR_Y + 5.0, 10.0, rc);
        ui::text(if p.full_sail { "Full sail" } else { "Battle sail" }, 230.0, BAR_Y + 19.0, 10.0, ui::LGRAY);
        if let Some(e) = &self.enemy {
            ui::text_right(&format!("{} ({}) {}", e.name, e.kind.name(), e.nation.adj()), 632.0, BAR_Y + 5.0, 10.0, e.nation.color());
            ui::text_right(&format!("Crew {}  Guns {}  Hull {}%", e.crew, e.guns, e.hull.max(0.0).round() as i32), 632.0, BAR_Y + 19.0, 10.0, ui::WHITE);
        } else if let Some(f) = &self.fort {
            ui::text_right(&format!("Fort of {} ({})", f.town, f.nation.adj()), 632.0, BAR_Y + 5.0, 10.0, f.nation.color());
            ui::text_right(&format!("Guns {}  Soldiers {}", f.guns, f.soldiers), 632.0, BAR_Y + 19.0, 10.0, ui::WHITE);
        }
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn hash2(x: i32, y: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374761393) ^ (y as u32).wrapping_mul(668265263);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^= h >> 16;
    (h & 0xffff) as f32 / 65535.0
}

fn hit_text(who: &str, killed: i32, gun: bool) -> String {
    match (killed > 0, gun) {
        (true, true) => format!("{who}! {killed} men killed, a gun dismounted."),
        (true, false) => format!("{who}! {killed} men killed."),
        (false, true) => format!("{who}! A gun dismounted."),
        _ => format!("{who}!"),
    }
}

/// Apply one cannonball hit. Returns (crew killed, gun dismounted).
fn apply_hit(s: &mut Ship, weight: f32) -> (i32, bool) {
    let size = s.kind.spec().size;
    s.hull -= 3.6 * weight / size * gen_range(0.6, 1.4);
    let mut killed = 0;
    if gen_range(0.0, 1.0) < 0.45 {
        killed = ((gen_range(1, 4) as f32) * weight).round().max(1.0) as i32;
        killed = killed.min(s.crew);
        s.crew -= killed;
    }
    let mut gun = false;
    if gen_range(0.0, 1.0) < 0.12 * weight.min(2.0) && s.guns > 0 {
        s.guns -= 1;
        gun = true;
    }
    (killed, gun)
}

fn fire_broadside(s: &Ship, side: f32, spread: f32, owner: u8, balls: &mut Vec<Ball>, parts: &mut Vec<Particle>) {
    let guns_side = (s.guns as f32 / 2.0).ceil().max(1.0);
    let n = guns_side.min(10.0) as i32;
    let weight = guns_side / n as f32;
    let base = s.heading + side * FRAC_PI_2;
    let l = s.length();
    let fwd = dir(s.heading);
    for _ in 0..n {
        let along = gen_range(-l * 0.35, l * 0.35);
        let origin = s.pos + fwd * along + dir(base) * s.radius() * 0.4;
        let a = base + gen_range(-spread, spread);
        let life = GUN_RANGE / BALL_SPEED * gen_range(0.8, 1.08);
        balls.push(Ball { pos: origin, vel: dir(a) * BALL_SPEED + s.velocity() * 0.5, life, owner, weight });
        parts.push(Particle { pos: origin + dir(base) * 4.0, vel: dir(base) * 18.0, life: 1.1, max: 1.1, kind: PKind::Smoke });
    }
}

fn move_ship(s: &mut Ship, wind: f32, nav: f32, dt: f32, coast: bool) {
    let target = s.max_speed(wind, nav);
    // Accelerate gently toward the target speed.
    let k = (0.8 * dt).min(1.0);
    s.speed += (target - s.speed) * k;
    let next = s.pos + dir(s.heading) * s.speed * dt;
    if coast && in_land(next, s.radius() * 0.6) {
        s.speed *= 0.3;
        // Allow sliding out of the coast if we're already stuck.
        if in_land(s.pos, s.radius() * 0.6) {
            s.pos += dir(s.heading) * 10.0 * dt;
        }
    } else {
        s.pos = next;
    }
}

fn spawn_wake(s: &mut Ship, dt: f32, parts: &mut Vec<Particle>) {
    s.wake_t -= dt;
    if s.wake_t <= 0.0 && s.speed > 5.0 {
        s.wake_t = 0.09;
        let stern = s.pos - dir(s.heading) * s.length() * 0.5;
        parts.push(Particle { pos: stern, vel: Vec2::ZERO, life: 1.3, max: 1.3, kind: PKind::Wake });
    }
}
