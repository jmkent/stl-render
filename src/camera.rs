use glam::{Mat4, Vec3};

use crate::cli::ViewPreset;
use crate::mesh::BoundingBox;

pub struct Camera {
    pub view_matrix: Mat4,
    pub proj_matrix: Mat4,
}

impl Camera {
    pub fn from_preset(preset: ViewPreset, bounds: &BoundingBox, padding: f32) -> Self {
        let (azimuth, elevation) = preset_to_angles(preset);
        Self::from_angles(azimuth, elevation, bounds, padding)
    }

    pub fn from_angles(azimuth: f32, elevation: f32, bounds: &BoundingBox, padding: f32) -> Self {
        let center = bounds.center();
        let dims = bounds.dimensions();
        let max_dim = dims.x.max(dims.y).max(dims.z);

        // Convert degrees to radians
        let az_rad = azimuth.to_radians();
        let el_rad = elevation.to_radians();

        // Camera position on sphere around center
        let distance = max_dim * 2.0;
        let eye = Vec3::new(
            center.x + distance * el_rad.cos() * az_rad.sin(),
            center.y + distance * el_rad.sin(),
            center.z + distance * el_rad.cos() * az_rad.cos(),
        );

        let view_matrix = Mat4::look_at_rh(eye, center, Vec3::Y);

        // Orthographic projection sized to fit model with padding
        let extent = max_dim * (1.0 + padding);
        let proj_matrix = Mat4::orthographic_rh(-extent, extent, -extent, extent, 0.1, distance * 4.0);

        Self {
            view_matrix,
            proj_matrix,
        }
    }
}

fn preset_to_angles(preset: ViewPreset) -> (f32, f32) {
    match preset {
        ViewPreset::Front => (0.0, 0.0),
        ViewPreset::Back => (180.0, 0.0),
        ViewPreset::Left => (90.0, 0.0),
        ViewPreset::Right => (-90.0, 0.0),
        ViewPreset::Top => (0.0, 90.0),
        ViewPreset::Bottom => (0.0, -90.0),
        ViewPreset::Iso => (45.0, 35.264), // arctan(1/sqrt(2)) ≈ 35.264°
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_angles() {
        assert_eq!(preset_to_angles(ViewPreset::Front), (0.0, 0.0));
        assert_eq!(preset_to_angles(ViewPreset::Top), (0.0, 90.0));

        let (az, el) = preset_to_angles(ViewPreset::Iso);
        assert!((az - 45.0).abs() < 0.01);
        assert!((el - 35.264).abs() < 0.01);
    }

    #[test]
    fn test_camera_from_preset() {
        let mut bounds = BoundingBox::new();
        bounds.extend(glam::Vec3::splat(-0.5));
        bounds.extend(glam::Vec3::splat(0.5));

        let camera = Camera::from_preset(ViewPreset::Iso, &bounds, 0.08);
        assert!(!camera.view_matrix.is_nan());
        assert!(!camera.proj_matrix.is_nan());
    }
}
