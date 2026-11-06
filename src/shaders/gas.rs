//! "Crono": gas giant with flowing bands and a great storm.

use raylib::prelude::Color;

use super::common::*;
use crate::fragment::Fragment;
use crate::math::Vec3;
use crate::noise::{fbm, noise};
use crate::render::Uniforms;

/// Storm position on the unit sphere (object space).
pub const STORM_CENTER: Vec3 = Vec3::new(0.3, 0.38, 0.87);
/// Radius of the vortex influence on the unit sphere.
pub const STORM_RADIUS: f32 = 0.3;
/// Band rotation at the storm center, radians; bigger curls the bands more.
pub const STORM_TWIST: f32 = 4.5;
/// Extra spin of the storm interior, radians per second.
pub const STORM_SPIN: f32 = 0.5;
/// Number of bands from pole to pole (roughly).
pub const BAND_FREQUENCY: f32 = 13.0;
/// Speed of the shearing flow between neighbour bands.
pub const FLOW_SPEED: f32 = 0.06;

const PALE: Vec3 = hex(0xf3e3c3);
const CREAM: Vec3 = hex(0xdfc08a);
const OCHRE: Vec3 = hex(0xc4874a);
const RUST: Vec3 = hex(0x8f4a2a);
const CHOCOLATE: Vec3 = hex(0x5a2f1f);
const EDDY: Vec3 = hex(0xfdf3e1);
const STORM_RED: Vec3 = hex(0xb23c22);
const STORM_CORE: Vec3 = hex(0xf2a35e);
const POLAR: Vec3 = hex(0x6a7a92);
const OVAL: Vec3 = hex(0xfaf7f0);
const HAZE: Vec3 = hex(0xffd9a0);

pub fn gas(frag: &Fragment, u: &Uniforms) -> Option<Color> {
    let s = surface(frag, u)?;
    let p = s.p;
    let t = u.time;

    // Vortex: rotate the sample point around the storm axis by an angle
    // that decays with distance, so the bands wrap around the storm.
    let center = STORM_CENTER.normalize();
    let d = (p - center).length();
    let profile = (-(d / STORM_RADIUS).powi(2)).exp();
    let q = rotate_axis(p, center, STORM_TWIST * profile);

    // Differential rotation: neighbour latitudes drift at different speeds,
    // plus an anisotropic warp stretched along the longitude.
    let lat = q.y;
    let flow = rotate_y(q, t * FLOW_SPEED * (lat * 9.0).sin());
    let warp = fbm(flow * Vec3::new(2.0, 7.0, 2.0) + Vec3::new(0.0, 0.0, t * 0.02), 4);
    let y = lat + warp * 0.1;

    // Layer 1, bands: irregular belts and zones through a warm ramp.
    let phase = y * BAND_FREQUENCY + (y * 4.3).sin() * 1.6;
    let tone = 0.5 + 0.5 * phase.sin() + fbm(flow * Vec3::new(3.0, 12.0, 3.0), 2) * 0.3;
    let mut col = ramp(
        &[(0.0, CHOCOLATE), (0.25, RUST), (0.5, OCHRE), (0.75, CREAM), (1.0, PALE)],
        tone,
    );

    // Layer 2, eddies: bright turbulent streaks along band boundaries.
    let edge = phase.cos().abs().powi(3);
    let eddies = fbm(flow * Vec3::new(6.0, 16.0, 6.0) + Vec3::splat(t * 0.03), 3);
    col = mix(col, EDDY, smoothstep(0.0, 0.18, eddies) * (0.1 + edge) * 0.45);

    // Layer 3, the great storm: spinning red eye with a pale collar.
    let spun = rotate_axis(p, center, (STORM_TWIST + t * STORM_SPIN) * profile);
    let swirl = fbm(spun * 9.0, 3);
    let eye = smoothstep(STORM_RADIUS * 0.75, STORM_RADIUS * 0.25, d);
    let collar = (-((d - STORM_RADIUS * 0.8) / (STORM_RADIUS * 0.12)).powi(2)).exp();
    let storm = mix(STORM_RED, STORM_CORE, smoothstep(STORM_RADIUS * 0.5, 0.0, d)) * (0.85 + swirl * 1.2);
    col = mix(col, storm, eye);
    col = mix(col, EDDY, collar * 0.45);

    // Layer 4, polar haze: cold blue-gray caps with noisy edges.
    let polar = smoothstep(0.74, 0.95, p.y.abs() + fbm(p * 5.0, 2) * 0.12);
    col = mix(col, POLAR, polar * 0.8);

    // Layer 5, white ovals: small storms riding a southern belt.
    let ovals = smoothstep(0.42, 0.55, noise(flow * Vec3::new(10.0, 22.0, 10.0)))
        * smoothstep(0.07, 0.0, (lat + 0.62).abs());
    col = mix(col, OVAL, ovals);

    // Soft wrap lighting and limb darkening.
    let ndl = s.n.dot(s.l);
    let diffuse = saturate((ndl + 0.2) / 1.2);
    let limb = 0.55 + 0.45 * s.n.dot(s.v).max(0.0).sqrt();
    let mut c = col * (diffuse * limb + 0.008);
    c += HAZE * (fresnel(s.n, s.v, 4.0) * saturate(ndl + 0.3) * 0.25);
    Some(to_color(c, 1.0))
}
