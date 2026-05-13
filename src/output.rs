use std::path::Path;

use image::RgbaImage;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::mesh::BoundingBox;

#[derive(Debug, Error)]
pub enum OutputError {
    #[error("failed to write image: {0}")]
    Image(#[from] image::ImageError),

    #[error("failed to write file: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to serialize metadata: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderMetadata {
    pub input_file: String,
    pub triangle_count: u64,
    pub bounding_box: BoundingBox,
    pub dimensions: [f32; 3],
}

pub fn write_png(image: &RgbaImage, path: &Path) -> Result<(), OutputError> {
    image.save(path)?;
    Ok(())
}

pub fn write_metadata(meta: &RenderMetadata, path: &Path) -> Result<(), OutputError> {
    let json = serde_json::to_string_pretty(meta)?;
    std::fs::write(path, json)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_write_png() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.png");

        let img = RgbaImage::from_fn(10, 10, |_, _| image::Rgba([128, 128, 128, 255]));
        write_png(&img, &path).unwrap();

        assert!(path.exists());
    }

    #[test]
    fn test_write_metadata() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("meta.json");

        let meta = RenderMetadata {
            input_file: "test.stl".into(),
            triangle_count: 12,
            bounding_box: BoundingBox::new(),
            dimensions: [1.0, 1.0, 1.0],
        };
        write_metadata(&meta, &path).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("\"triangle_count\": 12"));
    }
}
