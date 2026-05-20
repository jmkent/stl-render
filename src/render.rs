use glam::{Mat4, Vec3, Vec4};
use image::RgbaImage;

use crate::camera::Camera;
use crate::cli::{AntiAliasing, Background, LightingPreset, RenderConfig};
use crate::mesh::compute_normal;
use crate::stl::Triangle;

pub struct Framebuffer {
    pub width: u32,
    pub height: u32,
    depth: Vec<f32>,
    color: Vec<[u8; 4]>,
}

impl Framebuffer {
    pub fn new(width: u32, height: u32, background: Background, bg_color: [u8; 3]) -> Self {
        let size = (width * height) as usize;
        let depth = vec![f32::INFINITY; size];

        let bg_pixel = match background {
            Background::Transparent => [0, 0, 0, 0],
            Background::Solid => [bg_color[0], bg_color[1], bg_color[2], 255],
        };
        let color = vec![bg_pixel; size];

        Self {
            width,
            height,
            depth,
            color,
        }
    }

    /// Get the color at a specific buffer index.
    pub fn get_color(&self, idx: usize) -> [u8; 4] {
        self.color.get(idx).copied().unwrap_or([0, 0, 0, 0])
    }

    pub fn rasterize_triangle(&mut self, tri: &Triangle, camera: &Camera, config: &RenderConfig) {
        let mvp = camera.matrix();

        let v0 = Vec3::from_array(tri.vertices[0]);
        let v1 = Vec3::from_array(tri.vertices[1]);
        let v2 = Vec3::from_array(tri.vertices[2]);

        let normal = compute_normal(v0, v1, v2);
        if normal == Vec3::ZERO {
            return;
        }

        let p0 = project_vertex(v0, &mvp, self.width, self.height);
        let p1 = project_vertex(v1, &mvp, self.width, self.height);
        let p2 = project_vertex(v2, &mvp, self.width, self.height);

        if p0.is_none() || p1.is_none() || p2.is_none() {
            return;
        }
        let (s0, z0) = p0.unwrap();
        let (s1, z1) = p1.unwrap();
        let (s2, z2) = p2.unwrap();

        if is_backfacing(&s0, &s1, &s2) {
            return;
        }

        // Pre-compute lighting factor for the triangle normal
        let view_normal = (camera.view_matrix.transform_vector3(normal)).normalize();
        let lighting = compute_lighting_factor(view_normal, config.lighting);

        // Determine if we should use per-vertex colors
        let vertex_colors = if config.use_mesh_colors {
            tri.vertex_colors
        } else {
            None
        };

        // For uniform material color, pre-compute the shade once
        let uniform_shade = if vertex_colors.is_none() {
            Some(apply_lighting(config.material_color, lighting))
        } else {
            None
        };

        let min_x = s0.x.min(s1.x).min(s2.x).max(0.0) as u32;
        let max_x = s0.x.max(s1.x).max(s2.x).min((self.width - 1) as f32) as u32;
        let min_y = s0.y.min(s1.y).min(s2.y).max(0.0) as u32;
        let max_y = s0.y.max(s1.y).max(s2.y).min((self.height - 1) as f32) as u32;

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let p = Vec3::new(x as f32 + 0.5, y as f32 + 0.5, 0.0);

                if let Some((u, v, w)) = barycentric(&s0, &s1, &s2, &p)
                    && u >= 0.0
                    && v >= 0.0
                    && w >= 0.0
                {
                    let z = u * z0 + v * z1 + w * z2;
                    let idx = (y * self.width + x) as usize;

                    if z < self.depth[idx] {
                        self.depth[idx] = z;

                        // Compute pixel color
                        let shade = if let Some(vc) = vertex_colors {
                            // Interpolate vertex colors using barycentric coordinates (sRGB space per 3MF spec)
                            let interp = interpolate_vertex_colors(vc, u, v, w);
                            apply_lighting([interp[0], interp[1], interp[2]], lighting)
                        } else {
                            uniform_shade.unwrap()
                        };

                        self.color[idx] = shade;
                    }
                }
            }
        }
    }

    pub fn into_image(self, aa: AntiAliasing) -> RgbaImage {
        match aa {
            AntiAliasing::None => RgbaImage::from_fn(self.width, self.height, |x, y| {
                let idx = (y * self.width + x) as usize;
                let [r, g, b, a] = self.color[idx];
                image::Rgba([r, g, b, a])
            }),
            AntiAliasing::X2 | AntiAliasing::X4 => {
                let scale = match aa {
                    AntiAliasing::X2 => 2,
                    AntiAliasing::X4 => 4,
                    AntiAliasing::None => 1,
                };
                let out_w = self.width / scale;
                let out_h = self.height / scale;

                RgbaImage::from_fn(out_w, out_h, |x, y| {
                    let mut r_sum = 0u32;
                    let mut g_sum = 0u32;
                    let mut b_sum = 0u32;
                    let mut a_sum = 0u32;

                    for dy in 0..scale {
                        for dx in 0..scale {
                            let sx = x * scale + dx;
                            let sy = y * scale + dy;
                            let idx = (sy * self.width + sx) as usize;
                            let [r, g, b, a] = self.color[idx];
                            r_sum += r as u32;
                            g_sum += g as u32;
                            b_sum += b as u32;
                            a_sum += a as u32;
                        }
                    }

                    let count = scale * scale;
                    image::Rgba([
                        (r_sum / count) as u8,
                        (g_sum / count) as u8,
                        (b_sum / count) as u8,
                        (a_sum / count) as u8,
                    ])
                })
            }
        }
    }

    /// Draw a depth-tested dashed line between two screen-space points.
    /// Points are (x, y, depth) where depth is the NDC z value.
    pub fn draw_dashed_line(
        &mut self,
        x0: f32,
        y0: f32,
        z0: f32,
        x1: f32,
        y1: f32,
        z1: f32,
        color: [u8; 4],
        dash_len: i32,
        gap_len: i32,
        thickness: i32,
    ) {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let dz = z1 - z0;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1.0 {
            return;
        }

        let steps = len as i32;
        let cycle = dash_len + gap_len;

        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let x = x0 + dx * t;
            let y = y0 + dy * t;
            let z = z0 + dz * t;

            let px = x as i32;
            let py = y as i32;

            // Check bounds
            if px < 0 || py < 0 || px >= self.width as i32 || py >= self.height as i32 {
                continue;
            }

            // Check if in dash (not gap)
            if (i % cycle) >= dash_len {
                continue;
            }

            let radius = (thickness.max(1) - 1) / 2;
            for oy in -radius..=radius {
                for ox in -radius..=radius {
                    let tx = px + ox;
                    let ty = py + oy;
                    if tx < 0 || ty < 0 || tx >= self.width as i32 || ty >= self.height as i32 {
                        continue;
                    }

                    let idx = (ty as u32 * self.width + tx as u32) as usize;

                    // Depth test: only draw if this pixel is closer than existing
                    if z < self.depth[idx] {
                        self.depth[idx] = z;
                        self.color[idx] = color;
                    }
                }
            }
        }
    }

    /// Draw the bounding box edges with depth testing.
    pub fn draw_bounding_box(
        &mut self,
        bounds: &crate::mesh::BoundingBox,
        camera: &Camera,
        color: [u8; 4],
        dash_len: i32,
        gap_len: i32,
        thickness: i32,
    ) {
        use glam::Vec3;

        let min = Vec3::from_array(bounds.min);
        let max = Vec3::from_array(bounds.max);

        // 8 corners of the bounding box
        let corners = [
            Vec3::new(min.x, min.y, min.z), // 0: ---
            Vec3::new(max.x, min.y, min.z), // 1: +--
            Vec3::new(min.x, max.y, min.z), // 2: -+-
            Vec3::new(max.x, max.y, min.z), // 3: ++-
            Vec3::new(min.x, min.y, max.z), // 4: --+
            Vec3::new(max.x, min.y, max.z), // 5: +-+
            Vec3::new(min.x, max.y, max.z), // 6: -++
            Vec3::new(max.x, max.y, max.z), // 7: +++
        ];

        // Project corners to screen space
        let mvp = camera.matrix();
        let projected: Vec<Option<(Vec3, f32)>> = corners
            .iter()
            .map(|&c| project_vertex(c, &mvp, self.width, self.height))
            .collect();

        // 12 edges of the box
        let edges = [
            (0, 1),
            (1, 3),
            (3, 2),
            (2, 0), // bottom face
            (4, 5),
            (5, 7),
            (7, 6),
            (6, 4), // top face
            (0, 4),
            (1, 5),
            (2, 6),
            (3, 7), // vertical edges
        ];

        for &(i, j) in &edges {
            if let (Some((p0, z0)), Some((p1, z1))) = (projected[i], projected[j]) {
                self.draw_dashed_line(
                    p0.x, p0.y, z0, p1.x, p1.y, z1, color, dash_len, gap_len, thickness,
                );
            }
        }
    }
}

fn project_vertex(v: Vec3, mvp: &Mat4, width: u32, height: u32) -> Option<(Vec3, f32)> {
    let clip = *mvp * Vec4::new(v.x, v.y, v.z, 1.0);

    if clip.w.abs() < 1e-6 {
        return None;
    }

    let ndc = Vec3::new(clip.x / clip.w, clip.y / clip.w, clip.z / clip.w);

    if ndc.z < -1.0 || ndc.z > 1.0 {
        return None;
    }

    let screen = Vec3::new(
        (ndc.x + 1.0) * 0.5 * width as f32,
        (1.0 - ndc.y) * 0.5 * height as f32,
        0.0,
    );

    Some((screen, ndc.z))
}

fn is_backfacing(v0: &Vec3, v1: &Vec3, v2: &Vec3) -> bool {
    let edge1 = *v1 - *v0;
    let edge2 = *v2 - *v0;
    let cross = edge1.x * edge2.y - edge1.y * edge2.x;
    cross >= 0.0
}

fn barycentric(v0: &Vec3, v1: &Vec3, v2: &Vec3, p: &Vec3) -> Option<(f32, f32, f32)> {
    let v0v1 = *v1 - *v0;
    let v0v2 = *v2 - *v0;
    let v0p = *p - *v0;

    let d00 = v0v1.x * v0v1.x + v0v1.y * v0v1.y;
    let d01 = v0v1.x * v0v2.x + v0v1.y * v0v2.y;
    let d11 = v0v2.x * v0v2.x + v0v2.y * v0v2.y;
    let d20 = v0p.x * v0v1.x + v0p.y * v0v1.y;
    let d21 = v0p.x * v0v2.x + v0p.y * v0v2.y;

    let denom = d00 * d11 - d01 * d01;
    if denom.abs() < 1e-10 {
        return None;
    }

    let v = (d11 * d20 - d01 * d21) / denom;
    let w = (d00 * d21 - d01 * d20) / denom;
    let u = 1.0 - v - w;

    Some((u, v, w))
}

/// Compute lighting intensity factor for a normal vector.
fn compute_lighting_factor(view_normal: Vec3, lighting: LightingPreset) -> f32 {
    let intensity = match lighting {
        LightingPreset::Flat => {
            let light_dir = Vec3::new(0.0, 0.0, 1.0);
            view_normal.dot(light_dir).max(0.0)
        }
        LightingPreset::Studio => {
            let key = Vec3::new(0.5, 0.5, 0.7).normalize();
            let fill = Vec3::new(-0.3, 0.2, 0.5).normalize();
            let rim = Vec3::new(0.0, -0.5, -0.5).normalize();

            let key_i = view_normal.dot(key).max(0.0) * 0.7;
            let fill_i = view_normal.dot(fill).max(0.0) * 0.3;
            let rim_i = view_normal.dot(rim).max(0.0) * 0.2;

            (key_i + fill_i + rim_i).min(1.0)
        }
        LightingPreset::Technical => {
            let lights = [
                Vec3::new(0.0, 0.0, 1.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(-1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ];
            let total: f32 = lights.iter().map(|l| view_normal.dot(*l).max(0.0)).sum();
            (total / lights.len() as f32 + 0.2).min(1.0)
        }
    };

    let ambient = 0.15;
    (ambient + intensity * (1.0 - ambient)).min(1.0)
}

/// Apply lighting factor to a base color.
fn apply_lighting(color: [u8; 3], lighting_factor: f32) -> [u8; 4] {
    [
        (color[0] as f32 * lighting_factor) as u8,
        (color[1] as f32 * lighting_factor) as u8,
        (color[2] as f32 * lighting_factor) as u8,
        255,
    ]
}

/// Interpolate vertex colors using barycentric coordinates (sRGB space per 3MF spec).
fn interpolate_vertex_colors(colors: [[u8; 4]; 3], u: f32, v: f32, w: f32) -> [u8; 4] {
    [
        (colors[0][0] as f32 * u + colors[1][0] as f32 * v + colors[2][0] as f32 * w) as u8,
        (colors[0][1] as f32 * u + colors[1][1] as f32 * v + colors[2][1] as f32 * w) as u8,
        (colors[0][2] as f32 * u + colors[1][2] as f32 * v + colors[2][2] as f32 * w) as u8,
        (colors[0][3] as f32 * u + colors[1][3] as f32 * v + colors[2][3] as f32 * w) as u8,
    ]
}

#[cfg(test)]
fn compute_shade(
    normal: Vec3,
    view_matrix: &Mat4,
    lighting: LightingPreset,
    material_color: [u8; 3],
) -> [u8; 4] {
    let view_normal = view_matrix.transform_vector3(normal).normalize_or_zero();
    let lighting_factor = compute_lighting_factor(view_normal, lighting);
    apply_lighting(material_color, lighting_factor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::ViewConfig;
    use crate::mesh::BoundingBox;
    use std::path::PathBuf;

    fn test_config() -> RenderConfig {
        RenderConfig {
            input: PathBuf::from("test.stl"),
            output: PathBuf::from("test.png"),
            width: 64,
            height: 64,
            view: ViewConfig::Preset(crate::cli::ViewPreset::Front),
            padding: 0.0,
            aa: AntiAliasing::None,
            background: Background::Transparent,
            background_color: [255, 255, 255],
            material_color: [200, 200, 200],
            use_mesh_colors: true,
            lighting: LightingPreset::Flat,
            metadata_path: None,
            quiet: true,
            verbose: false,
            animate: false,
            frames: 16,
            frame_delay: 100,
            dimension_config: crate::overlay::DimensionConfig::default(),
        }
    }

    fn unit_cube_bounds() -> BoundingBox {
        let mut bounds = BoundingBox::new();
        bounds.extend(Vec3::splat(-0.5));
        bounds.extend(Vec3::splat(0.5));
        bounds
    }

    #[test]
    fn test_framebuffer_new_transparent() {
        let fb = Framebuffer::new(10, 10, Background::Transparent, [255, 255, 255]);
        assert_eq!(fb.width, 10);
        assert_eq!(fb.height, 10);
        assert_eq!(fb.color[0], [0, 0, 0, 0]);
    }

    #[test]
    fn test_framebuffer_new_solid() {
        let fb = Framebuffer::new(10, 10, Background::Solid, [128, 64, 32]);
        assert_eq!(fb.color[0], [128, 64, 32, 255]);
    }

    #[test]
    fn test_framebuffer_into_image() {
        let fb = Framebuffer::new(2, 2, Background::Solid, [100, 100, 100]);
        let img = fb.into_image(AntiAliasing::None);
        assert_eq!(img.width(), 2);
        assert_eq!(img.height(), 2);
    }

    #[test]
    fn test_rasterize_fullscreen_triangle() {
        let mut fb = Framebuffer::new(64, 64, Background::Transparent, [0, 0, 0]);
        let config = test_config();
        let bounds = unit_cube_bounds();
        let camera = Camera::from_preset(crate::cli::ViewPreset::Front, &bounds, 64, 64, 0.0);

        let tri = Triangle {
            vertices: [[-0.4, -0.4, 0.0], [0.4, -0.4, 0.0], [0.0, 0.4, 0.0]],
            normal: [0.0, 0.0, 1.0],
            vertex_colors: None,
        };

        fb.rasterize_triangle(&tri, &camera, &config);

        let filled_count = fb.color.iter().filter(|c| c[3] > 0).count();
        assert!(
            filled_count > 100,
            "Triangle should fill significant area: {}",
            filled_count
        );
    }

    #[test]
    fn test_triangle_outside_viewport_clipped() {
        let mut fb = Framebuffer::new(64, 64, Background::Transparent, [0, 0, 0]);
        let config = test_config();
        let bounds = unit_cube_bounds();
        let camera = Camera::from_preset(crate::cli::ViewPreset::Front, &bounds, 64, 64, 0.0);

        let tri = Triangle {
            vertices: [[10.0, 10.0, 0.0], [11.0, 10.0, 0.0], [10.5, 11.0, 0.0]],
            normal: [0.0, 0.0, 1.0],
            vertex_colors: None,
        };

        fb.rasterize_triangle(&tri, &camera, &config);

        let filled_count = fb.color.iter().filter(|c| c[3] > 0).count();
        assert_eq!(
            filled_count, 0,
            "Triangle outside viewport should not render"
        );
    }

    #[test]
    fn test_depth_test_nearer_wins() {
        let mut fb = Framebuffer::new(64, 64, Background::Transparent, [0, 0, 0]);
        let bounds = unit_cube_bounds();
        let camera = Camera::from_preset(crate::cli::ViewPreset::Front, &bounds, 64, 64, 0.0);

        let mut config1 = test_config();
        config1.material_color = [255, 0, 0];

        let tri_far = Triangle {
            vertices: [[-0.4, -0.4, -0.2], [0.4, -0.4, -0.2], [0.0, 0.4, -0.2]],
            normal: [0.0, 0.0, 1.0],
            vertex_colors: None,
        };
        fb.rasterize_triangle(&tri_far, &camera, &config1);

        let mut config2 = test_config();
        config2.material_color = [0, 255, 0];

        let tri_near = Triangle {
            vertices: [[-0.3, -0.3, 0.1], [0.3, -0.3, 0.1], [0.0, 0.3, 0.1]],
            normal: [0.0, 0.0, 1.0],
            vertex_colors: None,
        };
        fb.rasterize_triangle(&tri_near, &camera, &config2);

        let center_idx = (32 * 64 + 32) as usize;
        let pixel = fb.color[center_idx];
        assert!(
            pixel[1] > pixel[0],
            "Nearer green triangle should win: {:?}",
            pixel
        );
    }

    #[test]
    fn test_backface_culled() {
        let mut fb = Framebuffer::new(64, 64, Background::Transparent, [0, 0, 0]);
        let config = test_config();
        let bounds = unit_cube_bounds();
        let camera = Camera::from_preset(crate::cli::ViewPreset::Front, &bounds, 64, 64, 0.0);

        let tri = Triangle {
            vertices: [[-0.4, -0.4, 0.0], [0.0, 0.4, 0.0], [0.4, -0.4, 0.0]],
            normal: [0.0, 0.0, -1.0],
            vertex_colors: None,
        };

        fb.rasterize_triangle(&tri, &camera, &config);

        let filled_count = fb.color.iter().filter(|c| c[3] > 0).count();
        assert_eq!(filled_count, 0, "Backfacing triangle should be culled");
    }

    #[test]
    fn test_shade_facing_light_is_bright() {
        let normal = Vec3::new(0.0, 0.0, 1.0);
        let view_matrix = Mat4::IDENTITY;
        let material = [200, 200, 200];

        let shade = compute_shade(normal, &view_matrix, LightingPreset::Flat, material);

        assert!(shade[0] > 150, "Facing light should be bright: {:?}", shade);
    }

    #[test]
    fn test_shade_facing_away_is_dark() {
        let normal = Vec3::new(0.0, 0.0, -1.0);
        let view_matrix = Mat4::IDENTITY;
        let material = [200, 200, 200];

        let shade = compute_shade(normal, &view_matrix, LightingPreset::Flat, material);

        assert!(
            shade[0] < 50,
            "Facing away should be dark (ambient only): {:?}",
            shade
        );
    }

    #[test]
    fn test_material_color_affects_output() {
        let normal = Vec3::new(0.0, 0.0, 1.0);
        let view_matrix = Mat4::IDENTITY;

        let red = compute_shade(normal, &view_matrix, LightingPreset::Flat, [255, 0, 0]);
        let blue = compute_shade(normal, &view_matrix, LightingPreset::Flat, [0, 0, 255]);

        assert!(red[0] > red[2], "Red material should have more R");
        assert!(blue[2] > blue[0], "Blue material should have more B");
    }

    #[test]
    fn test_aa_none_preserves_resolution() {
        let fb = Framebuffer::new(64, 64, Background::Solid, [100, 100, 100]);
        let img = fb.into_image(AntiAliasing::None);
        assert_eq!(img.width(), 64);
        assert_eq!(img.height(), 64);
    }

    #[test]
    fn test_aa_2x_halves_resolution() {
        let fb = Framebuffer::new(128, 128, Background::Solid, [100, 100, 100]);
        let img = fb.into_image(AntiAliasing::X2);
        assert_eq!(img.width(), 64);
        assert_eq!(img.height(), 64);
    }

    #[test]
    fn test_aa_4x_quarters_resolution() {
        let fb = Framebuffer::new(256, 256, Background::Solid, [100, 100, 100]);
        let img = fb.into_image(AntiAliasing::X4);
        assert_eq!(img.width(), 64);
        assert_eq!(img.height(), 64);
    }

    #[test]
    fn test_aa_box_filter_averages() {
        let mut fb = Framebuffer::new(4, 4, Background::Solid, [0, 0, 0]);
        // Set top-left 2x2 block to white
        fb.color[0] = [255, 255, 255, 255];
        fb.color[1] = [255, 255, 255, 255];
        fb.color[4] = [255, 255, 255, 255];
        fb.color[5] = [255, 255, 255, 255];

        let img = fb.into_image(AntiAliasing::X2);
        assert_eq!(img.width(), 2);
        assert_eq!(img.height(), 2);

        // Top-left pixel should be white (average of 4 white pixels)
        let tl = img.get_pixel(0, 0);
        assert_eq!(tl[0], 255);

        // Top-right pixel should be black (average of 4 black pixels)
        let tr = img.get_pixel(1, 0);
        assert_eq!(tr[0], 0);
    }

    #[test]
    fn test_flat_lighting_single_front_light() {
        let view_matrix = Mat4::IDENTITY;
        let material = [200, 200, 200];

        let front = compute_shade(
            Vec3::new(0.0, 0.0, 1.0),
            &view_matrix,
            LightingPreset::Flat,
            material,
        );
        let side = compute_shade(
            Vec3::new(1.0, 0.0, 0.0),
            &view_matrix,
            LightingPreset::Flat,
            material,
        );
        let back = compute_shade(
            Vec3::new(0.0, 0.0, -1.0),
            &view_matrix,
            LightingPreset::Flat,
            material,
        );

        assert!(front[0] > side[0], "Front should be brighter than side");
        assert!(
            side[0] >= back[0],
            "Side should be >= back (both get only ambient)"
        );
    }

    #[test]
    fn test_studio_lighting_multiple_lights() {
        let view_matrix = Mat4::IDENTITY;
        let material = [200, 200, 200];

        let front = compute_shade(
            Vec3::new(0.0, 0.0, 1.0),
            &view_matrix,
            LightingPreset::Studio,
            material,
        );
        let left = compute_shade(
            Vec3::new(-1.0, 0.0, 0.0),
            &view_matrix,
            LightingPreset::Studio,
            material,
        );
        let right = compute_shade(
            Vec3::new(1.0, 0.0, 0.0),
            &view_matrix,
            LightingPreset::Studio,
            material,
        );

        assert!(
            front[0] > 100,
            "Front should be well-lit in studio: {:?}",
            front
        );
        assert!(
            left[0] != right[0],
            "Studio lighting should be asymmetric: left={:?}, right={:?}",
            left,
            right
        );
    }

    #[test]
    fn test_technical_lighting_uniform() {
        let view_matrix = Mat4::IDENTITY;
        let material = [200, 200, 200];

        let front = compute_shade(
            Vec3::new(0.0, 0.0, 1.0),
            &view_matrix,
            LightingPreset::Technical,
            material,
        );
        let left = compute_shade(
            Vec3::new(-1.0, 0.0, 0.0),
            &view_matrix,
            LightingPreset::Technical,
            material,
        );
        let right = compute_shade(
            Vec3::new(1.0, 0.0, 0.0),
            &view_matrix,
            LightingPreset::Technical,
            material,
        );
        let top = compute_shade(
            Vec3::new(0.0, 1.0, 0.0),
            &view_matrix,
            LightingPreset::Technical,
            material,
        );

        assert_eq!(
            left[0], right[0],
            "Technical should be symmetric left/right"
        );
        let variance = [front[0], left[0], top[0]]
            .iter()
            .map(|&v| v as i32)
            .map(|v| (v - 128).abs())
            .max()
            .unwrap();
        assert!(
            variance < 80,
            "Technical lighting should be relatively uniform"
        );
    }

    #[test]
    fn test_lighting_presets_differ() {
        let view_matrix = Mat4::IDENTITY;
        let material = [200, 200, 200];
        let normal = Vec3::new(0.5, 0.5, 0.7).normalize();

        let flat = compute_shade(normal, &view_matrix, LightingPreset::Flat, material);
        let studio = compute_shade(normal, &view_matrix, LightingPreset::Studio, material);
        let technical = compute_shade(normal, &view_matrix, LightingPreset::Technical, material);

        assert!(
            flat != studio || studio != technical,
            "Different presets should produce different results: flat={:?}, studio={:?}, technical={:?}",
            flat,
            studio,
            technical
        );
    }
}
