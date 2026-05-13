use glam::Vec3;
use serde::{Deserialize, Serialize};

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
}

pub fn compute_normal(v0: Vec3, v1: Vec3, v2: Vec3) -> Vec3 {
    let edge1 = v1 - v0;
    let edge2 = v2 - v0;
    edge1.cross(edge2).normalize_or_zero()
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let center = bbox.center();
        assert_eq!(center, Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_bounding_box_dimensions() {
        let mut bbox = BoundingBox::new();
        bbox.extend(Vec3::new(1.0, 2.0, 3.0));
        bbox.extend(Vec3::new(4.0, 6.0, 9.0));

        let dims = bbox.dimensions();
        assert_eq!(dims, Vec3::new(3.0, 4.0, 6.0));
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
    fn test_compute_normal_degenerate() {
        let v0 = Vec3::new(0.0, 0.0, 0.0);
        let v1 = Vec3::new(1.0, 0.0, 0.0);
        let v2 = Vec3::new(2.0, 0.0, 0.0); // collinear

        let normal = compute_normal(v0, v1, v2);
        assert_eq!(normal, Vec3::ZERO);
    }
}
