# stl-render

A fast, headless CLI tool to render STL files to PNG images.

```bash
stl-render model.stl -o preview.png --view iso
```

## Features

- **Headless:** No GPU or display required. Works in CI, containers, scripts.
- **Deterministic:** Same input + flags = identical output bytes.
- **Handles large files:** Streams 500MB+ STLs with bounded memory.
- **Simple:** One STL in, one PNG out. No configuration files.

## Installation

```bash
cargo install stl-render
```

Or build from source:

```bash
git clone https://github.com/youruser/stl-render
cd stl-render
cargo build --release
./target/release/stl-render --help
```

## Usage

### Basic

```bash
# Render with default settings (iso view, 512x512, transparent background)
stl-render model.stl -o preview.png

# Specify view and size
stl-render model.stl -o preview.png --view front --width 1024 --height 1024

# Custom camera angles
stl-render model.stl -o preview.png --azimuth 45 --elevation 30
```

### View Presets

```bash
--view front    # Looking down -Z axis
--view back     # Looking down +Z axis
--view left     # Looking down +X axis
--view right    # Looking down -X axis
--view top      # Looking down -Y axis
--view bottom   # Looking down +Y axis
--view iso      # Isometric (45° azimuth, 35° elevation)
```

### Appearance

```bash
# Solid background
stl-render model.stl -o preview.png --background solid --background-color "#ffffff"

# Material color
stl-render model.stl -o preview.png --material-color "#ff6600"

# Lighting presets
stl-render model.stl -o preview.png --lighting flat      # Single front light
stl-render model.stl -o preview.png --lighting studio    # Key + fill + rim
stl-render model.stl -o preview.png --lighting technical # Uniform
```

### Image Quality

```bash
# Anti-aliasing (supersampling)
stl-render model.stl -o preview.png --aa none  # Fastest
stl-render model.stl -o preview.png --aa 2x    # Default, good quality
stl-render model.stl -o preview.png --aa 4x    # Best quality, slower

# Padding (space around model)
stl-render model.stl -o preview.png --padding 0.1  # 10% margin
```

### Metadata

```bash
# Output render metadata as JSON
stl-render model.stl -o preview.png --metadata info.json
```

Metadata includes triangle count, bounding box, dimensions, and render settings.

### Piping

```bash
# Read from stdin
cat model.stl | stl-render - -o preview.png

# Write to stdout
stl-render model.stl -o - > preview.png
```

### Batch Processing

```bash
# Multiple files to directory
stl-render *.stl -o output/

# Multiple views per file
stl-render model.stl -o output/ --views front,back,iso
```

## CLI Reference

```
stl-render <INPUT> -o <OUTPUT> [OPTIONS]

Arguments:
  <INPUT>   STL file path, or - for stdin

Options:
  -o, --output <PATH>        Output PNG path, or - for stdout
      --width <PX>           Image width [default: 512]
      --height <PX>          Image height [default: 512]
      --view <PRESET>        View preset: front|back|left|right|top|bottom|iso
      --azimuth <DEG>        Camera azimuth angle (conflicts with --view)
      --elevation <DEG>      Camera elevation angle (conflicts with --view)
      --padding <RATIO>      Padding around model [default: 0.08]
      --aa <LEVEL>           Anti-aliasing: none|2x|4x [default: 2x]
      --background <TYPE>    Background: transparent|solid [default: transparent]
      --background-color <HEX>  Background color for solid [default: #ffffff]
      --material-color <HEX> Model color [default: #cccccc]
      --lighting <PRESET>    Lighting: flat|studio|technical [default: studio]
      --metadata <PATH>      Write render metadata JSON
      --quiet                Suppress progress output
      --verbose              Detailed progress
  -h, --help                 Print help
  -V, --version              Print version
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Usage error (bad arguments) |
| 2 | Input error (can't read/parse STL) |
| 3 | Output error (can't write PNG) |

## Development

### Prerequisites

- Rust 1.70+
- No GPU required
- Python 3.10+ and [uv](https://docs.astral.sh/uv/) (for generating test fixtures only)

### Build

```bash
cargo build
cargo build --release
```

### Test

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_parse_binary_stl
```

### Lint

```bash
cargo clippy
cargo fmt --check
```

### Generate Test Fixtures

Test STL files are generated programmatically using numpy-stl. Python is **not** required to build or run stl-render, only to regenerate fixtures.

```bash
cd tools/fixtures

# Generate all fixtures to fixtures/ directory
uv run generate_fixtures.py -o ../../fixtures

# Or with higher-detail sphere
uv run generate_fixtures.py -o ../../fixtures --sphere-subdivisions 4
```

This creates:
- Basic shapes: cube, sphere, cylinder
- Edge cases: tall column, flat tile, long beam
- Minimal: single triangle
- Invalid: empty, truncated, degenerate triangles
- Both binary and ASCII formats

See [tools/fixtures/README.md](tools/fixtures/README.md) for details.

### Project Structure

```
src/
  main.rs           # Entry point
  cli.rs            # Argument parsing
  stl/              # STL parsing (streaming)
  mesh.rs           # Bounding box, normals
  camera.rs         # View transforms
  render.rs         # Software rasterizer
  output.rs         # PNG encoding
  lib.rs            # Public API

fixtures/           # Test STL files (generated)
  golden/           # Reference PNGs for comparison

tools/
  fixtures/         # Python fixture generator (uv project)
```

See [ARCHITECTURE.md](ARCHITECTURE.md) for detailed design documentation.

## Testing

### Unit Tests

Each module has unit tests covering:
- STL parsing (binary and ASCII)
- Bounding box computation
- Camera projection math
- Config validation

### Golden Image Tests

Reference PNGs in `fixtures/golden/` are compared against rendered output. Run with:

```bash
cargo test golden
```

### Test Fixtures

Generated via `tools/fixtures/generate_fixtures.py`:

| File | Description | Triangles |
|------|-------------|-----------|
| `cube.stl` | Unit cube, binary | 12 |
| `cube_ascii.stl` | Unit cube, ASCII | 12 |
| `sphere.stl` | Icosphere | 1280 |
| `cylinder.stl` | Cylinder | 128 |
| `tall_column.stl` | Tall thin box (aspect ratio) | 12 |
| `flat_tile.stl` | Flat wide box (near-planar) | 12 |
| `long_beam.stl` | Long thin box (elongated) | 12 |
| `single_triangle.stl` | Minimal valid STL | 1 |
| `degenerate.stl` | Zero-area triangles | 3 |
| `empty.stl` | Valid, 0 triangles | 0 |
| `truncated.stl` | Invalid (truncated) | - |

## Performance

Typical render times (512×512, 2x AA):

| STL Size | Triangles | Time |
|----------|-----------|------|
| 1 MB | ~20K | <100ms |
| 50 MB | ~1M | ~500ms |
| 500 MB | ~10M | ~2s |

Memory usage is bounded by output resolution, not input size. A 500MB STL renders with <200MB RSS.

## License

MIT
