//! "Selene": cratered moon of the rocky planet.

use raylib::prelude::Color;

use super::common::*;
use crate::fragment::Fragment;
use crate::math::Vec3;
use crate::noise::{fbm, hash01, noise, ridged, worley};
use crate::render::Uniforms;

/// Normal perturbation from the crater height field.
pub const BUMP_STRENGTH: f32 = 0.35;
/// Parallax depth of the height field; 0 disables the effect.
pub const PARALLAX_SCALE: f32 = 0.6;
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
    // Craters stay inside their Voronoi cell: small radii and a fade
    // towards the cell border (F2 - F1 -> 0) avoid straight cut edges.
    let border = smoothstep(0.0, 0.12, w.f2 - w.f1);
    let radius = 0.14 + 0.16 * hash01(w.id.rotate_left(5));
    let d = w.f1 / radius;
    let bowl = (d * d - 1.0).min(0.0);
    let rim = 0.35 * (-((d - 1.0) / 0.22).powi(2)).exp() * border;
    out.height += (bowl + rim) * weight;
    out.floor = out.floor.max(smoothstep(0.75, 0.2, d));
    out.rim = out.rim.max(rim / 0.35);
    if hash01(w.id.rotate_left(13)) > 0.8 {
        let dir = (q - w.point).normalize();
        let rays = ridged(dir * 7.0 + Vec3::splat(hash01(w.id) * 50.0), 1).powi(6);
        let halo = (-(d - 1.0).max(0.0) * 2.5).exp() * smoothstep(0.8, 0.95, d) * border;
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

    // Parallax: move the sample point along the tangent part of the view
    // direction, proportionally to the height, so rims hide part of the
    // crater floors when seen at grazing angles.
    let s = if u.params.parallax {
        let v_obj = object_dir(u, s.v);
        let v_n = v_obj.dot(s.p).max(0.35);
        let offset = (v_obj - s.p * v_obj.dot(s.p)) * (height(s.p) * PARALLAX_SCALE / v_n);
        Surface {
            p: (s.p + offset).normalize(),
            ..s
        }
    } else {
        s
    };
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
