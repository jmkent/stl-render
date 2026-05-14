# stl-render

A fast, headless CLI tool to render STL and 3MF files to PNG images.

```bash
stl-render model.stl -o preview.png
stl-render model.3mf -o preview.png
```

| 3DBenchy, Blue Grey, Print View                           | 3DBenchy, Tan, Print View                      |
|-----------------------------------------------------------|------------------------------------------------|
| ![3DBenchy Blue Grey](examples/benchy_print_bluegrey.png) | ![3DBenchy Tan](examples/benchy_print_tan.png) |

<sub>Renders of [3DBenchy](https://www.3dbenchy.com) - the 3D printing benchmark model (public domain)</sub>

## Features

- **Headless:** No GPU or display required. Works in CI, containers, scripts.
- **Multiple formats:** STL (binary/ASCII) and 3MF with auto-detection.
- **Animated GIF:** `--animate` produces rotating preview animations.
- **Deterministic:** Same input + flags = identical output bytes.
- **Handles large files:** Streams 500MB+ STLs with bounded memory.
- **Print-ready:** `--view print` shows models in Z-up orientation for 3D printing.
- **Simple:** One mesh in, one PNG or GIF out. No configuration files.

## Installation

```bash
cargo install stl-render
```

Or build from source:

```bash
git clone https://github.com/jmkent/stl-render
cd stl-render
cargo build --release
./target/release/stl-render --help
```

## Quick Start

```bash
# Render with default settings (iso view, 512x512, transparent background)
stl-render model.stl -o preview.png

# Print bed view with tan filament color
stl-render model.stl -o preview.png --view print --material-color tan

# High quality render
stl-render model.stl -o preview.png --view print --aa 4x --width 1024 --height 1024

# Animated rotating preview
stl-render model.stl -o preview.gif --animate --material-color tan
```

## Supported Formats

| Format | Extensions | Notes |
|--------|------------|-------|
| STL Binary | .stl | Fastest, most common |
| STL ASCII | .stl | Auto-detected from content |
| 3MF | .3mf | ZIP with XML mesh data, multi-object support |

Format is auto-detected from file content (magic bytes), not extension. Multi-object 3MF files render all objects merged into a single view.

## View Presets

| Front | Isometric | Print (Z-up) |
|-------|-----------|--------------|
| ![Front](examples/view_front.png) | ![Iso](examples/view_iso.png) | ![Print](examples/view_print.png) |

**Standard views (Y-up):**
```bash
--view front    # Looking at front face
--view back     # Looking at back face  
--view left     # Looking at left face
--view right    # Looking at right face
--view top      # Looking down from above
--view bottom   # Looking up from below
--view iso      # Isometric (45° azimuth, 35° elevation)
```

**Print bed views (Z-up):**
```bash
--view print         # Print bed view (default print angle)
--view print-front   # Print view from front
--view print-left    # Print view from left
--view print-right   # Print view from right
--view print-back    # Print view from back
--view print-grid    # 2x2 grid of all four print angles
```

The `print` views are designed for 3D printing - they keep the Z axis vertical (as it would be on a print bed) while tilting slightly to show the top surface.

### Print Grid

Generate a single image showing all four print angles:

| Print Grid |
|------------|
| ![Print Grid](examples/benchy_print_grid.png) |

```bash
stl-render model.stl -o preview.png --view print-grid
```

## Lighting Presets

| Flat | Studio | Technical |
|------|--------|-----------|
| ![Flat](examples/lighting_flat.png) | ![Studio](examples/lighting_studio.png) | ![Technical](examples/lighting_technical.png) |

```bash
--lighting flat       # Single front light
--lighting studio     # Key + fill + rim (default)
--lighting technical  # Uniform multi-directional
```

## Material Colors

Use named filament presets or any 6-digit hex color:

```bash
stl-render model.stl -o preview.png --material-color tan
stl-render model.stl -o preview.png --material-color blue-grey
stl-render model.stl -o preview.png --material-color "#ffcc00"
```

Available presets: `tan`, `blue-grey`, `white`, `black`, `red`, `orange`, `green`, `blue`, `grey`/`gray`, `silver`. Preset names are case insensitive.

## More Examples

See [EXAMPLES.md](EXAMPLES.md) for comprehensive examples including:
- Animated GIF output
- Print view presets and grid
- Batch processing multiple files
- Material colors (filament presets)
- Background options
- Anti-aliasing comparison
- Custom camera angles
- Metadata output
- Piping and automation

## CLI Reference

```
stl-render <INPUT>... -o <OUTPUT> [OPTIONS]

Arguments:
  <INPUT>...  Mesh file(s) - STL or 3MF, supports multiple files

Options:
  -o, --output <PATH>           Output PNG path or directory (use trailing / for directory)
      --width <PX>              Image width [default: 512]
      --height <PX>             Image height [default: 512]
      --view <PRESET>           Single view preset
      --views <LIST>            Multiple views (comma-separated), outputs to directory
  -r, --recursive               Recursively render .stl files from input directories
      --azimuth <DEG>           Camera azimuth angle (conflicts with --view)
      --elevation <DEG>         Camera elevation angle (conflicts with --view)
      --padding <RATIO>         Padding around model [default: 0.08]
      --aa <LEVEL>              Anti-aliasing: none|2x|4x [default: 2x]
      --background <TYPE>       Background: transparent|solid [default: transparent]
      --background-color <HEX>  Background color for solid [default: #ffffff]
      --material-color <COLOR>  Model color: hex or preset [default: #cccccc]
      --lighting <PRESET>       Lighting: flat|studio|technical [default: studio]
      --animate                 Enable animated GIF output (rotating view)
      --frames <N>              Number of animation frames [default: 16]
      --frame-delay <MS>        Milliseconds between frames [default: 100]
      --metadata <PATH>         Write render metadata JSON
      --strict                  Abort on first error (default: continue processing)
      --quiet                   Suppress progress output
      --verbose                 Show progress info
  -h, --help                    Print help
  -V, --version                 Print version

View presets:
  front, back, left, right, top, bottom, iso
  print, print-front, print-left, print-right, print-back, print-grid
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Usage error (bad arguments) |
| 2 | Input error (can't read/parse mesh file) |
| 3 | Output error (can't write PNG) |

Empty mesh files (zero triangles) are rejected with exit code 2 because there is no geometry to frame or describe in metadata.

## Development

### Prerequisites

- Rust 1.88+
- No GPU required

### Build & Test

```bash
cargo build --release
cargo test
cargo clippy
```

### Generate Test Fixtures

Test mesh files (STL and 3MF) are generated using Python (optional, only for regenerating fixtures):

```bash
cd tools/fixtures
uv run generate_fixtures.py -o ../../fixtures
```

### Regenerate Example Renders

The checked-in example PNGs can be regenerated from fixtures and the local 3DBenchy reference:

```bash
cargo run --release -- ~/3dprinting/3dbenchy/files/3DBenchy.stl \
  -o examples/benchy_print_tan.png --view print --material-color tan --aa 4x
cargo run --release -- ~/3dprinting/3dbenchy/files/3DBenchy.stl \
  -o examples/benchy_print_bluegrey.png --view print --material-color blue-grey --aa 4x
cargo run --release -- fixtures/cube.stl \
  -o examples/cube_iso_bluegrey.png --view iso --material-color blue-grey
```

### Project Structure

```
src/
  main.rs           # Entry point
  cli.rs            # Argument parsing
  stl/              # STL parsing (streaming, mmap)
  tmf3/             # 3MF parsing (ZIP/XML)
  mesh.rs           # Bounding box, normals
  camera.rs         # View transforms, projection
  render.rs         # Software rasterizer
  output.rs         # PNG encoding
  lib.rs            # Public API, MeshReader

examples/           # Rendered example images
fixtures/           # Test mesh files (STL and 3MF)
```

See [ARCHITECTURE.md](ARCHITECTURE.md) for detailed design documentation.

## Performance

Typical render times (512x512, 2x AA, Apple M1):

| STL Size | Triangles | Time |
|----------|-----------|------|
| 1 MB | ~20K | <100ms |
| 50 MB | ~1M | ~500ms |
| 500 MB | ~10M | ~2s |

Memory usage is bounded by output resolution, not input size.

## Acknowledgments

Example renders include [3DBenchy](https://www.3dbenchy.com), the 3D printing benchmark model created by Creative Tools, now in the public domain.

## License

MIT
