//! "Crionica": frozen world with crystal fields and polar auroras.

use raylib::prelude::Color;

use super::common::*;
use crate::fragment::Fragment;
use crate::math::Vec3;
use crate::noise::{fbm, hash01, noise, worley};
use crate::render::Uniforms;

/// Crystal cells per unit of the sphere; larger means smaller facets.
pub const FACET_SCALE: f32 = 9.0;
/// How much each facet tilts the normal.
pub const FACET_TILT: f32 = 0.45;
/// Blinn-Phong exponent of the crystal sparkle.
pub const SPARKLE_SHININESS: f32 = 220.0;
/// Speed of the aurora curtains.
pub const AURORA_SPEED: f32 = 0.6;

const DEEP_ICE: Vec3 = hex(0x2a6f9a);
const GLACIER: Vec3 = hex(0x9cc7e0);
const SNOW: Vec3 = hex(0xf4f9ff);
const FRACTURE: Vec3 = hex(0x0b6f86);
const FRACTURE_GLOW: Vec3 = hex(0x37e0e8);
const AURORA_GREEN: Vec3 = hex(0x3dffa0);
const AURORA_VIOLET: Vec3 = hex(0xa14dff);
const RIM: Vec3 = hex(0x9fe8ff);

pub fn ice(frag: &Fragment, u: &Uniforms) -> Option<Color> {
    let s = surface(frag, u)?;
    let p = s.p;
    let t = u.time;

    // Layer 1, glacier base: deep blue ice to pale glacier.
    let base = fbm(p * 2.5, 4);
    let mut albedo = mix(DEEP_ICE, GLACIER, smoothstep(-0.15, 0.1, base));

    // Layer 2, frost plains: bright fresh snow.
    let frost = smoothstep(0.02, 0.12, fbm(p * 4.0 + Vec3::splat(9.0), 4));
    albedo = mix(albedo, SNOW, frost);

    // Layer 3, crystal fields: Worley facets, each with its own tilted
    // normal and a thin-film iridescent tint that changes with the angle.
    let field = smoothstep(-0.05, 0.1, fbm(p * 1.4 + Vec3::splat(20.0), 3));
    let cell = worley(p * FACET_SCALE);
    let jitter = Vec3::new(
        hash01(cell.id) - 0.5,
        hash01(cell.id.rotate_left(9)) - 0.5,
        hash01(cell.id.rotate_left(17)) - 0.5,
    );
    let facet = (s.n + u.normal_matrix.transform_dir(jitter * FACET_TILT)).normalize();
    let n = s.n.lerp(facet, field).normalize();
    let film = cosine_palette(
        n.dot(s.v) * 1.5 + hash01(cell.id) * 0.3,
        Vec3::splat(0.5),
        Vec3::splat(0.5),
        Vec3::ONE,
        Vec3::new(0.0, 0.33, 0.67),
    );
    albedo = mix(albedo, albedo * 0.5 + film * film * 0.6, field * 0.6);

    // Layer 4, fractures: two scales of ridged lines (lineae) that glow
    // faintly from below even on the night side.
    let warp = Vec3::splat(fbm(p * 2.0, 2) * 0.6);
    let lines = smoothstep(0.985, 0.998, 1.0 - noise(p * 2.2 + warp).abs())
        .max(smoothstep(0.99, 0.999, 1.0 - noise(p * 6.0 + Vec3::splat(5.0)).abs()) * 0.7);

    let ndl = n.dot(s.l).max(0.0);
    let day = smoothstep(-0.1, 0.2, s.n.dot(s.l));
    let sparkle = blinn_phong(n, s.l, s.v, SPARKLE_SHININESS) * 2.5 * (0.2 + field)
        + blinn_phong(s.n, s.l, s.v, 30.0) * 0.12;
    let mut c = albedo * (0.02 + ndl) + Vec3::ONE * sparkle;
    c = mix(c, FRACTURE * (0.1 + ndl), lines) + FRACTURE_GLOW * (lines * 0.22);

    // Layer 5, aurora: animated curtains around the poles, night side only.
    let lat = p.y.abs();
    let curtain = noise(Vec3::new(p.x * 4.0, t * AURORA_SPEED * 0.4, p.z * 4.0));
    let band = (-((lat - 0.8 - curtain * 0.06) / 0.05).powi(2)).exp();
    let shimmer = 0.5 + 0.5 * noise(Vec3::new(p.x * 25.0, t * AURORA_SPEED * 2.0, p.z * 25.0));
    let aurora = band * shimmer * (1.0 - day * 0.85);
    c += mix(AURORA_GREEN, AURORA_VIOLET, smoothstep(0.78, 0.9, lat)) * (aurora * 1.6);

    c += RIM * (fresnel(s.n, s.v, 3.0) * (0.2 + 0.8 * day) * 0.5);
    Some(to_color(c, 1.0))
}
