//! 3MF XML and ZIP parsing.

use std::io::{Read, Seek};

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::mesh::compute_normal;
use crate::stl::{StlError, Triangle};

/// Parse a 3MF file and return all triangles.
///
/// 3MF files are ZIP archives. This function:
/// 1. Opens the ZIP archive
/// 2. Finds and reads `3D/3dmodel.model` (or similar model file)
/// 3. Parses the XML to extract vertices and triangles
/// 4. Computes normals from vertex positions
pub fn parse_3mf<R: Read + Seek>(reader: R) -> Result<Vec<Triangle>, StlError> {
    let mut archive = zip::ZipArchive::new(reader).map_err(|e| {
        StlError::InvalidFormat(format!("invalid ZIP archive: {}", e))
    })?;

    // Find the model file - typically 3D/3dmodel.model
    let model_path = find_model_file(&mut archive)?;

    // Read the model XML
    let mut model_file = archive.by_name(&model_path).map_err(|e| {
        StlError::InvalidFormat(format!("failed to read model file '{}': {}", model_path, e))
    })?;

    let mut xml_content = String::new();
    model_file.read_to_string(&mut xml_content)?;

    // Parse the XML
    parse_model_xml(&xml_content)
}

/// Find the model file path within the ZIP archive.
fn find_model_file<R: Read + Seek>(archive: &mut zip::ZipArchive<R>) -> Result<String, StlError> {
    // Common paths for 3MF model files
    let candidates = [
        "3D/3dmodel.model",
        "3d/3dmodel.model",
        "3D/3DModel.model",
    ];

    for candidate in &candidates {
        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index(i) {
                if file.name().eq_ignore_ascii_case(candidate) {
                    return Ok(file.name().to_string());
                }
            }
        }
    }

    // Also search for any .model file in a 3D directory
    for i in 0..archive.len() {
        if let Ok(file) = archive.by_index(i) {
            let name = file.name().to_lowercase();
            if name.starts_with("3d/") && name.ends_with(".model") {
                return Ok(file.name().to_string());
            }
        }
    }

    Err(StlError::InvalidFormat(
        "3MF archive missing model file (expected 3D/3dmodel.model)".into(),
    ))
}

/// Parse the 3MF model XML and extract triangles.
fn parse_model_xml(xml: &str) -> Result<Vec<Triangle>, StlError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut triangles = Vec::new();
    let mut current_vertices: Vec<[f32; 3]> = Vec::new();
    let mut in_vertices = false;
    let mut in_triangles = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let local_name = e.local_name();
                match local_name.as_ref() {
                    b"vertices" => {
                        in_vertices = true;
                        current_vertices.clear();
                    }
                    b"triangles" => {
                        in_triangles = true;
                    }
                    b"vertex" if in_vertices => {
                        let vertex = parse_vertex(&e)?;
                        current_vertices.push(vertex);
                    }
                    b"triangle" if in_triangles => {
                        let (v1, v2, v3) = parse_triangle_indices(&e)?;

                        if v1 >= current_vertices.len()
                            || v2 >= current_vertices.len()
                            || v3 >= current_vertices.len()
                        {
                            return Err(StlError::InvalidFormat(format!(
                                "triangle references invalid vertex index: {}, {}, {} (only {} vertices)",
                                v1, v2, v3, current_vertices.len()
                            )));
                        }

                        let vertices = [
                            current_vertices[v1],
                            current_vertices[v2],
                            current_vertices[v3],
                        ];

                        // Compute normal from vertices
                        let normal = compute_normal(
                            glam::Vec3::from_array(vertices[0]),
                            glam::Vec3::from_array(vertices[1]),
                            glam::Vec3::from_array(vertices[2]),
                        );

                        triangles.push(Triangle {
                            vertices,
                            normal: normal.to_array(),
                        });
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let local_name = e.local_name();
                match local_name.as_ref() {
                    b"vertices" => in_vertices = false,
                    b"triangles" => in_triangles = false,
                    b"mesh" => {
                        // End of mesh - vertices list stays for the triangles that follow
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(StlError::InvalidFormat(format!(
                    "XML parse error at position {}: {}",
                    reader.buffer_position(),
                    e
                )));
            }
            _ => {}
        }
    }

    Ok(triangles)
}

/// Parse a vertex element's x, y, z attributes.
fn parse_vertex(e: &quick_xml::events::BytesStart<'_>) -> Result<[f32; 3], StlError> {
    let mut x: Option<f32> = None;
    let mut y: Option<f32> = None;
    let mut z: Option<f32> = None;

    for attr in e.attributes().flatten() {
        match attr.key.as_ref() {
            b"x" => x = Some(parse_float_attr(&attr.value)?),
            b"y" => y = Some(parse_float_attr(&attr.value)?),
            b"z" => z = Some(parse_float_attr(&attr.value)?),
            _ => {}
        }
    }

    match (x, y, z) {
        (Some(x), Some(y), Some(z)) => Ok([x, y, z]),
        _ => Err(StlError::InvalidFormat(
            "vertex element missing x, y, or z attribute".into(),
        )),
    }
}

/// Parse a triangle element's v1, v2, v3 attributes.
fn parse_triangle_indices(e: &quick_xml::events::BytesStart<'_>) -> Result<(usize, usize, usize), StlError> {
    let mut v1: Option<usize> = None;
    let mut v2: Option<usize> = None;
    let mut v3: Option<usize> = None;

    for attr in e.attributes().flatten() {
        match attr.key.as_ref() {
            b"v1" => v1 = Some(parse_usize_attr(&attr.value)?),
            b"v2" => v2 = Some(parse_usize_attr(&attr.value)?),
            b"v3" => v3 = Some(parse_usize_attr(&attr.value)?),
            _ => {}
        }
    }

    match (v1, v2, v3) {
        (Some(v1), Some(v2), Some(v3)) => Ok((v1, v2, v3)),
        _ => Err(StlError::InvalidFormat(
            "triangle element missing v1, v2, or v3 attribute".into(),
        )),
    }
}

fn parse_float_attr(value: &[u8]) -> Result<f32, StlError> {
    let s = std::str::from_utf8(value)
        .map_err(|_| StlError::InvalidFormat("invalid UTF-8 in attribute".into()))?;
    s.parse::<f32>()
        .map_err(|_| StlError::InvalidFormat(format!("invalid float value: {}", s)))
}

fn parse_usize_attr(value: &[u8]) -> Result<usize, StlError> {
    let s = std::str::from_utf8(value)
        .map_err(|_| StlError::InvalidFormat("invalid UTF-8 in attribute".into()))?;
    s.parse::<usize>()
        .map_err(|_| StlError::InvalidFormat(format!("invalid integer value: {}", s)))
}
