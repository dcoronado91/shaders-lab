//! Procedural 3D noise: gradient (Perlin-style) noise, fBm, ridged fBm and
//! Worley cellular noise. All functions are deterministic and seedless.

use crate::math::Vec3;

#[inline]
pub fn hash3(x: i32, y: i32, z: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343)
        ^ (y as u32).wrapping_mul(0xd816_3841)
        ^ (z as u32).wrapping_mul(0xcb1a_b31f);
    h = (h ^ (h >> 16)).wrapping_mul(0x7feb_352d);
    h = (h ^ (h >> 15)).wrapping_mul(0x846c_a68b);
    h ^ (h >> 16)
}

/// Maps a hash to [0, 1).
#[inline]
pub fn hash01(h: u32) -> f32 {
    (h >> 8) as f32 / (1u32 << 24) as f32
}

#[inline]
fn grad(h: u32, x: f32, y: f32, z: f32) -> f32 {
    match h & 15 {
        0 | 12 => x + y,
        1 | 13 => -x + y,
        2 => x - y,
        3 => -x - y,
        4 => x + z,
        5 => -x + z,
        6 => x - z,
        7 => -x - z,
        8 => y + z,
        9 | 14 => -y + z,
        10 => y - z,
        _ => -y - z,
    }
}

#[inline]
fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Gradient noise in roughly [-1, 1].
pub fn noise(p: Vec3) -> f32 {
    let i = p.floor();
    let f = p - i;
    let (x, y, z) = (i.x as i32, i.y as i32, i.z as i32);
    let (u, v, w) = (fade(f.x), fade(f.y), fade(f.z));
    let g = |dx: i32, dy: i32, dz: i32| {
        grad(
            hash3(x + dx, y + dy, z + dz),
            f.x - dx as f32,
            f.y - dy as f32,
            f.z - dz as f32,
        )
    };
    lerp(
        lerp(lerp(g(0, 0, 0), g(1, 0, 0), u), lerp(g(0, 1, 0), g(1, 1, 0), u), v),
        lerp(lerp(g(0, 0, 1), g(1, 0, 1), u), lerp(g(0, 1, 1), g(1, 1, 1), u), v),
        w,
    )
}

/// Each octave is shifted so lattice artifacts do not line up.
const OCTAVE_SHIFT: Vec3 = Vec3::new(17.13, -9.71, 5.37);

/// Fractal Brownian motion normalized to roughly [-1, 1].
pub fn fbm(p: Vec3, octaves: u32) -> f32 {
    let (mut sum, mut amp, mut norm, mut q) = (0.0, 1.0, 0.0, p);
    for _ in 0..octaves {
        sum += amp * noise(q);
        norm += amp;
        amp *= 0.5;
        q = q * 2.03 + OCTAVE_SHIFT;
    }
    sum / norm
}

/// Ridged fBm in [0, 1]: sharp crests where the noise crosses zero.
pub fn ridged(p: Vec3, octaves: u32) -> f32 {
    let (mut sum, mut amp, mut norm, mut q) = (0.0, 1.0, 0.0, p);
    for _ in 0..octaves {
        let r = 1.0 - noise(q).abs();
        sum += amp * r * r;
        norm += amp;
        amp *= 0.5;
        q = q * 2.03 + OCTAVE_SHIFT;
    }
    sum / norm
}

pub struct Worley {
    /// Distance to the closest and second closest feature point.
    pub f1: f32,
    pub f2: f32,
    /// Hash of the closest cell, stable per cell.
    pub id: u32,
    /// Closest feature point, in the same space as the input.
    pub point: Vec3,
}

/// Cellular noise: one jittered feature point per unit cell.
pub fn worley(p: Vec3) -> Worley {
    let cell = p.floor();
    let (cx, cy, cz) = (cell.x as i32, cell.y as i32, cell.z as i32);
    let mut best = Worley {
        f1: f32::INFINITY,
        f2: f32::INFINITY,
        id: 0,
        point: p,
    };
    for dz in -1..=1 {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (x, y, z) = (cx + dx, cy + dy, cz + dz);
                let h = hash3(x, y, z);
                let jitter = Vec3::new(
                    hash01(h),
                    hash01(h.wrapping_mul(0x9e37_79b9)),
                    hash01(h.wrapping_mul(0x85eb_ca6b)),
                );
                let point = Vec3::new(x as f32, y as f32, z as f32) + jitter;
                let d = (point - p).length();
                if d < best.f1 {
                    best.f2 = best.f1;
                    best.f1 = d;
                    best.id = h;
                    best.point = point;
                } else if d < best.f2 {
                    best.f2 = d;
                }
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_is_deterministic_and_bounded() {
        for i in 0..2000 {
            let p = Vec3::new(i as f32 * 0.173, i as f32 * -0.311, i as f32 * 0.057);
            let n = noise(p);
            assert_eq!(n, noise(p));
            assert!((-1.1..=1.1).contains(&n));
            assert!((-1.1..=1.1).contains(&fbm(p, 5)));
            assert!((0.0..=1.0).contains(&ridged(p, 3)));
        }
    }

    #[test]
    fn noise_is_zero_on_lattice_points() {
        assert_eq!(noise(Vec3::new(3.0, -2.0, 7.0)), 0.0);
    }

    #[test]
    fn worley_distances_are_ordered() {
        for i in 0..200 {
            let w = worley(Vec3::new(i as f32 * 0.37, 1.3, -i as f32 * 0.21));
            assert!(w.f1 <= w.f2);
            assert!(w.f1 < 1.8);
        }
    }
}
