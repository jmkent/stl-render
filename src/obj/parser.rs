//! OBJ file format parsing.

use std::io::{BufRead, BufReader, Read};

use crate::mesh::compute_normal;
use crate::stl::{StlError, Triangle};

/// Parse an OBJ file and return all triangles.
///
/// OBJ files contain:
/// - `v x y z` - vertex positions
/// - `vn x y z` - vertex normals (optional, we compute our own)
/// - `f v1 v2 v3` - triangle faces (indices are 1-based)
/// - `f v1/vt1/vn1 v2/vt2/vn2 v3/vt3/vn3` - faces with texture/normal indices
///
/// Faces with more than 3 vertices are triangulated as a fan.
pub fn parse_obj<R: Read>(reader: R) -> Result<Vec<Triangle>, StlError> {
    let reader = BufReader::new(reader);
    let mut vertices: Vec<[f32; 3]> = Vec::new();
    let mut triangles: Vec<Triangle> = Vec::new();

    for (line_num, line_result) in reader.lines().enumerate() {
        let line = line_result?;
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut parts = line.split_whitespace();
        let cmd = match parts.next() {
            Some(c) => c,
            None => continue,
        };

        match cmd {
            "v" => {
                // Vertex: v x y z [w]
                let coords: Vec<f32> = parts
                    .take(3)
                    .map(|s| s.parse::<f32>())
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| {
                        StlError::InvalidFormat(format!(
                            "invalid vertex coordinates at line {}",
                            line_num + 1
                        ))
                    })?;

                if coords.len() < 3 {
                    return Err(StlError::InvalidFormat(format!(
                        "vertex needs 3 coordinates at line {}",
                        line_num + 1
                    )));
                }

                vertices.push([coords[0], coords[1], coords[2]]);
            }
            "f" => {
                // Face: f v1 v2 v3 ... or f v1/vt1/vn1 v2/vt2/vn2 v3/vt3/vn3 ...
                let indices: Vec<usize> = parts
                    .map(parse_face_vertex)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| {
                        StlError::InvalidFormat(format!(
                            "invalid face at line {}: {}",
                            line_num + 1,
                            e
                        ))
                    })?;

                if indices.len() < 3 {
                    return Err(StlError::InvalidFormat(format!(
                        "face needs at least 3 vertices at line {}",
                        line_num + 1
                    )));
                }

                // Validate indices
                for &idx in &indices {
                    if idx == 0 || idx > vertices.len() {
                        return Err(StlError::InvalidFormat(format!(
                            "face references invalid vertex index {} at line {} (only {} vertices)",
                            idx,
                            line_num + 1,
                            vertices.len()
                        )));
                    }
                }

                // Triangulate as fan: (0, 1, 2), (0, 2, 3), (0, 3, 4), ...
                for i in 1..indices.len() - 1 {
                    let v0 = vertices[indices[0] - 1]; // OBJ indices are 1-based
                    let v1 = vertices[indices[i] - 1];
                    let v2 = vertices[indices[i + 1] - 1];

                    let normal = compute_normal(
                        glam::Vec3::from_array(v0),
                        glam::Vec3::from_array(v1),
                        glam::Vec3::from_array(v2),
                    );

                    triangles.push(Triangle {
                        vertices: [v0, v1, v2],
                        normal: normal.to_array(),
                        vertex_colors: None,
                    });
                }
            }
            // Ignore other commands: vt, vn, g, o, s, mtllib, usemtl, etc.
            _ => {}
        }
    }

    Ok(triangles)
}

/// Parse a face vertex specification.
/// Can be: "v", "v/vt", "v/vt/vn", or "v//vn"
/// Returns the vertex index (1-based).
fn parse_face_vertex(s: &str) -> Result<usize, String> {
    let parts: Vec<&str> = s.split('/').collect();
    parts[0]
        .parse::<usize>()
        .map_err(|_| format!("invalid vertex index '{}'", parts[0]))
}

/// Check if data looks like an OBJ file.
/// OBJ files are text and typically start with comments or vertex definitions.
pub fn is_obj_format(data: &[u8]) -> bool {
    // Must be valid UTF-8 text
    let text = match std::str::from_utf8(data) {
        Ok(t) => t,
        Err(_) => return false,
    };

    // Check first few non-empty, non-comment lines for OBJ commands
    let mut found_obj_command = false;
    for line in text.lines().take(50) {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Check for common OBJ commands
        if line.starts_with("v ")
            || line.starts_with("vn ")
            || line.starts_with("vt ")
            || line.starts_with("f ")
            || line.starts_with("g ")
            || line.starts_with("o ")
            || line.starts_with("s ")
            || line.starts_with("mtllib ")
            || line.starts_with("usemtl ")
        {
            found_obj_command = true;
            break;
        }

        // If we hit something that's not an OBJ command, it's probably not OBJ
        // (unless it's a very unusual file)
        if !line.starts_with("v")
            && !line.starts_with("f")
            && !line.starts_with("g")
            && !line.starts_with("o")
            && !line.starts_with("s")
            && !line.starts_with("mtllib")
            && !line.starts_with("usemtl")
        {
            return false;
        }
    }

    found_obj_command
}
