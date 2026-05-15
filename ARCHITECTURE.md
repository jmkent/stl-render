# Architecture

Module responsibilities, data flow, and design rationale for stl-render.

## Supported Formats

| Format | Extensions | Memory Model | Notes |
|--------|------------|--------------|-------|
| STL Binary | .stl | Streaming (mmap) | Fastest, handles 500MB+ |
| STL ASCII | .stl | Streaming (mmap) | Auto-detected from content |
| OBJ | .obj | Buffered | Text-based, fan triangulation |
| 3MF | .3mf | Buffered | Full scene graph support |

Format is auto-detected from file content, not extension:
1. ZIP magic `PK\x03\x04` → 3MF
2. OBJ keywords (`v `, `f `) → OBJ
3. Otherwise → STL

### 3MF Scene Graph

The 3MF parser (`src/tmf3/`) supports the full Core specification scene graph:

- **Build items** (`<build><item>`) with transform matrices
- **Component references** (`<components><component>`) with nested transforms
- **Unit metadata** (`<model unit="...">`) - mm, cm, inch, foot, micron
- **Cycle detection** for component references (100-depth limit)

Transform matrices are accumulated through the hierarchy and applied to vertices during parsing. Multi-object files render all referenced objects with correct positioning.

**Not supported:** Materials extension (colors, textures). Renders use `--material-color`.

### Format Limitations

**OBJ:** Negative (relative) indices, line continuations, and mid-line comments are not supported. Materials and textures are ignored.

---

## Design Rationale

### Primary Goals

This project provides a narrow, scriptable rendering pipeline for generating consistent visualizations of STL and mesh assets.

Unlike thumbnail-focused utilities or heavyweight 3D authoring tools, the focus is on deterministic, batch-friendly rendering with configurable output and minimal operational overhead.

Core features:

- Lightweight - Single static binary with headless rendering support
- Batch-oriented - Efficient processing of large model collections
- Deterministic - Stable framing, lighting, orientation, and output across runs
- Configurable - Parameterized rendering without requiring a full 3D application
- Rich previews - Static renders and lightweight animated visualizations
- Zero configuration - Automatic mesh loading, framing, and sensible defaults

### Why Software Rasterizer

GPU rendering (wgpu, OpenGL) requires platform-specific context setup and produces driver-dependent output. For generating consistent preview assets across machines and CI environments, software rendering is simpler and more reliable.

Performance is adequate: at 512×512, the rasterizer sustains ~14 million triangles/second. Models up to 1M triangles render in under 100ms. See [Performance](#performance) for benchmarks.

### Why Custom Parsers

**STL:** Existing crates like `stl_io` load the entire mesh into memory. A custom streaming parser via memory-mapped I/O keeps memory usage bounded regardless of file size, with no external dependencies for the core format.

**OBJ/3MF:** These formats require buffering (ZIP decompression, text parsing), so we use `zip` and `quick-xml` crates. For 3MF, the parser resolves the full scene graph (build items, components, transforms) rather than relying on external libraries. The triangle interface remains uniform across all formats.

STL binary format (for reference):
```
[80 bytes] header (ignored)
[4 bytes]  triangle count (u32 LE)
[50 bytes] per triangle:
  [12 bytes] normal (3 × f32)
  [36 bytes] vertices (9 × f32)
  [2 bytes]  attribute (ignored)
```

### Why Two-Pass Rendering

Camera auto-framing requires knowing the model's bounding box before rendering:

1. **Pass 1:** Stream triangles, accumulate bounds
2. **Pass 2:** Stream triangles again, transform and rasterize

For STL, memory-mapping makes this efficient. For OBJ/3MF, the buffered triangles are iterated twice from memory.

---

## Directory Structure

```
src/
  main.rs           # Entry point, error handling, exit codes
  cli.rs            # Argument parsing, validation, RenderConfig
  lib.rs            # Public API, MeshReader, render functions
  mesh.rs           # BoundingBox, normal computation
  camera.rs         # ViewPreset, Camera, projection math
  render.rs         # Framebuffer, triangle rasterization, shading
  output.rs         # PNG/GIF encoding, metadata JSON
  stl/
    mod.rs          # StlReader, Triangle, TriangleIter
    parser.rs       # Format detection
    binary.rs       # Binary STL parsing (streaming)
    ascii.rs        # ASCII STL parsing (streaming)
  obj/
    mod.rs          # ObjReader, ObjIter
    parser.rs       # OBJ parsing (buffered)
  tmf3/
    mod.rs          # Tmf3Reader, Tmf3Iter, Unit3mf
    parser.rs       # 3MF scene graph parsing (transforms, builds, components)
```

---

## Data Flow

### Single File Mode

```
┌─────────────────────────────────────────────────────────────────┐
│                         CLI (cli.rs)                            │
│  Parse args → validate → build BatchConfig → iter_jobs()        │
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

### Batch Mode

When multiple inputs or views are requested:

```
BatchConfig.iter_jobs() → [RenderConfig, RenderConfig, ...]
                                │
            ┌───────────────────┼───────────────────┐
            ▼                   ▼                   ▼
    ┌───────────────┐   ┌───────────────┐   ┌───────────────┐
    │ render(job1)  │   │ render(job2)  │   │ render(jobN)  │
    └───────────────┘   └───────────────┘   └───────────────┘
            │                   │                   │
            └───────────────────┼───────────────────┘
                                ▼
                    Collect results, print summary
                    Exit with worst error code
```

For `--recursive`, input directories are expanded to individual STL files before iteration. Output paths preserve the relative directory structure.

---

## Module Details

### main.rs

Entry point with batch processing loop:
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

fn run() -> Result<(), RenderError> {
    let batch_config = cli::parse_args()?;
    
    for config in batch_config.iter_jobs() {
        match stl_render::render(&config) {
            Ok(_) => success_count += 1,
            Err(e) if batch_config.strict => return Err(e),
            Err(e) => errors.push(e),
        }
    }
    
    // Return worst error or Ok
}
```

### cli.rs

Responsibilities:
- Define CLI args with clap derive
- Validate mutual exclusivity (--view vs --azimuth/--elevation, --view vs --views)
- Build `BatchConfig` from validated args (handles single and batch modes)
- Expand recursive directory inputs
- Parse material color presets and hex values
- Handle `--help`, `--version`

```rust
pub struct BatchConfig {
    pub inputs: Vec<BatchInput>,        // Expanded input files
    pub output_dir: Option<PathBuf>,    // For batch mode
    pub output_file: Option<PathBuf>,   // For single file mode
    pub views: Vec<ViewConfig>,         // One or more views to render
    pub strict: bool,                   // Abort on first error
    pub recursive: bool,                // Traverse directories
    // ... shared render settings
}

pub struct RenderConfig {
    pub input: PathBuf,
    pub output: PathBuf,
    pub width: u32,
    pub height: u32,
    pub view: ViewConfig,
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

Batch mode is detected when:
- Multiple input files provided
- `--views` flag used with multiple views
- `--recursive` flag with directory input

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
    Print, PrintFront, PrintLeft, PrintRight, PrintBack, PrintGrid,
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

PNG and GIF encoding:
```rust
pub fn write_png(image: &RgbaImage, path: &Path) -> Result<(), OutputError>;
pub fn write_gif(frames: &[RgbaImage], path: &Path, delay_ms: u16) -> Result<(), OutputError>;
pub fn write_metadata(meta: &RenderMetadata, path: &Path) -> Result<(), OutputError>;
```

---

## Animated GIF Output

When `--animate` is specified, `render_animated()` produces a rotating preview:

1. Parse mesh once, compute bounds once
2. Compute bounding sphere radius for consistent scaling
3. For each frame (default 16):
   - Compute azimuth: `(frame / total) * 360°`
   - Create camera with fixed-scale projection (uses sphere radius)
   - Render frame
4. Encode all frames as GIF with infinite loop

The bounding sphere radius ensures consistent viewport across all rotation angles (no zoom in/out effect).

---

## Material Color Presets

CLI accepts named presets or hex colors:
```bash
--material-color tan
--material-color "#ff6600"
```

| Preset | Hex | RGB |
|--------|-----|-----|
| tan | #C19A6B | 193, 154, 107 |
| blue-grey | #708090 | 112, 128, 144 |
| white | #FFFFFF | 255, 255, 255 |
| black | #1A1A1A | 26, 26, 26 |
| red | #CC3333 | 204, 51, 51 |
| orange | #FF6600 | 255, 102, 0 |
| green | #339933 | 51, 153, 51 |
| blue | #3366CC | 51, 102, 204 |
| grey/gray | #808080 | 128, 128, 128 |
| silver | #C0C0C0 | 192, 192, 192 |

---

## Configuration Validation

`RenderConfig::validate()` checks:
- Width and height > 0
- Padding in range [0.0, 1.0]
- Frames > 0 when animate=true
- AA-scaled dimensions don't overflow u32

Called automatically by `render()` and `render_to_image()`. Returns `RenderError::Config` with clear message on failure.

### lib.rs

Unified mesh reader and public API:

```rust
/// Format-agnostic mesh reader with auto-detection
pub enum MeshReader {
    Stl(StlReader),
    Tmf3(Tmf3Reader),
    Obj(ObjReader),
}

impl MeshReader {
    pub fn open(path: &Path) -> Result<Self, StlError>;
    pub fn from_reader<R: Read>(reader: R) -> Result<Self, StlError>;
    pub fn triangles(&self) -> Result<MeshTriangleIter<'_>, StlError>;
}

/// Public API functions
pub fn render(config: &RenderConfig) -> Result<RenderMetadata, RenderError>;
pub fn render_to_image(config: &RenderConfig) -> Result<(RgbaImage, RenderMetadata), RenderError>;
pub fn render_animated(config: &RenderConfig) -> Result<RenderMetadata, RenderError>;
```

### obj/parser.rs

OBJ parsing (buffered in memory):
```rust
pub fn parse_obj(data: &[u8]) -> Result<Vec<Triangle>, StlError>;
pub fn is_obj_format(data: &[u8]) -> bool;
```

Handles `v` (vertex) and `f` (face) lines. Faces with >3 vertices are triangulated using fan triangulation.

### tmf3/parser.rs

3MF parsing with scene graph resolution:
```rust
pub enum Unit3mf {
    Millimeter, Centimeter, Inch, Foot, Micron,
}

pub struct Parse3mfResult {
    pub triangles: Vec<Triangle>,
    pub unit: Unit3mf,
}

pub fn parse_3mf<R: Read + Seek>(reader: R) -> Result<Parse3mfResult, StlError>;
```

Parse flow:
1. Open ZIP archive, find `3D/*.model` file
2. Parse XML to collect objects, components, and build items
3. Resolve scene graph: follow build items → objects → components recursively
4. Apply accumulated transform matrices to vertices
5. Return triangles with unit metadata

Transform format (3MF 3x4 row-major):
```
"m00 m01 m02 m10 m11 m12 m20 m21 m22 m30 m31 m32"
```
Where m30/m31/m32 are translation components.

---

## Performance

### Render Time

Benchmarks on Apple Silicon (M-series), release build, 512×512 output with 2x AA:

| Model Complexity | File Size | Triangles | Time |
|------------------|-----------|-----------|------|
| Simple component | ~1 MB | 18K | <10ms |
| Small part | ~2 MB | 38K | 30ms |
| Medium model | ~24 MB | 500K | 40ms |
| Large model | ~44 MB | 920K | 80ms |
| Complex character | ~48 MB | 1.0M | 70ms |

Throughput scales linearly with triangle count at approximately 14 million triangles/second for the rasterization pass.

### High Resolution & Animation

| Configuration | Time |
|---------------|------|
| 1M triangles @ 1024×1024, 4x AA | 220ms |
| 1M triangles, animated GIF (16 frames) | 1.1s |
| Batch: 9 models (1.4M total triangles) | 450ms |
| Recursive batch: 26 models | 1.8s |

### Memory Usage

Memory is dominated by the framebuffer, not input geometry. STL files stream via mmap; OBJ and 3MF are buffered.

| Configuration | Max RSS |
|---------------|---------|
| 48MB STL @ 512×512, 2x AA | 59 MB |
| 48MB STL @ 1024×1024, 4x AA | 182 MB |
| 48MB STL, animated GIF (16 frames) | 95 MB |

For typical models at default settings (512×512, 2x AA), expect 20-60 MB RSS regardless of input file size.

---

## Error Handling

Each module defines its own error type:

```rust
// stl/mod.rs - used for all mesh formats
#[derive(Debug, thiserror::Error)]
pub enum StlError {
    #[error("failed to open file: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid format: {0}")]
    InvalidFormat(String),
    #[error("unexpected end of file")]
    UnexpectedEof,
    #[error("ZIP error: {0}")]
    ZipError(String),
    #[error("3MF XML error: {0}")]
    Tmf3ParseError(String),
}

// Top-level
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("mesh error: {0}")]
    Mesh(#[from] StlError),  // handles STL, OBJ, and 3MF errors
    #[error("output error: {0}")]
    Output(#[from] OutputError),
    #[error("invalid config: {0}")]
    Config(String),
}

impl RenderError {
    pub fn exit_code(&self) -> ExitCode {
        match self {
            Self::Config(_) => ExitCode::from(1),
            Self::Mesh(_) => ExitCode::from(2),
            Self::Output(_) => ExitCode::from(3),
        }
    }
}
```

**Batch error aggregation:** Returns the highest severity error (lowest exit code). Input errors (2) take precedence over output errors (3).

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
```

No GPU dependencies. No runtime requirements beyond libc.

### Why These Crates

| Crate | Purpose | Why not alternatives |
|-------|---------|---------------------|
| clap | CLI parsing | Standard, derive macros |
| glam | Vec3/Mat4 math | Fast, minimal, ergonomic |
| image | PNG/GIF encoding | Standard, minimal features |
| memmap2 | Memory-mapped files | Maintained fork of memmap |
| quick-xml | 3MF XML parsing | Fast, low-allocation |
| zip | 3MF archive handling | Standard, handles deflate |
| thiserror | Error types | Derive macro, zero runtime cost |

Avoided:
- `nalgebra`: overkill for basic transforms
- `stl_io`: can't stream, loads full mesh
- `three-d`, `kiss3d`, `bevy`: GPU deps, heavy
