//! OBJ file format parser.
//!
//! OBJ is a simple text-based 3D geometry format. Unlike streaming STL,
//! OBJ must buffer all vertices before building triangles (faces reference
//! vertex indices). Triangles are buffered in memory.

mod parser;

use std::io::Read;
use std::path::Path;

use crate::stl::{StlError, Triangle};

/// Reader for OBJ files.
///
/// OBJ files are text-based with vertex and face definitions. This reader
/// extracts and parses the geometry, buffering all triangles in memory.
pub struct ObjReader {
    triangles: Vec<Triangle>,
}

impl ObjReader {
    /// Open an OBJ file from disk.
    pub fn open(path: &Path) -> Result<Self, StlError> {
        let file = std::fs::File::open(path)?;
        Self::from_reader(file)
    }

    /// Read an OBJ file from any reader.
    pub fn from_reader<R: Read>(reader: R) -> Result<Self, StlError> {
        let triangles = parser::parse_obj(reader)?;
        Ok(Self { triangles })
    }

    /// Get the number of triangles in the mesh.
    pub fn triangle_count(&self) -> u64 {
        self.triangles.len() as u64
    }

    /// Get an iterator over the triangles.
    pub fn triangles(&self) -> ObjIter<'_> {
        ObjIter {
            inner: self.triangles.iter(),
        }
    }
}

/// Iterator over triangles in an OBJ file.
pub struct ObjIter<'a> {
    inner: std::slice::Iter<'a, Triangle>,
}

impl<'a> Iterator for ObjIter<'a> {
    type Item = Result<Triangle, StlError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|t| Ok(*t))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl ExactSizeIterator for ObjIter<'_> {}

pub use parser::is_obj_format;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_cube_obj() {
        let path = Path::new("fixtures/cube.obj");
        if path.exists() {
            let reader = ObjReader::open(path).unwrap();
            assert_eq!(reader.triangle_count(), 12);
        }
    }

    #[test]
    fn test_open_sphere_obj() {
        let path = Path::new("fixtures/sphere.obj");
        if path.exists() {
            let reader = ObjReader::open(path).unwrap();
            assert_eq!(reader.triangle_count(), 1280);
        }
    }

    #[test]
    fn test_iterate_cube_obj() {
        let path = Path::new("fixtures/cube.obj");
        if path.exists() {
            let reader = ObjReader::open(path).unwrap();
            let triangles: Vec<_> = reader.triangles().map(|r| r.unwrap()).collect();
            assert_eq!(triangles.len(), 12);
        }
    }

    #[test]
    fn test_obj_stl_same_geometry() {
        let stl_path = Path::new("fixtures/cube.stl");
        let obj_path = Path::new("fixtures/cube.obj");

        if stl_path.exists() && obj_path.exists() {
            use crate::stl::StlReader;

            let stl_reader = StlReader::open(stl_path).unwrap();
            let obj_reader = ObjReader::open(obj_path).unwrap();

            assert_eq!(
                stl_reader.triangle_count(),
                Some(obj_reader.triangle_count())
            );
        }
    }
}
