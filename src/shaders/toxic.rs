//! "Miasma": acid seas, fungal continents and sulfur haze.

use raylib::prelude::Color;

use super::common::*;
use crate::fragment::Fragment;
use crate::math::Vec3;
use crate::noise::{fbm, noise, ridged};
use crate::render::Uniforms;

/// Height threshold of the fungal continents.
pub const SHORE: f32 = 0.05;
/// Speed of the algae swirls and haze bands.
pub const DRIFT_SPEED: f32 = 0.04;
/// Maximum opacity of the sulfur haze bands.
pub const HAZE_OPACITY: f32 = 0.3;

const ACID_DEEP: Vec3 = hex(0x0b241e);
const ACID: Vec3 = hex(0x1f5a3a);
const ALGAE_DARK: Vec3 = hex(0x4f8f1a);
const ALGAE: Vec3 = hex(0xb6ff3a);
const FUNGUS: Vec3 = hex(0x4e2563);
const FUNGUS_RIDGE: Vec3 = hex(0xc04fd0);
const SPORE: Vec3 = hex(0x40fff0);
const HAZE: Vec3 = hex(0xd8e85a);

pub fn toxic(frag: &Fragment, u: &Uniforms) -> Option<Color> {
    let s = surface(frag, u)?;
    let p = s.p;
    let t = u.time * DRIFT_SPEED;

    // Layer 1, acid sea: darker where deeper.
    let h = fbm(p * 2.0 + Vec3::new(-4.0, 2.0, 1.0), 5);
    let land = smoothstep(SHORE, SHORE + 0.03, h);
    let mut sea = mix(ACID, ACID_DEEP, smoothstep(SHORE, -0.15, h));

    // Layer 2, algae blooms: swirling, animated domain-warped fBm.
    let w = Vec3::new(
        fbm(p * 2.5 + Vec3::new(t, 0.0, 0.0), 2),
        fbm(p * 2.5 + Vec3::new(0.0, t * 0.8, 3.0), 2),
        0.0,
    ) * 1.5;
    let bloom = smoothstep(0.04, 0.2, fbm(p * 3.5 + w, 4));
    sea = mix(sea, mix(ALGAE_DARK, ALGAE, bloom), bloom * 0.65);

    // Layer 3, fungal continents: violet crust with bright ridges.
    let ridges = ridged(p * 6.0, 3);
    let crust = mix(FUNGUS, FUNGUS_RIDGE, smoothstep(0.6, 0.9, ridges));
    let albedo = mix(sea, crust, land);

    // Oily sea with an iridescent Blinn-Phong sheen; matte crust.
    let ndl = s.n.dot(s.l);
    let day = smoothstep(-0.1, 0.2, ndl);
    let sheen = blinn_phong(s.n, s.l, s.v, 60.0) * (1.0 - land);
    let film = cosine_palette(
        s.n.dot(s.v) * 2.0 + h * 4.0,
        Vec3::splat(0.55),
        Vec3::splat(0.45),
        Vec3::ONE,
        Vec3::new(0.1, 0.4, 0.7),
    );
    let mut c = albedo * (0.02 + ndl.max(0.0)) + film * (sheen * 1.2);

    // Layer 4, bioluminescent spores on the ridges, night side only.
    let spores = smoothstep(0.35, 0.6, noise(p * 45.0)) * land * smoothstep(0.5, 0.9, ridges);
    let flicker = 0.7 + 0.3 * (u.time * 3.0 + noise(p * 8.0) * 10.0).sin();
    c += SPORE * (spores * (1.0 - day) * flicker * 1.3);

    // Layer 5, sulfur haze: latitudinal veils drifting over everything.
    let band = 0.5 + 0.5 * (p.y * 11.0 + fbm(rotate_y(p, t) * 3.0, 3) * 3.0).sin();
    let haze = smoothstep(0.55, 1.0, band) * HAZE_OPACITY;
    c = mix(c, HAZE * (0.03 + saturate(ndl + 0.2)), haze);

    c += HAZE * (fresnel(s.n, s.v, 3.0) * (0.15 + 0.85 * day) * 0.2);
    Some(to_color(c, 1.0))
}
