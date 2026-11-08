//! Wavefront OBJ loading.
//!
//! Only geometry is read: `v`, `vt`, `vn` and `f`. Object and group names,
//! smoothing groups and material references are skipped, because materials are
//! chosen by the draw call rather than by the file. Faces may be triangles or
//! larger polygons; polygons are triangulated as a fan.
//!
//! Texture coordinates are flipped on the V axis: OBJ counts V upwards from the
//! bottom-left corner, while [`Texture`](super::Texture) and [`crate::Image`]
//! count it downwards from the top-left. Flipping here means a model maps the
//! way its author saw it in a modelling tool.

use std::collections::HashMap;
use std::error::Error as StdError;
use std::fmt;
use std::io;
use std::path::Path;

use glam::{Vec2, Vec3};

use super::mesh::{Mesh, Vertex};

/// What went wrong while reading an OBJ model.
#[derive(Debug)]
pub enum ObjError {
    /// The file could not be read from disk.
    Io(io::Error),
    /// A line was malformed.
    Syntax {
        /// One-based line number in the source.
        line: usize,
        /// What was wrong with it.
        message: String,
    },
    /// The file parsed, but contains no faces to draw.
    Empty,
}

impl fmt::Display for ObjError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ObjError::Io(err) => write!(f, "could not read the OBJ file: {err}"),
            ObjError::Syntax { line, message } => write!(f, "OBJ line {line}: {message}"),
            ObjError::Empty => write!(f, "the OBJ file has no faces"),
        }
    }
}

impl StdError for ObjError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            ObjError::Io(err) => Some(err),
            ObjError::Syntax { .. } | ObjError::Empty => None,
        }
    }
}

impl From<io::Error> for ObjError {
    fn from(err: io::Error) -> Self {
        ObjError::Io(err)
    }
}

/// One face corner: indices into the position, UV and normal lists.
#[derive(Debug, Clone, Copy)]
struct Corner {
    position: usize,
    uv: Option<usize>,
    normal: Option<usize>,
}

/// Parses OBJ text into a triangle mesh.
pub(crate) fn parse(source: &str) -> Result<Mesh, ObjError> {
    let mut positions: Vec<Vec3> = Vec::new();
    let mut uvs: Vec<Vec2> = Vec::new();
    let mut normals: Vec<Vec3> = Vec::new();

    let mut vertices: Vec<Vertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    // Vertices are shared when the position, UV and normal indices all match.
    let mut seen: HashMap<(usize, usize, usize), u32> = HashMap::new();

    let mut saw_face = false;

    for (index, raw) in source.lines().enumerate() {
        let line = index + 1;
        // A `#` starts a comment anywhere on the line.
        let text = raw.split('#').next().unwrap_or("").trim();

        if text.is_empty() {
            continue;
        }

        let mut parts = text.split_whitespace();
        let keyword = parts.next().unwrap_or("");

        match keyword {
            "v" => positions.push(read_vec3(parts, line, "vertex")?),
            "vt" => {
                let u = read_number(parts.next(), line, "texture coordinate")?;
                let v = read_number(parts.next(), line, "texture coordinate")?;
                // OBJ has V pointing up; textures here have it pointing down.
                uvs.push(Vec2::new(u, 1.0 - v));
            }
            "vn" => normals.push(read_vec3(parts, line, "normal")?),
            "f" => {
                saw_face = true;
                read_face(
                    parts,
                    line,
                    &positions,
                    &uvs,
                    &normals,
                    &mut vertices,
                    &mut indices,
                    &mut seen,
                )?;
            }
            // `o`, `g`, `s`, `usemtl`, `mtllib`, `l`, `p` and anything else are
            // deliberately ignored: the renderer has no notion of them.
            _ => {}
        }
    }

    if !saw_face {
        return Err(ObjError::Empty);
    }

    Ok(Mesh::from_vertices(vertices, indices))
}

fn read_face(
    parts: std::str::SplitWhitespace<'_>,
    line: usize,
    positions: &[Vec3],
    uvs: &[Vec2],
    normals: &[Vec3],
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    seen: &mut HashMap<(usize, usize, usize), u32>,
) -> Result<(), ObjError> {
    let corners: Vec<Corner> = parts
        .map(|token| read_corner(token, line, positions.len(), uvs.len(), normals.len()))
        .collect::<Result<_, _>>()?;

    if corners.len() < 3 {
        return Err(ObjError::Syntax {
            line,
            message: format!("a face needs at least 3 vertices, found {}", corners.len()),
        });
    }

    // A face without normals gets one flat normal of its own. Mixing indexed
    // and computed normals on one face would need a normal per corner that is
    // not in the file, so the whole face falls back to flat shading.
    let flat_normal = corners
        .iter()
        .any(|corner| corner.normal.is_none())
        .then(|| face_normal(positions, &corners));

    let mut face = Vec::with_capacity(corners.len());

    for corner in &corners {
        let position = positions[corner.position];
        let uv = corner.uv.map_or(Vec2::ZERO, |index| uvs[index]);
        let normal = match (corner.normal, flat_normal) {
            (Some(index), _) => safe_normalize(normals[index]),
            (None, Some(flat)) => flat,
            (None, None) => Vec3::Y,
        };

        let vertex = Vertex::with_uv(position.to_array(), normal.to_array(), uv.to_array());

        let index = match corner.normal {
            // Indexed normals let neighbouring faces share a corner.
            Some(_) => {
                let key = (
                    corner.position,
                    corner.uv.unwrap_or(usize::MAX),
                    corner.normal.unwrap_or(usize::MAX),
                );

                *seen.entry(key).or_insert_with(|| {
                    let index = vertices.len() as u32;
                    vertices.push(vertex);
                    index
                })
            }
            // Flat shading: every face owns its corners.
            None => {
                let index = vertices.len() as u32;
                vertices.push(vertex);
                index
            }
        };

        face.push(index);
    }

    // Fan triangulation: 0-1-2, 0-2-3, and so on.
    for i in 1..face.len() - 1 {
        indices.extend_from_slice(&[face[0], face[i], face[i + 1]]);
    }

    Ok(())
}

fn read_corner(
    token: &str,
    line: usize,
    position_count: usize,
    uv_count: usize,
    normal_count: usize,
) -> Result<Corner, ObjError> {
    let mut fields = token.split('/');

    let position = fields.next().unwrap_or("");
    if position.is_empty() {
        return Err(ObjError::Syntax {
            line,
            message: format!("face corner `{token}` has no vertex index"),
        });
    }

    let position = resolve(position, position_count, line, "vertex")?;

    let uv = match fields.next() {
        Some(field) if !field.is_empty() => {
            Some(resolve(field, uv_count, line, "texture coordinate")?)
        }
        _ => None,
    };

    let normal = match fields.next() {
        Some(field) if !field.is_empty() => Some(resolve(field, normal_count, line, "normal")?),
        _ => None,
    };

    Ok(Corner {
        position,
        uv,
        normal,
    })
}

/// Turns a one-based OBJ index into a zero-based one.
///
/// Negative indices count backwards from the end of the list, which is how OBJ
/// files may refer to the most recently declared element.
fn resolve(raw: &str, count: usize, line: usize, what: &str) -> Result<usize, ObjError> {
    let value: i64 = raw.parse().map_err(|_| ObjError::Syntax {
        line,
        message: format!("`{raw}` is not an integer {what} index"),
    })?;

    if value == 0 {
        return Err(ObjError::Syntax {
            line,
            message: format!("{what} indices start at 1, found 0"),
        });
    }

    let index = if value > 0 {
        value - 1
    } else {
        count as i64 + value
    };

    if index < 0 || index as usize >= count {
        return Err(ObjError::Syntax {
            line,
            message: format!("{what} index {value} is out of range (the file has {count} of them)"),
        });
    }

    Ok(index as usize)
}

fn read_vec3(
    mut parts: std::str::SplitWhitespace<'_>,
    line: usize,
    what: &str,
) -> Result<Vec3, ObjError> {
    let x = read_number(parts.next(), line, what)?;
    let y = read_number(parts.next(), line, what)?;
    let z = read_number(parts.next(), line, what)?;
    Ok(Vec3::new(x, y, z))
}

fn read_number(token: Option<&str>, line: usize, what: &str) -> Result<f32, ObjError> {
    let token = token.ok_or_else(|| ObjError::Syntax {
        line,
        message: format!("a {what} needs three numbers"),
    })?;

    let value: f32 = token.parse().map_err(|_| ObjError::Syntax {
        line,
        message: format!("`{token}` is not a number for a {what}"),
    })?;

    if !value.is_finite() {
        return Err(ObjError::Syntax {
            line,
            message: format!("a {what} coordinate must be finite, found `{token}`"),
        });
    }

    Ok(value)
}

/// Newell's method: works for any polygon, not just triangles.
fn face_normal(positions: &[Vec3], corners: &[Corner]) -> Vec3 {
    let mut normal = Vec3::ZERO;

    for i in 0..corners.len() {
        let current = positions[corners[i].position];
        let next = positions[corners[(i + 1) % corners.len()].position];
        normal += current.cross(next);
    }

    safe_normalize(normal)
}

/// Normalizes a vector, falling back to up for a degenerate one.
fn safe_normalize(vector: Vec3) -> Vec3 {
    let normalized = vector.normalize_or_zero();

    if normalized == Vec3::ZERO {
        Vec3::Y
    } else {
        normalized
    }
}

/// Reads an OBJ file from disk.
pub(crate) fn parse_file(path: &Path) -> Result<Mesh, ObjError> {
    let source = std::fs::read_to_string(path)?;
    parse(&source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_single_triangle_becomes_three_vertices() {
        let mesh = parse(
            "v 0 0 0\nv 1 0 0\nv 0 1 0\n\
             vt 0 0\nvt 1 0\nvt 0 1\n\
             vn 0 0 1\n\
             f 1/1/1 2/2/1 3/3/1\n",
        )
        .unwrap();

        assert_eq!(mesh.vertices.len(), 3);
        assert_eq!(mesh.indices, vec![0, 1, 2]);
    }

    #[test]
    fn texture_coordinates_are_flipped_vertically() {
        let mesh = parse(
            "v 0 0 0\nv 1 0 0\nv 0 1 0\n\
             vt 0.25 0.75\n\
             f 1/1 2/1 3/1\n",
        )
        .unwrap();

        // OBJ V is bottom-up, so 0.75 becomes 0.25.
        assert_eq!(mesh.vertices[0].uv, [0.25, 0.25]);
    }

    #[test]
    fn a_quad_becomes_two_triangles() {
        let mesh = parse(
            "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\n\
             vn 0 0 1\n\
             f 1//1 2//1 3//1 4//1\n",
        )
        .unwrap();

        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.indices, vec![0, 1, 2, 0, 2, 3]);
    }

    #[test]
    fn negative_indices_count_from_the_end() {
        let mesh = parse(
            "v 0 0 0\nv 1 0 0\nv 0 1 0\n\
             vn 0 0 1\n\
             f -3//-1 -2//-1 -1//-1\n",
        )
        .unwrap();

        assert_eq!(mesh.vertices.len(), 3);
        assert_eq!(mesh.indices, vec![0, 1, 2]);
    }

    #[test]
    fn missing_normals_are_computed_per_face() {
        let mesh = parse(
            "v 0 0 0\nv 1 0 0\nv 0 1 0\n\
             f 1 2 3\n",
        )
        .unwrap();

        let normal = Vec3::from_array(mesh.vertices[0].normal);
        assert!((normal - Vec3::Z).length() < 1e-6);
    }

    #[test]
    fn missing_texture_coordinates_default_to_zero() {
        let mesh = parse("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n").unwrap();

        assert_eq!(mesh.vertices[0].uv, [0.0, 0.0]);
    }

    #[test]
    fn comments_and_unknown_keywords_are_skipped() {
        let mesh = parse(
            "# a cube\n\
             o thing\n\
             mtllib thing.mtl\n\
             v 0 0 0\nv 1 0 0\nv 0 1 0 # side\n\
             usemtl red\n\
             s off\n\
             f 1 2 3\n",
        )
        .unwrap();

        assert_eq!(mesh.indices.len(), 3);
    }

    #[test]
    fn an_empty_file_is_an_error() {
        assert!(matches!(parse("# nothing here\n"), Err(ObjError::Empty)));
    }

    #[test]
    fn a_face_with_too_few_corners_is_an_error() {
        let error = parse("v 0 0 0\nv 1 0 0\nf 1 2\n").unwrap_err();

        assert!(matches!(error, ObjError::Syntax { line: 3, .. }));
    }

    #[test]
    fn an_out_of_range_index_is_an_error() {
        let error = parse("v 0 0 0\nf 1 2 3\n").unwrap_err();

        match error {
            ObjError::Syntax { message, .. } => assert!(message.contains("out of range")),
            other => panic!("expected a syntax error, got {other:?}"),
        }
    }

    #[test]
    fn a_non_number_is_an_error() {
        let error = parse("v 0 0 nope\n").unwrap_err();

        assert!(matches!(error, ObjError::Syntax { line: 1, .. }));
    }
}
