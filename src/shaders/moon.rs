//! "Selene": cratered moon of the rocky planet.

use raylib::prelude::Color;

use super::common::*;
use crate::fragment::Fragment;
use crate::math::Vec3;
use crate::noise::{fbm, hash01, noise, ridged, worley};
use crate::render::Uniforms;

/// Normal perturbation from the crater height field.
pub const BUMP_STRENGTH: f32 = 0.35;
/// Fraction of Worley cells that hold a crater, per size class.
pub const CRATER_CHANCE: [f32; 2] = [0.6, 0.5];

const HIGHLAND: Vec3 = hex(0xb8b0a3);
const MARE: Vec3 = hex(0x4a4e58);
const FLOOR: Vec3 = hex(0x3b3a3c);
const EJECTA: Vec3 = hex(0xe9eef5);
const EARTHSHINE: Vec3 = hex(0x1a2a45);

#[derive(Default)]
struct Craters {
    height: f32,
    floor: f32,
    rim: f32,
    ejecta: f32,
}

/// One size class of craters: a bowl with a raised rim per Worley cell.
/// Rare "fresh" craters also throw bright radial ejecta rays.
fn crater_layer(q: Vec3, chance: f32, out: &mut Craters, weight: f32) {
    let w = worley(q);
    if hash01(w.id) > chance {
        return;
    }
    let radius = 0.22 + 0.22 * hash01(w.id.rotate_left(5));
    let d = w.f1 / radius;
    let bowl = (d * d - 1.0).min(0.0);
    let rim = 0.35 * (-((d - 1.0) / 0.22).powi(2)).exp();
    out.height += (bowl + rim) * weight;
    out.floor = out.floor.max(smoothstep(0.75, 0.2, d));
    out.rim = out.rim.max(rim / 0.35);
    if hash01(w.id.rotate_left(13)) > 0.8 {
        let dir = (q - w.point).normalize();
        let rays = ridged(dir * 7.0 + Vec3::splat(hash01(w.id) * 50.0), 1).powi(6);
        let halo = (-(d - 1.0).max(0.0) * 1.2).exp() * (d > 0.85) as u8 as f32;
        out.ejecta = out.ejecta.max(halo * (0.35 + rays));
    }
}

fn craters(p: Vec3) -> Craters {
    let mut c = Craters::default();
    crater_layer(p * 4.0, CRATER_CHANCE[0], &mut c, 0.09);
    crater_layer(p * 11.0 + Vec3::splat(3.7), CRATER_CHANCE[1], &mut c, 0.035);
    c
}

pub fn height(p: Vec3) -> f32 {
    craters(p).height + fbm(p * 2.0, 3) * 0.03 + noise(p * 40.0) * 0.004
}

pub fn moon(frag: &Fragment, u: &Uniforms) -> Option<Color> {
    let s = surface(frag, u)?;
    let p = s.p;
    let cr = craters(p);
    let h = cr.height + fbm(p * 2.0, 3) * 0.03 + noise(p * 40.0) * 0.004;
    let n = bump(&s, u, BUMP_STRENGTH, 0.003, h, height);

    // Layer 1, maria: dark basalt plains over bright highlands.
    let maria = smoothstep(0.02, 0.12, fbm(p * 1.3 + Vec3::new(5.0, 1.0, 2.0), 3));
    let mut albedo = mix(HIGHLAND, MARE, maria);

    // Layer 2, craters: darker floors and brighter rims.
    albedo = mix(albedo, FLOOR, cr.floor * 0.45);
    albedo = albedo * (1.0 + cr.rim * 0.25);

    // Layer 3, ejecta: bright rays around fresh craters.
    albedo = mix(albedo, EJECTA, saturate(cr.ejecta) * 0.55);

    // Layer 4, regolith speckle: fine albedo noise.
    albedo = albedo * (0.9 + 0.2 * noise(p * 90.0));

    // Harsh, airless lighting with a hint of planetshine at night.
    let diffuse = n.dot(s.l).max(0.0);
    let c = albedo * (diffuse * 1.1) + EARTHSHINE * 0.25 * (1.0 - smoothstep(-0.1, 0.2, s.n.dot(s.l)));
    Some(to_color(c, 1.0))
}
