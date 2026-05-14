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
    use image::ImageEncoder;
    use std::fs::File;
    use std::io::BufWriter;

    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    let encoder = image::codecs::png::PngEncoder::new(writer);
    encoder.write_image(
        image.as_raw(),
        image.width(),
        image.height(),
        image::ExtendedColorType::Rgba8,
    )?;
    Ok(())
}

pub fn write_png_to_stdout(image: &RgbaImage) -> Result<(), OutputError> {
    use image::ImageEncoder;
    use std::io::Write;

    let stdout = std::io::stdout();
    let mut handle = stdout.lock();

    let encoder = image::codecs::png::PngEncoder::new(&mut handle);
    encoder.write_image(
        image.as_raw(),
        image.width(),
        image.height(),
        image::ExtendedColorType::Rgba8,
    )?;

    handle.flush()?;
    Ok(())
}

pub fn write_metadata(meta: &RenderMetadata, path: &Path) -> Result<(), OutputError> {
    let json = serde_json::to_string_pretty(meta)?;
    std::fs::write(path, json)?;
    Ok(())
}

pub fn write_gif(
    frames: &[RgbaImage],
    path: &Path,
    frame_delay_ms: u16,
) -> Result<(), OutputError> {
    use image::Frame;
    use image::codecs::gif::{GifEncoder, Repeat};
    use std::fs::File;

    let file = File::create(path)?;
    let mut encoder = GifEncoder::new(file);
    encoder.set_repeat(Repeat::Infinite)?;

    for img in frames {
        let frame = Frame::from_parts(
            img.clone(),
            0,
            0,
            image::Delay::from_numer_denom_ms(frame_delay_ms as u32, 1),
        );
        encoder.encode_frame(frame)?;
    }

    Ok(())
}

pub fn write_gif_to_stdout(frames: &[RgbaImage], frame_delay_ms: u16) -> Result<(), OutputError> {
    use image::Frame;
    use image::codecs::gif::{GifEncoder, Repeat};
    use std::io::Write;

    let stdout = std::io::stdout();
    let mut handle = stdout.lock();

    {
        let mut encoder = GifEncoder::new(&mut handle);
        encoder.set_repeat(Repeat::Infinite)?;

        for img in frames {
            let frame = Frame::from_parts(
                img.clone(),
                0,
                0,
                image::Delay::from_numer_denom_ms(frame_delay_ms as u32, 1),
            );
            encoder.encode_frame(frame)?;
        }
    }

    handle.flush()?;
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
