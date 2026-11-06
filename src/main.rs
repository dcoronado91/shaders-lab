use std::path::Path;

use raylib::prelude::*;
use shaders_lab::background::starfield;
use shaders_lab::framebuffer::Framebuffer;
use shaders_lab::render::render;
use shaders_lab::scene::{Body, Scene, SceneState};

const FB_WIDTH: i32 = 960;
const FB_HEIGHT: i32 = 600;
const MODEL_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/models/sphere.obj");
const EVIDENCE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/evidence");

fn fail(err: String) -> ! {
    eprintln!("error: {err}");
    std::process::exit(1);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [flag, name] = args.as_slice() else {
        fail("usage: shaders_lab --screenshot <body>".into());
    };
    if flag != "--screenshot" {
        fail(format!("unexpected argument {flag}"));
    }
    set_trace_log(TraceLogLevel::LOG_WARNING);
    let body: Body = name.parse().unwrap_or_else(|e| fail(e));
    let scene = Scene::load(MODEL_PATH).unwrap_or_else(|e| fail(e));
    let mut fb = Framebuffer::new(FB_WIDTH, FB_HEIGHT, Color::BLACK);
    fb.set_background(starfield(FB_WIDTH, FB_HEIGHT));
    fb.clear();
    let state = SceneState::new(body);
    let start = std::time::Instant::now();
    render(&mut fb, &scene.draws(&state, FB_WIDTH, FB_HEIGHT));
    println!("rendered in {:?}", start.elapsed());
    let path = Path::new(EVIDENCE_DIR).join(format!("{body}.png"));
    fb.save_png(&path).unwrap_or_else(|e| fail(e));
    println!("Saved {}", path.display());
}
