#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StlFormat {
    Binary,
    Ascii,
}

pub fn detect_format(data: &[u8]) -> StlFormat {
    // ASCII STL starts with "solid" and contains "facet"
    // Binary STL has 80-byte header + 4-byte count, then triangle data
    //
    // Edge case: binary header could start with "solid", so we check
    // for "facet" keyword to confirm ASCII format

    if data.len() < 84 {
        // Too small for valid binary STL, try ASCII
        if data.starts_with(b"solid") {
            return StlFormat::Ascii;
        }
    }

    // Check if it looks like ASCII
    if data.starts_with(b"solid") {
        // Look for "facet" keyword in first ~1000 bytes
        let search_range = std::cmp::min(data.len(), 1000);
        if let Some(slice) = data.get(..search_range)
            && slice
                .windows(5)
                .any(|w| w.eq_ignore_ascii_case(b"facet"))
        {
            return StlFormat::Ascii;
        }
    }

    StlFormat::Binary
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_binary() {
        let mut data = vec![0u8; 84];
        data[80..84].copy_from_slice(&1u32.to_le_bytes());
        assert_eq!(detect_format(&data), StlFormat::Binary);
    }

    #[test]
    fn test_detect_ascii() {
        let data = b"solid cube\nfacet normal 0 0 1\n";
        assert_eq!(detect_format(data), StlFormat::Ascii);
    }

    #[test]
    fn test_binary_starting_with_solid() {
        // Binary file with header that happens to start with "solid"
        let mut data = vec![0u8; 200];
        data[0..5].copy_from_slice(b"solid");
        data[80..84].copy_from_slice(&1u32.to_le_bytes());
        // No "facet" keyword, so should be detected as binary
        assert_eq!(detect_format(&data), StlFormat::Binary);
    }
}
