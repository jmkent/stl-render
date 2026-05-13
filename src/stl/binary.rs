use crate::stl::StlError;

const HEADER_SIZE: usize = 80;
const COUNT_SIZE: usize = 4;
#[allow(dead_code)] // Used in M2: STL parsing
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

#[allow(dead_code)] // Used in M2: STL parsing
pub fn validate_size(data: &[u8], triangle_count: u64) -> Result<(), StlError> {
    let expected_size = HEADER_SIZE + COUNT_SIZE + (triangle_count as usize) * TRIANGLE_SIZE;
    if data.len() < expected_size {
        return Err(StlError::UnexpectedEof);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_triangle_count() {
        let mut data = vec![0u8; 100];
        data[80..84].copy_from_slice(&12u32.to_le_bytes());
        assert_eq!(read_triangle_count(&data).unwrap(), 12);
    }

    #[test]
    fn test_validate_size_ok() {
        // 84 bytes header + 50 bytes per triangle
        let data = vec![0u8; 84 + 50 * 2];
        assert!(validate_size(&data, 2).is_ok());
    }

    #[test]
    fn test_validate_size_truncated() {
        let data = vec![0u8; 100];
        assert!(validate_size(&data, 10).is_err());
    }
}
