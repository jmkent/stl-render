# Architecture

Module responsibilities, data flow, and design rationale for stl-render.

## Design Rationale

### Why Software Rasterizer

Evaluated three options:

| Option | Pros | Cons |
|--------|------|------|
| wgpu | Modern, cross-platform | Headless varies by platform, driver-dependent output |
| OpenGL/glow | Mature | Context creation painful, platform-sensitive |
| **Software** | Headless by design, deterministic, no GPU deps | More implementation work |

For this use case, software rendering wins:
- No GPU context setup headaches in CI/containers
- Deterministic output (no driver variance)
- Works everywhere without GPU libraries
- For 500MB+ STLs, GPU would need chunked uploads anyway

Performance is not a concern: at 512x512, even a naive rasterizer handles millions of triangles per second. A 500MB binary STL (~10M triangles) renders in <2 seconds single-threaded.

### Why Custom STL Parser

Existing crates like `stl_io` load the entire mesh into memory. For 500MB+ files, we need streaming. Rolling a custom parser:

- Zero external deps for core parsing
- Streaming via memory-mapped I/O
- Full control over error handling
- STL format is trivial (~300 lines for both binary and ASCII)

Binary format:
```
[80 bytes] header (ignored)
[4 bytes]  triangle count (u32 LE)
[50 bytes] per triangle:
  [12 bytes] normal (3 × f32)
  [36 bytes] vertices (9 × f32)
  [2 bytes]  attribute (ignored)
```

ASCII format:
```
solid [name]
  facet normal nx ny nz
    outer loop
      vertex x y z
      vertex x y z
      vertex x y z
    endloop
  endfacet
  ...
endsolid [name]
```

### Why Two-Pass Rendering

Camera auto-framing requires knowing the model's bounding box before rendering. Without storing the full mesh, we must:

1. **Pass 1:** Stream triangles, accumulate bounds
2. **Pass 2:** Stream triangles again, transform and rasterize

Memory-mapping makes this efficient: if the file fits in page cache, the second pass is essentially free.

Alternative approaches rejected:
- Store full mesh: 500MB+ memory usage
- User-provided bounds: poor UX
- Fixed camera: can't auto-fit

---

## Directory Structure

```
src/
  main.rs           # Entry point, error handling, exit codes
  cli.rs            # Argument parsing with clap, validation
  stl/
    mod.rs          # Public interface: StlReader, Triangle
    parser.rs       # Format detection, streaming iteration
    binary.rs       # Binary STL parsing
    ascii.rs        # ASCII STL parsing
  mesh.rs           # BoundingBox, normal computation
  camera.rs         # ViewPreset, Camera, projection math
  render.rs         # DepthBuffer, triangle rasterization, shading
  output.rs         # PNG encoding, metadata JSON
  lib.rs            # Public API for library consumers
```

---

## Data Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                         CLI (cli.rs)                            │
│  Parse args → validate → build RenderConfig                     │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Pass 1: Bounds (stl/ + mesh.rs)              │
│  mmap file → stream triangles → accumulate bounding box         │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Camera Setup (camera.rs)                     │
│  bounds + view preset → compute projection matrix               │
│  auto-fit model to frame with padding                           │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Pass 2: Render (stl/ + render.rs)            │
│  stream triangles again → transform → rasterize → shade         │
│  accumulate into depth buffer + color buffer                    │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Output (output.rs)                           │
│  color buffer → anti-alias downsample → encode PNG              │
│  optionally write metadata JSON                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Module Details

### main.rs

Minimal entry point:
```rust
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            e.exit_code()
        }
    }
}
```

### cli.rs

Responsibilities:
- Define CLI args with clap derive
- Validate mutual exclusivity (--view vs --azimuth/--elevation)
- Build `RenderConfig` from validated args
- Handle `--help`, `--version`

```rust
pub struct RenderConfig {
    pub input: PathBuf,
    pub output: PathBuf,
    pub width: u32,
    pub height: u32,
    pub view: ViewConfig,
    pub projection: Projection,
    pub padding: f32,
    pub background: Background,
    pub material_color: [u8; 3],
    pub lighting: LightingPreset,
    pub aa: AntiAliasing,
    pub metadata_path: Option<PathBuf>,
}

pub enum ViewConfig {
    Preset(ViewPreset),
    Custom { azimuth: f32, elevation: f32 },
}
```

### stl/mod.rs

Public interface for STL reading:
```rust
pub struct StlReader { /* mmap handle, format, position */ }

impl StlReader {
    pub fn open(path: &Path) -> Result<Self, StlError>;
    pub fn triangle_count(&self) -> Option<u64>;  // None for ASCII
    pub fn iter(&self) -> TriangleIter<'_>;
}

pub struct Triangle {
    pub vertices: [[f32; 3]; 3],
    pub normal: [f32; 3],
}
```

### stl/parser.rs

Format detection:
```rust
pub fn detect_format(data: &[u8]) -> StlFormat;
```

Heuristic: if file starts with "solid" and contains "facet", treat as ASCII. Otherwise binary. This handles the edge case where binary header happens to start with "solid".

### stl/binary.rs

Binary STL iteration:
```rust
pub struct BinaryStlIter<'a> {
    data: &'a [u8],
    offset: usize,
    remaining: u32,
}
```

Each `next()` reads 50 bytes, parses floats (little-endian), returns `Triangle`.

### stl/ascii.rs

ASCII STL iteration:
```rust
pub struct AsciiStlIter<'a> {
    data: &'a [u8],
    position: usize,
}
```

Parses line-by-line, handles whitespace variations, scientific notation.

### mesh.rs

Geometry utilities (no full mesh storage):
```rust
pub struct BoundingBox {
    pub min: Vec3,
    pub max: Vec3,
}

impl BoundingBox {
    pub fn new() -> Self;           // Empty box
    pub fn extend(&mut self, point: Vec3);
    pub fn center(&self) -> Vec3;
    pub fn dimensions(&self) -> Vec3;
}

pub fn compute_normal(v0: Vec3, v1: Vec3, v2: Vec3) -> Vec3;
```

### camera.rs

View and projection:
```rust
pub enum ViewPreset {
    Front, Back, Left, Right, Top, Bottom, Iso,
}

pub struct Camera {
    pub view_matrix: Mat4,
    pub proj_matrix: Mat4,
}

impl Camera {
    pub fn from_preset(preset: ViewPreset, bounds: &BoundingBox, padding: f32) -> Self;
    pub fn from_angles(azimuth: f32, elevation: f32, bounds: &BoundingBox, padding: f32) -> Self;
}
```

Auto-framing algorithm:
1. Transform bounding box 8 corners by view matrix
2. Find min/max X, Y in view space
3. Compute orthographic bounds to fit with padding
4. Center projection on model centroid

### render.rs

Software rasterizer:
```rust
pub struct Framebuffer {
    pub width: u32,
    pub height: u32,
    depth: Vec<f32>,
    color: Vec<[u8; 4]>,
}

impl Framebuffer {
    pub fn new(width: u32, height: u32, background: Background) -> Self;
    pub fn rasterize_triangle(&mut self, tri: &Triangle, camera: &Camera, config: &RenderConfig);
    pub fn into_image(self, aa: AntiAliasing) -> RgbaImage;
}
```

Rasterization algorithm:
1. Transform vertices: model → view → clip → NDC → screen
2. Clip against near plane
3. Compute screen-space bounding box
4. For each pixel in bbox:
   - Compute barycentric coordinates
   - If inside (all >= 0): interpolate depth
   - If depth < buffer: compute shading, write color

Shading: `color = material_color * max(0, dot(normal, light_dir))`

### output.rs

Final output:
```rust
pub fn write_png(image: &RgbaImage, path: &Path) -> Result<(), OutputError>;
pub fn write_metadata(meta: &RenderMetadata, path: &Path) -> Result<(), OutputError>;

pub struct RenderMetadata {
    pub input_file: String,
    pub triangle_count: u64,
    pub bounding_box: BoundingBox,
    pub dimensions: [f32; 3],
    pub render_config: RenderConfig,
}
```

### lib.rs

Public API:
```rust
pub use crate::cli::RenderConfig;
pub use crate::stl::{StlReader, Triangle, StlError};
pub use crate::mesh::BoundingBox;
pub use crate::output::RenderMetadata;

pub fn render(config: &RenderConfig) -> Result<RenderMetadata, RenderError>;
```

---

## Memory Budget

For 500MB+ STLs, memory usage is bounded by framebuffer, not geometry:

| Component | Memory |
|-----------|--------|
| mmap file | 0 (kernel manages pages) |
| Pass 1 working set | ~100 bytes |
| Framebuffer (512×512, no AA) | 4 MB |
| Framebuffer (1024×1024, 2x AA) | 16 MB |
| Framebuffer (2048×2048, 4x AA) | 64 MB |

Total RSS should stay under 200MB even for 500MB+ input files.

---

## Error Handling

Each module defines its own error type:

```rust
// stl/mod.rs
#[derive(Debug, thiserror::Error)]
pub enum StlError {
    #[error("failed to open file: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid STL format: {0}")]
    InvalidFormat(String),
    #[error("unexpected end of file")]
    UnexpectedEof,
}

// Top-level
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("STL error: {0}")]
    Stl(#[from] StlError),
    #[error("output error: {0}")]
    Output(#[from] OutputError),
    #[error("invalid config: {0}")]
    Config(String),
}

impl RenderError {
    pub fn exit_code(&self) -> ExitCode {
        match self {
            Self::Config(_) => ExitCode::from(1),
            Self::Stl(_) => ExitCode::from(2),
            Self::Output(_) => ExitCode::from(3),
        }
    }
}
```

---

## Dependencies

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
glam = "0.27"
image = { version = "0.25", default-features = false, features = ["png"] }
memmap2 = "0.9"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
tiny-skia = "0.11"
```

No GPU dependencies. No runtime requirements beyond libc.

### Why These Crates

| Crate | Purpose | Why not alternatives |
|-------|---------|---------------------|
| clap | CLI parsing | Standard, derive macros |
| glam | Vec3/Mat4 math | Fast, minimal, ergonomic |
| image | PNG encoding | Standard, minimal features enabled |
| memmap2 | Memory-mapped files | Maintained fork of memmap |
| thiserror | Error types | Derive macro, zero runtime cost |
| tiny-skia | 2D compositing | Software-only, good for downsampling |

Avoided:
- `nalgebra`: overkill for basic transforms
- `stl_io`: can't stream, loads full mesh
- `three-d`, `kiss3d`, `bevy`: GPU deps, heavy
