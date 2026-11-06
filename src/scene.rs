use std::f32::consts::PI;
use std::fmt;
use std::str::FromStr;

use crate::math::{Mat4, Vec3};
use crate::obj::Obj;
use crate::render::{BodyInfo, DrawCall, FragmentShader, Params, Uniforms};
use crate::shaders::common::hex;
use crate::shaders::{atmosphere, rocky, star};
use crate::vertex::Vertex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Body {
    Star,
    Rocky,
    All,
}

impl Body {
    pub const ALL: [Body; 3] = [Body::Star, Body::Rocky, Body::All];

    /// Name used on the command line and for evidence files.
    pub fn name(self) -> &'static str {
        match self {
            Body::Star => "estrella",
            Body::Rocky => "rocoso",
            Body::All => "todos",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Body::Star => "Solara - estrella",
            Body::Rocky => "Eden - planeta rocoso + luna Selene",
            Body::All => "Sistema completo",
        }
    }

    fn camera_distance(self) -> f32 {
        match self {
            Body::Star => 3.6,
            Body::Rocky => 4.8,
            Body::All => 9.0,
        }
    }
}

impl fmt::Display for Body {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for Body {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        Body::ALL
            .into_iter()
            .find(|b| b.name() == s)
            .ok_or_else(|| format!("unknown body '{s}'"))
    }
}

/// Orbital camera around the focused body.
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
}

impl Camera {
    pub fn eye(&self, target: Vec3) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        target + Vec3::new(cp * sy, sp, cp * cy) * self.distance
    }

    pub fn orbit(&mut self, d_yaw: f32, d_pitch: f32) {
        self.yaw += d_yaw;
        self.pitch = (self.pitch + d_pitch).clamp(-1.45, 1.45);
    }

    pub fn zoom(&mut self, factor: f32, base: f32) {
        self.distance = (self.distance * factor).clamp(base * 0.45, base * 3.0);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SceneState {
    pub body: Body,
    pub time: f32,
    /// Azimuth of the light for single-body views (radians).
    pub light_angle: f32,
    pub camera: Camera,
    pub params: Params,
}

pub const DEFAULT_LIGHT_ANGLE: f32 = -1.75;

impl SceneState {
    pub fn new(body: Body) -> Self {
        SceneState {
            body,
            time: 0.0,
            light_angle: DEFAULT_LIGHT_ANGLE,
            camera: Camera {
                yaw: 0.0,
                pitch: 0.18,
                distance: body.camera_distance(),
            },
            params: Params::default(),
        }
    }

    pub fn base_distance(&self) -> f32 {
        self.body.camera_distance()
    }
}

/// Halo shell drawn around a body.
#[derive(Clone, Copy)]
struct Glow {
    shader: FragmentShader,
    color: Vec3,
    strength: f32,
    extent: f32,
    lit: bool,
}

#[derive(Clone, Copy)]
struct PlanetSpec {
    shader: FragmentShader,
    radius: f32,
    /// Spin around the planet axis, radians per second.
    spin: f32,
    /// Axis tilt (rotation around X and Z).
    tilt: (f32, f32),
    glow: Option<Glow>,
}

fn spec(body: Body) -> PlanetSpec {
    match body {
        Body::Star | Body::All => PlanetSpec {
            shader: star::star,
            radius: 1.0,
            spin: 0.03,
            tilt: (0.1, 0.0),
            glow: Some(Glow {
                shader: atmosphere::corona,
                color: Vec3::ONE,
                strength: 0.95,
                extent: 1.6,
                lit: false,
            }),
        },
        Body::Rocky => PlanetSpec {
            shader: rocky::rocky,
            radius: 1.0,
            spin: 0.06,
            tilt: (0.15, 0.35),
            glow: Some(Glow {
                shader: atmosphere::atmosphere,
                color: hex(0x4d9bff),
                strength: 0.9,
                extent: 1.12,
                lit: true,
            }),
        },
    }
}

/// Bodies placed in the full-system view: (body, orbit radius, scale, phase).
const SYSTEM: [(Body, f32, f32, f32); 2] = [(Body::Star, 0.0, 1.4, 0.0), (Body::Rocky, 4.2, 0.45, 2.2)];

pub struct Scene {
    vertices: Vec<Vertex>,
    /// Scale that makes the mesh enclose a sphere of radius 1.
    cover: f32,
}

impl Scene {
    pub fn load(path: &str) -> Result<Self, String> {
        let obj = Obj::load(path)?;
        Ok(Scene {
            vertices: obj.get_vertex_array(),
            cover: 1.0 / obj.inner_radius(),
        })
    }

    pub fn draws(&self, state: &SceneState, width: i32, height: i32) -> Vec<DrawCall<'_>> {
        let placements: Vec<(Body, Vec3, f32)> = match state.body {
            Body::All => SYSTEM
                .iter()
                .map(|&(body, orbit, scale, phase)| {
                    let a = phase + state.time * 0.6 / (orbit + 1.0).powf(1.5);
                    (body, Vec3::new(a.cos() * orbit, 0.0, a.sin() * orbit), scale)
                })
                .collect(),
            body => vec![(body, Vec3::ZERO, 1.0)],
        };

        let target = Vec3::ZERO;
        let eye = state.camera.eye(target);
        let light_pos = match state.body {
            Body::All => Vec3::ZERO,
            _ => {
                let (s, c) = state.light_angle.sin_cos();
                Vec3::new(s, 0.3, c).normalize() * 500.0
            }
        };
        let base = Uniforms {
            model: Mat4::IDENTITY,
            model_inv: Mat4::IDENTITY,
            normal_matrix: Mat4::IDENTITY,
            view: Mat4::look_at(eye, target, Vec3::Y),
            projection: Mat4::perspective(PI / 4.0, width as f32 / height as f32, 0.05, 300.0),
            viewport: Mat4::viewport(width as f32, height as f32),
            time: state.time,
            light_pos,
            camera_pos: eye,
            params: state.params,
            body: BodyInfo::default(),
        };

        let mut opaque = Vec::new();
        let mut transparent = Vec::new();
        for (body, center, scale) in placements {
            let spec = spec(body);
            let radius = spec.radius * scale;
            let tilt = Vec3::new(spec.tilt.0, 0.0, spec.tilt.1);
            let axis = (Mat4::rotation_z(tilt.z) * Mat4::rotation_x(tilt.x)).transform_dir(Vec3::Y);
            let mut info = BodyInfo {
                center,
                radius,
                axis,
                ..BodyInfo::default()
            };
            let spin = Vec3::new(tilt.x, state.time * spec.spin, tilt.z);
            let model = Mat4::model(center, spin, Vec3::splat(radius * self.cover));
            opaque.push(self.draw(base.with_model(model, info), spec.shader, false));

            if let Some(glow) = spec.glow {
                info.glow = glow.color;
                info.glow_strength = glow.strength;
                info.glow_extent = glow.extent;
                info.glow_lit = glow.lit;
                let shell = Mat4::model(center, spin, Vec3::splat(radius * glow.extent * self.cover));
                transparent.push((center, self.draw(base.with_model(shell, info), glow.shader, true)));
            }
        }

        // Back to front; the sort is stable, so per-body order is kept.
        transparent.sort_by(|a, b| {
            let (da, db) = ((a.0 - eye).length(), (b.0 - eye).length());
            db.total_cmp(&da)
        });
        opaque.extend(transparent.into_iter().map(|(_, d)| d));
        opaque
    }

    fn draw(&self, uniforms: Uniforms, shader: FragmentShader, transparent: bool) -> DrawCall<'_> {
        DrawCall {
            vertices: &self.vertices,
            uniforms,
            shader,
            transparent,
        }
    }
}
