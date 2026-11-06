//! "Solara": star with animated plasma. Fully emissive, ignores the light.

use raylib::prelude::Color;

use super::common::*;
use crate::fragment::Fragment;
use crate::math::Vec3;
use crate::noise::{fbm, ridged, worley};
use crate::render::Uniforms;

/// Speed of the animated turbulence (warp, granulation and flares).
pub const TURBULENCE_SPEED: f32 = 0.35;
/// Sunspot threshold on low-frequency fBm; lower means more spots.
pub const SPOT_THRESHOLD: f32 = 0.24;

const DEEP_RED: Vec3 = hex(0x7a1203);
const ORANGE: Vec3 = hex(0xe8550c);
const GOLD: Vec3 = hex(0xffb52e);
const WHITE_HOT: Vec3 = hex(0xfff4d6);
const LANE: Vec3 = hex(0x6a1400);
const PENUMBRA: Vec3 = hex(0x7a2a08);
const UMBRA: Vec3 = hex(0x1c0703);
const FACULA: Vec3 = hex(0xfffbe8);
const LIMB_RED: Vec3 = hex(0xc02a05);

pub fn star(frag: &Fragment, u: &Uniforms) -> Option<Color> {
    let s = surface(frag, u)?;
    let p = s.p;
    let t = u.time * TURBULENCE_SPEED;

    // Layer 1, temperature zones: animated domain-warped fBm through a
    // red -> orange -> gold -> white ramp.
    let q = p * 2.2;
    let warp = Vec3::new(
        fbm(q + Vec3::new(t * 0.6, 0.0, 1.7), 3),
        fbm(q + Vec3::new(5.2, -t * 0.5, 0.0), 3),
        fbm(q + Vec3::new(-3.1, 2.8, t * 0.55), 3),
    );
    let temp = saturate(0.5 + fbm(p * 3.0 + warp * 1.8 + Vec3::new(0.0, t * 0.25, 0.0), 4) * 1.5);
    let mut c = ramp(
        &[(0.0, DEEP_RED), (0.4, ORANGE), (0.78, GOLD), (1.0, WHITE_HOT)],
        temp,
    );

    // Layer 2, granulation: drifting Worley cells, bright convection cells
    // separated by dark intergranular lanes.
    let cells = worley(p * 26.0 + warp * 1.2 + Vec3::new(t * 0.5, -t * 0.4, t * 0.3));
    let lanes = 1.0 - smoothstep(0.0, 0.25, cells.f2 - cells.f1);
    let g = 1.0 - cells.f1.min(1.0);
    c = mix(c * (0.85 + 0.3 * g), LANE, lanes * 0.4);

    // Layer 3, sunspots: slow low-frequency noise thresholded into a dark
    // umbra surrounded by a filamentary penumbra.
    let spot = fbm(p * 1.7 + Vec3::new(13.0, t * 0.05, -7.0), 3);
    let penumbra = smoothstep(SPOT_THRESHOLD - 0.03, SPOT_THRESHOLD + 0.03, spot);
    let umbra = smoothstep(SPOT_THRESHOLD + 0.05, SPOT_THRESHOLD + 0.1, spot);
    c = mix(c, PENUMBRA * (0.5 + 0.9 * ridged(p * 40.0 + warp, 1)), penumbra * 0.85);
    c = mix(c, UMBRA, umbra);

    // Layer 4, faculae: bright pulsing filaments in a ring around spots.
    let ring = smoothstep(SPOT_THRESHOLD - 0.12, SPOT_THRESHOLD - 0.04, spot) * (1.0 - penumbra);
    let flare = ridged(p * 6.0 + warp * 1.5 - Vec3::new(0.0, t * 0.3, 0.0), 3);
    let pulse = 0.75 + 0.25 * (t * 2.3 + spot * 20.0).sin();
    c += FACULA * (ring * smoothstep(0.6, 0.9, flare) * pulse * 1.2);

    // Layer 5, limb: darker and redder towards the edge of the disk.
    let mu = s.n.dot(s.v).max(0.0);
    c = mix(c * (0.45 + 0.55 * mu.powf(0.6)), LIMB_RED * 0.8, (1.0 - mu).powi(3) * 0.7);

    Some(to_color(c * 1.25, 1.0))
}
