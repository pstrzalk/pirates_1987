//! Land battle minigame: the player's landing party marches on a town
//! through jungle and open ground against the garrison.

use crate::data::{self, Nation};
use crate::input::Input;
use crate::ui;
use macroquad::prelude::*;
use macroquad::rand::gen_range;

#[derive(Clone, Debug)]
pub struct LandSetup {
    pub town: String,
    pub nation: Nation,
    pub player_men: i32,
    pub defenders: i32,
    /// 0 = no fort, 1 = fort, 2 = fortress
    pub fort_level: u8,
    /// 0 = Apprentice .. 3 = Swashbuckler
    pub difficulty: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LandOutcomeKind {
    /// The party reached and took the town.
    Victory,
    /// The party was destroyed or broken.
    Defeat,
    /// The player retreated back to the ships.
    Retreat,
}

#[derive(Clone, Debug)]
pub struct LandOutcome {
    pub kind: LandOutcomeKind,
    pub men_left: i32,
    pub defenders_left: i32,
}

const TS: f32 = 20.0;
const GW: usize = 32;
const GH: usize = 18;
const FIELD_H: f32 = TS * GH as f32;
const FIRE_RANGE: f32 = 5.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Terrain {
    Open,
    Beach,
    Forest,
    Hill,
    River,
    Ford,
    Wall,
    Gate,
    Town,
}

impl Terrain {
    fn passable(self) -> bool {
        !matches!(self, Terrain::River | Terrain::Wall | Terrain::Town)
    }
    /// Multiplier on incoming musket fire.
    fn cover(self) -> f32 {
        match self {
            Terrain::Forest => 0.5,
            Terrain::Hill => 0.7,
            _ => 1.0,
        }
    }
    fn move_mult(self) -> f32 {
        match self {
            Terrain::Forest => 2.0,
            Terrain::Hill => 1.5,
            Terrain::Ford => 1.6,
            _ => 1.0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Buccaneers,
    Pirates,
    Musketeers,
    Infantry,
    Cavalry,
    Militia,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Kind::Buccaneers => "Buccaneers",
            Kind::Pirates => "Pirates",
            Kind::Musketeers => "Musketeers",
            Kind::Infantry => "Infantry",
            Kind::Cavalry => "Cavalry",
            Kind::Militia => "Militia",
        }
    }
    fn fire(self) -> f32 {
        match self {
            Kind::Buccaneers => 1.4,
            Kind::Pirates => 0.7,
            Kind::Musketeers => 1.0,
            Kind::Infantry => 1.0,
            Kind::Cavalry => 0.3,
            Kind::Militia => 0.6,
        }
    }
    fn melee(self) -> f32 {
        match self {
            Kind::Buccaneers => 0.8,
            Kind::Pirates => 1.4,
            Kind::Musketeers => 1.0,
            Kind::Infantry => 1.0,
            Kind::Cavalry => 1.35,
            Kind::Militia => 0.6,
        }
    }
    /// Seconds per tile on open ground.
    fn move_time(self) -> f32 {
        match self {
            Kind::Cavalry => 0.2,
            Kind::Militia => 0.55,
            Kind::Infantry => 0.45,
            _ => 0.3,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Side {
    Player,
    Enemy,
}

#[derive(Clone, Debug)]
struct Group {
    side: Side,
    kind: Kind,
    men: i32,
    initial: i32,
    x: i32,
    y: i32,
    /// Smoothed draw position (tile units).
    fx: f32,
    fy: f32,
    home: (i32, i32),
    move_cd: f32,
    reload: f32,
    routed: bool,
    alive: bool,
    flash: f32,
}

impl Group {
    fn new(side: Side, kind: Kind, men: i32, x: i32, y: i32) -> Group {
        Group {
            side,
            kind,
            men,
            initial: men,
            x,
            y,
            fx: x as f32,
            fy: y as f32,
            home: (x, y),
            move_cd: 0.0,
            reload: gen_range(0.5, 1.5),
            routed: false,
            alive: true,
            flash: 0.0,
        }
    }
    fn active(&self) -> bool {
        self.alive && !self.routed
    }
}

enum Particle {
    Smoke { x: f32, y: f32, t: f32, r: f32 },
    Number { x: f32, y: f32, t: f32, text: String, color: Color },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Intro,
    Playing,
    ConfirmRetreat,
    Ending,
}

/// Small deterministic RNG for terrain generation.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }
    fn f(&mut self) -> f32 {
        self.next() as f32 / (u32::MAX >> 1) as f32
    }
    fn range(&mut self, lo: i32, hi: i32) -> i32 {
        lo + (self.next() % ((hi - lo).max(1) as u32)) as i32
    }
}

pub struct LandBattle {
    pub setup: LandSetup,
    terrain: Vec<Terrain>,
    groups: Vec<Group>,
    selected: usize,
    gate_y: i32,
    phase: Phase,
    timer: f32,
    particles: Vec<Particle>,
    outcome: Option<LandOutcome>,
    message: String,
    alarm: bool,
    time: f32,
}

fn dist(a: (i32, i32), b: (i32, i32)) -> f32 {
    (((a.0 - b.0).pow(2) + (a.1 - b.1).pow(2)) as f32).sqrt()
}

fn stochastic_round(v: f32) -> i32 {
    let f = v.max(0.0);
    let base = f.floor();
    base as i32 + if gen_range(0.0, 1.0) < f - base { 1 } else { 0 }
}

impl LandBattle {
    pub fn new(setup: LandSetup) -> Self {
        let mut seed: u64 = 0xcbf29ce484222325;
        for b in setup.town.bytes() {
            seed ^= b as u64;
            seed = seed.wrapping_mul(0x100000001b3);
        }
        let mut rng = Lcg(seed);
        let gate_y = rng.range(6, 11);
        let terrain = Self::gen_terrain(&mut rng, gate_y);

        let mut groups = vec![];
        let pm = setup.player_men.max(3);
        let b = (pm as f32 * 0.35).round() as i32;
        let p = (pm as f32 * 0.35).round() as i32;
        let m = (pm - b - p).max(1);
        groups.push(Group::new(Side::Player, Kind::Buccaneers, b.max(1), 1, 4));
        groups.push(Group::new(Side::Player, Kind::Pirates, p.max(1), 1, 9));
        groups.push(Group::new(Side::Player, Kind::Musketeers, m, 1, 13));

        let bonus = [1.0, 1.15, 1.3][setup.fort_level.min(2) as usize];
        let total = ((setup.defenders.max(1) as f32) * bonus).round() as i32;
        let has_cav = setup.fort_level >= 1 || total >= 60;
        let mut parts: Vec<(Kind, f32, (i32, i32))> = vec![];
        if has_cav {
            parts.push((Kind::Cavalry, 0.25, (21, (gate_y - 4).max(1))));
        }
        if total >= 40 {
            parts.push((Kind::Infantry, 0.3, (25, gate_y - 2)));
            parts.push((Kind::Infantry, 0.25, (25, gate_y + 3)));
        } else {
            parts.push((Kind::Infantry, 0.55, (25, gate_y - 1)));
        }
        parts.push((Kind::Militia, 0.2, (26, gate_y)));
        let wsum: f32 = parts.iter().map(|p| p.1).sum();
        let mut assigned = 0;
        let n = parts.len();
        for (i, (kind, w, pos)) in parts.into_iter().enumerate() {
            let men = if i == n - 1 { total - assigned } else { (total as f32 * w / wsum).round() as i32 };
            assigned += men;
            if men >= 3 || (i == n - 1 && men > 0) {
                groups.push(Group::new(Side::Enemy, kind, men, pos.0, pos.1.clamp(1, GH as i32 - 2)));
            } else if men > 0 {
                // fold tiny groups into the militia
                assigned -= men;
            }
        }
        let defenders = total;
        let message = format!("Your men land near {}. {} soldiers defend the town.", setup.town, defenders);
        LandBattle {
            setup,
            terrain,
            groups,
            selected: 0,
            gate_y,
            phase: Phase::Intro,
            timer: 4.0,
            particles: vec![],
            outcome: None,
            message,
            alarm: false,
            time: 0.0,
        }
    }

    fn gen_terrain(rng: &mut Lcg, gate_y: i32) -> Vec<Terrain> {
        let mut t = vec![Terrain::Open; GW * GH];
        let set = |t: &mut Vec<Terrain>, x: i32, y: i32, v: Terrain| {
            if x >= 0 && y >= 0 && (x as usize) < GW && (y as usize) < GH {
                t[y as usize * GW + x as usize] = v;
            }
        };
        // forests
        for _ in 0..rng.range(7, 12) {
            let cx = rng.range(3, 25) as f32;
            let cy = rng.range(0, GH as i32) as f32;
            let r = 1.2 + rng.f() * 2.0;
            for y in 0..GH as i32 {
                for x in 2..26 {
                    let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
                    if d < r && rng.f() < 0.85 {
                        set(&mut t, x, y, Terrain::Forest);
                    }
                }
            }
        }
        // hills
        for _ in 0..rng.range(2, 5) {
            let cx = rng.range(4, 24) as f32;
            let cy = rng.range(1, GH as i32 - 1) as f32;
            let r = 0.8 + rng.f() * 1.3;
            for y in 0..GH as i32 {
                for x in 3..25 {
                    let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
                    if d < r {
                        set(&mut t, x, y, Terrain::Hill);
                    }
                }
            }
        }
        // river with fords
        if rng.f() < 0.7 {
            let mut rx = rng.range(11, 18);
            let mut fords = vec![rng.range(1, 6), rng.range(7, 11), rng.range(12, GH as i32 - 1)];
            if rng.f() < 0.4 {
                fords.remove(rng.range(0, 3) as usize);
            }
            for y in 0..GH as i32 {
                let v = if fords.contains(&y) { Terrain::Ford } else { Terrain::River };
                set(&mut t, rx, y, v);
                let r = rng.f();
                if r < 0.25 && rx > 10 {
                    rx -= 1;
                } else if r > 0.75 && rx < 19 {
                    rx += 1;
                }
            }
        }
        // beach
        for y in 0..GH as i32 {
            set(&mut t, 0, y, Terrain::Beach);
            set(&mut t, 1, y, Terrain::Beach);
        }
        // town: walls and houses
        let (top, bot) = (3, GH as i32 - 4);
        for y in top..=bot {
            for x in 27..GW as i32 {
                let edge = x == 27 || y == top || y == bot;
                set(&mut t, x, y, if edge { Terrain::Wall } else { Terrain::Town });
            }
        }
        set(&mut t, 27, gate_y, Terrain::Gate);
        set(&mut t, 27, gate_y + 1, Terrain::Gate);
        // clear approach to the gate and the open ground just outside walls
        for y in 0..GH as i32 {
            for x in 24..27 {
                let i = y as usize * GW + x as usize;
                if t[i] == Terrain::Forest || t[i] == Terrain::Hill {
                    t[i] = Terrain::Open;
                }
            }
        }
        // starting tiles open
        for &(x, y) in &[(1, 4), (1, 9), (1, 13), (2, 4), (2, 9), (2, 13)] {
            set(&mut t, x, y, if x <= 1 { Terrain::Beach } else { Terrain::Open });
        }
        t
    }

    /// A default setup used by `PIRATES_TEST=land`.
    pub fn test_default() -> Self {
        Self::new(LandSetup { town: "Maracaibo".into(), nation: Nation::Spain, player_men: 150, defenders: 120, fort_level: 1, difficulty: 1 })
    }

    fn ter(&self, x: i32, y: i32) -> Terrain {
        if x < 0 || y < 0 || x as usize >= GW || y as usize >= GH {
            return Terrain::Wall;
        }
        self.terrain[y as usize * GW + x as usize]
    }

    fn occupant(&self, x: i32, y: i32) -> Option<usize> {
        self.groups.iter().position(|g| g.alive && g.x == x && g.y == y)
    }

    fn total_men(&self, side: Side) -> i32 {
        self.groups
            .iter()
            .filter(|g| g.side == side && g.alive)
            .map(|g| if g.routed { g.men / 2 } else { g.men })
            .sum()
    }

    fn nearest_enemy(&self, gi: usize, active_only: bool) -> Option<(usize, f32)> {
        let g = &self.groups[gi];
        self.groups
            .iter()
            .enumerate()
            .filter(|(_, o)| o.side != g.side && o.alive && (!active_only || !o.routed))
            .map(|(i, o)| (i, dist((g.x, g.y), (o.x, o.y))))
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
    }

    fn enemy_acc(&self) -> f32 {
        [0.7, 0.9, 1.1, 1.3][self.setup.difficulty.min(3)]
    }

    fn tile_center(x: f32, y: f32) -> Vec2 {
        vec2(x * TS + TS / 2.0, y * TS + TS / 2.0)
    }

    fn apply_losses(&mut self, gi: usize, loss: i32, melee_lost: bool) {
        let g = &mut self.groups[gi];
        let loss = loss.min(g.men).max(0);
        g.men -= loss;
        g.flash = 0.3;
        let c = Self::tile_center(g.fx, g.fy);
        if loss > 0 {
            self.particles.push(Particle::Number {
                x: c.x,
                y: c.y - 8.0,
                t: 1.2,
                text: format!("-{loss}"),
                color: if g.side == Side::Player { ui::LRED } else { ui::YELLOW },
            });
        }
        if g.men <= 0 {
            g.alive = false;
        } else if !g.routed
            && ((g.men as f32) < g.initial as f32 * 0.2 || (melee_lost && (g.men as f32) < g.initial as f32 * 0.5 && gen_range(0.0, 1.0) < 0.25))
        {
            g.routed = true;
            g.move_cd = 0.3;
        }
        if g.side == Side::Enemy {
            self.alarm = true;
        }
    }

    fn volley(&mut self, si: usize, ti: usize) {
        let (s, t) = (&self.groups[si], &self.groups[ti]);
        let d = dist((s.x, s.y), (t.x, t.y));
        let range_f = (1.0 - d / (FIRE_RANGE * 1.6)).max(0.1);
        let cover = self.ter(t.x, t.y).cover();
        let height = if self.ter(s.x, s.y) == Terrain::Hill { 1.2 } else { 1.0 };
        let acc = if s.side == Side::Enemy { self.enemy_acc() } else { 1.0 };
        let kills = s.men as f32 * s.kind.fire() * 0.07 * acc * range_f * cover * height * gen_range(0.6, 1.4);
        let a = Self::tile_center(s.fx, s.fy);
        let b = Self::tile_center(t.fx, t.fy);
        for k in 0..3 {
            let p = a.lerp(b, 0.12 + k as f32 * 0.08);
            self.particles.push(Particle::Smoke { x: p.x + gen_range(-3.0, 3.0), y: p.y + gen_range(-3.0, 3.0), t: 0.8, r: 3.0 + k as f32 });
        }
        let loss = stochastic_round(kills);
        if loss == 0 {
            self.particles.push(Particle::Number { x: b.x, y: b.y - 8.0, t: 0.8, text: "miss".into(), color: ui::LGRAY });
        }
        self.apply_losses(ti, loss, false);
    }

    fn melee(&mut self, ai: usize, di: usize) {
        let ediff = [0.8, 0.9, 1.0, 1.15][self.setup.difficulty.min(3)];
        let side_mult = |g: &Group| if g.side == Side::Enemy { ediff } else { 1.0 };
        let (a, d) = (&self.groups[ai], &self.groups[di]);
        let dcover = match self.ter(d.x, d.y) {
            Terrain::Forest | Terrain::Hill | Terrain::Gate => 1.25,
            _ => 1.0,
        };
        let sa = a.men as f32 * a.kind.melee() * side_mult(a) * gen_range(0.7, 1.3);
        let sd = d.men as f32 * d.kind.melee() * side_mult(d) * gen_range(0.7, 1.3) * dcover;
        let total = (sa + sd).max(0.01);
        let mut a_loss = stochastic_round(d.men as f32 * 0.22 * sd / total * gen_range(0.8, 1.2));
        let mut d_loss = stochastic_round(a.men as f32 * 0.22 * sa / total * gen_range(0.8, 1.2));
        if sa >= sd {
            d_loss = d_loss.max(1);
        } else {
            a_loss = a_loss.max(1);
        }
        let mid = Self::tile_center((a.fx + d.fx) / 2.0, (a.fy + d.fy) / 2.0);
        self.particles.push(Particle::Smoke { x: mid.x, y: mid.y, t: 0.5, r: 6.0 });
        self.groups[ai].move_cd = 0.8;
        self.apply_losses(ai, a_loss, sa < sd);
        self.apply_losses(di, d_loss, sa >= sd);
    }

    /// Try to move group `gi` by (dx, dy). Bumping an enemy starts melee.
    fn try_move(&mut self, gi: usize, dx: i32, dy: i32) -> bool {
        let (nx, ny) = (self.groups[gi].x + dx, self.groups[gi].y + dy);
        if let Some(o) = self.occupant(nx, ny) {
            if self.groups[o].side != self.groups[gi].side {
                self.melee(gi, o);
                return true;
            }
            return false;
        }
        let t = self.ter(nx, ny);
        if !t.passable() {
            return false;
        }
        let g = &mut self.groups[gi];
        g.x = nx;
        g.y = ny;
        g.move_cd = g.kind.move_time() * t.move_mult();
        true
    }

    /// Dijkstra distance field toward `target` over passable tiles.
    fn distance_field(&self, target: (i32, i32), mover: usize) -> Vec<f32> {
        let mut d = vec![f32::MAX; GW * GH];
        let idx = |x: i32, y: i32| y as usize * GW + x as usize;
        let mut open = vec![(target.0, target.1)];
        d[idx(target.0, target.1)] = 0.0;
        while !open.is_empty() {
            // pick min (grid is tiny)
            let (bi, _) = open.iter().enumerate().min_by(|a, b| d[idx(a.1.0, a.1.1)].partial_cmp(&d[idx(b.1.0, b.1.1)]).unwrap()).unwrap();
            let (x, y) = open.swap_remove(bi);
            let cd = d[idx(x, y)];
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x + dx, y + dy);
                let t = self.ter(nx, ny);
                if !t.passable() {
                    continue;
                }
                let blocked = self.occupant(nx, ny).is_some_and(|o| o != mover && self.groups[o].side == self.groups[mover].side);
                let cost = t.move_mult() + if blocked { 6.0 } else { 0.0 };
                let nd = cd + cost;
                if nd < d[idx(nx, ny)] {
                    d[idx(nx, ny)] = nd;
                    open.push((nx, ny));
                }
            }
        }
        d
    }

    fn step_toward(&mut self, gi: usize, target: (i32, i32)) {
        let field = self.distance_field(target, gi);
        let (x, y) = (self.groups[gi].x, self.groups[gi].y);
        let mut best: Option<((i32, i32), f32)> = None;
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx as usize >= GW || ny as usize >= GH {
                continue;
            }
            let v = field[ny as usize * GW + nx as usize];
            if v < f32::MAX && best.is_none_or(|b| v < b.1) {
                best = Some(((dx, dy), v));
            }
        }
        let here = field[y as usize * GW + x as usize];
        if let Some(((dx, dy), v)) = best {
            if v < here && !self.try_move(gi, dx, dy) {
                self.groups[gi].move_cd = 0.4;
            }
        }
    }

    fn enemy_ai(&mut self, gi: usize) {
        let g = self.groups[gi].clone();
        if g.move_cd > 0.0 {
            return;
        }
        let Some((ti, d)) = self.nearest_enemy(gi, true) else { return };
        let target = (self.groups[ti].x, self.groups[ti].y);
        let diff = self.setup.difficulty.min(3);
        let slow = [1.3, 1.1, 1.0, 0.9][diff];
        if d <= 1.01 {
            self.melee(gi, ti);
            self.groups[gi].move_cd *= slow;
            return;
        }
        let aggro = [6.0, 8.0, 10.0, 12.0][diff] + if g.kind == Kind::Cavalry { 6.0 } else { 0.0 };
        let leash = dist(target, g.home);
        let near_gate = self.groups.iter().any(|p| p.side == Side::Player && p.active() && dist((p.x, p.y), (27, self.gate_y)) < 6.0);
        let advance = match g.kind {
            Kind::Militia => d <= 2.5,
            _ => (d <= aggro || self.alarm || near_gate) && leash < 18.0,
        };
        // Infantry prefers to stand and fire when the target is in range.
        let stand_fire = g.kind != Kind::Cavalry && d <= FIRE_RANGE - 1.0 && g.reload > 0.6 && !near_gate;
        if advance && !stand_fire {
            self.step_toward(gi, target);
        } else if !advance && (g.x, g.y) != g.home {
            self.step_toward(gi, g.home);
        } else {
            self.groups[gi].move_cd = 0.3;
            return;
        }
        if self.groups[gi].move_cd <= 0.0 {
            self.groups[gi].move_cd = 0.3;
        }
        self.groups[gi].move_cd *= slow;
    }

    fn rout_move(&mut self, gi: usize) {
        let g = &self.groups[gi];
        if g.move_cd > 0.0 {
            return;
        }
        let (x, y) = (g.x, g.y);
        let (dx, side) = if g.side == Side::Player { (-1, Side::Player) } else { (1, Side::Enemy) };
        if (side == Side::Player && x <= 0) || (side == Side::Enemy && (self.ter(x + 1, y) == Terrain::Gate || x >= GW as i32 - 1)) {
            // escaped the field
            self.groups[gi].alive = false;
            self.groups[gi].men /= 2;
            return;
        }
        let target = if side == Side::Player { (0, y) } else { (27, self.gate_y) };
        if target == (x, y) {
            self.groups[gi].alive = false;
            self.groups[gi].men /= 2;
            return;
        }
        let before = (x, y);
        if self.occupant(x + dx, y).is_none() && self.ter(x + dx, y).passable() && side == Side::Player {
            self.try_move(gi, dx, 0);
        } else {
            let field = self.distance_field(target, gi);
            let mut best = None;
            for (ddx, ddy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x + ddx, y + ddy);
                if nx < 0 || ny < 0 || nx as usize >= GW || ny as usize >= GH || self.occupant(nx, ny).is_some() {
                    continue;
                }
                let v = field[ny as usize * GW + nx as usize];
                if v < f32::MAX && best.is_none_or(|b: ((i32, i32), f32)| v < b.1) {
                    best = Some(((ddx, ddy), v));
                }
            }
            if let Some(((ddx, ddy), _)) = best {
                self.try_move(gi, ddx, ddy);
            }
        }
        let g = &mut self.groups[gi];
        if (g.x, g.y) == before {
            g.move_cd = 0.5;
        } else {
            g.move_cd *= 0.8;
        }
    }

    fn finish(&mut self, kind: LandOutcomeKind) {
        let men_left = match kind {
            LandOutcomeKind::Retreat => self.groups.iter().filter(|g| g.side == Side::Player && g.alive).map(|g| g.men).sum(),
            _ => self.total_men(Side::Player),
        };
        let defenders_left = self.total_men(Side::Enemy);
        self.message = match kind {
            LandOutcomeKind::Victory => format!("Victory! Your men storm into {}!", self.setup.town),
            LandOutcomeKind::Defeat => "Defeat! Your landing party has been broken.".to_string(),
            LandOutcomeKind::Retreat => "You sound the retreat and fall back to the ships.".to_string(),
        };
        self.outcome = Some(LandOutcome { kind, men_left, defenders_left });
        self.phase = Phase::Ending;
        self.timer = 3.0;
    }

    pub fn update(&mut self, inp: &Input, dt: f32) -> Option<LandOutcome> {
        self.time += dt;
        for p in &mut self.particles {
            match p {
                Particle::Smoke { t, r, y, .. } => {
                    *t -= dt;
                    *r += dt * 4.0;
                    *y -= dt * 4.0;
                }
                Particle::Number { t, y, .. } => {
                    *t -= dt;
                    *y -= dt * 12.0;
                }
            }
        }
        self.particles.retain(|p| match p {
            Particle::Smoke { t, .. } | Particle::Number { t, .. } => *t > 0.0,
        });
        for g in &mut self.groups {
            let k = (dt * 10.0).min(1.0);
            g.fx += (g.x as f32 - g.fx) * k;
            g.fy += (g.y as f32 - g.fy) * k;
            g.flash = (g.flash - dt).max(0.0);
        }

        match self.phase {
            Phase::Intro => {
                self.timer -= dt;
                if self.timer <= 0.0 || (inp.any_key() && self.timer < 3.7) {
                    self.phase = Phase::Playing;
                }
                None
            }
            Phase::ConfirmRetreat => {
                if inp.pressed(KeyCode::Y) {
                    self.finish(LandOutcomeKind::Retreat);
                } else if inp.pressed(KeyCode::N) || inp.cancel() {
                    self.phase = Phase::Playing;
                }
                None
            }
            Phase::Ending => {
                self.timer -= dt;
                if self.timer <= 0.0 || (inp.any_key() && self.timer < 2.3) {
                    return self.outcome.take();
                }
                None
            }
            Phase::Playing => {
                self.play(inp, dt);
                None
            }
        }
    }

    fn select_next(&mut self) {
        for k in 1..=3 {
            let i = (self.selected + k) % 3;
            if self.groups[i].active() {
                self.selected = i;
                return;
            }
        }
    }

    fn play(&mut self, inp: &Input, dt: f32) {
        if inp.cancel() {
            self.phase = Phase::ConfirmRetreat;
            return;
        }
        if inp.pressed(KeyCode::Tab) {
            self.select_next();
        }
        for (k, key) in [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3].iter().enumerate() {
            if inp.pressed(*key) && self.groups[k].active() {
                self.selected = k;
            }
        }
        if !self.groups[self.selected].active() {
            self.select_next();
        }

        for g in &mut self.groups {
            g.move_cd -= dt;
            g.reload -= dt;
        }

        // player movement
        let sel = self.selected;
        if self.groups[sel].active() && self.groups[sel].move_cd <= 0.0 {
            let dir = [(KeyCode::Left, (-1, 0)), (KeyCode::Right, (1, 0)), (KeyCode::Up, (0, -1)), (KeyCode::Down, (0, 1))]
                .iter()
                .find(|(k, _)| inp.pressed(*k) || inp.down(*k))
                .map(|(_, d)| *d);
            if let Some((dx, dy)) = dir {
                self.try_move(sel, dx, dy);
            }
        }
        // player manual fire
        if inp.pressed(KeyCode::Space) && self.groups[sel].active() && self.groups[sel].reload <= 0.0 {
            if let Some((ti, d)) = self.nearest_enemy(sel, false) {
                if d <= FIRE_RANGE {
                    self.volley(sel, ti);
                    self.groups[sel].reload = 2.0;
                }
            }
        }

        // automatic fire and AI
        for gi in 0..self.groups.len() {
            if !self.groups[gi].alive {
                continue;
            }
            if self.groups[gi].routed {
                self.rout_move(gi);
                continue;
            }
            let side = self.groups[gi].side;
            if side == Side::Enemy {
                self.enemy_ai(gi);
            }
            if !self.groups[gi].active() || self.groups[gi].reload > 0.0 {
                continue;
            }
            if side == Side::Player && gi == sel {
                continue;
            }
            if let Some((ti, d)) = self.nearest_enemy(gi, false) {
                if d <= FIRE_RANGE && d > 1.01 {
                    self.volley(gi, ti);
                    self.groups[gi].reload = if side == Side::Player { 3.5 } else { gen_range(3.0, 4.0) };
                }
            }
        }

        // victory / defeat
        let at_gate = self.groups.iter().any(|g| g.side == Side::Player && g.active() && self.ter(g.x, g.y) == Terrain::Gate);
        let enemies = self.groups.iter().any(|g| g.side == Side::Enemy && g.active());
        let players = self.groups.iter().any(|g| g.side == Side::Player && g.active());
        if !players {
            self.finish(LandOutcomeKind::Defeat);
        } else if at_gate || !enemies {
            self.finish(LandOutcomeKind::Victory);
        }
    }

    // ---------------------------------------------------------------- drawing

    fn draw_terrain(&self) {
        for y in 0..GH as i32 {
            for x in 0..GW as i32 {
                let (px, py) = (x as f32 * TS, y as f32 * TS);
                let t = self.ter(x, y);
                let h = ((x * 7919 + y * 104729) % 97) as f32 / 97.0;
                let grass = if (x + y) % 2 == 0 { Color::new(0.2, 0.6, 0.15, 1.0) } else { Color::new(0.22, 0.63, 0.17, 1.0) };
                match t {
                    Terrain::Open => {
                        ui::rect(px, py, TS, TS, grass);
                        if h > 0.6 {
                            ui::rect(px + h * 14.0, py + 4.0 + h * 8.0, 2.0, 2.0, Color::new(0.35, 0.75, 0.25, 1.0));
                        }
                    }
                    Terrain::Beach => {
                        ui::rect(px, py, TS, TS, Color::new(0.88, 0.8, 0.5, 1.0));
                        if x == 0 {
                            ui::rect(px, py, 6.0, TS, ui::SHALLOW);
                            ui::rect(px + 6.0, py + h * 12.0, 2.0, 4.0, ui::WHITE);
                        }
                    }
                    Terrain::Forest => {
                        ui::rect(px, py, TS, TS, Color::new(0.1, 0.42, 0.1, 1.0));
                        for (ox, oy) in [(5.0, 6.0), (14.0, 5.0), (9.0, 13.0), (16.0, 15.0)] {
                            draw_circle(px + ox, py + oy, 4.5, Color::new(0.02, 0.3, 0.05, 1.0));
                            draw_circle(px + ox - 1.0, py + oy - 1.0, 3.0, Color::new(0.1, 0.55, 0.12, 1.0));
                        }
                    }
                    Terrain::Hill => {
                        ui::rect(px, py, TS, TS, Color::new(0.5, 0.55, 0.2, 1.0));
                        draw_triangle(vec2(px + 2.0, py + 17.0), vec2(px + 10.0, py + 4.0), vec2(px + 18.0, py + 17.0), Color::new(0.55, 0.42, 0.22, 1.0));
                        draw_triangle(vec2(px + 7.0, py + 9.0), vec2(px + 10.0, py + 4.0), vec2(px + 13.0, py + 9.0), Color::new(0.7, 0.6, 0.4, 1.0));
                    }
                    Terrain::River => {
                        ui::rect(px, py, TS, TS, ui::SHALLOW);
                        let o = ((self.time * 2.0 + y as f32) % 4.0) * 4.0;
                        ui::rect(px + 4.0, py + o, 6.0, 1.0, Color::new(0.5, 0.7, 1.0, 1.0));
                    }
                    Terrain::Ford => {
                        ui::rect(px, py, TS, TS, Color::new(0.45, 0.62, 0.9, 1.0));
                        for (ox, oy) in [(4.0, 5.0), (12.0, 9.0), (6.0, 15.0), (15.0, 16.0)] {
                            draw_circle(px + ox, py + oy, 2.0, ui::LGRAY);
                        }
                    }
                    Terrain::Wall => {
                        ui::rect(px, py, TS, TS, ui::LGRAY);
                        for r in 0..3 {
                            let off = if r % 2 == 0 { 0.0 } else { 5.0 };
                            ui::rect(px, py + r as f32 * 7.0, TS, 1.0, ui::DGRAY);
                            ui::rect(px + off + 4.0, py + r as f32 * 7.0, 1.0, 7.0, ui::DGRAY);
                            ui::rect(px + off + 14.0, py + r as f32 * 7.0, 1.0, 7.0, ui::DGRAY);
                        }
                    }
                    Terrain::Gate => {
                        ui::rect(px, py, TS, TS, ui::LGRAY);
                        ui::rect(px + 3.0, py, TS - 6.0, TS, ui::BROWN);
                        for k in 0..3 {
                            ui::rect(px + 5.0 + k as f32 * 4.0, py, 1.0, TS, Color::new(0.4, 0.2, 0.0, 1.0));
                        }
                    }
                    Terrain::Town => {
                        ui::rect(px, py, TS, TS, Color::new(0.75, 0.68, 0.5, 1.0));
                        if (x + y) % 2 == 0 || h > 0.5 {
                            ui::rect(px + 3.0, py + 8.0, 14.0, 10.0, ui::WHITE);
                            draw_triangle(vec2(px + 1.0, py + 9.0), vec2(px + 10.0, py + 2.0), vec2(px + 19.0, py + 9.0), ui::RED);
                            ui::rect(px + 8.0, py + 12.0, 4.0, 6.0, ui::BROWN);
                        }
                    }
                }
            }
        }
        // church tower + flag inside town
        let (tx, ty) = (29.5 * TS, (self.gate_y as f32 + 0.5) * TS);
        ui::rect(tx - 5.0, ty - 14.0, 12.0, 26.0, ui::WHITE);
        ui::frame(tx - 5.0, ty - 14.0, 12.0, 26.0, ui::DGRAY, 1.0);
        ui::rect(tx + 10.0, ty - 34.0, 1.5, 34.0, ui::BLACK);
        data::draw_flag(self.setup.nation, tx + 11.5, ty - 34.0, 16.0, 10.0);
        if self.setup.fort_level >= 1 {
            // bastions at wall corners
            for by in [3.0, GH as f32 - 4.0] {
                draw_rectangle(27.0 * TS - 4.0, by * TS - 4.0, TS + 8.0, TS + 8.0, ui::DGRAY);
                ui::rect(27.0 * TS - 2.0, by * TS - 2.0, TS + 4.0, TS + 4.0, ui::LGRAY);
            }
        }
    }

    fn draw_group(&self, gi: usize) {
        let g = &self.groups[gi];
        if !g.alive {
            return;
        }
        let (px, py) = (g.fx * TS, g.fy * TS);
        let body = match g.side {
            Side::Player => ui::LCYAN,
            Side::Enemy => {
                let c = self.setup.nation.color();
                if self.setup.nation == Nation::Pirate { ui::BLACK } else { c }
            }
        };
        let body = if g.flash > 0.0 && (g.flash * 20.0) as i32 % 2 == 0 { ui::WHITE } else { body };
        let n = (g.men / 10 + 1).clamp(1, 6);
        let offs = [(5.0, 6.0), (11.0, 6.0), (17.0, 6.0), (5.0, 14.0), (11.0, 14.0), (17.0, 14.0)];
        let skin = Color::new(0.95, 0.75, 0.55, 1.0);
        let bob = if g.routed { (self.time * 12.0).sin() * 1.5 } else { 0.0 };
        for &(ox, oy) in offs.iter().take(n as usize) {
            let (fx, fy) = (px + ox - 2.0, py + oy - 3.0 + bob);
            if g.kind == Kind::Cavalry {
                ui::rect(fx - 2.0, fy + 3.0, 7.0, 3.0, ui::BROWN);
                ui::rect(fx - 2.0, fy + 6.0, 1.0, 2.0, ui::BROWN);
                ui::rect(fx + 4.0, fy + 6.0, 1.0, 2.0, ui::BROWN);
                ui::rect(fx, fy, 3.0, 4.0, body);
                draw_circle(fx + 1.5, fy - 1.0, 1.4, skin);
            } else {
                ui::rect(fx, fy, 3.0, 5.0, body);
                ui::rect(fx, fy + 5.0, 1.0, 2.0, ui::BLACK);
                ui::rect(fx + 2.0, fy + 5.0, 1.0, 2.0, ui::BLACK);
                draw_circle(fx + 1.5, fy - 1.2, 1.4, skin);
                if g.kind != Kind::Pirates {
                    draw_line(fx + 3.0, fy + 3.0, fx + 5.0, fy - 2.0, 1.0, ui::BLACK);
                } else {
                    draw_line(fx + 3.0, fy + 3.0, fx + 5.0, fy, 1.0, ui::LGRAY);
                }
            }
        }
        // strength banner
        let label = format!("{}", g.men);
        let w = ui::measure(&label, 8.0) + 4.0;
        let bg = if g.side == Side::Player { ui::BLUE } else { ui::RED };
        ui::rect(px + TS / 2.0 - w / 2.0, py - 6.0, w, 9.0, if g.routed { ui::DGRAY } else { bg });
        ui::text_center(&label, px + TS / 2.0, py - 5.5, 8.0, ui::WHITE);
        if g.side == Side::Player && gi == self.selected && self.phase == Phase::Playing && ui::blink() {
            ui::frame(px - 1.0, py - 1.0, TS + 2.0, TS + 2.0, ui::YELLOW, 2.0);
        }
    }

    fn draw_status(&self) {
        ui::rect(0.0, FIELD_H, ui::W, ui::H - FIELD_H, ui::BLACK);
        ui::rect(0.0, FIELD_H, ui::W, 1.0, ui::BROWN);
        for i in 0..3 {
            let g = &self.groups[i];
            let x = 4.0 + i as f32 * 150.0;
            let y = FIELD_H + 4.0;
            if i == self.selected && g.active() {
                ui::rect(x - 2.0, y - 1.0, 146.0, 13.0, ui::BLUE);
            }
            let state = if !g.alive {
                "lost".to_string()
            } else if g.routed {
                "routed".to_string()
            } else {
                format!("{}", g.men)
            };
            let col = if g.active() { ui::WHITE } else { ui::DGRAY };
            ui::text(&format!("{} {} {}", i + 1, g.kind.name(), state), x, y, 10.0, col);
            if g.active() {
                let ready = (1.0 - g.reload.max(0.0) / 3.5).clamp(0.0, 1.0);
                ui::rect(x + 120.0, y + 3.0, 20.0, 4.0, ui::DGRAY);
                ui::rect(x + 120.0, y + 3.0, 20.0 * ready, 4.0, if ready >= 1.0 { ui::LGREEN } else { ui::YELLOW });
            }
        }
        let foes = self.total_men(Side::Enemy);
        ui::text_right(&format!("{} {}: {}", self.setup.nation.adj(), "soldiers", foes), ui::W - 4.0, FIELD_H + 4.0, 10.0, ui::LRED);
        ui::text(
            "Tab/1-3 select group   Arrows move   Space fire   Reach the gate!   Esc retreat",
            4.0,
            FIELD_H + 22.0,
            10.0,
            ui::LGRAY,
        );
    }

    fn banner(&self, lines: &[String]) {
        let w = 420.0;
        let h = 30.0 + lines.len() as f32 * 16.0;
        let (x, y) = ((ui::W - w) / 2.0, (FIELD_H - h) / 2.0);
        ui::parchment(x, y, w, h);
        for (i, l) in lines.iter().enumerate() {
            ui::text_center(l, ui::W / 2.0, y + 14.0 + i as f32 * 16.0, 12.0, ui::INK);
        }
    }

    pub fn draw(&self) {
        self.draw_terrain();
        for gi in 0..self.groups.len() {
            self.draw_group(gi);
        }
        for p in &self.particles {
            match p {
                Particle::Smoke { x, y, t, r } => {
                    draw_circle(*x, *y, *r, Color::new(0.9, 0.9, 0.9, (t / 0.8).clamp(0.0, 0.8)));
                }
                Particle::Number { x, y, t, text, color } => {
                    let mut c = *color;
                    c.a = t.clamp(0.0, 1.0);
                    ui::text_shadow(text, *x - ui::measure(text, 10.0) / 2.0, *y, 10.0, c);
                }
            }
        }
        self.draw_status();
        match self.phase {
            Phase::Intro => {
                let mut lines = ui::wrap(&self.message, 12.0, 390.0);
                lines.push(String::new());
                lines.push("Lead your men to the town gate.".into());
                self.banner(&lines);
            }
            Phase::ConfirmRetreat => self.banner(&["Retreat to the ships? (Y/N)".to_string()]),
            Phase::Ending => {
                let mut lines = ui::wrap(&self.message, 12.0, 390.0);
                if let Some(o) = &self.outcome {
                    lines.push(format!("Your men remaining: {}   Defenders: {}", o.men_left, o.defenders_left));
                }
                self.banner(&lines);
            }
            Phase::Playing => {}
        }
    }
}
