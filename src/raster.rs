//! Bounding-box triangle fill with edge functions (barycentric coordinates).

use crate::math::Vec3;
use crate::vertex::Vertex;

/// `E(p) = (b - a) x (p - a)`; positive inside a positively oriented triangle.
#[inline]
fn edge(a: Vec3, b: Vec3, px: f32, py: f32) -> f32 {
    (b.x - a.x) * (py - a.y) - (b.y - a.y) * (px - a.x)
}

/// Top-left rule in y-down screen space: pixels exactly on a top or left
/// edge belong to the triangle, so shared edges are covered only once.
#[inline]
fn is_top_left(a: Vec3, b: Vec3) -> bool {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    dy < 0.0 || (dy == 0.0 && dx > 0.0)
}

/// Twice the signed screen-space area; negative for counter-clockwise
/// triangles in NDC (y up), which are the front faces.
pub fn signed_area2(a: &Vertex, b: &Vertex, c: &Vertex) -> f32 {
    let (a, b, c) = (a.screen_position, b.screen_position, c.screen_position);
    edge(a, b, c.x, c.y)
}

/// Visits every pixel center covered by the triangle in rows
/// `[row_start, row_end)`. `emit` receives the pixel, the interpolated NDC
/// depth and perspective-correct barycentric weights for `(v0, v1, v2)`.
pub fn triangle_barycentric(
    v: [&Vertex; 3],
    width: i32,
    row_start: i32,
    row_end: i32,
    mut emit: impl FnMut(i32, i32, f32, [f32; 3]),
) {
    let mut order = [0usize, 1, 2];
    let mut area = signed_area2(v[0], v[1], v[2]);
    if area.abs() < 1e-9 {
        return;
    }
    if area < 0.0 {
        order.swap(1, 2);
        area = -area;
    }
    let p = order.map(|i| v[i].screen_position);
    let inv_w = order.map(|i| v[i].inv_w);
    let inv_area = 1.0 / area;

    let min_x = p[0].x.min(p[1].x).min(p[2].x);
    let max_x = p[0].x.max(p[1].x).max(p[2].x);
    let min_y = p[0].y.min(p[1].y).min(p[2].y);
    let max_y = p[0].y.max(p[1].y).max(p[2].y);
    let x0 = ((min_x - 0.5).ceil() as i32).max(0);
    let x1 = ((max_x - 0.5).floor() as i32).min(width - 1);
    let y0 = ((min_y - 0.5).ceil() as i32).max(row_start);
    let y1 = ((max_y - 0.5).floor() as i32).min(row_end - 1);
    if x0 > x1 || y0 > y1 {
        return;
    }

    let edges = [(p[1], p[2]), (p[2], p[0]), (p[0], p[1])];
    let top_left = edges.map(|(a, b)| is_top_left(a, b));
    for y in y0..=y1 {
        let py = y as f32 + 0.5;
        for x in x0..=x1 {
            let px = x as f32 + 0.5;
            let w = edges.map(|(a, b)| edge(a, b, px, py));
            let inside = (0..3).all(|i| w[i] > 0.0 || (w[i] == 0.0 && top_left[i]));
            if !inside {
                continue;
            }
            let l = w.map(|wi| wi * inv_area);
            let depth = l[0] * p[0].z + l[1] * p[1].z + l[2] * p[2].z;
            let q = [l[0] * inv_w[0], l[1] * inv_w[1], l[2] * inv_w[2]];
            let s = 1.0 / (q[0] + q[1] + q[2]);
            let mut weights = [0.0; 3];
            for (k, &i) in order.iter().enumerate() {
                weights[i] = q[k] * s;
            }
            emit(x, y, depth, weights);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen(x: f32, y: f32) -> Vertex {
        Vertex {
            screen_position: Vec3::new(x, y, 0.0),
            inv_w: 1.0,
            ..Default::default()
        }
    }

    #[test]
    fn shared_edges_are_covered_exactly_once() {
        let (a, b, c, d) = (screen(1.0, 1.0), screen(9.0, 1.0), screen(9.0, 9.0), screen(1.0, 9.0));
        let mut hits = vec![0u8; 100];
        for tri in [[&a, &b, &c], [&a, &c, &d]] {
            triangle_barycentric(tri, 10, 0, 10, |x, y, _, _| hits[(y * 10 + x) as usize] += 1);
        }
        let covered: Vec<_> = hits.iter().filter(|&&h| h > 0).collect();
        assert_eq!(covered.len(), 64);
        assert!(covered.iter().all(|&&h| h == 1));
    }

    #[test]
    fn weights_sum_to_one_and_respect_rows() {
        let (a, b, c) = (screen(0.0, 0.0), screen(20.0, 0.0), screen(0.0, 20.0));
        triangle_barycentric([&a, &b, &c], 32, 5, 8, |_, y, _, w| {
            assert!((5..8).contains(&y));
            assert!((w[0] + w[1] + w[2] - 1.0).abs() < 1e-5);
        });
    }
}
