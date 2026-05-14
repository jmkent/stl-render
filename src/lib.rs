pub mod camera;
pub mod cli;
pub mod mesh;
pub mod output;
pub mod render;
pub mod stl;

pub use cli::RenderConfig;
pub use mesh::BoundingBox;
pub use output::RenderMetadata;
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
    pub fn exit_code(&self) -> std::process::ExitCode {
        match self {
            Self::Config(_) => std::process::ExitCode::from(1),
            Self::Stl(_) => std::process::ExitCode::from(2),
            Self::Output(_) => std::process::ExitCode::from(3),
        }
    }
}

pub fn render(config: &RenderConfig) -> Result<RenderMetadata, RenderError> {
    use cli::ViewConfig;

    // Verify input file exists
    if config.input.to_str() != Some("-") && !config.input.exists() {
        return Err(RenderError::Stl(StlError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("input file not found: {}", config.input.display()),
        ))));
    }

    // Parse STL and compute bounds (first pass)
    let is_stdin = config.input.to_str() == Some("-");
    let reader = if is_stdin {
        StlReader::from_reader(std::io::stdin())?
    } else {
        StlReader::open(&config.input)?
    };
    let (bounds, triangle_count) = mesh::compute_bounds(&reader)?;

    if config.verbose {
        let dims = bounds.dimensions();
        eprintln!(
            "Loaded {} triangles, bounds: [{:.2}, {:.2}, {:.2}]",
            triangle_count, dims.x, dims.y, dims.z
        );
    }

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
            camera::Camera::from_preset(preset, &bounds, render_width, render_height, config.padding)
        }
        ViewConfig::Custom { azimuth, elevation } => {
            camera::Camera::from_angles(azimuth, elevation, &bounds, render_width, render_height, config.padding)
        }
    };

    // Create framebuffer
    let mut fb = render::Framebuffer::new(
        render_width,
        render_height,
        config.background,
        config.background_color,
    );

    // Render triangles (second pass)
    for result in reader.triangles()? {
        let tri = result?;
        fb.rasterize_triangle(&tri, &cam, config);
    }

    // Downsample if AA enabled
    let image = fb.into_image(config.aa);

    if config.verbose {
        eprintln!("Rendered {}x{} image", config.width, config.height);
    }

    // Write output
    if config.output.to_str() == Some("-") {
        output::write_png_to_stdout(&image)?;
    } else {
        output::write_png(&image, &config.output)?;
    }

    if config.verbose && config.output.to_str() != Some("-") {
        eprintln!("Wrote {}", config.output.display());
    }

    // Write metadata if requested
    let dims = bounds.dimensions();
    let metadata = RenderMetadata {
        input_file: config.input.to_string_lossy().to_string(),
        triangle_count,
        bounding_box: bounds,
        dimensions: [dims.x, dims.y, dims.z],
    };

    if let Some(ref meta_path) = config.metadata_path {
        output::write_metadata(&metadata, meta_path)?;
    }

    Ok(metadata)
}
