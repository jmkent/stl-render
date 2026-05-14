mod ascii;
mod binary;
mod parser;

use std::io::Read;
use std::ops::Deref;
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Triangle {
    pub vertices: [[f32; 3]; 3],
    pub normal: [f32; 3],
}

enum StlData {
    Mmap(memmap2::Mmap),
    Memory(Vec<u8>),
}

impl Deref for StlData {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        match self {
            StlData::Mmap(m) => m,
            StlData::Memory(v) => v,
        }
    }
}

pub struct StlReader {
    data: StlData,
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
            StlFormat::Binary => Some(binary::read_triangle_count(&data)? as u64),
            StlFormat::Ascii => None,
        };

        Ok(Self {
            data: StlData::Mmap(data),
            format,
            triangle_count,
        })
    }

    pub fn from_reader<R: Read>(mut reader: R) -> Result<Self, StlError> {
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;

        if data.is_empty() {
            return Err(StlError::InvalidFormat("empty input".into()));
        }

        let format = parser::detect_format(&data);
        let triangle_count = match format {
            StlFormat::Binary => Some(binary::read_triangle_count(&data)? as u64),
            StlFormat::Ascii => None,
        };

        Ok(Self {
            data: StlData::Memory(data),
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

    pub fn triangles(&self) -> Result<TriangleIter<'_>, StlError> {
        match self.format {
            StlFormat::Binary => {
                let iter = binary::BinaryStlIter::new(&self.data)?;
                Ok(TriangleIter::Binary(iter))
            }
            StlFormat::Ascii => {
                let iter = ascii::AsciiStlIter::new(&self.data)?;
                Ok(TriangleIter::Ascii(iter))
            }
        }
    }
}

pub enum TriangleIter<'a> {
    Binary(binary::BinaryStlIter<'a>),
    Ascii(ascii::AsciiStlIter<'a>),
}

impl Iterator for TriangleIter<'_> {
    type Item = Result<Triangle, StlError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            TriangleIter::Binary(iter) => iter.next(),
            TriangleIter::Ascii(iter) => iter.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            TriangleIter::Binary(iter) => iter.size_hint(),
            TriangleIter::Ascii(_) => (0, None),
        }
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

    #[test]
    fn test_iterate_cube_binary() {
        let path = Path::new("fixtures/cube.stl");
        if path.exists() {
            let reader = StlReader::open(path).unwrap();
            let triangles: Vec<_> = reader.triangles().unwrap().map(|r| r.unwrap()).collect();
            assert_eq!(triangles.len(), 12);
        }
    }

    #[test]
    fn test_iterate_cube_ascii() {
        let path = Path::new("fixtures/cube_ascii.stl");
        if path.exists() {
            let reader = StlReader::open(path).unwrap();
            assert_eq!(reader.format(), StlFormat::Ascii);

            let triangles: Vec<_> = reader.triangles().unwrap().map(|r| r.unwrap()).collect();
            assert_eq!(triangles.len(), 12);
        }
    }

    #[test]
    fn test_binary_ascii_same_vertices() {
        let binary_path = Path::new("fixtures/cube.stl");
        let ascii_path = Path::new("fixtures/cube_ascii.stl");

        if binary_path.exists() && ascii_path.exists() {
            let binary_reader = StlReader::open(binary_path).unwrap();
            let ascii_reader = StlReader::open(ascii_path).unwrap();

            let binary_tris: Vec<_> = binary_reader
                .triangles()
                .unwrap()
                .map(|r| r.unwrap())
                .collect();
            let ascii_tris: Vec<_> = ascii_reader
                .triangles()
                .unwrap()
                .map(|r| r.unwrap())
                .collect();

            assert_eq!(binary_tris.len(), ascii_tris.len());

            // Vertices should be approximately equal (allowing for float formatting)
            for (b, a) in binary_tris.iter().zip(ascii_tris.iter()) {
                for (bv, av) in b.vertices.iter().zip(a.vertices.iter()) {
                    for i in 0..3 {
                        assert!(
                            (bv[i] - av[i]).abs() < 1e-5,
                            "vertex mismatch: {} vs {}",
                            bv[i],
                            av[i]
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_single_triangle() {
        let path = Path::new("fixtures/single_triangle.stl");
        if path.exists() {
            let reader = StlReader::open(path).unwrap();
            let triangles: Vec<_> = reader.triangles().unwrap().map(|r| r.unwrap()).collect();
            assert_eq!(triangles.len(), 1);
        }
    }

    #[test]
    fn test_empty_stl_iteration() {
        let path = Path::new("fixtures/empty.stl");
        if path.exists() {
            let reader = StlReader::open(path).unwrap();
            let triangles: Vec<_> = reader.triangles().unwrap().map(|r| r.unwrap()).collect();
            assert_eq!(triangles.len(), 0);
        }
    }

    #[test]
    fn test_truncated_stl_error() {
        let path = Path::new("fixtures/truncated.stl");
        if path.exists() {
            let reader = StlReader::open(path).unwrap();
            let result = reader.triangles();
            assert!(result.is_err());
        }
    }
}
