mod ascii;
mod binary;
mod parser;

use std::path::Path;

use thiserror::Error;

pub use parser::StlFormat;

#[derive(Debug, Error)]
pub enum StlError {
    #[error("failed to open file: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid STL format: {0}")]
    InvalidFormat(String),

    #[error("unexpected end of file")]
    UnexpectedEof,
}

#[derive(Debug, Clone, Copy)]
pub struct Triangle {
    pub vertices: [[f32; 3]; 3],
    pub normal: [f32; 3],
}

pub struct StlReader {
    _data: memmap2::Mmap,
    format: StlFormat,
    triangle_count: Option<u64>,
}

impl StlReader {
    pub fn open(path: &Path) -> Result<Self, StlError> {
        let file = std::fs::File::open(path)?;
        let data = unsafe { memmap2::Mmap::map(&file)? };

        if data.is_empty() {
            return Err(StlError::InvalidFormat("empty file".into()));
        }

        let format = parser::detect_format(&data);
        let triangle_count = match format {
            StlFormat::Binary => Some(binary::read_triangle_count(&data)?),
            StlFormat::Ascii => None,
        };

        Ok(Self {
            _data: data,
            format,
            triangle_count,
        })
    }

    pub fn format(&self) -> StlFormat {
        self.format
    }

    pub fn triangle_count(&self) -> Option<u64> {
        self.triangle_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_cube_stl() {
        let path = Path::new("fixtures/cube.stl");
        if path.exists() {
            let reader = StlReader::open(path).unwrap();
            assert_eq!(reader.format(), StlFormat::Binary);
            assert_eq!(reader.triangle_count(), Some(12));
        }
    }

    #[test]
    fn test_open_empty_stl() {
        let path = Path::new("fixtures/empty.stl");
        if path.exists() {
            let reader = StlReader::open(path).unwrap();
            assert_eq!(reader.triangle_count(), Some(0));
        }
    }
}
