use crate::math::Vec3;

/// Mesh vertex. `position` and `normal` are in object space; the remaining
/// fields are filled by the vertex shader.
#[derive(Clone, Copy, Debug, Default)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub world_position: Vec3,
    pub world_normal: Vec3,
    /// Pixel coordinates (x right, y down) and NDC depth.
    pub screen_position: Vec3,
    /// 1 / clip w, used for perspective-correct interpolation.
    pub inv_w: f32,
}

impl Vertex {
    pub fn new(position: Vec3, normal: Vec3) -> Self {
        Vertex {
            position,
            normal,
            ..Default::default()
        }
    }
}
