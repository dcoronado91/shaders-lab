use std::path::{Path, PathBuf};

use raylib::prelude::*;
use shaders_lab::background::starfield;
use shaders_lab::framebuffer::Framebuffer;
use shaders_lab::render::{Params, render};
use shaders_lab::scene::{Body, Scene, SceneState};
use shaders_lab::shots::Shot;

const FB_WIDTH: i32 = 960;
const FB_HEIGHT: i32 = 600;
const MODEL_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/models/sphere.obj");
const EVIDENCE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/evidence");
/// Must match `$Fps` in scripts/record_gif.ps1.
const RECORD_FPS: u32 = 20;

const ORBIT_SPEED: f32 = 1.5;
const MOUSE_SENSITIVITY: f32 = 0.006;
const LIGHT_SPEED: f32 = 1.2;
const COVERAGE_SPEED: f32 = 0.35;
const HUD: Color = Color::new(200, 205, 220, 255);
const HUD_DIM: Color = Color::new(140, 146, 165, 255);

const USAGE: &str = "usage: shaders_lab [--screenshot <shot> | --record-frames <shot> <dir>]
  <shot>: estrella | rocoso | gaseoso | lava | hielo | toxico | todos
          nubes | sombras_nubes | dia_noche | especular | bump_rocoso | bump_luna | vortice | anillos
          parallax | parallax_off";

enum Command {
    Window,
    Screenshot(Shot),
    RecordFrames(Shot, PathBuf),
}

fn parse_args(args: &[String]) -> Result<Command, String> {
    match args {
        [] => Ok(Command::Window),
        [flag, shot] if flag == "--screenshot" => Shot::find(shot).map(Command::Screenshot),
        [flag, shot, dir] if flag == "--record-frames" => {
            Ok(Command::RecordFrames(Shot::find(shot)?, PathBuf::from(dir)))
        }
        _ => Err(format!("unexpected arguments: {}", args.join(" "))),
    }
}

fn fail(err: impl std::fmt::Display) -> ! {
    eprintln!("error: {err}");
    std::process::exit(1);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = parse_args(&args).unwrap_or_else(|err| {
        eprintln!("error: {err}\n{USAGE}");
        std::process::exit(2);
    });
    if !matches!(command, Command::Window) {
        set_trace_log(TraceLogLevel::LOG_WARNING);
    }
    let scene = Scene::load(MODEL_PATH).unwrap_or_else(|e| fail(e));
    let mut framebuffer = Framebuffer::new(FB_WIDTH, FB_HEIGHT, Color::BLACK);
    framebuffer.set_background(starfield(FB_WIDTH, FB_HEIGHT));

    match command {
        Command::Screenshot(shot) => screenshot(&scene, &mut framebuffer, &shot),
        Command::RecordFrames(shot, dir) => record_frames(&scene, &mut framebuffer, &shot, &dir),
        Command::Window => run_window(&scene, &mut framebuffer),
    }
}

fn draw_frame(scene: &Scene, framebuffer: &mut Framebuffer, state: &SceneState) {
    framebuffer.clear();
    render(framebuffer, &scene.draws(state, FB_WIDTH, FB_HEIGHT));
}

/// Renders the first frame of `shot` and saves it to `evidence/<shot>.png`.
fn screenshot(scene: &Scene, framebuffer: &mut Framebuffer, shot: &Shot) {
    draw_frame(scene, framebuffer, &shot.state_at(0.0));
    let path = Path::new(EVIDENCE_DIR).join(format!("{}.png", shot.name));
    framebuffer.save_png(&path).unwrap_or_else(|e| fail(e));
    println!("Saved {}", path.display());
}

/// Renders `shot` as `frame_0000.png`, `frame_0001.png`, ... in `dir`;
/// scripts/record_gif.ps1 assembles them into a GIF.
fn record_frames(scene: &Scene, framebuffer: &mut Framebuffer, shot: &Shot, dir: &Path) {
    let frames = (shot.seconds * RECORD_FPS as f32).round() as u32;
    for frame in 0..frames {
        draw_frame(scene, framebuffer, &shot.state_at(frame as f32 / RECORD_FPS as f32));
        let path = dir.join(format!("frame_{frame:04}.png"));
        framebuffer.save_png(&path).unwrap_or_else(|e| fail(e));
    }
    println!("Saved {frames} frames of '{}' ({RECORD_FPS} fps) to {}", shot.name, dir.display());
}

const BODY_KEYS: [(KeyboardKey, KeyboardKey); 7] = [
    (KeyboardKey::KEY_ONE, KeyboardKey::KEY_KP_1),
    (KeyboardKey::KEY_TWO, KeyboardKey::KEY_KP_2),
    (KeyboardKey::KEY_THREE, KeyboardKey::KEY_KP_3),
    (KeyboardKey::KEY_FOUR, KeyboardKey::KEY_KP_4),
    (KeyboardKey::KEY_FIVE, KeyboardKey::KEY_KP_5),
    (KeyboardKey::KEY_SIX, KeyboardKey::KEY_KP_6),
    (KeyboardKey::KEY_ZERO, KeyboardKey::KEY_KP_0),
];

fn run_window(scene: &Scene, framebuffer: &mut Framebuffer) {
    let (mut rl, thread) = raylib::init()
        .size(FB_WIDTH, FB_HEIGHT)
        .title("Shaders Lab - planetas y estrellas")
        .build();
    rl.set_target_fps(60);
    let mut texture = rl
        .load_texture_from_image(&thread, &Image::gen_image_color(FB_WIDTH, FB_HEIGHT, Color::BLACK))
        .unwrap_or_else(|e| fail(e));

    let mut state = SceneState::new(Body::Rocky);
    let mut paused = false;

    while !rl.window_should_close() {
        let dt = rl.get_frame_time();

        if let Some(i) = BODY_KEYS
            .iter()
            .position(|&(k, kp)| rl.is_key_pressed(k) || rl.is_key_pressed(kp))
        {
            let params = state.params;
            let time = state.time;
            state = SceneState::new(Body::ALL[i]);
            state.params = params;
            state.time = time;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_SPACE) {
            paused = !paused;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_R) {
            state = SceneState::new(state.body);
            state.params = Params::default();
        }
        if rl.is_key_pressed(KeyboardKey::KEY_P) {
            state.params.parallax = !state.params.parallax;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_M) && state.body == Body::Rocky {
            state.focus_moon = !state.focus_moon;
            state.camera.distance = if state.focus_moon { 1.2 } else { state.base_distance() };
        }

        let down = |k| rl.is_key_down(k) as i32 as f32;
        let shift = rl.is_key_down(KeyboardKey::KEY_LEFT_SHIFT) || rl.is_key_down(KeyboardKey::KEY_RIGHT_SHIFT);
        state.camera.orbit(
            (down(KeyboardKey::KEY_RIGHT) - down(KeyboardKey::KEY_LEFT)) * ORBIT_SPEED * dt,
            (down(KeyboardKey::KEY_UP) - down(KeyboardKey::KEY_DOWN)) * ORBIT_SPEED * dt,
        );
        if rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT) {
            let delta = rl.get_mouse_delta();
            state.camera.orbit(-delta.x * MOUSE_SENSITIVITY, delta.y * MOUSE_SENSITIVITY);
        }
        let wheel = rl.get_mouse_wheel_move();
        if wheel != 0.0 {
            let base = if state.focus_moon { 1.2 } else { state.base_distance() };
            state.camera.zoom(0.9f32.powf(wheel), base);
        }
        if rl.is_key_down(KeyboardKey::KEY_L) {
            state.light_angle += if shift { -LIGHT_SPEED } else { LIGHT_SPEED } * dt;
        }
        let coverage = down(KeyboardKey::KEY_RIGHT_BRACKET) - down(KeyboardKey::KEY_LEFT_BRACKET);
        state.params.cloud_coverage = (state.params.cloud_coverage + coverage * COVERAGE_SPEED * dt).clamp(0.0, 1.0);
        if !paused {
            state.time += dt;
        }

        draw_frame(scene, framebuffer, &state);
        texture
            .update_texture(&framebuffer.color)
            .unwrap_or_else(|e| fail(e));

        // The HUD lives only in the window, never in the framebuffer, so
        // screenshots and recorded frames stay clean.
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::BLACK);
        d.draw_texture(&texture, 0, 0, Color::WHITE);
        let target = if state.focus_moon { "  [camara: luna]" } else { "" };
        d.draw_text(&format!("{}{target}", state.body.title()), 14, 12, 22, Color::RAYWHITE);
        d.draw_text(
            &format!(
                "nubes {:.0}%   parallax {}   tiempo {:.1}s{}",
                state.params.cloud_coverage * 100.0,
                if state.params.parallax { "on" } else { "off" },
                state.time,
                if paused { "  (pausa)" } else { "" }
            ),
            14,
            40,
            16,
            HUD,
        );
        d.draw_fps(FB_WIDTH - 90, 12);
        d.draw_text(
            "1 estrella  2 rocoso+luna  3 gaseoso+anillos  4 lava  5 hielo  6 toxico  0 todos",
            14,
            FB_HEIGHT - 46,
            16,
            HUD_DIM,
        );
        d.draw_text(
            "mouse/flechas orbitar  rueda zoom  Space pausa  L luz (Shift invierte)  [ ] nubes  M luna  P parallax  R reset",
            14,
            FB_HEIGHT - 24,
            16,
            HUD_DIM,
        );
    }
}
