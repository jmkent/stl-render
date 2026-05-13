# STL Fixture Generator

Generates geometric STL test fixtures for stl-render using numpy-stl.

## Prerequisites

- Python 3.10+
- [uv](https://docs.astral.sh/uv/) (recommended) or pip

## Usage

### With uv (recommended)

```bash
cd tools/fixtures

# Generate fixtures to ../../fixtures/
uv run generate_fixtures.py -o ../../fixtures

# Or run directly
uv run python generate_fixtures.py -o ../../fixtures
```

### With pip

```bash
cd tools/fixtures
pip install -e .
generate-fixtures -o ../../fixtures
```

## Generated Files

| File | Description | Triangles |
|------|-------------|-----------|
| `cube.stl` | Unit cube, binary | 12 |
| `cube_ascii.stl` | Unit cube, ASCII | 12 |
| `sphere.stl` | Icosphere, binary | 1280 |
| `cylinder.stl` | Cylinder, binary | 128 |
| `tall_column.stl` | Tall thin box (aspect ratio test) | 12 |
| `flat_tile.stl` | Flat wide box (near-planar test) | 12 |
| `long_beam.stl` | Long thin box (elongated test) | 12 |
| `single_triangle.stl` | Minimal valid STL | 1 |
| `single_triangle_ascii.stl` | Minimal valid ASCII STL | 1 |
| `degenerate.stl` | Contains zero-area triangles | 3 |
| `empty.stl` | Valid STL with 0 triangles | 0 |
| `truncated.stl` | Invalid (truncated data) | - |

## Options

```bash
generate_fixtures.py [-h] [-o OUTPUT] [--sphere-subdivisions N] [--large]

Options:
  -o, --output PATH       Output directory (default: fixtures)
  --sphere-subdivisions N Sphere detail level (default: 3 = 1280 triangles)
  --large                 Also generate large test files for performance testing
```

### Large Files (--large)

For performance and memory testing, generates:

| File | Triangles | Size |
|------|-----------|------|
| `large_1m.stl` | ~1.3M | ~50 MB |
| `large_10m.stl` | ~10M | ~500 MB |

These are **not** committed to the repo. Add to `.gitignore`:
```
fixtures/large_*.stl
```

## Adding New Fixtures

Edit `generate_fixtures.py` and add a new shape function:

```python
def create_my_shape() -> mesh.Mesh:
    """Create a custom shape."""
    # Build faces as list of 3-vertex triangles
    faces_list = [
        [[x1, y1, z1], [x2, y2, z2], [x3, y3, z3]],
        # ...
    ]
    
    m = mesh.Mesh(np.zeros(len(faces_list), dtype=mesh.Mesh.dtype))
    for i, f in enumerate(faces_list):
        m.vectors[i] = np.array(f)
    return m
```

Then add it to the `shapes` list in `main()`.
