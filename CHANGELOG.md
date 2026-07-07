# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-07-07

### Added

- **OBJ format support**: Wavefront `.obj` files (vertices and faces, fan-triangulated for polygons), auto-detected from file content.
- **3MF format support**: ZIP/XML `.3mf` packages with the full Core scene graph — build items, component references with nested transforms, and unit metadata (mm, cm, inch, foot, micron). Multi-object files render all objects with correct positioning; component cycles are detected.
- **3MF embedded colors**: colorgroups with per-face and per-vertex colors, interpolated and rendered by default. `--force-color` ignores embedded colors and uses `--material-color`; `--list-colors` prints the palette and exits.
- **Palette overrides**: `--color-map "0:#ff0000,2:#00ff00"` recolors embedded palette entries by index (indices match `--list-colors`).
- **Animated GIF output**: `--animate` renders a 360° rotating preview, with `--frames` and `--frame-delay`. A bounding-sphere radius keeps framing consistent across frames.
- **Dimension overlay**: `--dimensions` draws a depth-aware bounding box with X/Y/Z labels (embedded bitmap font, no external deps), plus `--units mm|in` and `--dimension-color`. Works with PNG, GIF, and the print grid.
- **Watermark overlay**: `--watermark <png>` composites a logo, with `--watermark-position`, `--watermark-opacity`, `--watermark-scale`, and `--watermark-margin`. Applies to PNG, every GIF frame, the print-grid composite, and batch outputs.
- **Directory and recursive input discovery**: directory arguments expand to supported `.stl`/`.obj`/`.3mf` files (case-insensitive); `-r`/`--recursive` traverses subtrees and preserves relative output paths.
- **Library API**: `render_to_image`, `render_animated`, and a unified `MeshReader` for embedding stl-render as a crate.
- **Configuration validation**: `RenderConfig::validate()` rejects invalid sizes, padding, and frame counts before rendering.
- **Continuous integration**: GitHub Actions for fmt/clippy/tests/MSRV and a cross-platform release workflow.

### Changed

- Batch mode reports one status line per file and aggregates errors by severity, returning the worst exit code (input errors take precedence over output errors).
- Empty meshes (zero triangles) are rejected as input errors instead of producing blank output.
- Standardized on Rust edition 2024 with a 1.88 MSRV.
- Colored 3MF test fixtures (`colored_cube.3mf`, `gradient.3mf`, `partial_colors.3mf`) are reproducible via `tools/fixtures/generate_fixtures.py`.

### Fixed

- Degenerate geometry handling: zero-area triangles are skipped and NaN coordinates are rejected.

## [0.1.0] - 2026-05-14

Initial release.

### Added

- **Core rendering**: Software rasterizer with orthographic projection, depth buffering, and flat shading
- **STL parsing**: Streaming parser for binary and ASCII STL files, handles 500MB+ files with bounded memory via mmap
- **View presets**: 
  - Standard views (Y-up): `front`, `back`, `left`, `right`, `top`, `bottom`, `iso`
  - Print bed views (Z-up): `print`, `print-front`, `print-left`, `print-right`, `print-back`
  - Grid composite: `print-grid` renders all four print angles in a 2x2 layout
- **Custom camera angles**: `--azimuth` and `--elevation` for precise control
- **Lighting presets**: `flat` (single light), `studio` (three-point), `technical` (uniform)
- **Material color presets**: `tan`, `blue-grey`, `white`, `black`, `red`, `orange`, `green`, `blue`, `grey`/`gray`, `silver`
- **Anti-aliasing**: Supersampling at 2x (default) or 4x resolution
- **Background options**: Transparent (default) or solid with custom color
- **Batch processing**:
  - Multiple input files: `stl-render *.stl -o output/`
  - Multiple views: `--views front,back,iso`
  - Recursive directories: `-r` / `--recursive`
  - Graceful error handling with `--strict` mode option
- **Piping support**: Read from stdin (`-`), write to stdout (`-`)
- **Metadata output**: `--metadata` writes JSON with triangle count, bounding box, dimensions
- **Deterministic output**: Same input + flags = identical output bytes

### Technical

- Headless operation: No GPU or display required
- Memory-efficient: Framebuffer size bounds memory, not input file size
- Tested on files up to 500MB (~10M triangles)

[Unreleased]: https://github.com/jmkent/stl-render/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/jmkent/stl-render/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/jmkent/stl-render/releases/tag/v0.1.0
