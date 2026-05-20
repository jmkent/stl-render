//! Dimension overlay for rendered images.
//!
//! Draws a projected 3D bounding box with dimension labels onto rendered output.

use glam::Vec3;
use image::{Rgba, RgbaImage};
use serde::{Deserialize, Serialize};

use crate::camera::Camera;
use crate::mesh::BoundingBox;

/// Display units for dimension overlay.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum DimensionUnits {
    #[default]
    Millimeters,
    Inches,
}

impl DimensionUnits {
    pub fn suffix(&self) -> &'static str {
        match self {
            DimensionUnits::Millimeters => "mm",
            DimensionUnits::Inches => "in",
        }
    }

    pub fn convert(&self, mm_value: f32) -> f32 {
        match self {
            DimensionUnits::Millimeters => mm_value,
            DimensionUnits::Inches => mm_value / 25.4,
        }
    }
}

/// Configuration for dimension overlay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionConfig {
    pub enabled: bool,
    pub units: DimensionUnits,
    pub color: Option<[u8; 3]>,
}

impl Default for DimensionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            units: DimensionUnits::Millimeters,
            color: None,
        }
    }
}

/// Fixed label edge indices for animation consistency.
/// Each tuple is (corner_a, corner_b) for the edge to label.
#[derive(Debug, Clone, Copy)]
pub struct LabelEdges {
    pub x_edge: (usize, usize),
    pub y_edge: (usize, usize),
    pub z_edge: (usize, usize),
}

/// 5x7 bitmap font for digits 0-9, decimal point, and letters for units.
const FONT_WIDTH: u32 = 5;
const FONT_HEIGHT: u32 = 7;

#[rustfmt::skip]
const FONT_DATA: &[(char, [u8; 7])] = &[
    ('0', [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110]),
    ('1', [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110]),
    ('2', [0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111]),
    ('3', [0b01110, 0b10001, 0b00001, 0b00110, 0b00001, 0b10001, 0b01110]),
    ('4', [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010]),
    ('5', [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110]),
    ('6', [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110]),
    ('7', [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000]),
    ('8', [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110]),
    ('9', [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100]),
    ('.', [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100]),
    ('m', [0b00000, 0b00000, 0b11010, 0b10101, 0b10101, 0b10001, 0b10001]),
    ('i', [0b00100, 0b00000, 0b01100, 0b00100, 0b00100, 0b00100, 0b01110]),
    ('n', [0b00000, 0b00000, 0b10110, 0b11001, 0b10001, 0b10001, 0b10001]),
    (' ', [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000]),
    ('x', [0b00000, 0b00000, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001]),
    ('y', [0b00000, 0b00000, 0b10001, 0b10001, 0b01111, 0b00001, 0b01110]),
    ('z', [0b00000, 0b00000, 0b11111, 0b00010, 0b00100, 0b01000, 0b11111]),
    (':', [0b00000, 0b01100, 0b01100, 0b00000, 0b01100, 0b01100, 0b00000]),
    ('X', [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001]),
    ('Y', [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100]),
    ('Z', [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111]),
];

fn get_glyph(c: char) -> Option<&'static [u8; 7]> {
    FONT_DATA
        .iter()
        .find(|(ch, _)| *ch == c)
        .map(|(_, data)| data)
}

fn measure_text(text: &str) -> u32 {
    let char_count = text.chars().count() as u32;
    if char_count == 0 {
        0
    } else {
        char_count * FONT_WIDTH + (char_count - 1)
    }
}

fn draw_char(
    image: &mut RgbaImage,
    c: char,
    x: i32,
    y: i32,
    color: Rgba<u8>,
    outline_color: Rgba<u8>,
    scale: u32,
) {
    let Some(glyph) = get_glyph(c) else { return };

    // Draw outline first
    for dy in 0..FONT_HEIGHT {
        let row = glyph[dy as usize];
        for dx in 0..FONT_WIDTH {
            if (row >> (FONT_WIDTH - 1 - dx)) & 1 == 1 {
                for oy in -1i32..=1 {
                    for ox in -1i32..=1 {
                        if ox == 0 && oy == 0 {
                            continue;
                        }
                        for sy in 0..scale {
                            for sx in 0..scale {
                                let px = x + (dx * scale) as i32 + sx as i32 + ox;
                                let py = y + (dy * scale) as i32 + sy as i32 + oy;
                                if px >= 0
                                    && py >= 0
                                    && (px as u32) < image.width()
                                    && (py as u32) < image.height()
                                {
                                    image.put_pixel(px as u32, py as u32, outline_color);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Draw foreground
    for dy in 0..FONT_HEIGHT {
        let row = glyph[dy as usize];
        for dx in 0..FONT_WIDTH {
            if (row >> (FONT_WIDTH - 1 - dx)) & 1 == 1 {
                for sy in 0..scale {
                    for sx in 0..scale {
                        let px = x + (dx * scale) as i32 + sx as i32;
                        let py = y + (dy * scale) as i32 + sy as i32;
                        if px >= 0
                            && py >= 0
                            && (px as u32) < image.width()
                            && (py as u32) < image.height()
                        {
                            image.put_pixel(px as u32, py as u32, color);
                        }
                    }
                }
            }
        }
    }
}

fn draw_text(
    image: &mut RgbaImage,
    text: &str,
    x: i32,
    y: i32,
    color: Rgba<u8>,
    outline_color: Rgba<u8>,
    scale: u32,
) {
    let mut cx = x;
    for c in text.chars() {
        draw_char(image, c, cx, y, color, outline_color, scale);
        cx += (FONT_WIDTH * scale + scale) as i32;
    }
}

/// Compute average brightness of image to determine text contrast color.
fn compute_average_brightness(image: &RgbaImage) -> f32 {
    let mut sum = 0u64;
    let mut count = 0u64;

    for pixel in image.pixels() {
        if pixel.0[3] > 0 {
            let r = pixel.0[0] as u64;
            let g = pixel.0[1] as u64;
            let b = pixel.0[2] as u64;
            sum += (r * 299 + g * 587 + b * 114) / 1000;
            count += 1;
        }
    }

    if count == 0 {
        128.0
    } else {
        sum as f32 / count as f32
    }
}

/// Get the dimension overlay color, suitable for use with Framebuffer.
/// Returns RGBA color based on config or contrasting with background.
pub fn get_dimension_color(config: &DimensionConfig, fb: &crate::render::Framebuffer) -> [u8; 4] {
    if let Some(color) = config.color {
        [color[0], color[1], color[2], 255]
    } else {
        // Determine contrast color based on background
        // Sample the center pixel of the framebuffer to determine brightness
        let center_idx = ((fb.height / 2) * fb.width + (fb.width / 2)) as usize;
        let bg = fb.get_color(center_idx);
        let brightness = (bg[0] as u32 * 299 + bg[1] as u32 * 587 + bg[2] as u32 * 114) / 1000;
        if bg[3] == 0 {
            [255, 255, 255, 255]
        } else if brightness > 128 {
            [40, 40, 40, 255]
        } else {
            [255, 255, 255, 255]
        }
    }
}

/// Format dimension value with appropriate precision.
fn format_dimension(value: f32, units: DimensionUnits) -> String {
    let converted = units.convert(value);
    let suffix = units.suffix();

    if converted >= 100.0 {
        format!("{:.0}{}", converted, suffix)
    } else if converted >= 10.0 {
        format!("{:.1}{}", converted, suffix)
    } else {
        format!("{:.2}{}", converted, suffix)
    }
}

/// Get the 8 corners of a bounding box.
fn bbox_corners(bounds: &BoundingBox) -> [Vec3; 8] {
    let min = Vec3::from_array(bounds.min);
    let max = Vec3::from_array(bounds.max);
    [
        Vec3::new(min.x, min.y, min.z), // 0: ---
        Vec3::new(max.x, min.y, min.z), // 1: +--
        Vec3::new(min.x, max.y, min.z), // 2: -+-
        Vec3::new(max.x, max.y, min.z), // 3: ++-
        Vec3::new(min.x, min.y, max.z), // 4: --+
        Vec3::new(max.x, min.y, max.z), // 5: +-+
        Vec3::new(min.x, max.y, max.z), // 6: -++
        Vec3::new(max.x, max.y, max.z), // 7: +++
    ]
}

/// The 6 faces of a bounding box, each defined by 4 corner indices (in order) and outward normal.
/// Face corners are ordered counter-clockwise when viewed from outside.
const BOX_FACES: [([usize; 4], [f32; 3]); 6] = [
    ([0, 1, 3, 2], [0.0, 0.0, -1.0]), // -Z face (front in Y-up)
    ([4, 6, 7, 5], [0.0, 0.0, 1.0]),  // +Z face (back in Y-up)
    ([0, 2, 6, 4], [-1.0, 0.0, 0.0]), // -X face (left)
    ([1, 5, 7, 3], [1.0, 0.0, 0.0]),  // +X face (right)
    ([0, 4, 5, 1], [0.0, -1.0, 0.0]), // -Y face (bottom)
    ([2, 3, 7, 6], [0.0, 1.0, 0.0]),  // +Y face (top)
];

/// Check if a face is front-facing (visible from camera).
fn is_face_front_facing(face_normal: Vec3, camera: &Camera) -> bool {
    // Transform the normal to view space
    // For orthographic projection, we can use the view matrix's Z axis as view direction
    // The view direction in world space points from camera toward scene (negative Z in view space)

    // Extract the camera's forward direction from view matrix
    // The view matrix transforms world to view space, so its inverse's -Z column is the forward direction
    // Simplified: if the dot product of normal with camera forward is negative, face is front-facing

    // Get camera forward direction (third column of view matrix transposed, negated)
    let view_mat = camera.view_matrix;
    let cam_forward = Vec3::new(view_mat.x_axis.z, view_mat.y_axis.z, view_mat.z_axis.z);

    // Face is front-facing if its normal points toward the camera (dot product < 0)
    face_normal.dot(cam_forward) < 0.0
}

/// Get the edges to draw based on which faces are front-facing.
fn get_front_facing_edges(camera: &Camera) -> Vec<(usize, usize)> {
    let mut edges = std::collections::HashSet::new();

    for (corners, normal) in &BOX_FACES {
        let face_normal = Vec3::from_array(*normal);
        if is_face_front_facing(face_normal, camera) {
            // Add all 4 edges of this face
            for i in 0..4 {
                let a = corners[i];
                let b = corners[(i + 1) % 4];
                // Normalize edge representation (smaller index first)
                let edge = if a < b { (a, b) } else { (b, a) };
                edges.insert(edge);
            }
        }
    }

    edges.into_iter().collect()
}

/// X edges connect corners that differ only in X.
const X_EDGES: [(usize, usize); 4] = [(0, 1), (2, 3), (4, 5), (6, 7)];
/// Y edges connect corners that differ only in Y.
const Y_EDGES: [(usize, usize); 4] = [(0, 2), (1, 3), (4, 6), (5, 7)];
/// Z edges connect corners that differ only in Z.
const Z_EDGES: [(usize, usize); 4] = [(0, 4), (1, 5), (2, 6), (3, 7)];

/// Find the best (longest visible) edge from a set of edges.
fn find_best_edge(
    edges: &[(usize, usize)],
    corners_2d: &[Option<(f32, f32)>],
    front_edges: &[(usize, usize)],
) -> Option<(usize, usize)> {
    let mut best: Option<((usize, usize), f32)> = None;

    for &(i, j) in edges {
        // Normalize edge representation
        let edge = if i < j { (i, j) } else { (j, i) };

        // Only consider front-facing edges
        if !front_edges.contains(&edge) {
            continue;
        }

        if let (Some(p0), Some(p1)) = (corners_2d[i], corners_2d[j]) {
            let dx = p1.0 - p0.0;
            let dy = p1.1 - p0.1;
            let len_sq = dx * dx + dy * dy;
            if best.is_none() || len_sq > best.unwrap().1 {
                best = Some(((i, j), len_sq));
            }
        }
    }

    best.map(|(edge, _)| edge)
}

/// Compute the best label edges for a given camera view.
pub fn compute_label_edges(
    bounds: &BoundingBox,
    camera: &Camera,
    width: u32,
    height: u32,
) -> LabelEdges {
    let corners_3d = bbox_corners(bounds);
    let corners_2d: Vec<Option<(f32, f32)>> = corners_3d
        .iter()
        .map(|&c| camera.project_to_screen(c, width, height))
        .collect();

    let front_edges = get_front_facing_edges(camera);

    // Find best edge for each axis, with fallbacks
    let x_edge = find_best_edge(&X_EDGES, &corners_2d, &front_edges).unwrap_or(X_EDGES[0]);
    let y_edge = find_best_edge(&Y_EDGES, &corners_2d, &front_edges).unwrap_or(Y_EDGES[0]);
    let z_edge = find_best_edge(&Z_EDGES, &corners_2d, &front_edges).unwrap_or(Z_EDGES[0]);

    LabelEdges {
        x_edge,
        y_edge,
        z_edge,
    }
}

/// Draw a label at the midpoint of an edge with an offset perpendicular to the edge.
fn draw_edge_label(
    image: &mut RgbaImage,
    p0: (f32, f32),
    p1: (f32, f32),
    label: &str,
    color: Rgba<u8>,
    outline_color: Rgba<u8>,
    scale: u32,
    offset: f32,
) {
    let mid_x = (p0.0 + p1.0) / 2.0;
    let mid_y = (p0.1 + p1.1) / 2.0;

    // Compute perpendicular direction for offset
    let dx = p1.0 - p0.0;
    let dy = p1.1 - p0.1;
    let len = (dx * dx + dy * dy).sqrt().max(1.0);
    let perp_x = -dy / len;
    let perp_y = dx / len;

    let text_width = measure_text(label) * scale;
    let text_height = FONT_HEIGHT * scale;

    // Position label with offset, centered on the edge midpoint
    let label_x = (mid_x + perp_x * offset) as i32 - (text_width as i32 / 2);
    let label_y = (mid_y + perp_y * offset) as i32 - (text_height as i32 / 2);

    draw_text(image, label, label_x, label_y, color, outline_color, scale);
}

/// Apply dimension labels to rendered image.
///
/// The depth-aware bounding box lines are rendered in the framebuffer before
/// this function runs. This overlay only places X, Y, Z labels.
pub fn apply_dimensions(
    image: &mut RgbaImage,
    bounds: &BoundingBox,
    camera: &Camera,
    config: &DimensionConfig,
) {
    apply_dimensions_with_labels(image, bounds, camera, config, None);
}

/// Apply dimension overlay with optional fixed label edges (for animation consistency).
pub fn apply_dimensions_with_labels(
    image: &mut RgbaImage,
    bounds: &BoundingBox,
    camera: &Camera,
    config: &DimensionConfig,
    fixed_labels: Option<LabelEdges>,
) {
    if !config.enabled {
        return;
    }

    let width = image.width();
    let height = image.height();

    // Determine colors
    let (fg_color, outline_color) = if let Some(color) = config.color {
        (
            Rgba([color[0], color[1], color[2], 255]),
            Rgba([255 - color[0], 255 - color[1], 255 - color[2], 255]),
        )
    } else {
        let brightness = compute_average_brightness(image);
        if brightness > 128.0 {
            (Rgba([40, 40, 40, 255]), Rgba([255, 255, 255, 220]))
        } else {
            (Rgba([255, 255, 255, 255]), Rgba([40, 40, 40, 220]))
        }
    };

    // Scale factor based on image size
    let min_dim = width.min(height);
    let scale = if min_dim >= 1024 {
        3
    } else if min_dim >= 512 {
        2
    } else {
        1
    };

    // Project all 8 corners to screen space for label placement.
    let corners_3d = bbox_corners(bounds);
    let corners_2d: Vec<Option<(f32, f32)>> = corners_3d
        .iter()
        .map(|&c| camera.project_to_screen(c, width, height))
        .collect();

    // Get dimension labels
    let dims = bounds.dimensions();
    let x_label = format!("X: {}", format_dimension(dims.x, config.units));
    let y_label = format!("Y: {}", format_dimension(dims.y, config.units));
    let z_label = format!("Z: {}", format_dimension(dims.z, config.units));

    let label_offset = 20.0 * scale as f32;

    // Use fixed labels if provided, otherwise compute best edges
    let label_edges =
        fixed_labels.unwrap_or_else(|| compute_label_edges(bounds, camera, width, height));

    // Draw labels at the specified edges
    if let (Some(p0), Some(p1)) = (
        corners_2d[label_edges.x_edge.0],
        corners_2d[label_edges.x_edge.1],
    ) {
        draw_edge_label(
            image,
            p0,
            p1,
            &x_label,
            fg_color,
            outline_color,
            scale,
            label_offset,
        );
    }

    if let (Some(p0), Some(p1)) = (
        corners_2d[label_edges.y_edge.0],
        corners_2d[label_edges.y_edge.1],
    ) {
        draw_edge_label(
            image,
            p0,
            p1,
            &y_label,
            fg_color,
            outline_color,
            scale,
            label_offset,
        );
    }

    if let (Some(p0), Some(p1)) = (
        corners_2d[label_edges.z_edge.0],
        corners_2d[label_edges.z_edge.1],
    ) {
        draw_edge_label(
            image,
            p0,
            p1,
            &z_label,
            fg_color,
            outline_color,
            scale,
            label_offset,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dimension_units_conversion() {
        assert_eq!(DimensionUnits::Millimeters.convert(25.4), 25.4);
        assert!((DimensionUnits::Inches.convert(25.4) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_format_dimension() {
        assert_eq!(
            format_dimension(150.0, DimensionUnits::Millimeters),
            "150mm"
        );
        assert_eq!(
            format_dimension(45.5, DimensionUnits::Millimeters),
            "45.5mm"
        );
        assert_eq!(
            format_dimension(1.234, DimensionUnits::Millimeters),
            "1.23mm"
        );
    }

    #[test]
    fn test_measure_text() {
        assert_eq!(measure_text(""), 0);
        assert_eq!(measure_text("1"), 5);
        assert_eq!(measure_text("12"), 11);
        assert_eq!(measure_text("123"), 17);
    }

    #[test]
    fn test_bbox_corners() {
        let mut bounds = BoundingBox::new();
        bounds.extend(Vec3::ZERO);
        bounds.extend(Vec3::ONE);
        let corners = bbox_corners(&bounds);
        assert_eq!(corners[0], Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(corners[7], Vec3::new(1.0, 1.0, 1.0));
    }
}
