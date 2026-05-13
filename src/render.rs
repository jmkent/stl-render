use image::RgbaImage;

use crate::cli::{AntiAliasing, Background, RenderConfig};
use crate::camera::Camera;
use crate::stl::Triangle;

pub struct Framebuffer {
    pub width: u32,
    pub height: u32,
    #[allow(dead_code)] // Used in M5: triangle rasterization
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

    pub fn rasterize_triangle(
        &mut self,
        _tri: &Triangle,
        _camera: &Camera,
        _config: &RenderConfig,
    ) {
        // Rasterization will be implemented in M5
        todo!("triangle rasterization")
    }

    pub fn into_image(self, _aa: AntiAliasing) -> RgbaImage {
        RgbaImage::from_fn(self.width, self.height, |x, y| {
            let idx = (y * self.width + x) as usize;
            let [r, g, b, a] = self.color[idx];
            image::Rgba([r, g, b, a])
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
