# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/jmkent/stl-render/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/jmkent/stl-render/releases/tag/v0.1.0
