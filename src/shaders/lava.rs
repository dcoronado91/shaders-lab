//! "Ignea": volcanic world of cracked basalt plates over molten rock.

use raylib::prelude::Color;

use super::common::*;
use crate::fragment::Fragment;
use crate::math::Vec3;
use crate::noise::{fbm, hash01, noise, worley};
use crate::render::Uniforms;

/// Speed of the churning inside the lava lakes.
pub const FLOW_SPEED: f32 = 0.12;
/// Width of the glowing cracks between plates (Worley F2 - F1).
pub const CRACK_WIDTH: f32 = 0.08;
/// fBm threshold for molten lakes; lower means more lava.
pub const LAKE_THRESHOLD: f32 = 0.17;
/// Emission multiplier of molten rock.
pub const GLOW: f32 = 1.5;

const BASALT: Vec3 = hex(0x1d1817);
const ASH: Vec3 = hex(0x4d4542);
const SULFUR: Vec3 = hex(0xe8c547);
const EMBER: Vec3 = hex(0xc2300a);
const LAVA_DARK: Vec3 = hex(0x8a1a05);
const LAVA_ORANGE: Vec3 = hex(0xff6a10);
const LAVA_YELLOW: Vec3 = hex(0xffc23a);
const LAVA_WHITE: Vec3 = hex(0xfff0c0);
const SMOKE: Vec3 = hex(0xff4a1a);

pub fn lava(frag: &Fragment, u: &Uniforms) -> Option<Color> {
    let s = surface(frag, u)?;
    let p = s.p;
    let t = u.time * FLOW_SPEED;

    // Tectonic plates from Worley cells; F2 - F1 is small along borders.
    let cells = worley(p * 4.5 + Vec3::splat(noise(p * 3.0) * 0.35));
    let crack = 1.0 - smoothstep(0.0, CRACK_WIDTH, cells.f2 - cells.f1);

    // Layer 1, basalt crust: grainy dark rock, each plate a bit different.
    let grain = fbm(p * 9.0, 3) * 0.5 + 0.5;
    let mut albedo = mix(BASALT, ASH, grain * 0.8) * (0.6 + 0.4 * hash01(cells.id));

    // Layer 2, sulfur deposits: yellow crusts away from the cracks.
    let sulfur = smoothstep(0.36, 0.48, noise(p * 6.0 + Vec3::splat(30.0))) * (1.0 - crack);
    albedo = mix(albedo, SULFUR, sulfur * 0.65);

    // Layer 3, lava lakes: churning molten rock (animated domain warp).
    let lake_n = fbm(p * 1.5 + Vec3::new(0.0, 3.3, 0.0), 4);
    let lake = smoothstep(LAKE_THRESHOLD, LAKE_THRESHOLD + 0.04, lake_n);
    let swirl = Vec3::new(fbm(p * 3.0 + Vec3::new(t, 0.0, 0.0), 2), fbm(p * 3.0 + Vec3::new(0.0, -t, 2.0), 2), 0.0);
    let flow = fbm(rotate_y(p, t * 0.3) * 5.0 + swirl * 2.0, 3);
    let heat = saturate(0.42 + flow * 1.6 + (lake_n - LAKE_THRESHOLD) * 2.0);
    let molten = ramp(
        &[(0.0, LAVA_DARK), (0.35, LAVA_ORANGE), (0.7, LAVA_YELLOW), (1.0, LAVA_WHITE)],
        heat,
    );

    // Layer 4, cooling rims: crust around the lakes still glows red.
    let rim = smoothstep(LAKE_THRESHOLD - 0.08, LAKE_THRESHOLD, lake_n) * (1.0 - lake);

    // Layer 5, cracks: veins of lava pulsing per plate.
    let pulse = 0.65 + 0.35 * (u.time * 2.0 + hash01(cells.id) * 6.28).sin();
    let vein = crack * pulse * (0.6 + 0.4 * noise(p * 25.0 - Vec3::splat(u.time * 0.4))) * (1.0 - lake);

    // The crust reflects the star; molten rock emits regardless of light.
    let diffuse = s.n.dot(s.l).max(0.0);
    let mut c = albedo * (0.03 + diffuse * 0.9);
    c = mix(c, EMBER * 0.6, rim * 0.7);
    c += mix(EMBER, LAVA_ORANGE, vein) * (vein * GLOW);
    c = mix(c, molten * GLOW, lake);
    c += SMOKE * (fresnel(s.n, s.v, 3.0) * 0.25);
    Some(to_color(c, 1.0))
}
