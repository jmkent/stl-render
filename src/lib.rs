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
    // Verify input file exists
    if config.input.to_str() != Some("-") && !config.input.exists() {
        return Err(RenderError::Stl(StlError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("input file not found: {}", config.input.display()),
        ))));
    }

    // Parse STL and compute metadata
    let (triangle_count, bounds) = if config.input.to_str() == Some("-") {
        // Stdin: placeholder values (stdin support deferred)
        (0, BoundingBox::new())
    } else {
        // Read STL and iterate triangles
        let reader = StlReader::open(&config.input)?;

        // Get count from header (binary) or count by iterating (ASCII)
        let count = match reader.triangle_count() {
            Some(c) => c,
            None => {
                // ASCII: must count by iterating
                reader.triangles()?.filter(|r| r.is_ok()).count() as u64
            }
        };

        // TODO: compute actual bounds in M3
        (count, BoundingBox::new())
    };

    // Create placeholder image (solid gray)
    let fb = render::Framebuffer::new(
        config.width,
        config.height,
        config.background,
        config.background_color,
    );
    let image = fb.into_image(config.aa);

    // Write output
    if config.output.to_str() == Some("-") {
        output::write_png_to_stdout(&image)?;
    } else {
        output::write_png(&image, &config.output)?;
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
