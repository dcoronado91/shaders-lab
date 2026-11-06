//! "Edén": rocky planet with oceans, biomes, deserts, ice, moving clouds
//! with shadows and city lights on the night side.

use raylib::prelude::Color;

use super::common::*;
use crate::fragment::Fragment;
use crate::math::Vec3;
use crate::noise::{fbm, noise, ridged};
use crate::render::Uniforms;

/// Height of the coastline in the terrain height field.
pub const SEA_LEVEL: f32 = 0.0;
/// Normal perturbation from the terrain gradient (land only).
pub const BUMP_STRENGTH: f32 = 0.22;
/// Cloud drift relative to the ground, radians per second.
pub const CLOUD_SPEED: f32 = 0.045;
/// How far the clouds float above the ground (unit sphere = 1); longer
/// offsets throw the cloud shadows further from their clouds.
pub const CLOUD_HEIGHT: f32 = 0.06;
/// Darkening under a fully opaque cloud.
pub const CLOUD_SHADOW: f32 = 0.75;
/// Blinn-Phong exponent of the ocean glint.
pub const OCEAN_SHININESS: f32 = 90.0;

const DEEP_OCEAN: Vec3 = hex(0x08214f);
const SHALLOW: Vec3 = hex(0x1b7aa3);
const LOWLAND: Vec3 = hex(0x5b8f3c);
const FOREST: Vec3 = hex(0x2c5527);
const HIGHLAND: Vec3 = hex(0x7d6a4d);
const ROCK: Vec3 = hex(0x9b958d);
const SAND: Vec3 = hex(0xe0bd7e);
const DUNE: Vec3 = hex(0xbf7f45);
const ICE: Vec3 = hex(0xf0f6ff);
const CLOUD: Vec3 = hex(0xffffff);
const DUSK: Vec3 = hex(0xffb38a);
const CITY: Vec3 = hex(0xffc46b);
const GLINT: Vec3 = hex(0xfff3dc);
const SKY: Vec3 = hex(0x5fa8ff);

/// Terrain height: continents plus ridged mountain chains on land.
pub fn height(p: Vec3) -> f32 {
    let continents = fbm(p * 1.6 + Vec3::new(3.1, -1.7, 0.4), 5);
    continents + ridged(p * 5.0, 2) * 0.09 * smoothstep(SEA_LEVEL, SEA_LEVEL + 0.15, continents)
}

/// Cloud mask in [0, 1] for a point already rotated into the cloud layer.
/// `coverage` moves the threshold of an animated, domain-warped fBm.
pub fn clouds(q: Vec3, coverage: f32, t: f32) -> f32 {
    let warp = Vec3::new(
        fbm(q * 1.8 + Vec3::new(0.0, t * 0.05, 0.0), 2),
        fbm(q * 1.8 + Vec3::new(7.3, 0.0, t * 0.04), 2),
        fbm(q * 1.8 + Vec3::new(-2.1, 4.4, 0.0), 2),
    ) * 1.1;
    let n = fbm(q * Vec3::new(3.2, 6.0, 3.2) + warp, 5);
    let threshold = 0.25 - coverage * 0.5;
    let detail = 0.65 + 0.35 * noise(q * 14.0 + warp * 3.0);
    smoothstep(threshold - 0.04, threshold + 0.16, n) * detail
}

fn city_lights(p: Vec3) -> f32 {
    let clusters = smoothstep(0.0, 0.18, fbm(p * 4.0 + Vec3::splat(20.0), 2));
    let towns = smoothstep(0.25, 0.5, noise(p * 110.0));
    let sparkle = smoothstep(0.35, 0.6, noise(p * 260.0));
    clusters * (towns + sparkle * 0.6)
}

pub fn rocky(frag: &Fragment, u: &Uniforms) -> Option<Color> {
    let s = surface(frag, u)?;
    let p = s.p;
    let t = u.time;
    let coverage = u.params.cloud_coverage;

    let h = height(p);
    let land = smoothstep(SEA_LEVEL - 0.004, SEA_LEVEL + 0.012, h);
    let alt = saturate((h - SEA_LEVEL) / 0.33);
    let lat = p.y.abs();
    let moisture = fbm(p * 2.3 + Vec3::new(11.0, 3.0, 7.0), 3);

    // Layer 1, ocean: shallow turquoise shelves to deep navy by depth.
    let depth = saturate((SEA_LEVEL - h) / 0.25);
    let ocean = mix(SHALLOW, DEEP_OCEAN, depth.sqrt());

    // Layer 2, land biomes: lowlands, forests, highlands and bare rock by
    // altitude, shifted by moisture.
    let mut ground = ramp(
        &[(0.0, LOWLAND), (0.22, FOREST), (0.55, HIGHLAND), (0.85, ROCK)],
        alt + moisture * 0.4,
    );

    // Layer 3, deserts: dry, low-latitude land with dune streaks.
    let desert = smoothstep(0.0, -0.12, moisture)
        * (1.0 - smoothstep(0.2, 0.5, lat))
        * (1.0 - smoothstep(0.45, 0.75, alt));
    let dunes = 0.5 + 0.5 * noise(p * Vec3::new(40.0, 10.0, 40.0));
    ground = mix(ground, mix(SAND, DUNE, dunes), desert);

    // Layer 4, ice: noisy polar caps (over land and sea) and snowy peaks.
    let caps = smoothstep(0.76, 0.82, lat + fbm(p * 6.0, 2) * 0.15);
    let snow = smoothstep(0.6, 0.75, alt + lat * 0.35) * land;
    let ice = caps.max(snow);
    let albedo = mix(mix(ocean, ground, land), ICE, ice);

    // Bump mapping on land from the terrain gradient.
    let n = if land > 0.0 {
        bump(&s, u, BUMP_STRENGTH * land, 0.004, h, height)
    } else {
        s.n
    };

    // Layer 5, clouds drifting faster than the ground (+ their shadows,
    // sampled from the cloud layer shifted towards the light).
    let spin = t * CLOUD_SPEED;
    let cloud = clouds(rotate_y(p, spin), coverage, t);
    let l_obj = object_dir(u, s.l);
    let shadow_p = rotate_y((p + l_obj * CLOUD_HEIGHT).normalize(), spin);
    let shadow = clouds(shadow_p, coverage, t) * CLOUD_SHADOW;

    // Lighting: matte Lambert ground, Blinn-Phong glint on open water.
    let ndl = s.n.dot(s.l);
    let day = smoothstep(-0.12, 0.2, ndl);
    let diffuse = n.dot(s.l).max(0.0) * (1.0 - shadow);
    let mut c = albedo * (0.006 + diffuse);
    let water = (1.0 - land) * (1.0 - ice);
    let glint = blinn_phong(s.n, s.l, s.v, OCEAN_SHININESS) * 1.6 + blinn_phong(s.n, s.l, s.v, 10.0) * 0.06;
    c += GLINT * (glint * water * (1.0 - shadow));

    // Layer 6, city lights: only on the night side, fading smoothly
    // through the terminator and dimmed by clouds.
    let cities = city_lights(p)
        * land
        * (1.0 - ice)
        * (1.0 - desert * 0.85)
        * (1.0 - smoothstep(0.3, 0.6, alt));
    c += CITY * (cities * (1.0 - day) * (1.0 - cloud * 0.8) * 1.4);

    // Clouds on top, warm-tinted near the terminator.
    let cloud_light = saturate((ndl + 0.1) / 1.1);
    let dusk = (1.0 - (ndl * 3.0).abs()).max(0.0);
    let cloud_color = mix(CLOUD, DUSK, dusk * 0.6) * (0.006 + cloud_light);
    c = mix(c, cloud_color, cloud * 0.95);

    // Layer 7, atmosphere rim.
    c += SKY * (fresnel(s.n, s.v, 3.0) * (0.1 + 0.9 * day) * 0.7);
    Some(to_color(c, 1.0))
}
