use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::MeshReader;
use crate::stl::{StlError, Triangle};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct BoundingBox {
    pub min: [f32; 3],
    pub max: [f32; 3],
    initialized: bool,
}

impl BoundingBox {
    pub fn new() -> Self {
        Self {
            min: [f32::INFINITY; 3],
            max: [f32::NEG_INFINITY; 3],
            initialized: false,
        }
    }

    pub fn extend(&mut self, point: Vec3) {
        self.min[0] = self.min[0].min(point.x);
        self.min[1] = self.min[1].min(point.y);
        self.min[2] = self.min[2].min(point.z);
        self.max[0] = self.max[0].max(point.x);
        self.max[1] = self.max[1].max(point.y);
        self.max[2] = self.max[2].max(point.z);
        self.initialized = true;
    }

    pub fn center(&self) -> Vec3 {
        Vec3::new(
            (self.min[0] + self.max[0]) / 2.0,
            (self.min[1] + self.max[1]) / 2.0,
            (self.min[2] + self.max[2]) / 2.0,
        )
    }

    pub fn dimensions(&self) -> Vec3 {
        Vec3::new(
            self.max[0] - self.min[0],
            self.max[1] - self.min[1],
            self.max[2] - self.min[2],
        )
    }

    pub fn is_valid(&self) -> bool {
        self.initialized
    }

    /// Extend bounds to include all vertices of a triangle.
    pub fn extend_triangle(&mut self, tri: &Triangle) {
        for v in &tri.vertices {
            self.extend(Vec3::from_array(*v));
        }
    }
}

/// Compute bounding box by streaming through all triangles.
/// Does not store the mesh in memory.
pub fn compute_bounds(reader: &MeshReader) -> Result<(BoundingBox, u64), StlError> {
    let mut bounds = BoundingBox::new();
    let mut count = 0u64;

    for result in reader.triangles()? {
        let tri = result?;
        bounds.extend_triangle(&tri);
        count += 1;
    }

    Ok((bounds, count))
}

pub fn compute_normal(v0: Vec3, v1: Vec3, v2: Vec3) -> Vec3 {
    let edge1 = v1 - v0;
    let edge2 = v2 - v0;
    edge1.cross(edge2).normalize_or_zero()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_bounding_box_new_is_invalid() {
        let bbox = BoundingBox::new();
        assert!(!bbox.is_valid());
    }

    #[test]
    fn test_bounding_box_single_point() {
        let mut bbox = BoundingBox::new();
        bbox.extend(Vec3::new(1.0, 2.0, 3.0));

        assert!(bbox.is_valid());
        assert_eq!(bbox.min, [1.0, 2.0, 3.0]);
        assert_eq!(bbox.max, [1.0, 2.0, 3.0]);
        assert_eq!(bbox.dimensions(), Vec3::ZERO);
    }

    #[test]
    fn test_bounding_box_extend() {
        let mut bbox = BoundingBox::new();
        bbox.extend(Vec3::new(0.0, 0.0, 0.0));
        bbox.extend(Vec3::new(1.0, 2.0, 3.0));

        assert_eq!(bbox.min, [0.0, 0.0, 0.0]);
        assert_eq!(bbox.max, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_bounding_box_center() {
        let mut bbox = BoundingBox::new();
        bbox.extend(Vec3::new(0.0, 0.0, 0.0));
        bbox.extend(Vec3::new(2.0, 4.0, 6.0));

        assert_eq!(bbox.center(), Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_bounding_box_dimensions() {
        let mut bbox = BoundingBox::new();
        bbox.extend(Vec3::new(1.0, 2.0, 3.0));
        bbox.extend(Vec3::new(4.0, 6.0, 9.0));

        assert_eq!(bbox.dimensions(), Vec3::new(3.0, 4.0, 6.0));
    }

    #[test]
    fn test_compute_normal_xy_plane() {
        let v0 = Vec3::new(0.0, 0.0, 0.0);
        let v1 = Vec3::new(1.0, 0.0, 0.0);
        let v2 = Vec3::new(0.0, 1.0, 0.0);

        let normal = compute_normal(v0, v1, v2);
        assert!((normal - Vec3::new(0.0, 0.0, 1.0)).length() < 0.001);
    }

    #[test]
    fn test_compute_normal_is_unit_length() {
        let v0 = Vec3::new(0.0, 0.0, 0.0);
        let v1 = Vec3::new(5.0, 0.0, 0.0);
        let v2 = Vec3::new(0.0, 3.0, 0.0);

        let normal = compute_normal(v0, v1, v2);
        assert!((normal.length() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_compute_normal_degenerate() {
        let v0 = Vec3::new(0.0, 0.0, 0.0);
        let v1 = Vec3::new(1.0, 0.0, 0.0);
        let v2 = Vec3::new(2.0, 0.0, 0.0); // collinear

        let normal = compute_normal(v0, v1, v2);
        assert_eq!(normal, Vec3::ZERO);
    }

    #[test]
    fn test_compute_bounds_cube() {
        let path = Path::new("fixtures/cube.stl");
        if path.exists() {
            let reader = crate::MeshReader::open(path).unwrap();
            let (bounds, count) = compute_bounds(&reader).unwrap();

            assert_eq!(count, 12);
            assert!(bounds.is_valid());

            // Cube should be centered at origin with size 1
            let dims = bounds.dimensions();
            assert!((dims.x - 1.0).abs() < 0.001);
            assert!((dims.y - 1.0).abs() < 0.001);
            assert!((dims.z - 1.0).abs() < 0.001);
        }
    }

    #[test]
    fn test_compute_bounds_empty() {
        let path = Path::new("fixtures/empty.stl");
        if path.exists() {
            let reader = crate::MeshReader::open(path).unwrap();
            let (bounds, count) = compute_bounds(&reader).unwrap();

            assert_eq!(count, 0);
            assert!(!bounds.is_valid());
        }
    }

    #[test]
    fn test_compute_bounds_sphere() {
        let path = Path::new("fixtures/sphere.stl");
        if path.exists() {
            let reader = crate::MeshReader::open(path).unwrap();
            let (bounds, count) = compute_bounds(&reader).unwrap();

            assert_eq!(count, 1280);
            assert!(bounds.is_valid());

            // Sphere with radius 0.5 should have dimensions ~1.0
            let dims = bounds.dimensions();
            assert!((dims.x - 1.0).abs() < 0.01);
            assert!((dims.y - 1.0).abs() < 0.01);
            assert!((dims.z - 1.0).abs() < 0.01);
        }
    }
}
