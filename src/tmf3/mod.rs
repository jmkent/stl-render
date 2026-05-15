//! 3MF file format parser.
//!
//! 3MF (3D Manufacturing Format) is a ZIP-based format containing XML mesh data.
//! Unlike STL, 3MF must be fully decompressed before parsing, so triangles are
//! buffered in memory rather than streamed.
//!
//! This parser supports:
//! - Multiple objects with mesh geometry
//! - Build items with transforms
//! - Component references with nested transforms
//! - Unit metadata (mm, cm, inch, foot, micron)

mod parser;

pub use parser::Unit3mf;

use std::io::{Read, Seek};
use std::path::Path;

use crate::stl::{StlError, Triangle};

/// Reader for 3MF files.
///
/// 3MF files are ZIP archives containing XML mesh data. This reader extracts
/// and parses the mesh data, resolving build items and component transforms.
pub struct Tmf3Reader {
    triangles: Vec<Triangle>,
    unit: Unit3mf,
    color_palette: Vec<[u8; 4]>,
    has_colors: bool,
}

impl Tmf3Reader {
    /// Open a 3MF file from disk.
    pub fn open(path: &Path) -> Result<Self, StlError> {
        let file = std::fs::File::open(path)?;
        Self::from_reader(file)
    }

    /// Read a 3MF file from any reader that supports Read + Seek.
    pub fn from_reader<R: Read + Seek>(reader: R) -> Result<Self, StlError> {
        let result = parser::parse_3mf(reader)?;
        Ok(Self {
            triangles: result.triangles,
            unit: result.unit,
            color_palette: result.color_palette,
            has_colors: result.has_colors,
        })
    }

    /// Get the number of triangles in the mesh.
    pub fn triangle_count(&self) -> u64 {
        self.triangles.len() as u64
    }

    /// Get the unit of measurement in the 3MF file.
    pub fn unit(&self) -> Unit3mf {
        self.unit
    }

    /// Get the color palette from the 3MF file.
    pub fn color_palette(&self) -> &[[u8; 4]] {
        &self.color_palette
    }

    /// Check if this 3MF file has embedded colors.
    pub fn has_colors(&self) -> bool {
        self.has_colors
    }

    /// Get an iterator over the triangles.
    pub fn triangles(&self) -> Tmf3Iter<'_> {
        Tmf3Iter {
            inner: self.triangles.iter(),
        }
    }
}

/// Iterator over triangles in a 3MF file.
pub struct Tmf3Iter<'a> {
    inner: std::slice::Iter<'a, Triangle>,
}

impl<'a> Iterator for Tmf3Iter<'a> {
    type Item = Result<Triangle, StlError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|t| Ok(*t))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl ExactSizeIterator for Tmf3Iter<'_> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_cube_3mf() {
        let path = Path::new("fixtures/cube.3mf");
        if path.exists() {
            let reader = Tmf3Reader::open(path).unwrap();
            assert_eq!(reader.triangle_count(), 12);
        }
    }

    #[test]
    fn test_open_sphere_3mf() {
        let path = Path::new("fixtures/sphere.3mf");
        if path.exists() {
            let reader = Tmf3Reader::open(path).unwrap();
            assert_eq!(reader.triangle_count(), 1280);
        }
    }

    #[test]
    fn test_open_single_triangle_3mf() {
        let path = Path::new("fixtures/single_triangle.3mf");
        if path.exists() {
            let reader = Tmf3Reader::open(path).unwrap();
            assert_eq!(reader.triangle_count(), 1);
        }
    }

    #[test]
    fn test_open_multi_object_3mf() {
        let path = Path::new("fixtures/multi_object.3mf");
        if path.exists() {
            let reader = Tmf3Reader::open(path).unwrap();
            // cube (12) + small sphere (320) = 332 triangles
            assert_eq!(reader.triangle_count(), 332);
        }
    }

    #[test]
    fn test_malformed_3mf_error() {
        let path = Path::new("fixtures/malformed.3mf");
        if path.exists() {
            let result = Tmf3Reader::open(path);
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_missing_model_3mf_error() {
        let path = Path::new("fixtures/missing_model.3mf");
        if path.exists() {
            let result = Tmf3Reader::open(path);
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_iterate_cube_3mf() {
        let path = Path::new("fixtures/cube.3mf");
        if path.exists() {
            let reader = Tmf3Reader::open(path).unwrap();
            let triangles: Vec<_> = reader.triangles().map(|r| r.unwrap()).collect();
            assert_eq!(triangles.len(), 12);
        }
    }

    #[test]
    fn test_3mf_stl_same_geometry() {
        let stl_path = Path::new("fixtures/cube.stl");
        let tmf3_path = Path::new("fixtures/cube.3mf");

        if stl_path.exists() && tmf3_path.exists() {
            use crate::stl::StlReader;

            let stl_reader = StlReader::open(stl_path).unwrap();
            let tmf3_reader = Tmf3Reader::open(tmf3_path).unwrap();

            assert_eq!(
                stl_reader.triangle_count(),
                Some(tmf3_reader.triangle_count())
            );

            // Both should have same number of triangles
            let stl_tris: Vec<_> = stl_reader
                .triangles()
                .unwrap()
                .map(|r| r.unwrap())
                .collect();
            let tmf3_tris: Vec<_> = tmf3_reader.triangles().map(|r| r.unwrap()).collect();

            assert_eq!(stl_tris.len(), tmf3_tris.len());
        }
    }
}
