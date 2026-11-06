//! Ring system of "Crono": the same sphere flattened into a thin disk.
//! Radii are measured in planet radii inside the ring plane.

use raylib::prelude::Color;

use super::common::*;
use crate::fragment::Fragment;
use crate::math::Vec3;
use crate::noise::noise;
use crate::render::Uniforms;

/// Inner and outer ring radius; fragments outside are discarded.
pub const INNER: f32 = 1.3;
pub const OUTER: f32 = 2.3;
/// Center and half width of the Cassini-like division (normalized radius).
pub const DIVISION: (f32, f32) = (0.64, 0.03);
/// Opacity of the ring shadow cast on the planet.
pub const SHADOW_OPACITY: f32 = 0.75;

const DUST: Vec3 = hex(0x7a6250);
const CREAM: Vec3 = hex(0xe8d6b0);
const TAN: Vec3 = hex(0xc9a982);
const ICE_GRAY: Vec3 = hex(0x9aa4ae);

/// Ring opacity at radius `r`: noisy radial bands with gaps.
pub fn density(r: f32) -> f32 {
    if !(INNER..=OUTER).contains(&r) {
        return 0.0;
    }
    let x = (r - INNER) / (OUTER - INNER);
    let mut d = 0.6 + 0.22 * (x * 37.0).sin() * (x * 11.0).cos() + noise(Vec3::new(r * 24.0, 0.5, 0.5)) * 0.4;
    // Faint inner ring, then the main gap and a thin outer gap.
    d *= 0.3 + 0.7 * smoothstep(0.1, 0.22, x);
    d *= smoothstep(DIVISION.1 * 0.4, DIVISION.1, (x - DIVISION.0).abs());
    d *= smoothstep(0.003, 0.008, (x - 0.9).abs());
    d *= smoothstep(0.0, 0.03, x) * smoothstep(1.0, 0.96, x);
    saturate(d)
}

/// Shadow of the rings on the planet: follows the ray towards the light
/// up to the ring plane and samples the ring density there.
pub fn shadow_on_planet(world_pos: Vec3, l: Vec3, u: &Uniforms) -> f32 {
    let (c, n) = (u.body.center, u.body.axis);
    let denom = l.dot(n);
    if denom.abs() < 1e-4 {
        return 1.0;
    }
    let t = (c - world_pos).dot(n) / denom;
    if t <= 0.0 {
        return 1.0;
    }
    let r = (world_pos + l * t - c).length() / u.body.radius;
    1.0 - density(r) * SHADOW_OPACITY
}

pub fn rings(frag: &Fragment, u: &Uniforms) -> Option<Color> {
    let (c, n) = (u.body.center, u.body.axis);
    let rel = frag.world_pos - c;
    let r = (rel - n * rel.dot(n)).length() / u.body.radius;

    // Layer 1, radial bands and gaps (opacity) — inner hole discarded.
    let dens = density(r);
    if dens < 0.02 {
        return None;
    }

    // Layer 2, fine ringlets: high-frequency radial noise.
    let ringlets = 0.72 + 0.28 * noise(Vec3::new(r * 180.0, 1.3, 0.0));

    // Layer 3, color by radius: dusty inner rings, cream core, icy edge.
    let mut col = ramp(
        &[(INNER, DUST), (1.65, CREAM), (1.95, TAN), (OUTER, ICE_GRAY)],
        r,
    ) * (0.85 + 0.3 * noise(Vec3::new(r * 60.0, 4.0, 2.0)));

    // Layer 4, spokes: dark radial smudges that drift around the ring.
    let angle = frag.obj_pos.z.atan2(frag.obj_pos.x);
    let spokes = smoothstep(0.2, 0.45, noise(Vec3::new(angle * 5.0 + u.time * 0.15, r * 2.5, 7.0)))
        * smoothstep(1.55, 1.7, r)
        * smoothstep(2.15, 1.95, r);
    col = col * (1.0 - spokes * 0.35);

    // Lighting: lit face, translucent back face with forward scattering,
    // and the planet's shadow.
    let l = u.light_dir(frag.world_pos);
    let v = (u.camera_pos - frag.world_pos).normalize();
    let ndl = n.dot(l).abs();
    let light = if n.dot(l) * n.dot(v) > 0.0 {
        0.2 + 0.9 * ndl
    } else {
        (1.0 - dens) * 0.5 * ndl + (-v).dot(l).max(0.0).powi(8) * 0.8
    };
    let shadow = sphere_shadow(frag.world_pos, l, c, u.body.radius, 0.03);
    col = col * (light * shadow + 0.02);
    Some(to_color(col, dens * ringlets))
}
