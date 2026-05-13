use crate::stl::{StlError, Triangle};

const HEADER_SIZE: usize = 80;
const COUNT_SIZE: usize = 4;
const TRIANGLE_SIZE: usize = 50; // 12 floats (48 bytes) + 2 byte attribute

pub fn read_triangle_count(data: &[u8]) -> Result<u64, StlError> {
    if data.len() < HEADER_SIZE + COUNT_SIZE {
        return Err(StlError::UnexpectedEof);
    }

    let count_bytes: [u8; 4] = data[HEADER_SIZE..HEADER_SIZE + COUNT_SIZE]
        .try_into()
        .map_err(|_| StlError::UnexpectedEof)?;

    Ok(u32::from_le_bytes(count_bytes) as u64)
}

pub fn validate_size(data: &[u8], triangle_count: u64) -> Result<(), StlError> {
    let expected_size = HEADER_SIZE + COUNT_SIZE + (triangle_count as usize) * TRIANGLE_SIZE;
    if data.len() < expected_size {
        return Err(StlError::UnexpectedEof);
    }
    Ok(())
}

pub struct BinaryStlIter<'a> {
    data: &'a [u8],
    offset: usize,
    remaining: u32,
}

impl<'a> BinaryStlIter<'a> {
    pub fn new(data: &'a [u8]) -> Result<Self, StlError> {
        let count = read_triangle_count(data)? as u32;
        validate_size(data, count as u64)?;

        Ok(Self {
            data,
            offset: HEADER_SIZE + COUNT_SIZE,
            remaining: count,
        })
    }

    pub fn triangle_count(&self) -> u32 {
        // Return original count, not remaining
        if let Ok(count) = read_triangle_count(self.data) {
            count as u32
        } else {
            0
        }
    }
}

impl<'a> Iterator for BinaryStlIter<'a> {
    type Item = Result<Triangle, StlError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }

        let end = self.offset + TRIANGLE_SIZE;
        if end > self.data.len() {
            return Some(Err(StlError::UnexpectedEof));
        }

        let chunk = &self.data[self.offset..end];

        // Parse normal (3 floats)
        let normal = [
            f32::from_le_bytes(chunk[0..4].try_into().unwrap()),
            f32::from_le_bytes(chunk[4..8].try_into().unwrap()),
            f32::from_le_bytes(chunk[8..12].try_into().unwrap()),
        ];

        // Parse 3 vertices (9 floats)
        let v0 = [
            f32::from_le_bytes(chunk[12..16].try_into().unwrap()),
            f32::from_le_bytes(chunk[16..20].try_into().unwrap()),
            f32::from_le_bytes(chunk[20..24].try_into().unwrap()),
        ];
        let v1 = [
            f32::from_le_bytes(chunk[24..28].try_into().unwrap()),
            f32::from_le_bytes(chunk[28..32].try_into().unwrap()),
            f32::from_le_bytes(chunk[32..36].try_into().unwrap()),
        ];
        let v2 = [
            f32::from_le_bytes(chunk[36..40].try_into().unwrap()),
            f32::from_le_bytes(chunk[40..44].try_into().unwrap()),
            f32::from_le_bytes(chunk[44..48].try_into().unwrap()),
        ];

        // Skip 2-byte attribute count (bytes 48-49)

        self.offset = end;
        self.remaining -= 1;

        Some(Ok(Triangle {
            vertices: [v0, v1, v2],
            normal,
        }))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.remaining as usize;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for BinaryStlIter<'a> {}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_binary_stl(triangles: &[Triangle]) -> Vec<u8> {
        let mut data = vec![0u8; 80]; // header
        data.extend_from_slice(&(triangles.len() as u32).to_le_bytes());

        for tri in triangles {
            // Normal
            for &n in &tri.normal {
                data.extend_from_slice(&n.to_le_bytes());
            }
            // Vertices
            for v in &tri.vertices {
                for &coord in v {
                    data.extend_from_slice(&coord.to_le_bytes());
                }
            }
            // Attribute byte count
            data.extend_from_slice(&0u16.to_le_bytes());
        }

        data
    }

    #[test]
    fn test_read_triangle_count() {
        let mut data = vec![0u8; 100];
        data[80..84].copy_from_slice(&12u32.to_le_bytes());
        assert_eq!(read_triangle_count(&data).unwrap(), 12);
    }

    #[test]
    fn test_validate_size_ok() {
        let data = vec![0u8; 84 + 50 * 2];
        assert!(validate_size(&data, 2).is_ok());
    }

    #[test]
    fn test_validate_size_truncated() {
        let data = vec![0u8; 100];
        assert!(validate_size(&data, 10).is_err());
    }

    #[test]
    fn test_parse_single_triangle() {
        let tri = Triangle {
            normal: [0.0, 0.0, 1.0],
            vertices: [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.5, 1.0, 0.0],
            ],
        };
        let data = make_binary_stl(&[tri]);

        let iter = BinaryStlIter::new(&data).unwrap();
        let triangles: Vec<_> = iter.map(|r| r.unwrap()).collect();

        assert_eq!(triangles.len(), 1);
        assert_eq!(triangles[0].normal, [0.0, 0.0, 1.0]);
        assert_eq!(triangles[0].vertices[0], [0.0, 0.0, 0.0]);
        assert_eq!(triangles[0].vertices[1], [1.0, 0.0, 0.0]);
        assert_eq!(triangles[0].vertices[2], [0.5, 1.0, 0.0]);
    }

    #[test]
    fn test_parse_multiple_triangles() {
        let triangles: Vec<Triangle> = (0..1000)
            .map(|i| Triangle {
                normal: [0.0, 0.0, 1.0],
                vertices: [
                    [i as f32, 0.0, 0.0],
                    [i as f32 + 1.0, 0.0, 0.0],
                    [i as f32 + 0.5, 1.0, 0.0],
                ],
            })
            .collect();

        let data = make_binary_stl(&triangles);
        let iter = BinaryStlIter::new(&data).unwrap();

        assert_eq!(iter.triangle_count(), 1000);
        assert_eq!(iter.count(), 1000);
    }

    #[test]
    fn test_parse_zero_triangles() {
        let data = make_binary_stl(&[]);
        let iter = BinaryStlIter::new(&data).unwrap();

        assert_eq!(iter.triangle_count(), 0);
        assert_eq!(iter.count(), 0);
    }

    #[test]
    fn test_error_on_truncated_file() {
        let mut data = vec![0u8; 84];
        data[80..84].copy_from_slice(&10u32.to_le_bytes()); // claims 10 triangles

        let result = BinaryStlIter::new(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_error_on_count_mismatch() {
        let tri = Triangle {
            normal: [0.0, 0.0, 1.0],
            vertices: [[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        };
        let mut data = make_binary_stl(&[tri]);

        // Corrupt the count to claim more triangles
        data[80..84].copy_from_slice(&5u32.to_le_bytes());

        let result = BinaryStlIter::new(&data);
        assert!(result.is_err());
    }
}
