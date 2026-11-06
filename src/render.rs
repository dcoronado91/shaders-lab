use std::sync::Mutex;

use raylib::prelude::Color;

use crate::fragment::Fragment;
use crate::framebuffer::Framebuffer;
use crate::math::{Mat4, Vec3};
use crate::raster::{signed_area2, triangle_barycentric};
use crate::vertex::Vertex;

pub type FragmentShader = fn(&Fragment, &Uniforms) -> Option<Color>;

/// Values the user can tweak at run time or that tune each shader.
#[derive(Clone, Copy, Debug)]
pub struct Params {
    /// Fraction of the rocky planet covered by clouds, 0..1.
    pub cloud_coverage: f32,
}

impl Default for Params {
    fn default() -> Self {
        Params {
            cloud_coverage: 0.42,
        }
    }
}

/// Description of the body a draw belongs to, in world space. Shaders use
/// it to clip the silhouette to a perfect sphere, place halos and compute
/// ring shadows.
#[derive(Clone, Copy, Debug)]
pub struct BodyInfo {
    pub center: Vec3,
    pub radius: f32,
    /// Normal of the ring plane (also the spin axis of the planet).
    pub axis: Vec3,
    /// Halo color and opacity for atmosphere shells.
    pub glow: Vec3,
    pub glow_strength: f32,
    /// Outer halo radius, in body radii.
    pub glow_extent: f32,
    /// Whether the halo is brighter on the lit side.
    pub glow_lit: bool,
}

impl Default for BodyInfo {
    fn default() -> Self {
        BodyInfo {
            center: Vec3::ZERO,
            radius: 1.0,
            axis: Vec3::Y,
            glow: Vec3::ONE,
            glow_strength: 0.0,
            glow_extent: 1.15,
            glow_lit: true,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Uniforms {
    pub model: Mat4,
    pub model_inv: Mat4,
    pub normal_matrix: Mat4,
    pub view: Mat4,
    pub projection: Mat4,
    pub viewport: Mat4,
    pub time: f32,
    pub light_pos: Vec3,
    pub camera_pos: Vec3,
    pub params: Params,
    pub body: BodyInfo,
}

impl Uniforms {
    /// Copy with a different model matrix (and its derived matrices).
    pub fn with_model(&self, model: Mat4, body: BodyInfo) -> Uniforms {
        let model_inv = model.inverse();
        Uniforms {
            model,
            model_inv,
            normal_matrix: model_inv.transpose(),
            body,
            ..*self
        }
    }

    pub fn light_dir(&self, world_pos: Vec3) -> Vec3 {
        (self.light_pos - world_pos).normalize()
    }
}

pub struct DrawCall<'a> {
    pub vertices: &'a [Vertex],
    pub uniforms: Uniforms,
    pub shader: FragmentShader,
    /// Transparent draws blend with alpha and do not write depth.
    pub transparent: bool,
}

/// Object space -> world -> clip -> NDC -> screen.
pub fn vertex_shader(vertex: &Vertex, u: &Uniforms) -> Vertex {
    let world_position = u.model.transform_point(vertex.position);
    let world_normal = u.normal_matrix.transform_dir(vertex.normal).normalize();
    let view_pos = u.view.transform_point(world_position);
    let [cx, cy, cz, cw] = u.projection.transform4(view_pos);
    let inv_w = 1.0 / cw;
    let screen_position = u
        .viewport
        .transform_point(Vec3::new(cx * inv_w, cy * inv_w, cz * inv_w));
    Vertex {
        world_position,
        world_normal,
        screen_position,
        inv_w,
        ..*vertex
    }
}

struct Triangle {
    draw: usize,
    v: [Vertex; 3],
    y_min: f32,
    y_max: f32,
}

/// Rows per work unit; small enough to balance the threads.
const BAND_ROWS: usize = 8;
const NEAR_W: f32 = 0.01;

/// Runs the pipeline for every draw, in order: vertex shading, primitive
/// assembly with back-face and near-plane culling, then rasterization and
/// fragment shading in parallel horizontal bands with an early depth test.
pub fn render(framebuffer: &mut Framebuffer, draws: &[DrawCall]) {
    let mut triangles = Vec::new();
    for (index, draw) in draws.iter().enumerate() {
        let shaded: Vec<Vertex> = draw
            .vertices
            .iter()
            .map(|v| vertex_shader(v, &draw.uniforms))
            .collect();
        for tri in shaded.chunks_exact(3) {
            if tri.iter().any(|v| v.inv_w <= 0.0 || 1.0 / v.inv_w < NEAR_W) {
                continue;
            }
            if signed_area2(&tri[0], &tri[1], &tri[2]) >= 0.0 {
                continue;
            }
            let ys = tri.iter().map(|v| v.screen_position.y);
            triangles.push(Triangle {
                draw: index,
                v: [tri[0], tri[1], tri[2]],
                y_min: ys.clone().fold(f32::INFINITY, f32::min),
                y_max: ys.fold(f32::NEG_INFINITY, f32::max),
            });
        }
    }

    let width = framebuffer.width as usize;
    let height = framebuffer.height as usize;
    let jobs: Vec<_> = framebuffer
        .color
        .chunks_mut(BAND_ROWS * width * 4)
        .zip(framebuffer.depth.chunks_mut(BAND_ROWS * width))
        .enumerate()
        .collect();
    let queue = Mutex::new(jobs);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());

    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                loop {
                    let Some((band, (color, depth))) = queue.lock().unwrap().pop() else {
                        break;
                    };
                    let row_start = band * BAND_ROWS;
                    let row_end = (row_start + BAND_ROWS).min(height);
                    shade_band(draws, &triangles, width, row_start, row_end, color, depth);
                }
            });
        }
    });
}

fn shade_band(
    draws: &[DrawCall],
    triangles: &[Triangle],
    width: usize,
    row_start: usize,
    row_end: usize,
    color: &mut [u8],
    depth: &mut [f32],
) {
    for tri in triangles {
        if tri.y_max < row_start as f32 || tri.y_min > row_end as f32 {
            continue;
        }
        let draw = &draws[tri.draw];
        let u = &draw.uniforms;
        let [a, b, c] = &tri.v;
        triangle_barycentric(
            [a, b, c],
            width as i32,
            row_start as i32,
            row_end as i32,
            |x, y, z, w| {
                let index = (y as usize - row_start) * width + x as usize;
                if z >= depth[index] || !(-1.0..=1.0).contains(&z) {
                    return;
                }
                let mix = |pa: Vec3, pb: Vec3, pc: Vec3| pa * w[0] + pb * w[1] + pc * w[2];
                let world_pos = mix(a.world_position, b.world_position, c.world_position);
                let normal = mix(a.world_normal, b.world_normal, c.world_normal);
                let fragment = Fragment {
                    x,
                    y,
                    depth: z,
                    obj_pos: mix(a.position, b.position, c.position),
                    world_pos,
                    normal,
                    intensity: normal.normalize().dot(u.light_dir(world_pos)).max(0.0),
                };
                let Some(out) = (draw.shader)(&fragment, u) else {
                    return;
                };
                let px = &mut color[index * 4..index * 4 + 4];
                if draw.transparent {
                    let alpha = out.a as f32 / 255.0;
                    for (dst, src) in px.iter_mut().zip([out.r, out.g, out.b]) {
                        *dst = (src as f32 * alpha + *dst as f32 * (1.0 - alpha)).round() as u8;
                    }
                } else {
                    px.copy_from_slice(&[out.r, out.g, out.b, 255]);
                    depth[index] = z;
                }
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obj::Obj;

    fn flat(_: &Fragment, _: &Uniforms) -> Option<Color> {
        Some(Color::WHITE)
    }

    fn uniforms(width: f32, height: f32) -> Uniforms {
        let eye = Vec3::new(0.0, 0.0, 4.0);
        let base = Uniforms {
            model: Mat4::IDENTITY,
            model_inv: Mat4::IDENTITY,
            normal_matrix: Mat4::IDENTITY,
            view: Mat4::look_at(eye, Vec3::ZERO, Vec3::Y),
            projection: Mat4::perspective(0.8, width / height, 0.1, 50.0),
            viewport: Mat4::viewport(width, height),
            time: 0.0,
            light_pos: Vec3::new(0.0, 0.0, 100.0),
            camera_pos: eye,
            params: Params::default(),
            body: BodyInfo::default(),
        };
        base.with_model(Mat4::IDENTITY, BodyInfo::default())
    }

    #[test]
    fn sphere_front_faces_survive_culling() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/models/sphere.obj");
        let vertices = Obj::load(path).unwrap().get_vertex_array();
        let u = uniforms(64.0, 64.0);
        let mut fb = Framebuffer::new(64, 64, Color::BLACK);
        let draw = DrawCall {
            vertices: &vertices,
            uniforms: u,
            shader: flat,
            transparent: false,
        };
        render(&mut fb, &[draw]);
        assert_eq!(fb.get_color(32, 32), Color::WHITE);
        assert_eq!(fb.get_color(1, 1), Color::BLACK);
        // The visible surface is the near side: closer than the center.
        let [_, _, cz, cw] = u.projection.transform4(u.view.transform_point(Vec3::ZERO));
        assert!(fb.get_depth(32, 32) < cz / cw);
    }
}
