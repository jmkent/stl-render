use glam::{Mat4, Vec3};

use crate::cli::ViewPreset;
use crate::mesh::BoundingBox;

pub struct Camera {
    pub view_matrix: Mat4,
    pub proj_matrix: Mat4,
}

impl Camera {
    pub fn from_preset(
        preset: ViewPreset,
        bounds: &BoundingBox,
        width: u32,
        height: u32,
        padding: f32,
    ) -> Self {
        match preset {
            ViewPreset::Print | ViewPreset::PrintFront => {
                Self::from_print_view_with_azimuth(20.0, bounds, width, height, padding)
            }
            ViewPreset::PrintLeft => {
                Self::from_print_view_with_azimuth(110.0, bounds, width, height, padding)
            }
            ViewPreset::PrintRight => {
                Self::from_print_view_with_azimuth(-70.0, bounds, width, height, padding)
            }
            ViewPreset::PrintBack => {
                Self::from_print_view_with_azimuth(200.0, bounds, width, height, padding)
            }
            ViewPreset::PrintGrid => {
                panic!("PrintGrid should be handled at a higher level, not in Camera::from_preset")
            }
            _ => {
                let (azimuth, elevation) = preset_to_angles(preset);
                Self::from_angles(azimuth, elevation, bounds, width, height, padding)
            }
        }
    }

    pub fn from_angles(
        azimuth: f32,
        elevation: f32,
        bounds: &BoundingBox,
        width: u32,
        height: u32,
        padding: f32,
    ) -> Self {
        let center = bounds.center();
        let dims = bounds.dimensions();
        let max_dim = dims.x.max(dims.y).max(dims.z).max(0.001); // Avoid zero

        // Convert degrees to radians
        let az_rad = azimuth.to_radians();
        let el_rad = elevation.to_radians();

        // Camera position on sphere around center (Y-up coordinate system)
        let distance = max_dim * 3.0;
        let eye = Vec3::new(
            center.x + distance * el_rad.cos() * az_rad.sin(),
            center.y + distance * el_rad.sin(),
            center.z + distance * el_rad.cos() * az_rad.cos(),
        );

        let view_matrix = Mat4::look_at_rh(eye, center, Vec3::Y);

        // Transform bounding box corners to view space to find visible extent
        let corners = bbox_corners(bounds);
        let (view_min, view_max) = project_bounds_to_view(&corners, &view_matrix);

        // Compute orthographic projection that fits the model with padding
        let proj_matrix =
            compute_ortho_projection(view_min, view_max, width, height, padding, distance);

        Self {
            view_matrix,
            proj_matrix,
        }
    }

    /// Print bed view: Z-up coordinate system with configurable azimuth.
    /// Models Z axis as vertical in the rendered image.
    /// - azimuth: rotation around Z axis in degrees (0 = front, 90 = right, etc.)
    fn from_print_view_with_azimuth(
        azimuth: f32,
        bounds: &BoundingBox,
        width: u32,
        height: u32,
        padding: f32,
    ) -> Self {
        let center = bounds.center();
        let dims = bounds.dimensions();
        let max_dim = dims.x.max(dims.y).max(dims.z).max(0.001);

        let distance = max_dim * 3.0;

        // Fixed elevation of 25° from XY plane
        let az_rad = azimuth.to_radians();
        let el_rad = 25.0_f32.to_radians();

        // Position camera using Z-up spherical coordinates
        // In Z-up coords: azimuth rotates in XY plane, elevation lifts from XY plane
        let horizontal_dist = distance * el_rad.cos();
        let eye = Vec3::new(
            center.x + horizontal_dist * az_rad.sin(),
            center.y - horizontal_dist * az_rad.cos(),
            center.z + distance * el_rad.sin(),
        );

        let view_matrix = Mat4::look_at_rh(eye, center, Vec3::Z);

        let corners = bbox_corners(bounds);
        let (view_min, view_max) = project_bounds_to_view(&corners, &view_matrix);

        let proj_matrix =
            compute_ortho_projection(view_min, view_max, width, height, padding, distance);

        Self {
            view_matrix,
            proj_matrix,
        }
    }

    /// Combined view-projection matrix.
    pub fn matrix(&self) -> Mat4 {
        self.proj_matrix * self.view_matrix
    }
}

fn preset_to_angles(preset: ViewPreset) -> (f32, f32) {
    match preset {
        ViewPreset::Front => (0.0, 0.0),
        ViewPreset::Back => (180.0, 0.0),
        ViewPreset::Left => (-90.0, 0.0),
        ViewPreset::Right => (90.0, 0.0),
        ViewPreset::Top => (0.0, 89.99), // Slightly less than 90 to avoid gimbal lock
        ViewPreset::Bottom => (0.0, -89.99),
        ViewPreset::Iso => (45.0, 35.264), // arctan(1/sqrt(2)) ≈ 35.264°
        ViewPreset::Print
        | ViewPreset::PrintFront
        | ViewPreset::PrintLeft
        | ViewPreset::PrintRight
        | ViewPreset::PrintBack
        | ViewPreset::PrintGrid => {
            unreachable!("Print views use Z-up, handled in from_preset")
        }
    }
}

/// Get the 8 corners of a bounding box.
fn bbox_corners(bounds: &BoundingBox) -> [Vec3; 8] {
    let min = Vec3::from_array(bounds.min);
    let max = Vec3::from_array(bounds.max);
    [
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(min.x, max.y, max.z),
        Vec3::new(max.x, max.y, max.z),
    ]
}

/// Project corners into view space and find the min/max XY extent.
fn project_bounds_to_view(corners: &[Vec3; 8], view_matrix: &Mat4) -> (Vec3, Vec3) {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);

    for &corner in corners {
        let view_pos = view_matrix.transform_point3(corner);
        min = min.min(view_pos);
        max = max.max(view_pos);
    }

    (min, max)
}

/// Compute orthographic projection that fits the view-space bounds.
fn compute_ortho_projection(
    view_min: Vec3,
    view_max: Vec3,
    width: u32,
    height: u32,
    padding: f32,
    distance: f32,
) -> Mat4 {
    // Model extent in view space (X = right, Y = up)
    let model_width = view_max.x - view_min.x;
    let model_height = view_max.y - view_min.y;

    // Apply padding
    let padded_width = model_width * (1.0 + padding);
    let padded_height = model_height * (1.0 + padding);

    // Aspect ratio of output image
    let aspect = width as f32 / height as f32;

    // Choose extent that fits model while respecting aspect ratio
    let (half_w, half_h) = if padded_width / padded_height > aspect {
        // Model is wider than viewport - fit to width
        let half_w = padded_width / 2.0;
        let half_h = half_w / aspect;
        (half_w, half_h)
    } else {
        // Model is taller than viewport - fit to height
        let half_h = padded_height / 2.0;
        let half_w = half_h * aspect;
        (half_w, half_h)
    };

    // Center the projection on the model
    let center_x = (view_min.x + view_max.x) / 2.0;
    let center_y = (view_min.y + view_max.y) / 2.0;

    Mat4::orthographic_rh(
        center_x - half_w,
        center_x + half_w,
        center_y - half_h,
        center_y + half_h,
        0.1,
        distance * 4.0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit_cube_bounds() -> BoundingBox {
        let mut bounds = BoundingBox::new();
        bounds.extend(Vec3::splat(-0.5));
        bounds.extend(Vec3::splat(0.5));
        bounds
    }

    fn tall_bounds() -> BoundingBox {
        let mut bounds = BoundingBox::new();
        bounds.extend(Vec3::new(-0.5, -5.0, -0.5));
        bounds.extend(Vec3::new(0.5, 5.0, 0.5));
        bounds
    }

    fn wide_bounds() -> BoundingBox {
        let mut bounds = BoundingBox::new();
        bounds.extend(Vec3::new(-5.0, -0.5, -0.5));
        bounds.extend(Vec3::new(5.0, 0.5, 0.5));
        bounds
    }

    #[test]
    fn test_preset_angles_front() {
        let (az, el) = preset_to_angles(ViewPreset::Front);
        assert_eq!(az, 0.0);
        assert_eq!(el, 0.0);
    }

    #[test]
    fn test_preset_angles_iso() {
        let (az, el) = preset_to_angles(ViewPreset::Iso);
        assert!((az - 45.0).abs() < 0.01);
        assert!((el - 35.264).abs() < 0.01);
    }

    #[test]
    fn test_all_presets_produce_valid_matrices() {
        let bounds = unit_cube_bounds();

        for preset in [
            ViewPreset::Front,
            ViewPreset::Back,
            ViewPreset::Left,
            ViewPreset::Right,
            ViewPreset::Top,
            ViewPreset::Bottom,
            ViewPreset::Iso,
            ViewPreset::Print,
        ] {
            let camera = Camera::from_preset(preset, &bounds, 512, 512, 0.08);
            assert!(!camera.view_matrix.is_nan(), "NaN in {:?} view", preset);
            assert!(!camera.proj_matrix.is_nan(), "NaN in {:?} proj", preset);
        }
    }

    #[test]
    fn test_front_view_looks_down_negative_z() {
        let bounds = unit_cube_bounds();
        let camera = Camera::from_angles(0.0, 0.0, &bounds, 512, 512, 0.0);

        // Camera at +Z looking toward -Z
        // Point at +Z is nearer to camera, point at -Z is farther
        let near_point = camera.view_matrix.transform_point3(Vec3::new(0.0, 0.0, 1.0));
        let far_point = camera.view_matrix.transform_point3(Vec3::new(0.0, 0.0, -1.0));

        // Farther point has more negative Z in view space
        assert!(far_point.z < near_point.z);
    }

    #[test]
    fn test_azimuth_rotates_around_y() {
        let bounds = unit_cube_bounds();

        // At azimuth 0, point at +X should be on the right
        let cam0 = Camera::from_angles(0.0, 0.0, &bounds, 512, 512, 0.0);
        let p0 = cam0.view_matrix.transform_point3(Vec3::new(1.0, 0.0, 0.0));

        // At azimuth 90, point at +X should be behind camera (negative Z)
        let cam90 = Camera::from_angles(90.0, 0.0, &bounds, 512, 512, 0.0);
        let p90 = cam90.view_matrix.transform_point3(Vec3::new(1.0, 0.0, 0.0));

        assert!(p0.x > 0.0, "At az=0, +X should be on right side");
        assert!(p90.z < 0.0, "At az=90, +X should be behind camera");
    }

    #[test]
    fn test_elevation_tilts_view() {
        let bounds = unit_cube_bounds();

        // At elevation 0, point at +Y should be above center
        let cam0 = Camera::from_angles(0.0, 0.0, &bounds, 512, 512, 0.0);
        let p0 = cam0.view_matrix.transform_point3(Vec3::new(0.0, 1.0, 0.0));

        // At elevation 90 (top view), point at +Y should be behind camera
        let cam90 = Camera::from_angles(0.0, 89.0, &bounds, 512, 512, 0.0);
        let p90 = cam90.view_matrix.transform_point3(Vec3::new(0.0, 1.0, 0.0));

        assert!(p0.y > 0.0, "At el=0, +Y should be above center");
        assert!(p90.z < 0.0, "At el=90, +Y should be behind camera");
    }

    #[test]
    fn test_unit_cube_fits_in_square_viewport() {
        let bounds = unit_cube_bounds();
        let camera = Camera::from_preset(ViewPreset::Iso, &bounds, 512, 512, 0.0);

        // All corners should project to NDC within [-1, 1]
        let mvp = camera.matrix();
        for corner in bbox_corners(&bounds) {
            let clip = mvp.project_point3(corner);
            assert!(
                clip.x.abs() <= 1.01 && clip.y.abs() <= 1.01,
                "Corner {:?} projects outside NDC: {:?}",
                corner,
                clip
            );
        }
    }

    #[test]
    fn test_tall_object_centered_vertically() {
        let bounds = tall_bounds();
        let camera = Camera::from_preset(ViewPreset::Front, &bounds, 512, 512, 0.0);

        let mvp = camera.matrix();
        let top = mvp.project_point3(Vec3::new(0.0, 5.0, 0.0));
        let bottom = mvp.project_point3(Vec3::new(0.0, -5.0, 0.0));

        // Should be symmetric around Y=0
        assert!(
            (top.y + bottom.y).abs() < 0.1,
            "Not centered: top.y={}, bottom.y={}",
            top.y,
            bottom.y
        );
    }

    #[test]
    fn test_wide_object_centered_horizontally() {
        let bounds = wide_bounds();
        let camera = Camera::from_preset(ViewPreset::Front, &bounds, 512, 512, 0.0);

        let mvp = camera.matrix();
        let left = mvp.project_point3(Vec3::new(-5.0, 0.0, 0.0));
        let right = mvp.project_point3(Vec3::new(5.0, 0.0, 0.0));

        // Should be symmetric around X=0
        assert!(
            (left.x + right.x).abs() < 0.1,
            "Not centered: left.x={}, right.x={}",
            left.x,
            right.x
        );
    }

    #[test]
    fn test_padding_zero_fills_viewport() {
        let bounds = unit_cube_bounds();
        let camera = Camera::from_preset(ViewPreset::Iso, &bounds, 512, 512, 0.0);

        let mvp = camera.matrix();
        let mut max_extent = 0.0f32;
        for corner in bbox_corners(&bounds) {
            let clip = mvp.project_point3(corner);
            max_extent = max_extent.max(clip.x.abs()).max(clip.y.abs());
        }

        // With no padding, corners should reach close to edge
        assert!(max_extent > 0.9, "Model should fill viewport: {}", max_extent);
    }

    #[test]
    fn test_padding_leaves_margin() {
        let bounds = unit_cube_bounds();
        let camera = Camera::from_preset(ViewPreset::Iso, &bounds, 512, 512, 0.2);

        let mvp = camera.matrix();
        let mut max_extent = 0.0f32;
        for corner in bbox_corners(&bounds) {
            let clip = mvp.project_point3(corner);
            max_extent = max_extent.max(clip.x.abs()).max(clip.y.abs());
        }

        // With 20% padding, model should be smaller
        assert!(
            max_extent < 0.9,
            "With padding, model should not fill viewport: {}",
            max_extent
        );
    }

    #[test]
    fn test_non_square_aspect_ratio() {
        let bounds = unit_cube_bounds();

        // Wide viewport
        let camera = Camera::from_preset(ViewPreset::Iso, &bounds, 800, 400, 0.0);
        let mvp = camera.matrix();

        // Check corners fit
        for corner in bbox_corners(&bounds) {
            let clip = mvp.project_point3(corner);
            assert!(
                clip.x.abs() <= 1.01 && clip.y.abs() <= 1.01,
                "Corner {:?} outside NDC in wide viewport: {:?}",
                corner,
                clip
            );
        }
    }

    #[test]
    fn test_print_views_produce_valid_matrices() {
        let bounds = unit_cube_bounds();

        for preset in [
            ViewPreset::Print,
            ViewPreset::PrintFront,
            ViewPreset::PrintLeft,
            ViewPreset::PrintRight,
            ViewPreset::PrintBack,
        ] {
            let camera = Camera::from_preset(preset, &bounds, 512, 512, 0.08);
            let mvp = camera.matrix();

            // Matrix should have finite values
            for i in 0..4 {
                for j in 0..4 {
                    assert!(
                        mvp.col(i)[j].is_finite(),
                        "Print preset {:?} produced NaN/Inf in matrix",
                        preset
                    );
                }
            }

            // Corners should project to valid NDC
            for corner in bbox_corners(&bounds) {
                let clip = mvp.project_point3(corner);
                assert!(
                    clip.x.is_finite() && clip.y.is_finite() && clip.z.is_finite(),
                    "Print preset {:?} produced NaN/Inf projection for {:?}",
                    preset,
                    corner
                );
            }
        }
    }

    #[test]
    fn test_print_views_use_z_up() {
        let bounds = unit_cube_bounds();

        // All print views should show Z as vertical in the image
        // This means a point moving in +Z should move in +Y in screen space
        for preset in [
            ViewPreset::Print,
            ViewPreset::PrintFront,
            ViewPreset::PrintLeft,
            ViewPreset::PrintRight,
            ViewPreset::PrintBack,
        ] {
            let camera = Camera::from_preset(preset, &bounds, 512, 512, 0.08);
            let mvp = camera.matrix();

            let origin = mvp.project_point3(Vec3::ZERO);
            let up_z = mvp.project_point3(Vec3::new(0.0, 0.0, 0.5));

            // +Z in world should map to +Y in screen (up in image)
            assert!(
                up_z.y > origin.y,
                "Print preset {:?} does not show Z-up: origin.y={}, up_z.y={}",
                preset,
                origin.y,
                up_z.y
            );
        }
    }
}
