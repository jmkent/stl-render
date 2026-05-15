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
| 3MF scene graph (transforms, builds, components, units) | ✓ |
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
| Release packaging (M16) | Planned |
| Dimension overlay (M17) | Planned |
| Watermark overlay (M18) | Planned |
| 3MF materials/colors (M19) | Planned |

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

### M17: Dimension Overlay

**Goal:** Project physical dimensions (X/Y/Z extents in mm) onto rendered output to show real-world print size.

#### Use Case

When browsing a model collection, knowing actual print dimensions at a glance helps with print planning. Currently requires opening models in a slicer or CAD tool.

#### CLI Interface

```bash
stl-render model.stl -o preview.png --dimensions
stl-render model.stl -o preview.png --dimensions --units in
stl-render model.stl -o preview.png --dimensions --dimension-color "#ffffff"
```

**Flags:**
- `--dimensions` - Enable dimension overlay
- `--units <mm|in>` - Display units (default: mm)
- `--dimension-color <hex>` - Line/text color (default: auto-contrast)

#### Visual Design

```
┌────────────────────────────┐
│                            │
│     ┌─────────────┐        │
│     │             │ ↕ 45mm │
│     │   [model]   │        │
│     │             │        │
│     └─────────────┘        │
│       ←── 62mm ──→         │
│                            │
│              depth: 38mm   │
└────────────────────────────┘
```

- X/Y dimensions as lines with end caps along model edges
- Z (depth) as text label (can't draw orthogonal line in 2D projection)
- Auto-contrast: white text with dark outline, or vice versa
- Position lines outside model bounds with small margin

#### Implementation

**Text rendering options:**
1. **Embedded bitmap font** - No deps, ~2KB for digits/units, pixel-perfect at small sizes
2. **`ab_glyph` crate** - TrueType rendering, more flexible, adds ~50KB

Recommend option 1 for v1 (lightweight goal), with option to upgrade later.

**Drawing:**
- Use existing `image` crate for pixel manipulation
- Draw lines with configurable thickness (1-2px)
- Render dimension text at line endpoints

**Files to modify:**
- `src/cli.rs` - Add flags
- `src/output.rs` or new `src/overlay.rs` - Drawing logic
- `src/lib.rs` - Apply overlay after render, before encode

#### Test Plan

- [ ] Dimension lines visible on rendered output
- [ ] Dimensions match metadata bounding box values
- [ ] Units display correctly (mm/in conversion)
- [ ] Works with transparent and solid backgrounds
- [ ] Works with animated GIF (overlay on each frame)
- [ ] Auto-contrast readable against light and dark models

**Acceptance:** `--dimensions` shows accurate X/Y/Z measurements overlaid on render.

---

### M18: Watermark Overlay

**Goal:** Composite a creator logo/watermark onto output for branding model previews.

#### Use Case

Creators sharing models want consistent branding. Manual watermarking in image editors doesn't scale for batch processing collections.

#### CLI Interface

```bash
stl-render model.stl -o preview.png --watermark logo.png
stl-render model.stl -o preview.png --watermark logo.png --watermark-position bottom-right
stl-render model.stl -o preview.png --watermark logo.png --watermark-opacity 50 --watermark-scale 20
```

**Flags:**
- `--watermark <path>` - Path to watermark image (PNG with transparency)
- `--watermark-position <pos>` - Placement: `top-left`, `top-right`, `bottom-left`, `bottom-right`, `center` (default: bottom-right)
- `--watermark-opacity <0-100>` - Opacity percentage (default: 100)
- `--watermark-scale <percent>` - Scale relative to output width (default: 15)
- `--watermark-margin <px>` - Margin from edges (default: 10)

#### Implementation

**Image compositing:**
- Load watermark PNG (must support alpha channel)
- Scale to target size based on output dimensions
- Alpha-blend onto rendered frame at specified position
- Apply opacity by multiplying alpha channel

**Algorithm:**
```rust
fn apply_watermark(image: &mut RgbaImage, watermark: &RgbaImage, config: &WatermarkConfig) {
    let scaled = resize(watermark, target_width, target_height);
    let (x, y) = compute_position(image.dimensions(), scaled.dimensions(), config);
    
    for (wx, wy, pixel) in scaled.enumerate_pixels() {
        let dst = image.get_pixel_mut(x + wx, y + wy);
        *dst = alpha_blend(*dst, *pixel, config.opacity);
    }
}
```

**Files to modify:**
- `src/cli.rs` - Add flags, watermark config
- `src/output.rs` or `src/overlay.rs` - Watermark compositing
- `src/lib.rs` - Apply watermark after render

**Dependencies:**
- No new deps needed; `image` crate handles PNG loading and pixel manipulation

#### Test Plan

- [ ] Watermark appears at correct position
- [ ] Opacity reduces watermark visibility correctly
- [ ] Scale produces expected watermark size
- [ ] Margin offsets from edges correctly
- [ ] Transparency preserved (watermark alpha + model)
- [ ] Works with animated GIF (watermark on each frame)
- [ ] Error on missing/invalid watermark file
- [ ] Works in batch mode (same watermark on all outputs)

**Acceptance:** `--watermark logo.png` composites logo at specified position with configurable opacity and scale.

---

### M19: 3MF Materials and Colors

**Goal:** Parse and render 3MF files with embedded colors from the Materials extension.

#### Use Case

Many 3MF files from slicers (PrusaSlicer, Bambu Studio, Cura) contain per-face or per-vertex color information for multi-color prints. Rendering these with their native colors provides accurate previews without manual color specification.

#### 3MF Materials Extension Structure

Colors are defined in `<colorgroup>` resources and referenced by triangles:

```xml
<resources>
  <colorgroup id="1">
    <color color="#FF0000"/>  <!-- index 0: red -->
    <color color="#00FF00"/>  <!-- index 1: green -->
    <color color="#0000FF"/>  <!-- index 2: blue -->
  </colorgroup>
  <object id="2" pid="1" pindex="0">  <!-- default: colorgroup 1, color 0 -->
    <mesh>
      <triangles>
        <!-- Per-face: all vertices same color -->
        <triangle v1="0" v2="1" v3="2" pid="1" p1="0" p2="0" p3="0"/>
        <!-- Per-vertex: gradient across face -->
        <triangle v1="3" v2="4" v3="5" pid="1" p1="0" p2="1" p3="2"/>
      </triangles>
    </mesh>
  </object>
</resources>
```

**Key concepts:**
- `pid` references a `<colorgroup>` or `<basematerials>` by ID
- `p1`, `p2`, `p3` are 0-based indices into the color group (per-vertex)
- Objects can specify default `pid`/`pindex` for triangles without explicit assignment
- Colors are sRGB hex: `#RRGGBB` or `#RRGGBBAA`

#### CLI Interface

```bash
# Default: use embedded colors if present, fall back to --material-color
stl-render model.3mf -o preview.png

# Explicitly enable mesh colors (default behavior, for clarity)
stl-render model.3mf -o preview.png --mesh-colors

# Ignore embedded colors, use uniform material color
stl-render model.3mf -o preview.png --no-mesh-colors --material-color tan

# Override specific colors in the palette (0-indexed)
stl-render model.3mf -o preview.png --color-map "0:#ffffff,2:#000000"

# List colors found in the file (diagnostic)
stl-render model.3mf --list-colors
```

**Flags:**
- `--mesh-colors` - Use embedded mesh colors (default when colors present)
- `--no-mesh-colors` - Ignore embedded colors, use `--material-color`
- `--color-map <SPEC>` - Override specific palette indices: `"index:hex,index:hex,..."`
- `--list-colors` - Print color palette and exit (no render)

**Behavior matrix:**

| File has colors | `--mesh-colors` | `--no-mesh-colors` | Result |
|-----------------|-----------------|--------------------| -------|
| Yes | (default) | - | Use mesh colors |
| Yes | - | Set | Use `--material-color` |
| No | (ignored) | (ignored) | Use `--material-color` |

#### Implementation

##### Phase 1: Color Parsing

**New types** (`src/tmf3/parser.rs`):
```rust
pub struct ColorGroup {
    pub id: u32,
    pub colors: Vec<[u8; 4]>,  // RGBA
}

pub struct TriangleColors {
    pub pid: Option<u32>,       // Property group ID
    pub indices: Option<[u32; 3]>,  // p1, p2, p3 color indices
}

pub struct ColoredTriangle {
    pub vertices: [[f32; 3]; 3],
    pub normal: [f32; 3],
    pub colors: Option<[[u8; 4]; 3]>,  // Per-vertex RGBA, None if no colors
}
```

**Parse flow:**
1. Parse `<colorgroup>` elements into `HashMap<u32, ColorGroup>`
2. Track object default `pid`/`pindex` attributes
3. For each triangle, resolve `p1`/`p2`/`p3` to actual RGBA values
4. Store resolved colors in `ColoredTriangle`

##### Phase 2: Colored Rendering

**Rasterizer changes** (`src/render.rs`):
```rust
// Per-vertex color interpolation during rasterization
fn shade_pixel(
    bary: Vec3,           // Barycentric coordinates
    normal: Vec3,
    vertex_colors: Option<[[u8; 4]; 3]>,
    material_color: [u8; 3],
    light_dir: Vec3,
) -> [u8; 4] {
    let base_color = match vertex_colors {
        Some(vc) => interpolate_colors(vc, bary),  // sRGB interpolation
        None => [material_color[0], material_color[1], material_color[2], 255],
    };
    apply_lighting(base_color, normal, light_dir)
}
```

**sRGB interpolation:** Interpolate in sRGB space (not linear) per 3MF spec for perceptual correctness.

##### Phase 3: Color Map Override

**CLI parsing:**
```rust
fn parse_color_map(spec: &str) -> Result<HashMap<u32, [u8; 4]>, String> {
    // "0:#ff0000,2:#00ff00" -> {0: [255,0,0,255], 2: [0,255,0,255]}
    spec.split(',')
        .map(|pair| {
            let (idx, color) = pair.split_once(':')?;
            Ok((idx.parse()?, parse_hex_color(color)?))
        })
        .collect()
}
```

**Application:** After parsing color groups, apply overrides before resolving triangle colors.

#### Files to Modify

| File | Changes |
|------|---------|
| `src/tmf3/parser.rs` | Parse `<colorgroup>`, resolve per-triangle colors |
| `src/tmf3/mod.rs` | New `ColoredTriangle` type, color accessors |
| `src/stl/mod.rs` | Extend `Triangle` or add parallel `ColoredTriangle` support |
| `src/cli.rs` | Add `--mesh-colors`, `--no-mesh-colors`, `--color-map`, `--list-colors` |
| `src/render.rs` | Per-vertex color interpolation in rasterizer |
| `src/lib.rs` | Thread color settings through render pipeline |

#### Test Plan

**Unit Tests:**
- [ ] Parse `<colorgroup>` with multiple colors
- [ ] Parse sRGB hex with and without alpha
- [ ] Resolve triangle colors from `p1`/`p2`/`p3` indices
- [ ] Apply object default `pid`/`pindex`
- [ ] Handle triangles without color assignment
- [ ] Parse `--color-map` specification
- [ ] Apply color map overrides

**Integration Tests:**
- [ ] Render 3MF with per-face colors
- [ ] Render 3MF with per-vertex gradient colors
- [ ] `--no-mesh-colors` ignores embedded colors
- [ ] `--color-map` overrides specific palette entries
- [ ] `--list-colors` outputs palette without rendering
- [ ] Mixed: some triangles colored, some not
- [ ] Animated GIF preserves colors across frames

**Test Fixtures:**
- [ ] `fixtures/colored_cube.3mf` - Cube with 6 face colors
- [ ] `fixtures/gradient.3mf` - Triangle with per-vertex gradient
- [ ] `fixtures/partial_colors.3mf` - Some faces colored, some default

#### Acceptance Criteria

1. 3MF files with `<colorgroup>` render with correct per-face/per-vertex colors
2. Default behavior uses mesh colors when present
3. `--no-mesh-colors` forces uniform material color
4. `--color-map` allows selective palette overrides
5. `--list-colors` prints diagnostic palette info
6. No performance regression for files without colors
7. Works with transforms, components, and animation

---

### Remaining Documentation Tasks

From Known Issues review - functional code complete, documentation pending:

- [ ] **KI4:** Document 3MF support scope (scene graph, transforms, units; materials in M19)
- [ ] **KI5:** Add OBJ format limitations section to README.md
- [ ] **KI8:** Add memory note to README for high-frame-count GIF animations

---

## Known Issues Summary

| KI | Issue | Status |
|----|-------|--------|
| KI1 | Public API Surface | ✓ Reviewed, acceptable for v1 |
| KI2 | Configuration Validation | ✓ Implemented (`RenderConfig::validate()`) |
| KI3 | Batch Error Aggregation | ✓ Implemented (severity-based exit codes) |
| KI4 | 3MF Compatibility | ✓ Full scene graph support (M14) |
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
