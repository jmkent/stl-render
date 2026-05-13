use crate::stl::{StlError, Triangle};

const HEADER_SIZE: usize = 80;
const COUNT_SIZE: usize = 4;
const TRIANGLE_SIZE: usize = 50; // 12 floats (48 bytes) + 2 byte attribute

pub fn read_triangle_count(data: &[u8]) -> Result<u32, StlError> {
    if data.len() < HEADER_SIZE + COUNT_SIZE {
        return Err(StlError::UnexpectedEof);
    }

    let bytes: [u8; 4] = data[HEADER_SIZE..HEADER_SIZE + COUNT_SIZE]
        .try_into()
        .unwrap(); // Length already checked

    Ok(u32::from_le_bytes(bytes))
}

fn expected_size(triangle_count: u32) -> usize {
    HEADER_SIZE + COUNT_SIZE + (triangle_count as usize) * TRIANGLE_SIZE
}

/// Iterator over triangles in a binary STL file.
pub struct BinaryStlIter<'a> {
    data: &'a [u8],
    offset: usize,
    remaining: u32,
    total: u32,
}

impl<'a> BinaryStlIter<'a> {
    pub fn new(data: &'a [u8]) -> Result<Self, StlError> {
        let count = read_triangle_count(data)?;

        if data.len() < expected_size(count) {
            return Err(StlError::UnexpectedEof);
        }

        Ok(Self {
            data,
            offset: HEADER_SIZE + COUNT_SIZE,
            remaining: count,
            total: count,
        })
    }

    pub fn triangle_count(&self) -> u32 {
        self.total
    }
}

impl Iterator for BinaryStlIter<'_> {
    type Item = Result<Triangle, StlError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }

        let chunk = &self.data[self.offset..self.offset + TRIANGLE_SIZE];

        let normal = read_vec3(chunk, 0);
        let vertices = [
            read_vec3(chunk, 12),
            read_vec3(chunk, 24),
            read_vec3(chunk, 36),
        ];

        self.offset += TRIANGLE_SIZE;
        self.remaining -= 1;

        Some(Ok(Triangle { vertices, normal }))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.remaining as usize;
        (n, Some(n))
    }
}

impl ExactSizeIterator for BinaryStlIter<'_> {}

/// Read 3 consecutive little-endian f32s starting at byte offset.
fn read_vec3(data: &[u8], offset: usize) -> [f32; 3] {
    [
        read_f32(data, offset),
        read_f32(data, offset + 4),
        read_f32(data, offset + 8),
    ]
}

fn read_f32(data: &[u8], offset: usize) -> f32 {
    let bytes: [u8; 4] = data[offset..offset + 4].try_into().unwrap();
    f32::from_le_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_binary_stl(triangles: &[Triangle]) -> Vec<u8> {
        let mut data = vec![0u8; 80]; // header
        data.extend_from_slice(&(triangles.len() as u32).to_le_bytes());

        for tri in triangles {
            for &n in &tri.normal {
                data.extend_from_slice(&n.to_le_bytes());
            }
            for v in &tri.vertices {
                for &c in v {
                    data.extend_from_slice(&c.to_le_bytes());
                }
            }
            data.extend_from_slice(&0u16.to_le_bytes()); // attribute
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
    fn test_parse_single_triangle() {
        let tri = Triangle {
            normal: [0.0, 0.0, 1.0],
            vertices: [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
        };
        let data = make_binary_stl(&[tri]);

        let tris: Vec<_> = BinaryStlIter::new(&data)
            .unwrap()
            .map(Result::unwrap)
            .collect();

        assert_eq!(tris.len(), 1);
        assert_eq!(tris[0].normal, [0.0, 0.0, 1.0]);
        assert_eq!(tris[0].vertices[0], [0.0, 0.0, 0.0]);
        assert_eq!(tris[0].vertices[1], [1.0, 0.0, 0.0]);
        assert_eq!(tris[0].vertices[2], [0.5, 1.0, 0.0]);
    }

    #[test]
    fn test_parse_1000_triangles() {
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
        assert_eq!(iter.len(), 1000);
    }

    #[test]
    fn test_zero_triangles() {
        let data = make_binary_stl(&[]);
        let iter = BinaryStlIter::new(&data).unwrap();

        assert_eq!(iter.triangle_count(), 0);
        assert_eq!(iter.count(), 0);
    }

    #[test]
    fn test_error_truncated() {
        let mut data = vec![0u8; 84];
        data[80..84].copy_from_slice(&10u32.to_le_bytes()); // claims 10

        assert!(BinaryStlIter::new(&data).is_err());
    }

    #[test]
    fn test_error_count_mismatch() {
        let tri = Triangle {
            normal: [0.0, 0.0, 1.0],
            vertices: [[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        };
        let mut data = make_binary_stl(&[tri]);
        data[80..84].copy_from_slice(&5u32.to_le_bytes()); // lie about count

        assert!(BinaryStlIter::new(&data).is_err());
    }
}
