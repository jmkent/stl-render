use crate::stl::{StlError, Triangle};

/// Iterator over triangles in an ASCII STL file.
pub struct AsciiStlIter<'a> {
    text: &'a str,
    pos: usize,
}

impl<'a> AsciiStlIter<'a> {
    pub fn new(data: &'a [u8]) -> Result<Self, StlError> {
        let text = std::str::from_utf8(data)
            .map_err(|_| StlError::InvalidFormat("invalid UTF-8".into()))?;

        // Verify starts with "solid"
        let trimmed = text.trim_start();
        if !trimmed.get(..5).is_some_and(|s| s.eq_ignore_ascii_case("solid")) {
            return Err(StlError::InvalidFormat("missing 'solid' keyword".into()));
        }

        // Skip past "solid" line
        let pos = text.len() - trimmed.len();
        let pos = pos + trimmed.find('\n').map(|p| p + 1).unwrap_or(trimmed.len());

        Ok(Self { text, pos })
    }

    fn remaining(&self) -> &'a str {
        &self.text[self.pos..]
    }
}

impl Iterator for AsciiStlIter<'_> {
    type Item = Result<Triangle, StlError>;

    fn next(&mut self) -> Option<Self::Item> {
        let remaining = self.remaining().trim_start();

        // End conditions
        if remaining.is_empty()
            || remaining.get(..8).is_some_and(|s| s.eq_ignore_ascii_case("endsolid"))
        {
            return None;
        }

        // Update position to skip leading whitespace
        self.pos = self.text.len() - remaining.len();

        match parse_facet(remaining) {
            Ok((tri, consumed)) => {
                self.pos += consumed;
                Some(Ok(tri))
            }
            Err(e) => {
                // Stop iteration on error
                self.pos = self.text.len();
                Some(Err(e))
            }
        }
    }
}

fn parse_facet(text: &str) -> Result<(Triangle, usize), StlError> {
    let mut lines = text.lines().peekable();
    let mut consumed = 0;

    // "facet normal nx ny nz"
    let normal = {
        let line = next_line(&mut lines, &mut consumed, "facet normal")?;
        parse_normal(line)?
    };

    // "outer loop"
    let line = next_line(&mut lines, &mut consumed, "outer loop")?;
    expect_keyword(line, "outer loop")?;

    // 3x "vertex x y z"
    let mut vertices = [[0.0f32; 3]; 3];
    for vertex in &mut vertices {
        let line = next_line(&mut lines, &mut consumed, "vertex")?;
        *vertex = parse_vertex(line)?;
    }

    // "endloop"
    let line = next_line(&mut lines, &mut consumed, "endloop")?;
    expect_keyword(line, "endloop")?;

    // "endfacet"
    let line = next_line(&mut lines, &mut consumed, "endfacet")?;
    expect_keyword(line, "endfacet")?;

    Ok((Triangle { vertices, normal }, consumed))
}

fn next_line<'a>(
    lines: &mut std::iter::Peekable<std::str::Lines<'a>>,
    consumed: &mut usize,
    context: &str,
) -> Result<&'a str, StlError> {
    let line = lines
        .next()
        .ok_or_else(|| StlError::InvalidFormat(format!("unexpected EOF, expected {context}")))?;
    *consumed += line.len() + 1; // +1 for newline
    Ok(line)
}

fn expect_keyword(line: &str, keyword: &str) -> Result<(), StlError> {
    let trimmed = line.trim();
    if !trimmed
        .get(..keyword.len())
        .is_some_and(|s| s.eq_ignore_ascii_case(keyword))
    {
        return Err(StlError::InvalidFormat(format!(
            "expected '{keyword}', got '{trimmed}'"
        )));
    }
    Ok(())
}

fn parse_normal(line: &str) -> Result<[f32; 3], StlError> {
    parse_keyword_xyz(line, &["facet", "normal"])
}

fn parse_vertex(line: &str) -> Result<[f32; 3], StlError> {
    parse_keyword_xyz(line, &["vertex"])
}

fn parse_keyword_xyz(line: &str, keywords: &[&str]) -> Result<[f32; 3], StlError> {
    let mut parts = line.split_whitespace();
    let context = keywords.join(" ");

    for kw in keywords {
        let actual = parts.next().unwrap_or("");
        if !actual.eq_ignore_ascii_case(kw) {
            return Err(StlError::InvalidFormat(format!(
                "expected '{kw}', got '{actual}'"
            )));
        }
    }

    parse_xyz(&mut parts, &context)
}

fn parse_xyz(parts: &mut std::str::SplitWhitespace<'_>, context: &str) -> Result<[f32; 3], StlError> {
    let mut coords = [0.0f32; 3];
    for (i, axis) in ["x", "y", "z"].iter().enumerate() {
        coords[i] = parts
            .next()
            .ok_or_else(|| StlError::InvalidFormat(format!("{context}: missing {axis}")))?
            .parse()
            .map_err(|_| StlError::InvalidFormat(format!("{context}: invalid {axis}")))?;
    }
    Ok(coords)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SINGLE_TRIANGLE: &str = "\
solid test
facet normal 0 0 1
  outer loop
    vertex 0 0 0
    vertex 1 0 0
    vertex 0.5 1 0
  endloop
endfacet
endsolid test
";

    const TWO_TRIANGLES: &str = "\
solid cube
facet normal 0 0 1
  outer loop
    vertex 0 0 0
    vertex 1 0 0
    vertex 0 1 0
  endloop
endfacet
facet normal 0 0 1
  outer loop
    vertex 1 0 0
    vertex 1 1 0
    vertex 0 1 0
  endloop
endfacet
endsolid cube
";

    #[test]
    fn test_parse_single_triangle() {
        let tris: Vec<_> = AsciiStlIter::new(SINGLE_TRIANGLE.as_bytes())
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
    fn test_parse_multiple_triangles() {
        let count = AsciiStlIter::new(TWO_TRIANGLES.as_bytes())
            .unwrap()
            .count();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_various_whitespace() {
        let stl = "\
solid test
facet normal   0.0   0.0   1.0
\touter loop
\t\tvertex   0   0   0
\t\tvertex   1   0   0
\t\tvertex   0   1   0
\tendloop
endfacet
endsolid test
";
        let count = AsciiStlIter::new(stl.as_bytes()).unwrap().count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_scientific_notation() {
        let stl = "\
solid test
facet normal 0 0 1
outer loop
vertex 1.5e-3 2.0E+2 -3.14e0
vertex 1 0 0
vertex 0 1 0
endloop
endfacet
endsolid test
";
        let tris: Vec<_> = AsciiStlIter::new(stl.as_bytes())
            .unwrap()
            .map(Result::unwrap)
            .collect();

        assert_eq!(tris.len(), 1);
        let v = tris[0].vertices[0];
        assert!((v[0] - 0.0015).abs() < 1e-6);
        assert!((v[1] - 200.0).abs() < 1e-6);
        assert!((v[2] - (-3.14)).abs() < 1e-6);
    }

    #[test]
    fn test_error_on_malformed_facet() {
        let stl = "solid test\nfacet 0 0 1\nendsolid test\n";
        let results: Vec<_> = AsciiStlIter::new(stl.as_bytes()).unwrap().collect();
        assert_eq!(results.len(), 1);
        assert!(results[0].is_err());
    }

    #[test]
    fn test_missing_endsolid_still_parses() {
        let stl = "\
solid test
facet normal 0 0 1
outer loop
vertex 0 0 0
vertex 1 0 0
vertex 0 1 0
endloop
endfacet
";
        let count = AsciiStlIter::new(stl.as_bytes())
            .unwrap()
            .filter(Result::is_ok)
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_case_insensitive() {
        let stl = "\
SOLID Test
FACET NORMAL 0 0 1
OUTER LOOP
VERTEX 0 0 0
VERTEX 1 0 0
VERTEX 0 1 0
ENDLOOP
ENDFACET
ENDSOLID Test
";
        let count = AsciiStlIter::new(stl.as_bytes()).unwrap().count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_error_missing_solid() {
        let result = AsciiStlIter::new(b"facet normal 0 0 1\n");
        assert!(result.is_err());
    }
}
