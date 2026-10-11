//! Sword duel minigame: side-view fencing between two captains whose crews
//! fight behind them.

use crate::input::Input;
use crate::ui::{self, Menu};
use macroquad::prelude::*;
use macroquad::rand::gen_range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DuelKind {
    /// Boarding an enemy ship: captain vs captain, crews fight in the background.
    Boarding,
    /// Storming a town: player vs the garrison commander, soldiers behind him.
    TownAssault,
    /// A personal duel with no crews (e.g. with a rival or the evil nobleman).
    Personal,
}

#[derive(Clone, Debug)]
pub struct DuelSetup {
    pub kind: DuelKind,
    pub player_name: String,
    /// e.g. "Don Alonso"
    pub opponent_name: String,
    /// e.g. "Captain of the Spanish Galleon" or "Commandant of Havana"
    pub opponent_title: String,
    pub player_crew: i32,
    pub enemy_crew: i32,
    /// Player fencing ability, roughly 0.5 (old, sick) .. 1.5 (young expert).
    pub player_skill: f32,
    /// Opponent ability, roughly 0.5 .. 1.5 (scaled by difficulty).
    pub enemy_skill: f32,
}

#[derive(Clone, Debug)]
pub struct DuelOutcome {
    pub won: bool,
    pub player_crew: i32,
    pub enemy_crew: i32,
    /// Player took a serious wound during the fight.
    pub wounded: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Weapon {
    Rapier,
    Longsword,
    Cutlass,
}

impl Weapon {
    fn name(self) -> &'static str {
        match self {
            Weapon::Rapier => "Rapier",
            Weapon::Longsword => "Longsword",
            Weapon::Cutlass => "Cutlass",
        }
    }
    fn desc(self) -> &'static str {
        match self {
            Weapon::Rapier => "A light, quick blade with long reach. Fast thrusts are hard to parry, but each hit gains little ground.",
            Weapon::Longsword => "A balanced weapon, neither quick nor slow. A sound choice for any fight.",
            Weapon::Cutlass => "A heavy sailor's blade. Slow to swing and easy to read, but each blow drives the foe far back.",
        }
    }
    /// Wind-up time before a blow lands (seconds, at skill 1.0).
    fn windup(self) -> f32 {
        match self {
            Weapon::Rapier => 0.30,
            Weapon::Longsword => 0.40,
            Weapon::Cutlass => 0.52,
        }
    }
    /// Recovery after a blow.
    fn recover(self) -> f32 {
        match self {
            Weapon::Rapier => 0.25,
            Weapon::Longsword => 0.32,
            Weapon::Cutlass => 0.42,
        }
    }
    /// Ground gained per successful hit.
    fn push(self) -> f32 {
        match self {
            Weapon::Rapier => 0.8,
            Weapon::Longsword => 1.0,
            Weapon::Cutlass => 1.35,
        }
    }
    fn length(self) -> f32 {
        match self {
            Weapon::Rapier => 40.0,
            Weapon::Longsword => 36.0,
            Weapon::Cutlass => 28.0,
        }
    }
}

const WEAPONS: [Weapon; 3] = [Weapon::Rapier, Weapon::Longsword, Weapon::Cutlass];
/// Ground needed to drive the opponent off his footing.
const EDGE: f32 = 5.0;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Action {
    Idle,
    /// Preparing a blow at `height`; `t` counts down.
    WindUp { t: f32, total: f32, height: u8 },
    /// Blow extended (visual only).
    Strike { t: f32, height: u8 },
    Recover { t: f32 },
    Stagger { t: f32 },
    Dodge { t: f32 },
}

#[derive(Clone, Debug)]
struct Fighter {
    weapon: Weapon,
    skill: f32,
    /// 0 = high, 1 = middle, 2 = low
    guard: u8,
    action: Action,
    hits_taken: i32,
    dodge_cd: f32,
}

impl Fighter {
    fn new(weapon: Weapon, skill: f32) -> Self {
        Fighter { weapon, skill: skill.clamp(0.3, 2.0), guard: 1, action: Action::Idle, hits_taken: 0, dodge_cd: 0.0 }
    }
    fn busy(&self) -> bool {
        !matches!(self.action, Action::Idle)
    }
    fn can_parry(&self) -> bool {
        matches!(self.action, Action::Idle | Action::WindUp { .. } | Action::Recover { .. })
    }
}

#[derive(Clone, Debug)]
struct Flash {
    x: f32,
    y: f32,
    t: f32,
    hit: bool,
}

enum Phase {
    Choose(Menu),
    Fight,
    End { t: f32, won: bool, msg: String },
}

pub struct Duel {
    pub setup: DuelSetup,
    phase: Phase,
    player: Fighter,
    enemy: Fighter,
    /// Positive = player is winning ground.
    ground: f32,
    /// Smoothed visual ground.
    ground_vis: f32,
    player_crew: f32,
    enemy_crew: f32,
    crew_tick: f32,
    ai_timer: f32,
    /// Pending AI reaction: (delay, height)
    ai_react: Option<(f32, u8)>,
    ai_feinted: bool,
    flashes: Vec<Flash>,
    msg: String,
    msg_t: f32,
    time: f32,
}

fn height_name(h: u8) -> &'static str {
    ["High", "Middle", "Low"][h.min(2) as usize]
}

impl Duel {
    pub fn new(setup: DuelSetup) -> Self {
        let personal = setup.kind == DuelKind::Personal;
        let enemy_weapon = WEAPONS[gen_range(0, 3)];
        let menu = Menu::new(WEAPONS.iter().map(|w| w.name().to_string()).collect());
        let mut menu = menu;
        menu.sel = 1;
        Duel {
            player: Fighter::new(Weapon::Longsword, setup.player_skill),
            enemy: Fighter::new(enemy_weapon, setup.enemy_skill),
            player_crew: if personal { 0.0 } else { setup.player_crew.max(0) as f32 },
            enemy_crew: if personal { 0.0 } else { setup.enemy_crew.max(0) as f32 },
            setup,
            phase: Phase::Choose(menu),
            ground: 0.0,
            ground_vis: 0.0,
            crew_tick: 1.0,
            ai_timer: 1.5,
            ai_react: None,
            ai_feinted: false,
            flashes: vec![],
            msg: String::new(),
            msg_t: 0.0,
            time: 0.0,
        }
    }

    /// A default setup used by `PIRATES_TEST=fencing`.
    pub fn test_default() -> Self {
        Self::new(DuelSetup {
            kind: DuelKind::Boarding,
            player_name: "Nicholas".into(),
            opponent_name: "Don Alonso".into(),
            opponent_title: "Captain of the Spanish Merchantman".into(),
            player_crew: 80,
            enemy_crew: 60,
            player_skill: 1.0,
            enemy_skill: 1.0,
        })
    }

    fn personal(&self) -> bool {
        self.setup.kind == DuelKind::Personal
    }

    fn say(&mut self, s: &str) {
        self.msg = s.to_string();
        self.msg_t = 0.9;
    }

    pub fn update(&mut self, inp: &Input, dt: f32) -> Option<DuelOutcome> {
        self.time += dt;
        match &mut self.phase {
            Phase::Choose(menu) => {
                if let Some(i) = menu.update(inp) {
                    self.player.weapon = WEAPONS[i];
                    self.phase = Phase::Fight;
                    self.say("En garde!");
                }
                None
            }
            Phase::Fight => {
                self.update_fight(inp, dt);
                None
            }
            Phase::End { t, won, .. } => {
                *t += dt;
                let won = *won;
                if *t > 3.0 || (*t > 0.6 && inp.any_key()) {
                    return Some(DuelOutcome {
                        won,
                        player_crew: self.player_crew.round().max(0.0) as i32,
                        enemy_crew: self.enemy_crew.round().max(0.0) as i32,
                        wounded: !won || self.player.hits_taken >= 6,
                    });
                }
                None
            }
        }
    }

    fn update_fight(&mut self, inp: &Input, dt: f32) {
        self.msg_t -= dt;
        self.ground_vis += (self.ground - self.ground_vis) * (dt * 8.0).min(1.0);
        for f in &mut self.flashes {
            f.t -= dt;
        }
        self.flashes.retain(|f| f.t > 0.0);

        // --- Player input ---
        if inp.pressed(KeyCode::Up) && self.player.guard > 0 {
            self.player.guard -= 1;
        }
        if inp.pressed(KeyCode::Down) && self.player.guard < 2 {
            self.player.guard += 1;
        }
        let attack = inp.pressed(KeyCode::Space) || inp.pressed(KeyCode::Enter) || inp.pressed(KeyCode::Right) || inp.pressed(KeyCode::KpEnter);
        if attack && !self.player.busy() {
            let total = self.player.weapon.windup() / self.player.skill.sqrt();
            self.player.action = Action::WindUp { t: total, total, height: self.player.guard };
            self.plan_ai_reaction();
        }
        if inp.pressed(KeyCode::Left) && self.player.dodge_cd <= 0.0 && matches!(self.player.action, Action::Idle | Action::Recover { .. }) {
            self.player.action = Action::Dodge { t: 0.35 };
            self.player.dodge_cd = 1.0;
        }
        // While winding up, the blow follows the player's guard changes (feints).
        if let Action::WindUp { height, .. } = &mut self.player.action {
            *height = self.player.guard;
        }

        // --- AI ---
        self.update_ai(dt);

        // --- Advance actions & resolve blows ---
        if let Some(h) = Self::tick(&mut self.player, dt) {
            self.resolve(true, h);
        }
        if let Some(h) = Self::tick(&mut self.enemy, dt) {
            self.resolve(false, h);
        }

        // --- Crews ---
        if !self.personal() {
            self.crew_tick -= dt;
            if self.crew_tick <= 0.0 {
                self.crew_tick = 1.0;
                let adv = (self.ground / EDGE).clamp(-1.0, 1.0);
                let (p, e) = (self.player_crew, self.enemy_crew);
                let total = (p + e).max(1.0);
                // Each side inflicts losses proportional to its share of men,
                // boosted when its captain is winning.
                let e_loss = (e.min(p) * 0.015 + 0.35) * (p / total * 2.0) * (1.0 + 2.0 * adv).max(0.1) * gen_range(0.6, 1.4);
                let p_loss = (e.min(p) * 0.015 + 0.35) * (e / total * 2.0) * (1.0 - 2.0 * adv).max(0.1) * gen_range(0.6, 1.4);
                self.enemy_crew = (self.enemy_crew - e_loss.max(0.0)).max(0.0).floor();
                self.player_crew = (self.player_crew - p_loss.max(0.0)).max(0.0).floor();
            }
        }

        // --- Victory conditions ---
        let opp = self.setup.opponent_name.clone();
        if self.ground >= EDGE {
            let msg = if self.personal() {
                format!("You have defeated {opp}!")
            } else {
                format!("You have defeated {opp}! The {} surrender.", if self.setup.kind == DuelKind::TownAssault { "soldiers" } else { "crew" })
            };
            self.phase = Phase::End { t: 0.0, won: true, msg };
        } else if self.ground <= -EDGE {
            self.phase = Phase::End { t: 0.0, won: false, msg: format!("You are beaten! {opp} has bested you.") };
        } else if !self.personal() && self.enemy_crew <= 0.0 {
            self.phase = Phase::End { t: 0.0, won: true, msg: format!("The enemy is overwhelmed! {opp} throws down his sword.") };
        } else if !self.personal() && self.player_crew <= 0.0 {
            self.phase = Phase::End { t: 0.0, won: false, msg: "Your men are cut down! You are forced to surrender.".into() };
        }
    }

    /// Schedule the AI's attempt to parry the player's incoming blow.
    fn plan_ai_reaction(&mut self) {
        let s = self.enemy.skill;
        let chance = (0.2 + 0.45 * s).clamp(0.1, 0.9);
        if gen_range(0.0, 1.0) < chance {
            let delay = gen_range(0.12, 0.35) / s;
            self.ai_react = Some((delay, self.player.guard));
        } else {
            self.ai_react = None;
        }
    }

    fn update_ai(&mut self, dt: f32) {
        self.enemy.dodge_cd -= dt;
        self.player.dodge_cd -= dt;
        // Reaction to the player's blow (tracks the blow's current height).
        if let Some((d, _)) = self.ai_react.as_mut() {
            *d -= dt;
            if *d <= 0.0 {
                if let Action::WindUp { height, .. } = self.player.action {
                    if !matches!(self.enemy.action, Action::Strike { .. } | Action::Stagger { .. }) {
                        self.enemy.guard = height;
                        // Skilled foes follow a feint.
                        if gen_range(0.0, 1.0) < 0.3 * self.enemy.skill {
                            self.ai_react = Some((0.08 / self.enemy.skill, height));
                            return;
                        }
                    }
                }
                self.ai_react = None;
            }
        }
        // Feint: once per blow, switch height mid wind-up.
        if let Action::WindUp { t, total, height } = &mut self.enemy.action {
            if !self.ai_feinted && *t < *total * 0.5 && gen_range(0.0, 1.0) < 0.25 * self.enemy.skill * dt * 10.0 {
                self.ai_feinted = true;
                *height = (*height + gen_range(1u8, 3u8)) % 3;
                self.enemy.guard = *height;
            }
        }
        // React to player's guard: occasionally wander guard while idle.
        self.ai_timer -= dt;
        let incoming = matches!(self.player.action, Action::WindUp { .. });
        if self.ai_timer <= 0.0 && !self.enemy.busy() && !incoming {
            let h = if gen_range(0.0, 1.0) < 0.6 {
                // aim where the player is not guarding
                let mut h = gen_range(0u8, 3u8);
                if h == self.player.guard {
                    h = (h + 1) % 3;
                }
                h
            } else {
                gen_range(0u8, 3u8)
            };
            let total = self.enemy.weapon.windup() / self.enemy.skill.sqrt() * gen_range(1.0, 1.25);
            self.enemy.guard = h;
            self.enemy.action = Action::WindUp { t: total, total, height: h };
            self.ai_feinted = false;
            self.ai_timer = gen_range(0.9, 2.0) / self.enemy.skill;
        } else if self.ai_timer > 0.3 && !self.enemy.busy() && gen_range(0.0, 1.0) < dt * 0.6 {
            self.enemy.guard = gen_range(0u8, 3u8);
        }
    }

    /// Advance a fighter's action timer. Returns the height of a blow that lands now.
    fn tick(f: &mut Fighter, dt: f32) -> Option<u8> {
        match &mut f.action {
            Action::Idle => None,
            Action::WindUp { t, height, .. } => {
                *t -= dt;
                if *t <= 0.0 {
                    let h = *height;
                    f.action = Action::Strike { t: 0.15, height: h };
                    Some(h)
                } else {
                    None
                }
            }
            Action::Strike { t, .. } => {
                *t -= dt;
                if *t <= 0.0 {
                    f.action = Action::Recover { t: f.weapon.recover() / f.skill.sqrt() };
                }
                None
            }
            Action::Recover { t } | Action::Stagger { t } | Action::Dodge { t } => {
                *t -= dt;
                if *t <= 0.0 {
                    f.action = Action::Idle;
                }
                None
            }
        }
    }

    fn fighter_x(&self) -> (f32, f32) {
        let c = 320.0 + self.ground_vis * 36.0;
        (c - 42.0, c + 42.0)
    }

    fn resolve(&mut self, by_player: bool, h: u8) {
        let (px, ex) = self.fighter_x();
        let hy = 330.0 - [56.0, 40.0, 24.0][h as usize];
        let (att, def) = if by_player { (&mut self.player, &mut self.enemy) } else { (&mut self.enemy, &mut self.player) };
        let fx = (px + ex) / 2.0;
        if matches!(def.action, Action::Dodge { .. }) {
            att.action = Action::Recover { t: att.weapon.recover() * 1.5 };
            self.say(if by_player { "He leaps back!" } else { "You leap clear!" });
            return;
        }
        if def.can_parry() && def.guard == h {
            // Parried: attacker is thrown off balance.
            att.action = Action::Recover { t: att.weapon.recover() * 1.6 / att.skill.sqrt() };
            self.flashes.push(Flash { x: fx, y: hy, t: 0.25, hit: false });
            self.say(if by_player { "Parried!" } else { "You parry!" });
            crate::sound::play(crate::sound::Sfx::Clang);
            if by_player && gen_range(0.0, 1.0) < 0.35 + 0.35 * self.enemy.skill {
                // Riposte: the foe strikes back while you are off balance.
                self.ai_timer = gen_range(0.02, 0.15);
            }
            return;
        }
        // Hit!
        crate::sound::play(crate::sound::Sfx::Hit);
        let push = att.weapon.push() * (0.85 + 0.15 * att.skill);
        def.hits_taken += 1;
        def.action = Action::Stagger { t: 0.35 };
        if by_player {
            self.ground += push;
            self.flashes.push(Flash { x: ex - 8.0, y: hy, t: 0.3, hit: true });
            self.say(&format!("A hit! {} falls back.", self.setup.opponent_name));
        } else {
            self.ground -= push;
            self.flashes.push(Flash { x: px + 8.0, y: hy, t: 0.3, hit: true });
            self.say("You are hit and fall back!");
        }
    }

    // ------------------------------------------------------------------ draw

    pub fn draw(&self) {
        match &self.phase {
            Phase::Choose(menu) => self.draw_choose(menu),
            Phase::Fight => self.draw_fight(),
            Phase::End { msg, won, .. } => {
                self.draw_fight();
                let w = 460.0;
                ui::panel(320.0 - w / 2.0, 150.0, w, 70.0, if *won { ui::BLUE } else { ui::RED }, ui::YELLOW);
                let lines = ui::wrap(msg, 16.0, w - 30.0);
                for (i, l) in lines.iter().enumerate() {
                    ui::text_center(l, 320.0, 162.0 + i as f32 * 20.0, 16.0, ui::WHITE);
                }
            }
        }
    }

    fn draw_choose(&self, menu: &Menu) {
        self.draw_background();
        ui::parchment(120.0, 60.0, 400.0, 280.0);
        ui::text_center("Choose your weapon", 320.0, 76.0, 20.0, ui::INK);
        ui::text_center(&format!("You face {}", self.setup.opponent_name), 320.0, 104.0, 14.0, ui::INK);
        ui::text_center(&self.setup.opponent_title, 320.0, 122.0, 12.0, ui::BROWN);
        menu.draw(220.0, 150.0, 200.0, 16.0, ui::INK, ui::BROWN, ui::YELLOW);
        let w = WEAPONS[menu.sel.min(2)];
        for (i, l) in ui::wrap(w.desc(), 12.0, 350.0).iter().enumerate() {
            ui::text(l, 145.0, 240.0 + i as f32 * 15.0, 12.0, ui::INK);
        }
        // weapon picture
        let y = 300.0;
        let len = w.length() * 3.0;
        draw_line(320.0 - len / 2.0, y, 320.0 + len / 2.0, y, if w == Weapon::Rapier { 1.5 } else { 3.0 }, ui::DGRAY);
        draw_line(320.0 - len / 2.0 - 2.0, y - 8.0, 320.0 - len / 2.0 - 2.0, y + 8.0, 3.0, ui::BROWN);
        draw_line(320.0 - len / 2.0 - 18.0, y, 320.0 - len / 2.0 - 2.0, y, 4.0, ui::BROWN);
        ui::text_center("Up/Down to choose, Enter to fight", 320.0, 318.0, 10.0, ui::BROWN);
    }

    fn draw_background(&self) {
        match self.setup.kind {
            DuelKind::Boarding => {
                for i in 0..20 {
                    let t = i as f32 / 20.0;
                    ui::rect(0.0, i as f32 * 10.0, 640.0, 10.0, ui::lerp_color(ui::LBLUE, ui::LCYAN, t));
                }
                ui::rect(0.0, 200.0, 640.0, 60.0, ui::SEA);
                for i in 0..12 {
                    let x = ((i as f32 * 61.0 + self.time * 12.0) % 680.0) - 20.0;
                    draw_line(x, 215.0 + (i % 4) as f32 * 11.0, x + 14.0, 215.0 + (i % 4) as f32 * 11.0, 1.0, ui::LBLUE);
                }
                // masts and sails
                for &mx in &[150.0f32, 500.0] {
                    ui::rect(mx - 4.0, 20.0, 8.0, 250.0, ui::BROWN);
                    ui::rect(mx - 50.0, 40.0, 100.0, 60.0, ui::WHITE);
                    ui::rect(mx - 40.0, 110.0, 80.0, 50.0, ui::WHITE);
                    ui::frame(mx - 50.0, 40.0, 100.0, 60.0, ui::LGRAY, 1.0);
                    ui::frame(mx - 40.0, 110.0, 80.0, 50.0, ui::LGRAY, 1.0);
                    draw_line(mx - 55.0, 40.0, mx + 55.0, 40.0, 3.0, ui::BROWN);
                    draw_line(mx - 45.0, 110.0, mx + 45.0, 110.0, 3.0, ui::BROWN);
                }
                // railing
                ui::rect(0.0, 250.0, 640.0, 6.0, ui::BROWN);
                for i in 0..33 {
                    ui::rect(i as f32 * 20.0, 256.0, 4.0, 18.0, ui::BROWN);
                }
                ui::rect(0.0, 272.0, 640.0, 128.0, Color::new(0.55, 0.35, 0.15, 1.0));
                for i in 0..9 {
                    draw_line(0.0, 280.0 + i as f32 * 14.0, 640.0, 280.0 + i as f32 * 14.0, 1.0, Color::new(0.35, 0.2, 0.08, 1.0));
                }
            }
            DuelKind::TownAssault => {
                ui::rect(0.0, 0.0, 640.0, 160.0, ui::LCYAN);
                ui::rect(0.0, 90.0, 640.0, 180.0, ui::LGRAY);
                for r in 0..9 {
                    for c in 0..17 {
                        let off = if r % 2 == 0 { 0.0 } else { 20.0 };
                        ui::frame(c as f32 * 40.0 - off, 90.0 + r as f32 * 20.0, 40.0, 20.0, ui::DGRAY, 1.0);
                    }
                }
                for c in 0..16 {
                    ui::rect(c as f32 * 42.0, 70.0, 24.0, 20.0, ui::LGRAY);
                }
                ui::rect(280.0, 140.0, 80.0, 130.0, ui::BLACK);
                ui::rect(0.0, 270.0, 640.0, 130.0, Color::new(0.8, 0.7, 0.45, 1.0));
            }
            DuelKind::Personal => {
                ui::rect(0.0, 0.0, 640.0, 270.0, Color::new(0.4, 0.22, 0.1, 1.0));
                for i in 0..16 {
                    draw_line(i as f32 * 40.0, 0.0, i as f32 * 40.0, 270.0, 2.0, Color::new(0.3, 0.15, 0.05, 1.0));
                }
                ui::rect(100.0, 60.0, 80.0, 60.0, ui::LBLUE);
                ui::rect(460.0, 60.0, 80.0, 60.0, ui::LBLUE);
                ui::rect(0.0, 270.0, 640.0, 130.0, Color::new(0.5, 0.3, 0.12, 1.0));
                for &tx in &[80.0f32, 540.0] {
                    ui::rect(tx - 30.0, 220.0, 60.0, 8.0, ui::BROWN);
                    ui::rect(tx - 4.0, 228.0, 8.0, 42.0, ui::BROWN);
                }
            }
        }
    }

    fn draw_crowds(&self) {
        if self.personal() {
            return;
        }
        let pn = ((self.player_crew / 5.0).ceil() as i32).min(18);
        let en = ((self.enemy_crew / 5.0).ceil() as i32).min(18);
        let enemy_col = if self.setup.kind == DuelKind::TownAssault { ui::YELLOW } else { ui::RED };
        for i in 0..pn {
            let x = 20.0 + (i % 9) as f32 * 22.0 + if i >= 9 { 11.0 } else { 0.0 };
            let y = 268.0 + if i >= 9 { 10.0 } else { 0.0 };
            let bob = ((self.time * 6.0 + i as f32 * 1.3).sin()) * 2.0;
            small_man(x, y + bob, ui::LBLUE, true, self.time + i as f32);
        }
        for i in 0..en {
            let x = 620.0 - (i % 9) as f32 * 22.0 - if i >= 9 { 11.0 } else { 0.0 };
            let y = 268.0 + if i >= 9 { 10.0 } else { 0.0 };
            let bob = ((self.time * 6.0 + i as f32 * 1.7).sin()) * 2.0;
            small_man(x, y + bob, enemy_col, false, self.time + i as f32);
        }
    }

    fn draw_fight(&self) {
        self.draw_background();
        self.draw_crowds();
        let (px, ex) = self.fighter_x();
        draw_fighter(px, 330.0, true, &self.player, ui::WHITE, ui::BLUE);
        let ecol = match self.setup.kind {
            DuelKind::TownAssault => ui::YELLOW,
            _ => ui::RED,
        };
        draw_fighter(ex, 330.0, false, &self.enemy, ecol, ui::BLACK);
        for f in &self.flashes {
            let r = 4.0 + (0.3 - f.t) * 30.0;
            let c = if f.hit { ui::LRED } else { ui::YELLOW };
            for k in 0..8 {
                let a = k as f32 * std::f32::consts::TAU / 8.0;
                draw_line(f.x, f.y, f.x + a.cos() * r, f.y + a.sin() * r, 2.0, c);
            }
        }
        self.draw_hud();
    }

    fn draw_hud(&self) {
        ui::panel(0.0, 0.0, 640.0, 46.0, ui::BLACK, ui::BROWN);
        ui::text(&self.setup.player_name, 10.0, 6.0, 14.0, ui::LCYAN);
        ui::text_right(&self.setup.opponent_name, 630.0, 6.0, 14.0, ui::LRED);
        if !self.personal() {
            ui::text(&format!("Crew: {}", self.player_crew as i32), 10.0, 24.0, 12.0, ui::WHITE);
            let label = if self.setup.kind == DuelKind::TownAssault { "Soldiers" } else { "Crew" };
            ui::text_right(&format!("{label}: {}", self.enemy_crew as i32), 630.0, 24.0, 12.0, ui::WHITE);
        } else {
            ui::text(self.player.weapon.name(), 10.0, 24.0, 12.0, ui::WHITE);
            ui::text_right(self.enemy.weapon.name(), 630.0, 24.0, 12.0, ui::WHITE);
        }
        // Ground indicator
        let (x0, x1, y) = (200.0, 440.0, 26.0);
        ui::text_center("GROUND", 320.0, 4.0, 10.0, ui::LGRAY);
        draw_line(x0, y, x1, y, 2.0, ui::LGRAY);
        for i in -5..=5 {
            let x = 320.0 + i as f32 * (x1 - x0) / 10.0;
            draw_line(x, y - 4.0, x, y + 4.0, 1.0, if i == 0 { ui::WHITE } else { ui::DGRAY });
        }
        ui::rect(x0 - 6.0, y - 6.0, 6.0, 12.0, ui::LBLUE);
        ui::rect(x1, y - 6.0, 6.0, 12.0, ui::LRED);
        let mx = 320.0 + (self.ground_vis / EDGE).clamp(-1.0, 1.0) * (x1 - x0) / 2.0;
        draw_triangle(vec2(mx, y - 2.0), vec2(mx - 5.0, y - 10.0), vec2(mx + 5.0, y - 10.0), ui::YELLOW);
        // Guard & controls
        ui::rect(0.0, 376.0, 640.0, 24.0, Color::new(0.0, 0.0, 0.0, 0.7));
        ui::text(&format!("Guard: {}", height_name(self.player.guard)), 8.0, 382.0, 12.0, ui::YELLOW);
        ui::text_right("Up/Down: guard  Space/Right: attack  Left: leap back", 632.0, 383.0, 10.0, ui::LGRAY);
        if self.msg_t > 0.0 && matches!(self.phase, Phase::Fight) {
            ui::text_shadow(&self.msg, 320.0 - ui::measure(&self.msg, 16.0) / 2.0, 60.0, 16.0, ui::WHITE);
        }
    }
}

/// Small background crew figure.
fn small_man(x: f32, y: f32, shirt: Color, facing_right: bool, t: f32) {
    let d = if facing_right { 1.0 } else { -1.0 };
    draw_line(x - 2.0, y, x, y - 7.0, 1.5, ui::BLACK);
    draw_line(x + 2.0, y, x, y - 7.0, 1.5, ui::BLACK);
    ui::rect(x - 2.5, y - 15.0, 5.0, 8.0, shirt);
    draw_circle(x, y - 18.0, 3.0, Color::new(0.95, 0.75, 0.6, 1.0));
    let a = (t * 5.0).sin() * 0.8;
    let (hx, hy) = (x + d * 5.0, y - 13.0 - a * 4.0);
    draw_line(x, y - 13.0, hx, hy, 1.2, ui::BLACK);
    draw_line(hx, hy, hx + d * 6.0, hy - 4.0 + a * 4.0, 1.0, ui::LGRAY);
}

/// Draw a swordsman with feet at (x, y).
fn draw_fighter(x: f32, y: f32, facing_right: bool, f: &Fighter, shirt: Color, coat: Color) {
    let d = if facing_right { 1.0 } else { -1.0 };
    let skin = Color::new(0.95, 0.75, 0.6, 1.0);
    let lean = match f.action {
        Action::Stagger { t } => -t * 25.0,
        Action::Dodge { t } => -t * 30.0,
        Action::Strike { .. } => 5.0,
        _ => 0.0,
    } * d;
    let hip = vec2(x + lean * 0.3, y - 30.0);
    let neck = vec2(x + lean, y - 62.0);
    // legs (fencing stance)
    let stride = if matches!(f.action, Action::Strike { .. }) { 16.0 } else { 11.0 };
    draw_line(hip.x, hip.y, x + d * stride, y, 4.0, ui::BLACK);
    draw_line(hip.x, hip.y, x - d * 10.0, y, 4.0, ui::BLACK);
    draw_line(x + d * stride, y, x + d * (stride + 6.0), y, 4.0, ui::BROWN);
    draw_line(x - d * 10.0, y, x - d * 4.0, y, 4.0, ui::BROWN);
    // body
    draw_triangle(vec2(neck.x - 7.0, neck.y), vec2(neck.x + 7.0, neck.y), vec2(hip.x, hip.y + 4.0), coat);
    draw_line(neck.x, neck.y, hip.x, hip.y, 8.0, shirt);
    ui::rect(hip.x - 6.0, hip.y - 3.0, 12.0, 4.0, ui::BROWN);
    // back arm
    draw_line(neck.x, neck.y + 3.0, neck.x - d * 12.0, neck.y - 8.0, 3.0, shirt);
    // head and hat
    let head = vec2(neck.x, neck.y - 8.0);
    draw_circle(head.x, head.y, 7.0, skin);
    draw_circle(head.x + d * 3.0, head.y - 1.0, 1.2, ui::BLACK);
    draw_line(head.x - 12.0, head.y - 6.0, head.x + 12.0, head.y - 6.0, 3.0, ui::BLACK);
    draw_triangle(vec2(head.x - 8.0, head.y - 6.0), vec2(head.x + 8.0, head.y - 6.0), vec2(head.x - d * 2.0, head.y - 17.0), ui::BLACK);
    // sword arm: hand position from guard/attack
    let h = match f.action {
        Action::WindUp { height, .. } | Action::Strike { height, .. } => height,
        _ => f.guard,
    };
    let hy = y - [56.0, 40.0, 24.0][h as usize];
    let (reach, angle) = match f.action {
        Action::WindUp { t, total, .. } => (6.0 - (1.0 - t / total) * 10.0, -0.6),
        Action::Strike { .. } => (30.0, 0.0),
        Action::Stagger { .. } => (8.0, -1.0),
        _ => (16.0, 0.0),
    };
    let tilt: f32 = [-0.35f32, 0.0, 0.3][h as usize] + angle * if h == 2 { -1.0 } else { 1.0 } * 0.5;
    let shoulder = vec2(neck.x, neck.y + 4.0);
    let hand = vec2(shoulder.x + d * reach, hy);
    draw_line(shoulder.x, shoulder.y, hand.x, hand.y, 3.0, shirt);
    draw_circle(hand.x, hand.y, 2.5, skin);
    let len = f.weapon.length();
    let tip = vec2(hand.x + d * len * tilt.cos(), hand.y + len * tilt.sin());
    let thick = match f.weapon {
        Weapon::Rapier => 1.2,
        Weapon::Longsword => 2.0,
        Weapon::Cutlass => 3.0,
    };
    draw_line(hand.x, hand.y, tip.x, tip.y, thick, ui::WHITE);
    draw_line(hand.x, hand.y - 4.0, hand.x, hand.y + 4.0, 2.0, ui::YELLOW);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(skill: f32, enemy_skill: f32, bot: bool, seed: u64) -> DuelOutcome {
        macroquad::rand::srand(seed);
        let mut d = Duel::test_default();
        d.player.skill = skill;
        d.enemy.skill = enemy_skill;
        let dt = 1.0 / 60.0;
        let mut inp = Input::default();
        inp.pressed.push(KeyCode::Enter);
        d.update(&inp, dt);
        // Bot perception: the enemy blow height it has noticed, with a 0.2s delay.
        let mut seen: Option<u8> = None;
        let mut last: Option<u8> = None;
        let mut stable = 0.0f32;
        for frame in 0..60 * 600 {
            let mut inp = Input::default();
            if bot {
                let cur = match d.enemy.action {
                    Action::WindUp { height, .. } => Some(height),
                    _ => None,
                };
                if cur != last {
                    stable = 0.0;
                    last = cur;
                }
                stable += dt;
                if stable >= 0.2 {
                    seen = cur;
                }
                let want = seen;
                if let Some(h) = want {
                    if d.player.guard < h {
                        inp.pressed.push(KeyCode::Down);
                    } else if d.player.guard > h {
                        inp.pressed.push(KeyCode::Up);
                    }
                } else if matches!(d.enemy.action, Action::Recover { .. } | Action::Stagger { .. }) && !d.player.busy() {
                    // Aim away from the enemy's guard, then strike.
                    if d.player.guard == d.enemy.guard {
                        inp.pressed.push(if d.player.guard == 0 { KeyCode::Down } else { KeyCode::Up });
                    } else {
                        inp.pressed.push(KeyCode::Space);
                    }
                } else if frame % 50 == 0 && !d.player.busy() {
                    inp.pressed.push(if frame % 100 == 0 { KeyCode::Up } else { KeyCode::Down });
                }
            }
            if let Some(o) = d.update(&inp, dt) {
                println!("ground {:.1} hits p{} e{} crews {:?} t={:.0}s", d.ground, d.player.hits_taken, d.enemy.hits_taken, (o.player_crew, o.enemy_crew), d.time);
                return o;
            }
        }
        panic!("duel did not end");
    }

    #[test]
    fn passive_player_loses() {
        for s in 0..5 {
            assert!(!run(1.0, 1.0, false, s).won);
        }
    }

    #[test]
    fn skilled_bot_usually_wins() {
        let wins = (0..20).filter(|&s| run(1.0, 1.0, true, s).won).count();
        println!("bot wins {wins}/20");
        assert!(wins >= 12, "only {wins} wins");
        let hard = (0..20).filter(|&s| run(0.7, 1.4, true, 100 + s).won).count();
        println!("hard: bot wins {hard}/20");
        assert!(hard < wins);
    }
}
