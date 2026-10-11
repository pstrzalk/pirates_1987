//! Sid Meier's Pirates! (1987) clone in Rust.

mod art;
mod data;
mod ending;
mod fencing;
mod game;
mod geo;
mod info;
mod input;
mod landbattle;
mod market;
mod naval;
mod sail;
mod title;
mod sound;
mod ui;
mod world;

use input::{Input, Script};
use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "Pirates!".to_owned(),
        window_width: 1280,
        window_height: 800,
        high_dpi: false,
        ..Default::default()
    }
}

/// Screen layout: logical 640x400 area letterboxed into the window.
struct Viewport {
    x: f32,
    y: f32,
    scale: f32,
}

fn compute_viewport() -> Viewport {
    let (sw, sh) = (screen_width(), screen_height());
    let scale = (sw / ui::W).min(sh / ui::H);
    Viewport { x: (sw - ui::W * scale) / 2.0, y: (sh - ui::H * scale) / 2.0, scale }
}

fn apply_camera(vp: &Viewport) {
    let mut cam = Camera2D::from_display_rect(Rect::new(0.0, 0.0, ui::W, ui::H));
    cam.zoom.y = -cam.zoom.y;
    let sh = screen_height();
    cam.viewport = Some((vp.x as i32, (sh - vp.y - ui::H * vp.scale) as i32, (ui::W * vp.scale) as i32, (ui::H * vp.scale) as i32));
    set_camera(&cam);
}

enum TestScreen {
    Naval(naval::NavalBattle),
    Fencing(fencing::Duel),
    Land(landbattle::LandBattle),
    Map(geo::Geo),
    Game(Box<game::App>),
}

#[macroquad::main(window_conf)]
async fn main() {
    let script_path = std::env::var("PIRATES_SCRIPT").ok();
    let mut script = script_path.as_deref().and_then(Script::load);
    if script.is_some() {
        macroquad::rand::srand(12345);
    } else {
        macroquad::rand::srand((macroquad::miniquad::date::now() * 1000.0) as u64);
    }
    sound::init().await;
    let test = std::env::var("PIRATES_TEST").unwrap_or_default();
    let mut screen = match test.as_str() {
        "naval" => TestScreen::Naval(naval::NavalBattle::test_default()),
        "fencing" => TestScreen::Fencing(fencing::Duel::test_default()),
        "land" => TestScreen::Land(landbattle::LandBattle::test_default()),
        "map" => TestScreen::Map(geo::Geo::new()),
        _ => TestScreen::Game(Box::new(game::App::new())),
    };
    let mut last_mouse = Vec2::ZERO;
    loop {
        let vp = compute_viewport();
        ui::set_scale(vp.scale);
        let inp: Input = match script.as_mut() {
            Some(s) => s.next_frame(),
            None => input::gather(|p| vec2((p.x - vp.x) / vp.scale, (p.y - vp.y) / vp.scale), last_mouse),
        };
        last_mouse = inp.mouse;
        let dt = if script.is_some() { 1.0 / 60.0 } else { get_frame_time().min(0.1) };

        match &mut screen {
            TestScreen::Naval(b) => {
                if let Some(o) = b.update(&inp, dt) {
                    println!("naval outcome: {o:?}");
                    break;
                }
            }
            TestScreen::Fencing(d) => {
                if let Some(o) = d.update(&inp, dt) {
                    println!("duel outcome: {o:?}");
                    break;
                }
            }
            TestScreen::Land(l) => {
                if let Some(o) = l.update(&inp, dt) {
                    println!("land outcome: {o:?}");
                    break;
                }
            }
            TestScreen::Map(_) => {
                if inp.cancel() {
                    break;
                }
            }
            TestScreen::Game(app) => {
                app.update(&inp, dt);
                if app.quit {
                    break;
                }
            }
        }

        clear_background(BLACK);
        apply_camera(&vp);
        match &screen {
            TestScreen::Naval(b) => b.draw(),
            TestScreen::Fencing(d) => d.draw(),
            TestScreen::Land(l) => l.draw(),
            TestScreen::Game(app) => app.draw(),
            TestScreen::Map(g) => {
                draw_texture_ex(g.texture(), 0.0, 0.0, WHITE, DrawTextureParams { dest_size: Some(vec2(640.0, 394.0)), ..Default::default() });
                for t in data::TOWNS {
                    let (land, _) = g.snap_town(t.lon, t.lat);
                    let (gx, gy) = geo::to_grid(land.x, land.y);
                    let (x, y) = (gx * 640.0 / geo::GW as f32, gy * 394.0 / geo::GH as f32);
                    draw_circle(x, y, 2.0, RED);
                    ui::text(t.name, x + 3.0, y - 4.0, 8.0, WHITE);
                }
                ui::text("PIRATES! map test", 4.0, 4.0, 16.0, ui::YELLOW);
            }
        }
        set_default_camera();

        if let Some(s) = script.as_mut() {
            if let Some(path) = s.shot_request.take() {
                get_screen_data().export_png(&path);
            }
            if s.quit {
                break;
            }
        }
        next_frame().await;
    }
}
