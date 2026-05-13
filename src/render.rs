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

        let shade = compute_shade(normal, &camera.view_matrix, config.lighting, config.material_color);

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

fn compute_shade(
    normal: Vec3,
    view_matrix: &Mat4,
    lighting: LightingPreset,
    material_color: [u8; 3],
) -> [u8; 4] {
    let view_normal = view_matrix.transform_vector3(normal).normalize_or_zero();

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
    let final_intensity = (ambient + intensity * (1.0 - ambient)).min(1.0);

    [
        (material_color[0] as f32 * final_intensity) as u8,
        (material_color[1] as f32 * final_intensity) as u8,
        (material_color[2] as f32 * final_intensity) as u8,
        255,
    ]
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
            lighting: LightingPreset::Flat,
            metadata_path: None,
            quiet: true,
            verbose: false,
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
        };

        fb.rasterize_triangle(&tri, &camera, &config);

        let filled_count = fb.color.iter().filter(|c| c[3] > 0).count();
        assert!(filled_count > 100, "Triangle should fill significant area: {}", filled_count);
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
        };

        fb.rasterize_triangle(&tri, &camera, &config);

        let filled_count = fb.color.iter().filter(|c| c[3] > 0).count();
        assert_eq!(filled_count, 0, "Triangle outside viewport should not render");
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
        };
        fb.rasterize_triangle(&tri_far, &camera, &config1);

        let mut config2 = test_config();
        config2.material_color = [0, 255, 0];

        let tri_near = Triangle {
            vertices: [[-0.3, -0.3, 0.1], [0.3, -0.3, 0.1], [0.0, 0.3, 0.1]],
            normal: [0.0, 0.0, 1.0],
        };
        fb.rasterize_triangle(&tri_near, &camera, &config2);

        let center_idx = (32 * 64 + 32) as usize;
        let pixel = fb.color[center_idx];
        assert!(pixel[1] > pixel[0], "Nearer green triangle should win: {:?}", pixel);
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

        assert!(shade[0] < 50, "Facing away should be dark (ambient only): {:?}", shade);
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
    fn test_aa_downsampling() {
        let fb = Framebuffer::new(8, 8, Background::Solid, [100, 100, 100]);
        let img = fb.into_image(AntiAliasing::X2);
        assert_eq!(img.width(), 4);
        assert_eq!(img.height(), 4);

        let fb4 = Framebuffer::new(16, 16, Background::Solid, [100, 100, 100]);
        let img4 = fb4.into_image(AntiAliasing::X4);
        assert_eq!(img4.width(), 4);
        assert_eq!(img4.height(), 4);
    }
}
