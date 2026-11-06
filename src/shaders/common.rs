use raylib::prelude::Color;

use crate::fragment::Fragment;
use crate::math::Vec3;
use crate::render::Uniforms;

/// sRGB hex color to (approximately) linear RGB, gamma 2.
pub const fn hex(rgb: u32) -> Vec3 {
    let r = ((rgb >> 16) & 255) as f32 / 255.0;
    let g = ((rgb >> 8) & 255) as f32 / 255.0;
    let b = (rgb & 255) as f32 / 255.0;
    Vec3::new(r * r, g * g, b * b)
}

#[inline]
pub fn saturate(x: f32) -> f32 {
    x.clamp(0.0, 1.0)
}

#[inline]
pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = saturate((x - e0) / (e1 - e0));
    t * t * (3.0 - 2.0 * t)
}

#[inline]
pub fn mix(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    a.lerp(b, saturate(t))
}

/// Piecewise-linear color ramp; `stops` sorted by position.
pub fn ramp(stops: &[(f32, Vec3)], t: f32) -> Vec3 {
    let (first, last) = (stops[0], stops[stops.len() - 1]);
    if t <= first.0 {
        return first.1;
    }
    for pair in stops.windows(2) {
        let ((t0, c0), (t1, c1)) = (pair[0], pair[1]);
        if t <= t1 {
            return c0.lerp(c1, (t - t0) / (t1 - t0));
        }
    }
    last.1
}

/// Cosine palette `a + b·cos(2π(c·t + d))`, used for thin-film sheens.
pub fn cosine_palette(t: f32, a: Vec3, b: Vec3, c: Vec3, d: Vec3) -> Vec3 {
    let tau = std::f32::consts::TAU;
    let arg = (c * t + d) * tau;
    a + b * Vec3::new(arg.x.cos(), arg.y.cos(), arg.z.cos())
}

const EXPOSURE: f32 = 1.6;

/// Linear HDR color to an sRGB `Color`: exponential tone map and gamma 2.
pub fn to_color(c: Vec3, alpha: f32) -> Color {
    let map = |x: f32| {
        let v = (1.0 - (-x.max(0.0) * EXPOSURE).exp()).sqrt();
        (v * 255.0).round() as u8
    };
    Color::new(map(c.x), map(c.y), map(c.z), (saturate(alpha) * 255.0).round() as u8)
}

/// Data every body shader starts from.
pub struct Surface {
    /// Point on the unit sphere in object space: `normalize(obj_pos)`.
    pub p: Vec3,
    /// World normal: `normalize(interpolated normal)`.
    pub n: Vec3,
    /// Directions towards the viewer and the light, world space.
    pub v: Vec3,
    pub l: Vec3,
}

/// Builds the surface data, or `None` when the view ray misses the ideal
/// sphere. The mesh is scaled to enclose that sphere, so discarding those
/// fragments turns the low-poly outline into a round silhouette.
pub fn surface(frag: &Fragment, u: &Uniforms) -> Option<Surface> {
    if misses_sphere(u.camera_pos, frag.world_pos, u.body.center, u.body.radius) {
        return None;
    }
    Some(Surface {
        p: frag.obj_pos.normalize(),
        n: frag.normal.normalize(),
        v: (u.camera_pos - frag.world_pos).normalize(),
        l: u.light_dir(frag.world_pos),
    })
}

/// Squared distance between `center` and the ray from `eye` through `point`.
pub fn ray_distance2(eye: Vec3, point: Vec3, center: Vec3) -> f32 {
    let d = (point - eye).normalize();
    let oc = center - eye;
    let along = oc.dot(d);
    (oc.dot(oc) - along * along).max(0.0)
}

pub fn misses_sphere(eye: Vec3, point: Vec3, center: Vec3, radius: f32) -> bool {
    ray_distance2(eye, point, center) > radius * radius
}

/// Two unit tangents perpendicular to `p`.
pub fn tangent_frame(p: Vec3) -> (Vec3, Vec3) {
    let helper = if p.y.abs() < 0.95 { Vec3::Y } else { Vec3::new(1.0, 0.0, 0.0) };
    let t = helper.cross(p).normalize();
    (t, p.cross(t))
}

/// World direction expressed in object space (unit length).
pub fn object_dir(u: &Uniforms, world_dir: Vec3) -> Vec3 {
    u.model_inv.transform_dir(world_dir).normalize()
}

/// Bump mapping: perturbs the world normal with the gradient of a height
/// field on the sphere, measured with forward finite differences along two
/// tangents. `h0` is `height(p)`, already computed by the caller.
pub fn bump(
    s: &Surface,
    u: &Uniforms,
    strength: f32,
    eps: f32,
    h0: f32,
    height: impl Fn(Vec3) -> f32,
) -> Vec3 {
    let (t, b) = tangent_frame(s.p);
    let dt = (height((s.p + t * eps).normalize()) - h0) / eps;
    let db = (height((s.p + b * eps).normalize()) - h0) / eps;
    let delta_obj = (t * dt + b * db) * -strength;
    (s.n + u.normal_matrix.transform_dir(delta_obj)).normalize()
}

pub fn blinn_phong(n: Vec3, l: Vec3, v: Vec3, shininess: f32) -> f32 {
    let h = (l + v).normalize();
    // Fades in past the terminator instead of switching on abruptly.
    n.dot(h).max(0.0).powf(shininess) * smoothstep(0.0, 0.15, n.dot(l))
}

pub fn fresnel(n: Vec3, v: Vec3, power: f32) -> f32 {
    (1.0 - n.dot(v).max(0.0)).powf(power)
}

pub fn rotate_y(p: Vec3, a: f32) -> Vec3 {
    let (s, c) = a.sin_cos();
    Vec3::new(p.x * c + p.z * s, p.y, -p.x * s + p.z * c)
}

/// Rodrigues rotation of `p` around the unit `axis`.
pub fn rotate_axis(p: Vec3, axis: Vec3, a: f32) -> Vec3 {
    let (s, c) = a.sin_cos();
    p * c + axis.cross(p) * s + axis * (axis.dot(p) * (1.0 - c))
}

/// Soft shadow from a sphere: 0 when `dir` from `origin` hits its core,
/// 1 when it clears it by `softness` radii.
pub fn sphere_shadow(origin: Vec3, dir: Vec3, center: Vec3, radius: f32, softness: f32) -> f32 {
    let oc = center - origin;
    let along = oc.dot(dir);
    if along <= 0.0 {
        return 1.0;
    }
    let dist = (oc.dot(oc) - along * along).max(0.0).sqrt();
    smoothstep(radius * (1.0 - softness), radius * (1.0 + softness), dist)
}
