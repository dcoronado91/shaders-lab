use std::fs;
use std::path::Path;

use crate::math::Vec3;
use crate::vertex::Vertex;

/// Minimal Wavefront OBJ mesh: `v`, `vn` and `f` statements. Texture
/// coordinates, groups and materials are ignored; polygons are split as a
/// triangle fan.
pub struct Obj {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    /// Per corner: (position index, normal index if present).
    pub triangles: Vec<[(usize, Option<usize>); 3]>,
}

impl Obj {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let source = fs::read_to_string(path)
            .map_err(|err| format!("cannot read {}: {err}", path.display()))?;
        Self::parse(&source).map_err(|err| format!("{}: {err}", path.display()))
    }

    pub fn parse(source: &str) -> Result<Self, String> {
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut triangles = Vec::new();

        for (number, raw_line) in source.lines().enumerate() {
            let line_number = number + 1;
            let mut tokens = raw_line.split_whitespace();
            match tokens.next() {
                Some(kind @ ("v" | "vn")) => {
                    let mut c = [0.0f32; 3];
                    for coord in &mut c {
                        *coord = tokens
                            .next()
                            .and_then(|t| t.parse().ok())
                            .ok_or_else(|| format!("line {line_number}: invalid {kind}"))?;
                    }
                    let v = Vec3::new(c[0], c[1], c[2]);
                    if kind == "v" {
                        positions.push(v);
                    } else {
                        normals.push(v);
                    }
                }
                Some("f") => {
                    let face = tokens
                        .map(|t| parse_corner(t, positions.len(), normals.len()))
                        .collect::<Option<Vec<_>>>()
                        .ok_or_else(|| format!("line {line_number}: invalid face index"))?;
                    if face.len() < 3 {
                        return Err(format!("line {line_number}: face needs 3 vertices"));
                    }
                    for i in 1..face.len() - 1 {
                        triangles.push([face[0], face[i], face[i + 1]]);
                    }
                }
                _ => {}
            }
        }

        Ok(Obj {
            positions,
            normals,
            triangles,
        })
    }

    /// Flat vertex array, three vertices per triangle. Normals are averaged
    /// over every corner that shares a position, so a mesh exported with flat
    /// normals still shades smoothly; positions are left untouched.
    pub fn get_vertex_array(&self) -> Vec<Vertex> {
        let mut smooth = vec![Vec3::ZERO; self.positions.len()];
        for tri in &self.triangles {
            let [a, b, c] = tri.map(|(p, _)| self.positions[p]);
            let face = (b - a).cross(c - a).normalize();
            for &(p, n) in tri {
                smooth[p] += n.map_or(face, |n| self.normals[n]);
            }
        }
        self.triangles
            .iter()
            .flatten()
            .map(|&(p, _)| Vertex::new(self.positions[p], smooth[p].normalize()))
            .collect()
    }

    /// Radius of the largest origin-centred sphere inside the mesh: the
    /// smallest distance from the origin to a face plane.
    pub fn inner_radius(&self) -> f32 {
        self.triangles
            .iter()
            .map(|tri| {
                let [a, b, c] = tri.map(|(p, _)| self.positions[p]);
                let n = (b - a).cross(c - a).normalize();
                a.dot(n).abs()
            })
            .fold(f32::INFINITY, f32::min)
    }
}

/// Parses `7`, `7/1`, `7//2`, `7/1/2` or negative indices into zero-based
/// (position, normal) indices.
fn parse_corner(token: &str, position_count: usize, normal_count: usize) -> Option<(usize, Option<usize>)> {
    let mut parts = token.split('/');
    let position = resolve(parts.next()?, position_count)?;
    let normal = match parts.nth(1) {
        Some(n) if !n.is_empty() => Some(resolve(n, normal_count)?),
        _ => None,
    };
    Some((position, normal))
}

fn resolve(token: &str, count: usize) -> Option<usize> {
    let index: i64 = token.parse().ok()?;
    let resolved = if index > 0 { index - 1 } else { count as i64 + index };
    (0..count as i64).contains(&resolved).then_some(resolved as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_slash_formats_and_triangulates_quads() {
        let source = "\
v 0 0 0
v 1 0 0
v 1 1 0
v 0 1 0
vn 0 0 1
f 1/1/1 2/2/1 3/3/1 4/4/1
f 1//1 -3//1 -2
";
        let obj = Obj::parse(source).unwrap();
        assert_eq!(obj.positions.len(), 4);
        assert_eq!(obj.triangles.len(), 3);
        assert_eq!(obj.triangles[0][1], (1, Some(0)));
        assert_eq!(obj.triangles[2][2], (2, None));
    }

    #[test]
    fn rejects_out_of_range_indices() {
        assert!(Obj::parse("v 0 0 0\nf 1 2 3\n").is_err());
        assert!(Obj::parse("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1//2 2 3\n").is_err());
    }

    #[test]
    fn loads_lab_sphere_with_smooth_normals() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/models/sphere.obj");
        let obj = Obj::load(path).unwrap();
        assert_eq!(obj.positions.len(), 98);
        assert_eq!(obj.triangles.len(), 192);
        let vertices = obj.get_vertex_array();
        assert_eq!(vertices.len(), 192 * 3);
        // Averaged normals point (almost) radially outwards.
        for v in &vertices {
            assert!(v.normal.dot(v.position.normalize()) > 0.97);
        }
        let inner = obj.inner_radius();
        assert!(inner > 0.75 && inner < 0.84, "inner radius {inner}");
    }
}
