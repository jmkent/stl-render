//! # stl-render
//!
//! A fast, headless library for rendering STL files to PNG images.
//!
//! ## Quick Start
//!
//! ```no_run
//! use stl_render::{render, RenderConfigBuilder, ViewPreset};
//!
//! let config = RenderConfigBuilder::new("model.stl", "output.png")
//!     .view(ViewPreset::Print)
//!     .size(1024)
//!     .material_color([193, 154, 107]) // tan
//!     .build();
//!
//! let metadata = render(&config)?;
//! println!("Rendered {} triangles", metadata.triangle_count);
//! # Ok::<(), stl_render::RenderError>(())
//! ```
//!
//! ## Render to Image (without writing to disk)
//!
//! ```no_run
//! use stl_render::{render_to_image, RenderConfigBuilder};
//!
//! let config = RenderConfigBuilder::new("model.stl", "-") // output path ignored
//!     .size(512)
//!     .build();
//!
//! let (image, metadata) = render_to_image(&config)?;
//! // `image` is an `image::RgbaImage` you can manipulate or encode yourself
//! # Ok::<(), stl_render::RenderError>(())
//! ```

pub mod camera;
pub mod cli;
pub mod mesh;
pub mod output;
pub mod render;
pub mod stl;

// Re-export public types for library consumers
pub use cli::{
    AntiAliasing, Background, BatchConfig, LightingPreset, RenderConfig, RenderConfigBuilder,
    ViewConfig, ViewPreset,
};
pub use mesh::BoundingBox;
pub use output::{OutputError, RenderMetadata};
pub use stl::{StlError, StlReader, Triangle};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("STL error: {0}")]
    Stl(#[from] StlError),

    #[error("output error: {0}")]
    Output(#[from] output::OutputError),

    #[error("invalid config: {0}")]
    Config(String),
}

impl RenderError {
    /// Get the appropriate exit code for this error type.
    pub fn exit_code(&self) -> std::process::ExitCode {
        match self {
            Self::Config(_) => std::process::ExitCode::from(1),
            Self::Stl(_) => std::process::ExitCode::from(2),
            Self::Output(_) => std::process::ExitCode::from(3),
        }
    }
}

/// Render an STL file to an image without writing to disk.
///
/// Returns the rendered image and metadata. Use this when you want to
/// manipulate the image further or encode it yourself.
///
/// # Example
///
/// ```no_run
/// use stl_render::{render_to_image, RenderConfigBuilder, ViewPreset};
///
/// let config = RenderConfigBuilder::new("model.stl", "-")
///     .view(ViewPreset::Iso)
///     .size(512)
///     .build();
///
/// let (image, metadata) = render_to_image(&config)?;
/// println!("Rendered {}x{} image with {} triangles",
///     image.width(), image.height(), metadata.triangle_count);
/// # Ok::<(), stl_render::RenderError>(())
/// ```
pub fn render_to_image(
    config: &RenderConfig,
) -> Result<(image::RgbaImage, RenderMetadata), RenderError> {
    use cli::{ViewConfig, ViewPreset};

    // Check if this is a grid render
    if let ViewConfig::Preset(ViewPreset::PrintGrid) = config.view {
        return render_print_grid_to_image(config);
    }

    // Parse STL and compute bounds (first pass)
    let reader = open_stl_reader(config)?;
    let (bounds, triangle_count) = mesh::compute_bounds(&reader)?;
    validate_renderable_geometry(&bounds, triangle_count)?;

    if config.verbose {
        let dims = bounds.dimensions();
        eprintln!(
            "Loaded {} triangles, bounds: [{:.2}, {:.2}, {:.2}]",
            triangle_count, dims.x, dims.y, dims.z
        );
    }

    // Render to image
    let image = render_single_view(config, &reader, &bounds)?;

    if config.verbose {
        eprintln!("Rendered {}x{} image", config.width, config.height);
    }

    // Build metadata
    let dims = bounds.dimensions();
    let metadata = RenderMetadata {
        input_file: config.input.to_string_lossy().to_string(),
        triangle_count,
        bounding_box: bounds,
        dimensions: [dims.x, dims.y, dims.z],
    };

    Ok((image, metadata))
}

/// Render an STL file to a PNG file.
///
/// This is the main entry point for CLI-style rendering. For library use
/// without file I/O, see [`render_to_image`].
///
/// # Example
///
/// ```no_run
/// use stl_render::{render, RenderConfigBuilder, ViewPreset};
///
/// let config = RenderConfigBuilder::new("model.stl", "output.png")
///     .view(ViewPreset::Print)
///     .material_color([193, 154, 107])
///     .build();
///
/// let metadata = render(&config)?;
/// println!("Wrote {} triangles to output.png", metadata.triangle_count);
/// # Ok::<(), stl_render::RenderError>(())
/// ```
pub fn render(config: &RenderConfig) -> Result<RenderMetadata, RenderError> {
    let (image, metadata) = render_to_image(config)?;

    // Write output
    write_output(&image, config)?;

    // Write metadata if requested
    if let Some(ref meta_path) = config.metadata_path {
        output::write_metadata(&metadata, meta_path)?;
    }

    Ok(metadata)
}

fn render_single_view(
    config: &RenderConfig,
    reader: &StlReader,
    bounds: &BoundingBox,
) -> Result<image::RgbaImage, RenderError> {
    use cli::ViewConfig;

    // Compute render dimensions (scale up for AA)
    let aa_scale = match config.aa {
        cli::AntiAliasing::None => 1,
        cli::AntiAliasing::X2 => 2,
        cli::AntiAliasing::X4 => 4,
    };
    let render_width = config.width * aa_scale;
    let render_height = config.height * aa_scale;

    // Setup camera
    let cam = match config.view {
        ViewConfig::Preset(preset) => {
            camera::Camera::from_preset(preset, bounds, render_width, render_height, config.padding)
        }
        ViewConfig::Custom { azimuth, elevation } => camera::Camera::from_angles(
            azimuth,
            elevation,
            bounds,
            render_width,
            render_height,
            config.padding,
        ),
    };

    // Create framebuffer
    let mut fb = render::Framebuffer::new(
        render_width,
        render_height,
        config.background,
        config.background_color,
    );

    // Render triangles
    for result in reader.triangles()? {
        let tri = result?;
        fb.rasterize_triangle(&tri, &cam, config);
    }

    // Downsample if AA enabled
    Ok(fb.into_image(config.aa))
}

fn render_print_grid_to_image(
    config: &RenderConfig,
) -> Result<(image::RgbaImage, RenderMetadata), RenderError> {
    use cli::{ViewConfig, ViewPreset};
    use image::{GenericImage, RgbaImage};

    // Parse STL and compute bounds
    let reader = open_stl_reader(config)?;
    let (bounds, triangle_count) = mesh::compute_bounds(&reader)?;
    validate_renderable_geometry(&bounds, triangle_count)?;

    if config.verbose {
        let dims = bounds.dimensions();
        eprintln!(
            "Loaded {} triangles, bounds: [{:.2}, {:.2}, {:.2}]",
            triangle_count, dims.x, dims.y, dims.z
        );
    }

    // Each quadrant is half the final dimensions
    let quad_width = config.width / 2;
    let quad_height = config.height / 2;

    // The four print views for the grid:
    // +---------------+---------------+
    // | print-front   | print-right   |
    // +---------------+---------------+
    // | print-back    | print-left    |
    // +---------------+---------------+
    let views = [
        (ViewPreset::PrintFront, 0, 0),                   // top-left
        (ViewPreset::PrintRight, quad_width, 0),          // top-right
        (ViewPreset::PrintBack, 0, quad_height),          // bottom-left
        (ViewPreset::PrintLeft, quad_width, quad_height), // bottom-right
    ];

    // Create the final composite image
    let mut composite = RgbaImage::new(config.width, config.height);

    // Fill with background color first
    let bg_pixel = match config.background {
        cli::Background::Transparent => image::Rgba([0, 0, 0, 0]),
        cli::Background::Solid => image::Rgba([
            config.background_color[0],
            config.background_color[1],
            config.background_color[2],
            255,
        ]),
    };
    for pixel in composite.pixels_mut() {
        *pixel = bg_pixel;
    }

    // Render each quadrant
    for (preset, x_offset, y_offset) in views {
        let quad_config = RenderConfig {
            input: config.input.clone(),
            output: config.output.clone(),
            width: quad_width,
            height: quad_height,
            view: ViewConfig::Preset(preset),
            padding: config.padding,
            aa: config.aa,
            background: config.background,
            background_color: config.background_color,
            material_color: config.material_color,
            lighting: config.lighting,
            metadata_path: None,
            quiet: true,
            verbose: false,
        };

        let quad_image = render_single_view(&quad_config, &reader, &bounds)?;

        // Copy quadrant into composite
        composite
            .copy_from(&quad_image, x_offset, y_offset)
            .map_err(|e| RenderError::Config(format!("failed to composite grid: {}", e)))?;
    }

    if config.verbose {
        eprintln!("Rendered {}x{} grid (4 views)", config.width, config.height);
    }

    // Build metadata
    let dims = bounds.dimensions();
    let metadata = RenderMetadata {
        input_file: config.input.to_string_lossy().to_string(),
        triangle_count,
        bounding_box: bounds,
        dimensions: [dims.x, dims.y, dims.z],
    };

    Ok((composite, metadata))
}

fn open_stl_reader(config: &RenderConfig) -> Result<StlReader, RenderError> {
    if config.input.to_str() == Some("-") {
        return Ok(StlReader::from_reader(std::io::stdin())?);
    }

    if !config.input.exists() {
        return Err(RenderError::Stl(StlError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("input file not found: {}", config.input.display()),
        ))));
    }

    Ok(StlReader::open(&config.input)?)
}

fn validate_renderable_geometry(
    bounds: &BoundingBox,
    triangle_count: u64,
) -> Result<(), RenderError> {
    if triangle_count == 0 || !bounds.is_valid() {
        return Err(RenderError::Stl(StlError::InvalidFormat(
            "STL contains no triangles".into(),
        )));
    }

    let dims = bounds.dimensions();
    let values = [
        bounds.min[0],
        bounds.min[1],
        bounds.min[2],
        bounds.max[0],
        bounds.max[1],
        bounds.max[2],
        dims.x,
        dims.y,
        dims.z,
    ];
    if values.iter().any(|value| !value.is_finite()) {
        return Err(RenderError::Stl(StlError::InvalidFormat(
            "STL bounds contain non-finite values".into(),
        )));
    }

    Ok(())
}

fn write_output(image: &image::RgbaImage, config: &RenderConfig) -> Result<(), RenderError> {
    if config.output.to_str() == Some("-") {
        output::write_png_to_stdout(image)?;
    } else {
        output::write_png(image, &config.output)?;
    }

    if config.verbose && config.output.to_str() != Some("-") {
        eprintln!("Wrote {}", config.output.display());
    }

    Ok(())
}
