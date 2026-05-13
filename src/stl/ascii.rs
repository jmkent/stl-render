use crate::stl::{StlError, Triangle};

pub struct AsciiStlIter<'a> {
    data: &'a [u8],
    pos: usize,
    done: bool,
}

impl<'a> AsciiStlIter<'a> {
    pub fn new(data: &'a [u8]) -> Result<Self, StlError> {
        // Verify it starts with "solid"
        let text = std::str::from_utf8(data)
            .map_err(|_| StlError::InvalidFormat("invalid UTF-8".into()))?;

        if !text.trim_start().to_lowercase().starts_with("solid") {
            return Err(StlError::InvalidFormat("missing 'solid' keyword".into()));
        }

        // Find start of first facet (skip solid line)
        let pos = text.find('\n').map(|p| p + 1).unwrap_or(0);

        Ok(Self {
            data,
            pos,
            done: false,
        })
    }
}

impl<'a> Iterator for AsciiStlIter<'a> {
    type Item = Result<Triangle, StlError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let text = match std::str::from_utf8(&self.data[self.pos..]) {
            Ok(t) => t,
            Err(_) => return Some(Err(StlError::InvalidFormat("invalid UTF-8".into()))),
        };

        // Skip whitespace
        let text = text.trim_start();

        // Check for end of file
        if text.is_empty() || text.to_lowercase().starts_with("endsolid") {
            self.done = true;
            return None;
        }

        // Parse facet
        match parse_facet(text) {
            Ok((tri, consumed)) => {
                // Update position
                let trimmed_start = self.data.len() - self.pos
                    - std::str::from_utf8(&self.data[self.pos..])
                        .unwrap()
                        .trim_start()
                        .len();
                self.pos += trimmed_start + consumed;
                Some(Ok(tri))
            }
            Err(e) => {
                self.done = true;
                Some(Err(e))
            }
        }
    }
}

fn parse_facet(text: &str) -> Result<(Triangle, usize), StlError> {
    let mut lines = text.lines();
    let mut consumed = 0;

    // facet normal nx ny nz
    let facet_line = lines
        .next()
        .ok_or_else(|| StlError::InvalidFormat("unexpected end of file".into()))?;
    consumed += facet_line.len() + 1;

    let normal = parse_facet_normal(facet_line)?;

    // outer loop
    let outer_line = lines
        .next()
        .ok_or_else(|| StlError::InvalidFormat("missing 'outer loop'".into()))?;
    consumed += outer_line.len() + 1;

    if !outer_line.trim().to_lowercase().starts_with("outer loop") {
        return Err(StlError::InvalidFormat(format!(
            "expected 'outer loop', got '{}'",
            outer_line.trim()
        )));
    }

    // vertex x y z (3 times)
    let mut vertices = [[0.0f32; 3]; 3];
    for v in &mut vertices {
        let vertex_line = lines
            .next()
            .ok_or_else(|| StlError::InvalidFormat("missing vertex".into()))?;
        consumed += vertex_line.len() + 1;
        *v = parse_vertex(vertex_line)?;
    }

    // endloop
    let endloop_line = lines
        .next()
        .ok_or_else(|| StlError::InvalidFormat("missing 'endloop'".into()))?;
    consumed += endloop_line.len() + 1;

    if !endloop_line.trim().to_lowercase().starts_with("endloop") {
        return Err(StlError::InvalidFormat(format!(
            "expected 'endloop', got '{}'",
            endloop_line.trim()
        )));
    }

    // endfacet
    let endfacet_line = lines
        .next()
        .ok_or_else(|| StlError::InvalidFormat("missing 'endfacet'".into()))?;
    consumed += endfacet_line.len() + 1;

    if !endfacet_line.trim().to_lowercase().starts_with("endfacet") {
        return Err(StlError::InvalidFormat(format!(
            "expected 'endfacet', got '{}'",
            endfacet_line.trim()
        )));
    }

    Ok((Triangle { vertices, normal }, consumed))
}

fn parse_facet_normal(line: &str) -> Result<[f32; 3], StlError> {
    let line = line.trim().to_lowercase();
    let parts: Vec<&str> = line.split_whitespace().collect();

    if parts.len() < 5 || parts[0] != "facet" || parts[1] != "normal" {
        return Err(StlError::InvalidFormat(format!(
            "invalid facet normal line: '{}'",
            line
        )));
    }

    let nx = parse_float(parts[2])?;
    let ny = parse_float(parts[3])?;
    let nz = parse_float(parts[4])?;

    Ok([nx, ny, nz])
}

fn parse_vertex(line: &str) -> Result<[f32; 3], StlError> {
    let line = line.trim().to_lowercase();
    let parts: Vec<&str> = line.split_whitespace().collect();

    if parts.len() < 4 || parts[0] != "vertex" {
        return Err(StlError::InvalidFormat(format!(
            "invalid vertex line: '{}'",
            line
        )));
    }

    let x = parse_float(parts[1])?;
    let y = parse_float(parts[2])?;
    let z = parse_float(parts[3])?;

    Ok([x, y, z])
}

fn parse_float(s: &str) -> Result<f32, StlError> {
    s.parse::<f32>()
        .map_err(|_| StlError::InvalidFormat(format!("invalid float: '{}'", s)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SINGLE_TRIANGLE_ASCII: &str = r#"solid test
facet normal 0 0 1
  outer loop
    vertex 0 0 0
    vertex 1 0 0
    vertex 0.5 1 0
  endloop
endfacet
endsolid test
"#;

    const TWO_TRIANGLES_ASCII: &str = r#"solid cube
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
"#;

    #[test]
    fn test_parse_single_triangle() {
        let iter = AsciiStlIter::new(SINGLE_TRIANGLE_ASCII.as_bytes()).unwrap();
        let triangles: Vec<_> = iter.map(|r| r.unwrap()).collect();

        assert_eq!(triangles.len(), 1);
        assert_eq!(triangles[0].normal, [0.0, 0.0, 1.0]);
        assert_eq!(triangles[0].vertices[0], [0.0, 0.0, 0.0]);
        assert_eq!(triangles[0].vertices[1], [1.0, 0.0, 0.0]);
        assert_eq!(triangles[0].vertices[2], [0.5, 1.0, 0.0]);
    }

    #[test]
    fn test_parse_multiple_triangles() {
        let iter = AsciiStlIter::new(TWO_TRIANGLES_ASCII.as_bytes()).unwrap();
        let triangles: Vec<_> = iter.map(|r| r.unwrap()).collect();

        assert_eq!(triangles.len(), 2);
    }

    #[test]
    fn test_various_whitespace() {
        let stl = "solid test\n\
            facet normal   0.0   0.0   1.0\n\
            \touter loop\n\
            \t\tvertex   0   0   0\n\
            \t\tvertex   1   0   0\n\
            \t\tvertex   0   1   0\n\
            \tendloop\n\
            endfacet\n\
            endsolid test\n";

        let iter = AsciiStlIter::new(stl.as_bytes()).unwrap();
        let triangles: Vec<_> = iter.map(|r| r.unwrap()).collect();

        assert_eq!(triangles.len(), 1);
    }

    #[test]
    fn test_scientific_notation() {
        let stl = "solid test\n\
            facet normal 0 0 1\n\
            outer loop\n\
            vertex 1.5e-3 2.0E+2 -3.14e0\n\
            vertex 1 0 0\n\
            vertex 0 1 0\n\
            endloop\n\
            endfacet\n\
            endsolid test\n";

        let iter = AsciiStlIter::new(stl.as_bytes()).unwrap();
        let triangles: Vec<_> = iter.map(|r| r.unwrap()).collect();

        assert_eq!(triangles.len(), 1);
        assert!((triangles[0].vertices[0][0] - 0.0015).abs() < 1e-6);
        assert!((triangles[0].vertices[0][1] - 200.0).abs() < 1e-6);
        assert!((triangles[0].vertices[0][2] - (-3.14)).abs() < 1e-6);
    }

    #[test]
    fn test_error_on_malformed_facet() {
        let stl = "solid test\n\
            facet 0 0 1\n\
            endsolid test\n";

        let iter = AsciiStlIter::new(stl.as_bytes()).unwrap();
        let results: Vec<_> = iter.collect();

        assert_eq!(results.len(), 1);
        assert!(results[0].is_err());
    }

    #[test]
    fn test_error_on_missing_endsolid() {
        // This should still work - we stop at EOF
        let stl = "solid test\n\
            facet normal 0 0 1\n\
            outer loop\n\
            vertex 0 0 0\n\
            vertex 1 0 0\n\
            vertex 0 1 0\n\
            endloop\n\
            endfacet\n";

        let iter = AsciiStlIter::new(stl.as_bytes()).unwrap();
        let triangles: Vec<_> = iter.map(|r| r.unwrap()).collect();

        assert_eq!(triangles.len(), 1);
    }

    #[test]
    fn test_case_insensitive() {
        let stl = "SOLID Test\n\
            FACET NORMAL 0 0 1\n\
            OUTER LOOP\n\
            VERTEX 0 0 0\n\
            VERTEX 1 0 0\n\
            VERTEX 0 1 0\n\
            ENDLOOP\n\
            ENDFACET\n\
            ENDSOLID Test\n";

        let iter = AsciiStlIter::new(stl.as_bytes()).unwrap();
        let triangles: Vec<_> = iter.map(|r| r.unwrap()).collect();

        assert_eq!(triangles.len(), 1);
    }

    #[test]
    fn test_error_missing_solid() {
        let stl = "facet normal 0 0 1\n";
        let result = AsciiStlIter::new(stl.as_bytes());
        assert!(result.is_err());
    }
}
