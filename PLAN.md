# STL Renderer Project Plan

## Goal

Build a standalone Rust binary that renders 3D mesh files (STL, OBJ, 3MF) to deterministic 2D PNG previews or animated GIF.

```bash
stl-render model.stl -o preview.png --view print --material-color tan
stl-render model.3mf -o preview.gif --animate
```

## Project Status

**Completed milestones (M0-M15):** See ARCHITECTURE.md for implementation details.

| Feature | Status |
|---------|--------|
| STL parsing (binary/ASCII, streaming) | ✓ |
| OBJ parsing (buffered) | ✓ |
| 3MF parsing (ZIP/XML, buffered) | ✓ |
| Software rasterizer | ✓ |
| View presets (7 standard + 5 print) | ✓ |
| Print grid composite | ✓ |
| Lighting presets (flat/studio/technical) | ✓ |
| Material color presets | ✓ |
| Anti-aliasing (2x/4x SSAA) | ✓ |
| Animated GIF output | ✓ |
| Batch mode with error handling | ✓ |
| Library API | ✓ |
| Configuration validation | ✓ |

---

## Outstanding Work

### M16: Release Packaging

**Goal:** Automated release pipeline with cross-platform binaries and crates.io publishing.

#### Build Targets

| Target | OS | Archive |
|--------|-------|---------|
| `x86_64-unknown-linux-gnu` | ubuntu-latest | .tar.gz |
| `x86_64-unknown-linux-musl` | ubuntu-latest | .tar.gz |
| `aarch64-unknown-linux-gnu` | ubuntu-latest | .tar.gz |
| `x86_64-apple-darwin` | macos-latest | .tar.gz |
| `aarch64-apple-darwin` | macos-latest | .tar.gz |
| `x86_64-pc-windows-msvc` | windows-latest | .zip |

#### Release Process

1. Update `version` in `Cargo.toml`
2. Update `CHANGELOG.md` with release date
3. Commit and tag: `git tag v0.2.0`
4. Push: `git push origin main --tags`
5. Workflow builds binaries, creates release, publishes to crates.io

#### Pre-Release Checklist

- [ ] Run `cargo publish --dry-run --locked`
- [ ] Verify all 6 targets build in CI
- [ ] Test downloaded binaries on each platform

**Acceptance:** Push tag triggers workflow; all platforms build; GitHub Release and crates.io publish succeed.

---

### Remaining Documentation Tasks

From Known Issues review - functional code complete, documentation pending:

- [ ] **KI4/KI5:** Add format limitations section to README.md documenting 3MF and OBJ subset support
- [ ] **KI8:** Add memory note to README for high-frame-count GIF animations

---

## Known Issues Summary

| KI | Issue | Status |
|----|-------|--------|
| KI1 | Public API Surface | ✓ Reviewed, acceptable for v1 |
| KI2 | Configuration Validation | ✓ Implemented (`RenderConfig::validate()`) |
| KI3 | Batch Error Aggregation | ✓ Implemented (severity-based exit codes) |
| KI4 | 3MF Compatibility | ✓ Functional, needs README docs |
| KI5 | OBJ Compatibility | ✓ Functional, needs README docs |
| KI6 | User-Facing Terminology | ✓ Implemented (`RenderError::Mesh`) |
| KI7 | Release Hygiene | Mostly done, needs publish dry-run |
| KI8 | GIF Memory Use | ✓ Acceptable, needs README note |
| KI9 | Rasterizer Robustness | ✓ Acceptable (backface culling expected) |
| KI10 | Degenerate Geometry | ✓ Implemented (zero-area skip, NaN reject) |
| KI11 | Output Error Handling | ✓ Implemented |

---

## Dependencies

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
glam = "0.29"
image = { version = "0.25", default-features = false, features = ["png", "gif"] }
memmap2 = "0.9"
quick-xml = "0.37"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
zip = "2"

[dev-dependencies]
tempfile = "3"
```
