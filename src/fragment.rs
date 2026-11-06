use crate::math::Vec3;

/// Candidate pixel produced by the rasterizer, with attributes interpolated
/// in a perspective-correct way.
#[derive(Clone, Copy, Debug)]
pub struct Fragment {
    pub x: i32,
    pub y: i32,
    /// NDC depth (smaller is closer).
    pub depth: f32,
    pub obj_pos: Vec3,
    pub world_pos: Vec3,
    /// Interpolated world-space normal (not normalized).
    pub normal: Vec3,
    /// Lambert term `max(0, n·l)` towards the scene light.
    pub intensity: f32,
}
