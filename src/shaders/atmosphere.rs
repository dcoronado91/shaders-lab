//! Halo drawn on a second, larger copy of the sphere (front faces only,
//! alpha blended). Its opacity depends on how close the view ray passes to
//! the body center, so the halo is round even though the shell is low-poly.

use raylib::prelude::Color;

use super::common::*;
use crate::fragment::Fragment;
use crate::math::Vec3;
use crate::noise::fbm;
use crate::render::Uniforms;

const SUNSET: Vec3 = hex(0xff8a4a);

/// Closest approach of the view ray, in body radii, and the radial
/// direction (world space) of that closest point.
fn ray_profile(frag: &Fragment, u: &Uniforms) -> (f32, Vec3) {
    let body = &u.body;
    let d = (frag.world_pos - u.camera_pos).normalize();
    let along = (body.center - u.camera_pos).dot(d);
    let closest = u.camera_pos + d * along;
    let radial = closest - body.center;
    (radial.length() / body.radius, radial.normalize())
}

pub fn atmosphere(frag: &Fragment, u: &Uniforms) -> Option<Color> {
    let body = &u.body;
    let (b, radial) = ray_profile(frag, u);
    let glow = if b >= 1.0 {
        (1.0 - (b - 1.0) / (body.glow_extent - 1.0)).max(0.0).powf(2.5)
    } else {
        (-(1.0 - b) * 14.0).exp()
    };
    if glow <= 0.003 {
        return None;
    }

    let mut color = body.glow;
    let mut strength = body.glow_strength;
    if body.glow_lit {
        let l = u.light_dir(body.center);
        let ndl = radial.dot(l);
        let view = (body.center - u.camera_pos).normalize();
        // Backlit bodies show a bright forward-scattering ring.
        let forward = view.dot(l).max(0.0).powi(6);
        let sunset = (1.0 - (ndl * 2.5).abs()).max(0.0);
        color = mix(color, SUNSET, sunset * 0.6);
        strength *= smoothstep(-0.5, 0.6, ndl) + forward * 1.5;
    }
    Some(to_color(color * 1.4, glow * strength))
}

const CORONA_INNER: Vec3 = hex(0xffd27a);
const CORONA_MID: Vec3 = hex(0xff9a2e);
const CORONA_OUTER: Vec3 = hex(0xb3240a);

/// Star corona: same halo profile, with animated streamers whose reach
/// changes with the direction around the star. Ignores the scene light.
pub fn corona(frag: &Fragment, u: &Uniforms) -> Option<Color> {
    let body = &u.body;
    let (b, radial) = ray_profile(frag, u);
    if b < 1.0 {
        let rim = (-(1.0 - b) * 18.0).exp();
        return Some(to_color(CORONA_INNER * 1.6, rim * 0.3));
    }
    let x = (b - 1.0) / (body.glow_extent - 1.0);
    if x >= 1.0 {
        return None;
    }
    let t = u.time;
    let streamer = 0.55 + 0.9 * (0.5 + fbm(radial * 3.0 + Vec3::new(t * 0.15, -t * 0.1, t * 0.12), 3) * 2.0);
    let falloff = (-x * 3.2 / streamer.max(0.2)).exp() * (1.0 - x).powi(2);
    let color = ramp(
        &[(0.0, CORONA_INNER), (0.25, CORONA_MID), (1.0, CORONA_OUTER)],
        x / streamer.max(0.3),
    );
    Some(to_color(color * 1.6, falloff * body.glow_strength))
}
