//! 3MF XML and ZIP parsing with full scene graph support.

use std::collections::HashMap;
use std::io::{Read, Seek};

use glam::{Mat4, Vec3, Vec4};
use quick_xml::Reader;
use quick_xml::events::Event;

use crate::mesh::compute_normal;
use crate::stl::{StlError, Triangle};

/// Unit of measurement in a 3MF file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Unit3mf {
    #[default]
    Millimeter,
    Centimeter,
    Inch,
    Foot,
    Micron,
}

impl Unit3mf {
    /// Convert this unit to a scale factor relative to millimeters.
    pub fn to_mm_scale(&self) -> f32 {
        match self {
            Self::Millimeter => 1.0,
            Self::Centimeter => 10.0,
            Self::Inch => 25.4,
            Self::Foot => 304.8,
            Self::Micron => 0.001,
        }
    }

    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "millimeter" => Self::Millimeter,
            "centimeter" => Self::Centimeter,
            "inch" => Self::Inch,
            "foot" => Self::Foot,
            "micron" => Self::Micron,
            _ => Self::Millimeter, // default per spec
        }
    }
}

/// Result of parsing a 3MF file.
pub struct Parse3mfResult {
    pub triangles: Vec<Triangle>,
    pub unit: Unit3mf,
    /// Color palette extracted from colorgroups (for --list-colors).
    pub color_palette: Vec<[u8; 4]>,
    /// True if any triangles have vertex colors assigned.
    pub has_colors: bool,
}

/// Parse a 3MF file and return triangles with unit info.
pub fn parse_3mf<R: Read + Seek>(reader: R) -> Result<Parse3mfResult, StlError> {
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| StlError::InvalidFormat(format!("invalid ZIP archive: {}", e)))?;

    let model_path = find_model_file(&mut archive)?;

    let mut model_file = archive.by_name(&model_path).map_err(|e| {
        StlError::InvalidFormat(format!("failed to read model file '{}': {}", model_path, e))
    })?;

    let mut xml_content = String::new();
    model_file.read_to_string(&mut xml_content)?;

    parse_model_xml(&xml_content)
}

fn find_model_file<R: Read + Seek>(archive: &mut zip::ZipArchive<R>) -> Result<String, StlError> {
    let candidates = ["3D/3dmodel.model", "3d/3dmodel.model", "3D/3DModel.model"];

    for candidate in &candidates {
        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index(i)
                && file.name().eq_ignore_ascii_case(candidate)
            {
                return Ok(file.name().to_string());
            }
        }
    }

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

// Internal types for scene graph

struct ColorGroup {
    colors: Vec<[u8; 4]>,
}

struct TriangleData {
    v1: usize,
    v2: usize,
    v3: usize,
    pid: Option<u32>,
    p1: Option<u32>,
    p2: Option<u32>,
    p3: Option<u32>,
}

struct Mesh3mf {
    vertices: Vec<[f32; 3]>,
    triangles: Vec<TriangleData>,
}

struct Component3mf {
    object_id: u32,
    transform: Mat4,
}

struct Object3mf {
    mesh: Option<Mesh3mf>,
    components: Vec<Component3mf>,
    default_pid: Option<u32>,
    default_pindex: Option<u32>,
}

struct BuildItem {
    object_id: u32,
    transform: Mat4,
}

struct Model3mf {
    unit: Unit3mf,
    objects: HashMap<u32, Object3mf>,
    build_items: Vec<BuildItem>,
    colorgroups: HashMap<u32, ColorGroup>,
}

fn parse_model_xml(xml: &str) -> Result<Parse3mfResult, StlError> {
    let model = parse_model_structure(xml)?;
    let (triangles, has_colors) = resolve_scene_graph(&model)?;

    // Collect color palette from all colorgroups
    let mut color_palette = Vec::new();
    for colorgroup in model.colorgroups.values() {
        color_palette.extend_from_slice(&colorgroup.colors);
    }

    Ok(Parse3mfResult {
        triangles,
        unit: model.unit,
        color_palette,
        has_colors,
    })
}

fn parse_model_structure(xml: &str) -> Result<Model3mf, StlError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut unit = Unit3mf::default();
    let mut objects: HashMap<u32, Object3mf> = HashMap::new();
    let mut build_items: Vec<BuildItem> = Vec::new();
    let mut colorgroups: HashMap<u32, ColorGroup> = HashMap::new();

    // Current parse state
    let mut current_object_id: Option<u32> = None;
    let mut current_object_pid: Option<u32> = None;
    let mut current_object_pindex: Option<u32> = None;
    let mut current_mesh: Option<Mesh3mf> = None;
    let mut current_components: Vec<Component3mf> = Vec::new();
    let mut current_vertices: Vec<[f32; 3]> = Vec::new();
    let mut current_triangles: Vec<TriangleData> = Vec::new();

    // Colorgroup parsing state
    let mut current_colorgroup_id: Option<u32> = None;
    let mut current_colors: Vec<[u8; 4]> = Vec::new();

    let mut in_object = false;
    let mut in_mesh = false;
    let mut in_vertices = false;
    let mut in_triangles_elem = false;
    let mut in_components = false;
    let mut in_build = false;
    let mut in_colorgroup = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let local_name = e.local_name();
                match local_name.as_ref() {
                    b"model" => {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"unit"
                                && let Ok(s) = std::str::from_utf8(&attr.value)
                            {
                                unit = Unit3mf::from_str(s);
                            }
                        }
                    }
                    b"object" => {
                        in_object = true;
                        current_object_id = None;
                        current_object_pid = None;
                        current_object_pindex = None;
                        current_mesh = None;
                        current_components.clear();

                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"id" => current_object_id = Some(parse_u32_attr(&attr.value)?),
                                b"pid" => current_object_pid = Some(parse_u32_attr(&attr.value)?),
                                b"pindex" => {
                                    current_object_pindex = Some(parse_u32_attr(&attr.value)?)
                                }
                                _ => {}
                            }
                        }
                    }
                    b"mesh" if in_object => {
                        in_mesh = true;
                        current_vertices.clear();
                        current_triangles.clear();
                    }
                    b"vertices" if in_mesh => {
                        in_vertices = true;
                    }
                    b"triangles" if in_mesh => {
                        in_triangles_elem = true;
                    }
                    b"components" if in_object => {
                        in_components = true;
                    }
                    b"build" => {
                        in_build = true;
                    }
                    b"colorgroup" => {
                        in_colorgroup = true;
                        current_colorgroup_id = None;
                        current_colors.clear();

                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"id" {
                                current_colorgroup_id = Some(parse_u32_attr(&attr.value)?);
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(e)) => {
                // Self-closing elements (no End event follows)
                let local_name = e.local_name();
                match local_name.as_ref() {
                    b"vertex" if in_vertices => {
                        let vertex = parse_vertex(&e)?;
                        current_vertices.push(vertex);
                    }
                    b"triangle" if in_triangles_elem => {
                        let tri_data = parse_triangle_data(&e)?;
                        current_triangles.push(tri_data);
                    }
                    b"component" if in_components => {
                        let component = parse_component(&e)?;
                        current_components.push(component);
                    }
                    b"item" if in_build => {
                        let item = parse_build_item(&e)?;
                        build_items.push(item);
                    }
                    b"color" if in_colorgroup => {
                        if let Some(color) = parse_color(&e) {
                            current_colors.push(color);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let local_name = e.local_name();
                match local_name.as_ref() {
                    b"vertices" => in_vertices = false,
                    b"triangles" => in_triangles_elem = false,
                    b"mesh" => {
                        in_mesh = false;
                        if !current_vertices.is_empty() {
                            current_mesh = Some(Mesh3mf {
                                vertices: std::mem::take(&mut current_vertices),
                                triangles: std::mem::take(&mut current_triangles),
                            });
                        }
                    }
                    b"components" => in_components = false,
                    b"object" => {
                        in_object = false;
                        if let Some(id) = current_object_id {
                            objects.insert(
                                id,
                                Object3mf {
                                    mesh: current_mesh.take(),
                                    components: std::mem::take(&mut current_components),
                                    default_pid: current_object_pid,
                                    default_pindex: current_object_pindex,
                                },
                            );
                        }
                    }
                    b"build" => in_build = false,
                    b"colorgroup" => {
                        in_colorgroup = false;
                        if let Some(id) = current_colorgroup_id {
                            colorgroups.insert(
                                id,
                                ColorGroup {
                                    colors: std::mem::take(&mut current_colors),
                                },
                            );
                        }
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

    Ok(Model3mf {
        unit,
        objects,
        build_items,
        colorgroups,
    })
}

fn resolve_scene_graph(model: &Model3mf) -> Result<(Vec<Triangle>, bool), StlError> {
    let mut triangles = Vec::new();
    let mut has_colors = false;

    if model.build_items.is_empty() {
        // No build section: render all objects at origin (identity transform)
        for object in model.objects.values() {
            collect_object_triangles(
                object,
                Mat4::IDENTITY,
                &model.objects,
                &model.colorgroups,
                &mut triangles,
                &mut has_colors,
                0,
            )?;
        }
    } else {
        // Use build items with their transforms
        for item in &model.build_items {
            if let Some(object) = model.objects.get(&item.object_id) {
                collect_object_triangles(
                    object,
                    item.transform,
                    &model.objects,
                    &model.colorgroups,
                    &mut triangles,
                    &mut has_colors,
                    0,
                )?;
            }
        }
    }

    Ok((triangles, has_colors))
}

const MAX_COMPONENT_DEPTH: u32 = 100;

fn collect_object_triangles(
    object: &Object3mf,
    transform: Mat4,
    all_objects: &HashMap<u32, Object3mf>,
    colorgroups: &HashMap<u32, ColorGroup>,
    triangles: &mut Vec<Triangle>,
    has_colors: &mut bool,
    depth: u32,
) -> Result<(), StlError> {
    if depth > MAX_COMPONENT_DEPTH {
        return Err(StlError::InvalidFormat(
            "component reference depth exceeds maximum (possible cycle)".into(),
        ));
    }

    // Collect triangles from this object's mesh
    if let Some(mesh) = &object.mesh {
        for tri_data in &mesh.triangles {
            let (v1, v2, v3) = (tri_data.v1, tri_data.v2, tri_data.v3);

            if v1 >= mesh.vertices.len() || v2 >= mesh.vertices.len() || v3 >= mesh.vertices.len() {
                return Err(StlError::InvalidFormat(format!(
                    "triangle references invalid vertex index: {}, {}, {} (only {} vertices)",
                    v1,
                    v2,
                    v3,
                    mesh.vertices.len()
                )));
            }

            // Transform vertices
            let p0 = transform_point(mesh.vertices[v1], transform);
            let p1 = transform_point(mesh.vertices[v2], transform);
            let p2 = transform_point(mesh.vertices[v3], transform);

            let normal = compute_normal(
                Vec3::from_array(p0),
                Vec3::from_array(p1),
                Vec3::from_array(p2),
            );

            // Resolve vertex colors from colorgroups
            let vertex_colors = resolve_triangle_colors(
                tri_data,
                object.default_pid,
                object.default_pindex,
                colorgroups,
            );

            if vertex_colors.is_some() {
                *has_colors = true;
            }

            triangles.push(Triangle {
                vertices: [p0, p1, p2],
                normal: normal.to_array(),
                vertex_colors,
            });
        }
    }

    // Recursively collect from components
    for component in &object.components {
        if let Some(ref_object) = all_objects.get(&component.object_id) {
            // Accumulate transforms: parent * component
            let combined_transform = transform * component.transform;
            collect_object_triangles(
                ref_object,
                combined_transform,
                all_objects,
                colorgroups,
                triangles,
                has_colors,
                depth + 1,
            )?;
        }
    }

    Ok(())
}

/// Resolve vertex colors for a triangle from colorgroups.
fn resolve_triangle_colors(
    tri_data: &TriangleData,
    default_pid: Option<u32>,
    default_pindex: Option<u32>,
    colorgroups: &HashMap<u32, ColorGroup>,
) -> Option<[[u8; 4]; 3]> {
    // Use triangle's pid or fall back to object's default
    let pid = tri_data.pid.or(default_pid)?;

    let colorgroup = colorgroups.get(&pid)?;
    if colorgroup.colors.is_empty() {
        return None;
    }

    // Resolve per-vertex colors (p1, p2, p3) or use default pindex
    let resolve_index = |p: Option<u32>| -> [u8; 4] {
        let idx = p.or(default_pindex).unwrap_or(0) as usize;
        if idx < colorgroup.colors.len() {
            colorgroup.colors[idx]
        } else {
            colorgroup.colors[0]
        }
    };

    Some([
        resolve_index(tri_data.p1),
        resolve_index(tri_data.p2),
        resolve_index(tri_data.p3),
    ])
}

fn transform_point(point: [f32; 3], transform: Mat4) -> [f32; 3] {
    let p = Vec4::new(point[0], point[1], point[2], 1.0);
    let result = transform * p;
    [result.x, result.y, result.z]
}

fn parse_component(e: &quick_xml::events::BytesStart<'_>) -> Result<Component3mf, StlError> {
    let mut object_id: Option<u32> = None;
    let mut transform = Mat4::IDENTITY;

    for attr in e.attributes().flatten() {
        match attr.key.as_ref() {
            b"objectid" => {
                object_id = Some(parse_u32_attr(&attr.value)?);
            }
            b"transform" => {
                if let Ok(s) = std::str::from_utf8(&attr.value) {
                    transform = parse_transform(s);
                }
            }
            _ => {}
        }
    }

    match object_id {
        Some(id) => Ok(Component3mf {
            object_id: id,
            transform,
        }),
        None => Err(StlError::InvalidFormat(
            "component element missing objectid attribute".into(),
        )),
    }
}

fn parse_build_item(e: &quick_xml::events::BytesStart<'_>) -> Result<BuildItem, StlError> {
    let mut object_id: Option<u32> = None;
    let mut transform = Mat4::IDENTITY;

    for attr in e.attributes().flatten() {
        match attr.key.as_ref() {
            b"objectid" => {
                object_id = Some(parse_u32_attr(&attr.value)?);
            }
            b"transform" => {
                if let Ok(s) = std::str::from_utf8(&attr.value) {
                    transform = parse_transform(s);
                }
            }
            _ => {}
        }
    }

    match object_id {
        Some(id) => Ok(BuildItem {
            object_id: id,
            transform,
        }),
        None => Err(StlError::InvalidFormat(
            "build item element missing objectid attribute".into(),
        )),
    }
}

/// Parse a 3MF transform string into a Mat4.
///
/// 3MF uses a 3x4 row-major matrix (last row implied as [0,0,0,1]).
/// Format: "m00 m01 m02 m10 m11 m12 m20 m21 m22 m30 m31 m32"
/// where m30, m31, m32 are the translation components.
fn parse_transform(s: &str) -> Mat4 {
    let vals: Vec<f32> = s
        .split_whitespace()
        .filter_map(|v| v.parse().ok())
        .collect();

    if vals.len() != 12 {
        return Mat4::IDENTITY;
    }

    // 3MF matrix layout (row-major 3x4):
    // | m00 m01 m02 |   | vals[0] vals[1] vals[2]  |
    // | m10 m11 m12 | = | vals[3] vals[4] vals[5]  |
    // | m20 m21 m22 |   | vals[6] vals[7] vals[8]  |
    // | m30 m31 m32 |   | vals[9] vals[10] vals[11]| (translation)
    //
    // glam Mat4 is column-major, so we construct columns:
    Mat4::from_cols(
        Vec4::new(vals[0], vals[3], vals[6], 0.0),  // column 0
        Vec4::new(vals[1], vals[4], vals[7], 0.0),  // column 1
        Vec4::new(vals[2], vals[5], vals[8], 0.0),  // column 2
        Vec4::new(vals[9], vals[10], vals[11], 1.0), // column 3 (translation)
    )
}

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

fn parse_triangle_data(
    e: &quick_xml::events::BytesStart<'_>,
) -> Result<TriangleData, StlError> {
    let mut v1: Option<usize> = None;
    let mut v2: Option<usize> = None;
    let mut v3: Option<usize> = None;
    let mut pid: Option<u32> = None;
    let mut p1: Option<u32> = None;
    let mut p2: Option<u32> = None;
    let mut p3: Option<u32> = None;

    for attr in e.attributes().flatten() {
        match attr.key.as_ref() {
            b"v1" => v1 = Some(parse_usize_attr(&attr.value)?),
            b"v2" => v2 = Some(parse_usize_attr(&attr.value)?),
            b"v3" => v3 = Some(parse_usize_attr(&attr.value)?),
            b"pid" => pid = Some(parse_u32_attr(&attr.value)?),
            b"p1" => p1 = Some(parse_u32_attr(&attr.value)?),
            b"p2" => p2 = Some(parse_u32_attr(&attr.value)?),
            b"p3" => p3 = Some(parse_u32_attr(&attr.value)?),
            _ => {}
        }
    }

    match (v1, v2, v3) {
        (Some(v1), Some(v2), Some(v3)) => Ok(TriangleData {
            v1,
            v2,
            v3,
            pid,
            p1,
            p2,
            p3,
        }),
        _ => Err(StlError::InvalidFormat(
            "triangle element missing v1, v2, or v3 attribute".into(),
        )),
    }
}

/// Parse a color element from a colorgroup: <color color="#RRGGBB"/> or <color color="#RRGGBBAA"/>
fn parse_color(e: &quick_xml::events::BytesStart<'_>) -> Option<[u8; 4]> {
    for attr in e.attributes().flatten() {
        if attr.key.as_ref() == b"color"
            && let Ok(s) = std::str::from_utf8(&attr.value)
        {
            return parse_hex_color_3mf(s);
        }
    }
    None
}

/// Parse a 3MF hex color string: #RRGGBB or #RRGGBBAA
fn parse_hex_color_3mf(s: &str) -> Option<[u8; 4]> {
    let s = s.trim().strip_prefix('#')?;

    match s.len() {
        6 => {
            let r = u8::from_str_radix(&s[0..2], 16).ok()?;
            let g = u8::from_str_radix(&s[2..4], 16).ok()?;
            let b = u8::from_str_radix(&s[4..6], 16).ok()?;
            Some([r, g, b, 255])
        }
        8 => {
            let r = u8::from_str_radix(&s[0..2], 16).ok()?;
            let g = u8::from_str_radix(&s[2..4], 16).ok()?;
            let b = u8::from_str_radix(&s[4..6], 16).ok()?;
            let a = u8::from_str_radix(&s[6..8], 16).ok()?;
            Some([r, g, b, a])
        }
        _ => None,
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

fn parse_u32_attr(value: &[u8]) -> Result<u32, StlError> {
    let s = std::str::from_utf8(value)
        .map_err(|_| StlError::InvalidFormat("invalid UTF-8 in attribute".into()))?;
    s.parse::<u32>()
        .map_err(|_| StlError::InvalidFormat(format!("invalid integer value: {}", s)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_transform_identity() {
        let t = parse_transform("1 0 0 0 1 0 0 0 1 0 0 0");
        assert_eq!(t, Mat4::IDENTITY);
    }

    #[test]
    fn test_parse_transform_translation() {
        let t = parse_transform("1 0 0 0 1 0 0 0 1 10 20 30");
        let point = transform_point([0.0, 0.0, 0.0], t);
        assert_eq!(point, [10.0, 20.0, 30.0]);
    }

    #[test]
    fn test_parse_transform_scale() {
        let t = parse_transform("2 0 0 0 2 0 0 0 2 0 0 0");
        let point = transform_point([1.0, 1.0, 1.0], t);
        assert_eq!(point, [2.0, 2.0, 2.0]);
    }

    #[test]
    fn test_parse_transform_invalid_returns_identity() {
        let t = parse_transform("not a matrix");
        assert_eq!(t, Mat4::IDENTITY);
    }

    #[test]
    fn test_unit_from_str() {
        assert_eq!(Unit3mf::from_str("millimeter"), Unit3mf::Millimeter);
        assert_eq!(Unit3mf::from_str("inch"), Unit3mf::Inch);
        assert_eq!(Unit3mf::from_str("CENTIMETER"), Unit3mf::Centimeter);
        assert_eq!(Unit3mf::from_str("unknown"), Unit3mf::Millimeter);
    }

    #[test]
    fn test_unit_to_mm_scale() {
        assert_eq!(Unit3mf::Millimeter.to_mm_scale(), 1.0);
        assert_eq!(Unit3mf::Inch.to_mm_scale(), 25.4);
        assert_eq!(Unit3mf::Centimeter.to_mm_scale(), 10.0);
    }

    #[test]
    fn test_parse_simple_model() {
        let xml = r#"
            <model unit="millimeter">
                <resources>
                    <object id="1">
                        <mesh>
                            <vertices>
                                <vertex x="0" y="0" z="0"/>
                                <vertex x="1" y="0" z="0"/>
                                <vertex x="0" y="1" z="0"/>
                            </vertices>
                            <triangles>
                                <triangle v1="0" v2="1" v3="2"/>
                            </triangles>
                        </mesh>
                    </object>
                </resources>
                <build>
                    <item objectid="1"/>
                </build>
            </model>
        "#;

        let result = parse_model_xml(xml).unwrap();
        assert_eq!(result.triangles.len(), 1);
        assert_eq!(result.unit, Unit3mf::Millimeter);
    }

    #[test]
    fn test_parse_model_with_transform() {
        let xml = r#"
            <model>
                <resources>
                    <object id="1">
                        <mesh>
                            <vertices>
                                <vertex x="0" y="0" z="0"/>
                                <vertex x="1" y="0" z="0"/>
                                <vertex x="0" y="1" z="0"/>
                            </vertices>
                            <triangles>
                                <triangle v1="0" v2="1" v3="2"/>
                            </triangles>
                        </mesh>
                    </object>
                </resources>
                <build>
                    <item objectid="1" transform="1 0 0 0 1 0 0 0 1 10 0 0"/>
                </build>
            </model>
        "#;

        let result = parse_model_xml(xml).unwrap();
        assert_eq!(result.triangles.len(), 1);
        // First vertex should be translated by (10, 0, 0)
        assert_eq!(result.triangles[0].vertices[0], [10.0, 0.0, 0.0]);
    }

    #[test]
    fn test_parse_model_with_components() {
        let xml = r#"
            <model>
                <resources>
                    <object id="1">
                        <mesh>
                            <vertices>
                                <vertex x="0" y="0" z="0"/>
                                <vertex x="1" y="0" z="0"/>
                                <vertex x="0" y="1" z="0"/>
                            </vertices>
                            <triangles>
                                <triangle v1="0" v2="1" v3="2"/>
                            </triangles>
                        </mesh>
                    </object>
                    <object id="2">
                        <components>
                            <component objectid="1" transform="1 0 0 0 1 0 0 0 1 5 0 0"/>
                        </components>
                    </object>
                </resources>
                <build>
                    <item objectid="2"/>
                </build>
            </model>
        "#;

        let result = parse_model_xml(xml).unwrap();
        assert_eq!(result.triangles.len(), 1);
        // Component references object 1 with translation (5, 0, 0)
        assert_eq!(result.triangles[0].vertices[0], [5.0, 0.0, 0.0]);
    }

    #[test]
    fn test_parse_model_no_build_section() {
        let xml = r#"
            <model>
                <resources>
                    <object id="1">
                        <mesh>
                            <vertices>
                                <vertex x="0" y="0" z="0"/>
                                <vertex x="1" y="0" z="0"/>
                                <vertex x="0" y="1" z="0"/>
                            </vertices>
                            <triangles>
                                <triangle v1="0" v2="1" v3="2"/>
                            </triangles>
                        </mesh>
                    </object>
                </resources>
            </model>
        "#;

        let result = parse_model_xml(xml).unwrap();
        assert_eq!(result.triangles.len(), 1);
        // No transform, vertices at origin
        assert_eq!(result.triangles[0].vertices[0], [0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_parse_inch_unit() {
        let xml = r#"
            <model unit="inch">
                <resources>
                    <object id="1">
                        <mesh>
                            <vertices>
                                <vertex x="0" y="0" z="0"/>
                                <vertex x="1" y="0" z="0"/>
                                <vertex x="0" y="1" z="0"/>
                            </vertices>
                            <triangles>
                                <triangle v1="0" v2="1" v3="2"/>
                            </triangles>
                        </mesh>
                    </object>
                </resources>
            </model>
        "#;

        let result = parse_model_xml(xml).unwrap();
        assert_eq!(result.unit, Unit3mf::Inch);
    }

    #[test]
    fn test_parse_hex_color() {
        assert_eq!(parse_hex_color_3mf("#FF0000"), Some([255, 0, 0, 255]));
        assert_eq!(parse_hex_color_3mf("#00FF00"), Some([0, 255, 0, 255]));
        assert_eq!(parse_hex_color_3mf("#0000FF"), Some([0, 0, 255, 255]));
        assert_eq!(parse_hex_color_3mf("#FF00FF80"), Some([255, 0, 255, 128]));
        assert_eq!(parse_hex_color_3mf("invalid"), None);
        assert_eq!(parse_hex_color_3mf("#12345"), None);
    }

    #[test]
    fn test_parse_colorgroup() {
        let xml = r##"
            <model>
                <resources>
                    <colorgroup id="1">
                        <color color="#FF0000"/>
                        <color color="#00FF00"/>
                        <color color="#0000FF"/>
                    </colorgroup>
                    <object id="2" pid="1" pindex="0">
                        <mesh>
                            <vertices>
                                <vertex x="0" y="0" z="0"/>
                                <vertex x="1" y="0" z="0"/>
                                <vertex x="0" y="1" z="0"/>
                            </vertices>
                            <triangles>
                                <triangle v1="0" v2="1" v3="2"/>
                            </triangles>
                        </mesh>
                    </object>
                </resources>
            </model>
        "##;

        let result = parse_model_xml(xml).unwrap();
        assert_eq!(result.triangles.len(), 1);
        assert!(result.has_colors);
        assert_eq!(result.color_palette.len(), 3);
        assert_eq!(result.color_palette[0], [255, 0, 0, 255]);

        // Triangle should have vertex colors (all same from default pindex=0)
        let vc = result.triangles[0].vertex_colors.unwrap();
        assert_eq!(vc[0], [255, 0, 0, 255]);
        assert_eq!(vc[1], [255, 0, 0, 255]);
        assert_eq!(vc[2], [255, 0, 0, 255]);
    }

    #[test]
    fn test_parse_per_vertex_colors() {
        let xml = r##"
            <model>
                <resources>
                    <colorgroup id="1">
                        <color color="#FF0000"/>
                        <color color="#00FF00"/>
                        <color color="#0000FF"/>
                    </colorgroup>
                    <object id="2">
                        <mesh>
                            <vertices>
                                <vertex x="0" y="0" z="0"/>
                                <vertex x="1" y="0" z="0"/>
                                <vertex x="0" y="1" z="0"/>
                            </vertices>
                            <triangles>
                                <triangle v1="0" v2="1" v3="2" pid="1" p1="0" p2="1" p3="2"/>
                            </triangles>
                        </mesh>
                    </object>
                </resources>
            </model>
        "##;

        let result = parse_model_xml(xml).unwrap();
        assert!(result.has_colors);

        // Triangle should have different colors per vertex
        let vc = result.triangles[0].vertex_colors.unwrap();
        assert_eq!(vc[0], [255, 0, 0, 255]); // red
        assert_eq!(vc[1], [0, 255, 0, 255]); // green
        assert_eq!(vc[2], [0, 0, 255, 255]); // blue
    }

    #[test]
    fn test_parse_no_colors() {
        let xml = r#"
            <model>
                <resources>
                    <object id="1">
                        <mesh>
                            <vertices>
                                <vertex x="0" y="0" z="0"/>
                                <vertex x="1" y="0" z="0"/>
                                <vertex x="0" y="1" z="0"/>
                            </vertices>
                            <triangles>
                                <triangle v1="0" v2="1" v3="2"/>
                            </triangles>
                        </mesh>
                    </object>
                </resources>
            </model>
        "#;

        let result = parse_model_xml(xml).unwrap();
        assert!(!result.has_colors);
        assert!(result.color_palette.is_empty());
        assert!(result.triangles[0].vertex_colors.is_none());
    }
}
