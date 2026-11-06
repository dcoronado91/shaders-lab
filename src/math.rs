use std::ops::{Add, AddAssign, Div, Mul, MulAssign, Neg, Sub};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3::new(0.0, 0.0, 0.0);
    pub const ONE: Vec3 = Vec3::new(1.0, 1.0, 1.0);
    pub const Y: Vec3 = Vec3::new(0.0, 1.0, 0.0);

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Vec3 { x, y, z }
    }

    pub const fn splat(v: f32) -> Self {
        Vec3::new(v, v, v)
    }

    #[inline]
    pub fn dot(self, o: Vec3) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    #[inline]
    pub fn cross(self, o: Vec3) -> Vec3 {
        Vec3::new(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }

    #[inline]
    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    #[inline]
    pub fn normalize(self) -> Vec3 {
        let len = self.length();
        if len > 1e-12 { self * (1.0 / len) } else { self }
    }

    #[inline]
    pub fn lerp(self, o: Vec3, t: f32) -> Vec3 {
        self + (o - self) * t
    }

    #[inline]
    pub fn abs(self) -> Vec3 {
        Vec3::new(self.x.abs(), self.y.abs(), self.z.abs())
    }

    #[inline]
    pub fn floor(self) -> Vec3 {
        Vec3::new(self.x.floor(), self.y.floor(), self.z.floor())
    }

    #[inline]
    pub fn max_elem(self) -> f32 {
        self.x.max(self.y).max(self.z)
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    #[inline]
    fn add(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}

impl AddAssign for Vec3 {
    #[inline]
    fn add_assign(&mut self, o: Vec3) {
        *self = *self + o;
    }
}

impl Sub for Vec3 {
    type Output = Vec3;
    #[inline]
    fn sub(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}

impl Mul<f32> for Vec3 {
    type Output = Vec3;
    #[inline]
    fn mul(self, s: f32) -> Vec3 {
        Vec3::new(self.x * s, self.y * s, self.z * s)
    }
}

impl Mul<Vec3> for Vec3 {
    type Output = Vec3;
    #[inline]
    fn mul(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x * o.x, self.y * o.y, self.z * o.z)
    }
}

impl MulAssign<f32> for Vec3 {
    #[inline]
    fn mul_assign(&mut self, s: f32) {
        *self = *self * s;
    }
}

impl Div<f32> for Vec3 {
    type Output = Vec3;
    #[inline]
    fn div(self, s: f32) -> Vec3 {
        self * (1.0 / s)
    }
}

impl Neg for Vec3 {
    type Output = Vec3;
    #[inline]
    fn neg(self) -> Vec3 {
        Vec3::new(-self.x, -self.y, -self.z)
    }
}

/// 4x4 matrix stored by rows (`m[row][col]`), applied to column vectors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat4 {
    pub m: [[f32; 4]; 4],
}

impl Mat4 {
    pub const IDENTITY: Mat4 = Mat4 {
        m: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
    };

    pub fn translation(t: Vec3) -> Mat4 {
        let mut r = Mat4::IDENTITY;
        r.m[0][3] = t.x;
        r.m[1][3] = t.y;
        r.m[2][3] = t.z;
        r
    }

    pub fn scale(s: Vec3) -> Mat4 {
        let mut r = Mat4::IDENTITY;
        r.m[0][0] = s.x;
        r.m[1][1] = s.y;
        r.m[2][2] = s.z;
        r
    }

    pub fn rotation_x(a: f32) -> Mat4 {
        let (s, c) = a.sin_cos();
        let mut r = Mat4::IDENTITY;
        r.m[1][1] = c;
        r.m[1][2] = -s;
        r.m[2][1] = s;
        r.m[2][2] = c;
        r
    }

    pub fn rotation_y(a: f32) -> Mat4 {
        let (s, c) = a.sin_cos();
        let mut r = Mat4::IDENTITY;
        r.m[0][0] = c;
        r.m[0][2] = s;
        r.m[2][0] = -s;
        r.m[2][2] = c;
        r
    }

    pub fn rotation_z(a: f32) -> Mat4 {
        let (s, c) = a.sin_cos();
        let mut r = Mat4::IDENTITY;
        r.m[0][0] = c;
        r.m[0][1] = -s;
        r.m[1][0] = s;
        r.m[1][1] = c;
        r
    }

    /// Model matrix: scale, then rotation (Z·X·Y order), then translation.
    pub fn model(translation: Vec3, rotation: Vec3, scale: Vec3) -> Mat4 {
        Mat4::translation(translation)
            * Mat4::rotation_z(rotation.z)
            * Mat4::rotation_x(rotation.x)
            * Mat4::rotation_y(rotation.y)
            * Mat4::scale(scale)
    }

    /// Right-handed view matrix: the camera looks down its local -Z.
    pub fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Mat4 {
        let f = (target - eye).normalize();
        let s = f.cross(up).normalize();
        let u = s.cross(f);
        Mat4 {
            m: [
                [s.x, s.y, s.z, -s.dot(eye)],
                [u.x, u.y, u.z, -u.dot(eye)],
                [-f.x, -f.y, -f.z, f.dot(eye)],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// OpenGL-style perspective: maps the view frustum to NDC [-1, 1]^3.
    pub fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
        let f = 1.0 / (fov_y * 0.5).tan();
        Mat4 {
            m: [
                [f / aspect, 0.0, 0.0, 0.0],
                [0.0, f, 0.0, 0.0],
                [0.0, 0.0, (far + near) / (near - far), 2.0 * far * near / (near - far)],
                [0.0, 0.0, -1.0, 0.0],
            ],
        }
    }

    /// NDC to pixels: x right, y down, z kept as NDC depth.
    pub fn viewport(width: f32, height: f32) -> Mat4 {
        Mat4 {
            m: [
                [width * 0.5, 0.0, 0.0, width * 0.5],
                [0.0, -height * 0.5, 0.0, height * 0.5],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Homogeneous transform of a point (w = 1); returns (x, y, z, w).
    #[inline]
    pub fn transform4(&self, p: Vec3) -> [f32; 4] {
        let m = &self.m;
        let row = |r: usize| m[r][0] * p.x + m[r][1] * p.y + m[r][2] * p.z + m[r][3];
        [row(0), row(1), row(2), row(3)]
    }

    #[inline]
    pub fn transform_point(&self, p: Vec3) -> Vec3 {
        let [x, y, z, _] = self.transform4(p);
        Vec3::new(x, y, z)
    }

    #[inline]
    pub fn transform_dir(&self, d: Vec3) -> Vec3 {
        let m = &self.m;
        Vec3::new(
            m[0][0] * d.x + m[0][1] * d.y + m[0][2] * d.z,
            m[1][0] * d.x + m[1][1] * d.y + m[1][2] * d.z,
            m[2][0] * d.x + m[2][1] * d.y + m[2][2] * d.z,
        )
    }

    pub fn transpose(&self) -> Mat4 {
        let mut r = Mat4::IDENTITY;
        for i in 0..4 {
            for j in 0..4 {
                r.m[i][j] = self.m[j][i];
            }
        }
        r
    }

    /// General inverse by cofactors; returns the identity for singular input.
    pub fn inverse(&self) -> Mat4 {
        let a: Vec<f32> = self.m.iter().flatten().copied().collect();
        let mut inv = [0.0f32; 16];
        inv[0] = a[5] * a[10] * a[15] - a[5] * a[11] * a[14] - a[9] * a[6] * a[15]
            + a[9] * a[7] * a[14]
            + a[13] * a[6] * a[11]
            - a[13] * a[7] * a[10];
        inv[4] = -a[4] * a[10] * a[15] + a[4] * a[11] * a[14] + a[8] * a[6] * a[15]
            - a[8] * a[7] * a[14]
            - a[12] * a[6] * a[11]
            + a[12] * a[7] * a[10];
        inv[8] = a[4] * a[9] * a[15] - a[4] * a[11] * a[13] - a[8] * a[5] * a[15]
            + a[8] * a[7] * a[13]
            + a[12] * a[5] * a[11]
            - a[12] * a[7] * a[9];
        inv[12] = -a[4] * a[9] * a[14] + a[4] * a[10] * a[13] + a[8] * a[5] * a[14]
            - a[8] * a[6] * a[13]
            - a[12] * a[5] * a[10]
            + a[12] * a[6] * a[9];
        inv[1] = -a[1] * a[10] * a[15] + a[1] * a[11] * a[14] + a[9] * a[2] * a[15]
            - a[9] * a[3] * a[14]
            - a[13] * a[2] * a[11]
            + a[13] * a[3] * a[10];
        inv[5] = a[0] * a[10] * a[15] - a[0] * a[11] * a[14] - a[8] * a[2] * a[15]
            + a[8] * a[3] * a[14]
            + a[12] * a[2] * a[11]
            - a[12] * a[3] * a[10];
        inv[9] = -a[0] * a[9] * a[15] + a[0] * a[11] * a[13] + a[8] * a[1] * a[15]
            - a[8] * a[3] * a[13]
            - a[12] * a[1] * a[11]
            + a[12] * a[3] * a[9];
        inv[13] = a[0] * a[9] * a[14] - a[0] * a[10] * a[13] - a[8] * a[1] * a[14]
            + a[8] * a[2] * a[13]
            + a[12] * a[1] * a[10]
            - a[12] * a[2] * a[9];
        inv[2] = a[1] * a[6] * a[15] - a[1] * a[7] * a[14] - a[5] * a[2] * a[15]
            + a[5] * a[3] * a[14]
            + a[13] * a[2] * a[7]
            - a[13] * a[3] * a[6];
        inv[6] = -a[0] * a[6] * a[15] + a[0] * a[7] * a[14] + a[4] * a[2] * a[15]
            - a[4] * a[3] * a[14]
            - a[12] * a[2] * a[7]
            + a[12] * a[3] * a[6];
        inv[10] = a[0] * a[5] * a[15] - a[0] * a[7] * a[13] - a[4] * a[1] * a[15]
            + a[4] * a[3] * a[13]
            + a[12] * a[1] * a[7]
            - a[12] * a[3] * a[5];
        inv[14] = -a[0] * a[5] * a[14] + a[0] * a[6] * a[13] + a[4] * a[1] * a[14]
            - a[4] * a[2] * a[13]
            - a[12] * a[1] * a[6]
            + a[12] * a[2] * a[5];
        inv[3] = -a[1] * a[6] * a[11] + a[1] * a[7] * a[10] + a[5] * a[2] * a[11]
            - a[5] * a[3] * a[10]
            - a[9] * a[2] * a[7]
            + a[9] * a[3] * a[6];
        inv[7] = a[0] * a[6] * a[11] - a[0] * a[7] * a[10] - a[4] * a[2] * a[11]
            + a[4] * a[3] * a[10]
            + a[8] * a[2] * a[7]
            - a[8] * a[3] * a[6];
        inv[11] = -a[0] * a[5] * a[11] + a[0] * a[7] * a[9] + a[4] * a[1] * a[11]
            - a[4] * a[3] * a[9]
            - a[8] * a[1] * a[7]
            + a[8] * a[3] * a[5];
        inv[15] = a[0] * a[5] * a[10] - a[0] * a[6] * a[9] - a[4] * a[1] * a[10]
            + a[4] * a[2] * a[9]
            + a[8] * a[1] * a[6]
            - a[8] * a[2] * a[5];

        let det = a[0] * inv[0] + a[1] * inv[4] + a[2] * inv[8] + a[3] * inv[12];
        if det.abs() < 1e-20 {
            return Mat4::IDENTITY;
        }
        let mut r = Mat4::IDENTITY;
        for (i, value) in inv.iter().enumerate() {
            r.m[i / 4][i % 4] = value / det;
        }
        r
    }
}

impl Mul for Mat4 {
    type Output = Mat4;
    fn mul(self, o: Mat4) -> Mat4 {
        let mut r = Mat4 { m: [[0.0; 4]; 4] };
        for i in 0..4 {
            for j in 0..4 {
                r.m[i][j] = (0..4).map(|k| self.m[i][k] * o.m[k][j]).sum();
            }
        }
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-4
    }

    #[test]
    fn inverse_undoes_model_matrix() {
        let m = Mat4::model(
            Vec3::new(1.0, -2.0, 3.0),
            Vec3::new(0.3, 1.1, -0.4),
            Vec3::new(2.0, 0.5, 1.5),
        );
        let p = Vec3::new(0.7, -0.2, 0.9);
        assert!(close(m.inverse().transform_point(m.transform_point(p)), p));
    }

    #[test]
    fn look_at_puts_target_in_front() {
        let view = Mat4::look_at(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::Y);
        assert!(close(view.transform_point(Vec3::ZERO), Vec3::new(0.0, 0.0, -5.0)));
    }

    #[test]
    fn perspective_maps_near_and_far_planes() {
        let proj = Mat4::perspective(1.0, 1.5, 0.1, 100.0);
        let ndc_z = |z: f32| {
            let [_, _, cz, cw] = proj.transform4(Vec3::new(0.0, 0.0, z));
            cz / cw
        };
        assert!((ndc_z(-0.1) + 1.0).abs() < 1e-4);
        assert!((ndc_z(-100.0) - 1.0).abs() < 1e-3);
    }

    #[test]
    fn viewport_maps_ndc_corners_to_pixels() {
        let vp = Mat4::viewport(800.0, 600.0);
        assert!(close(vp.transform_point(Vec3::new(-1.0, 1.0, 0.0)), Vec3::ZERO));
        assert!(close(
            vp.transform_point(Vec3::new(1.0, -1.0, 0.0)),
            Vec3::new(800.0, 600.0, 0.0)
        ));
    }
}
