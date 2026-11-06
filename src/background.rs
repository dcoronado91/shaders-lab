//! Static space backdrop: faint nebula plus a hashed starfield. Generated
//! once per framebuffer size and restored on every clear.

use crate::math::Vec3;
use crate::noise::{fbm, hash01, hash3};
use crate::shaders::common::{hex, smoothstep, to_color};

const SPACE: Vec3 = hex(0x04050c);
const NEBULA_VIOLET: Vec3 = hex(0x3a1850);
const NEBULA_TEAL: Vec3 = hex(0x0e3240);
const STAR_COOL: Vec3 = hex(0xcfe0ff);
const STAR_WARM: Vec3 = hex(0xffe2b8);

pub fn starfield(width: i32, height: i32) -> Vec<u8> {
    let scale = 3.0 / height as f32;
    let mut light = vec![Vec3::ZERO; (width * height) as usize];
    for y in 0..height {
        for x in 0..width {
            let q = Vec3::new(x as f32 * scale, y as f32 * scale, 0.37);
            let violet = smoothstep(0.0, 0.35, fbm(q * 0.8, 5));
            let teal = smoothstep(0.05, 0.4, fbm(q * 1.1 + Vec3::new(4.0, -2.0, 9.0), 5));
            let mut c = SPACE + NEBULA_VIOLET * (violet * 0.2) + NEBULA_TEAL * (teal * 0.18);

            let h = hash3(x, y, 11);
            if hash01(h) > 0.9975 {
                let tone = if h & 1 == 0 { STAR_COOL } else { STAR_WARM };
                c += tone * hash01(h.rotate_left(7)).powi(3) * 1.5;
            }
            light[(y * width + x) as usize] = c;
        }
    }

    // A few bright stars with a small soft glow.
    for i in 0..70 {
        let h = hash3(i, 3, 5);
        let (cx, cy) = (
            (hash01(h) * width as f32) as i32,
            (hash01(h.rotate_left(11)) * height as f32) as i32,
        );
        let tone = if i % 3 == 0 { STAR_WARM } else { STAR_COOL };
        let power = 0.6 + hash01(h.rotate_left(19)) * 1.6;
        for dy in -3..=3 {
            for dx in -3..=3 {
                let (x, y) = (cx + dx, cy + dy);
                if x < 0 || y < 0 || x >= width || y >= height {
                    continue;
                }
                let r2 = (dx * dx + dy * dy) as f32;
                let cross = if dx == 0 || dy == 0 { 0.25 } else { 0.0 };
                light[(y * width + x) as usize] += tone * (power * ((-r2 * 0.9).exp() + cross / (1.0 + r2)));
            }
        }
    }

    light
        .iter()
        .flat_map(|&c| {
            let color = to_color(c, 1.0);
            [color.r, color.g, color.b, 255]
        })
        .collect()
}
