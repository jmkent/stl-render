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

pub fn render(_config: &RenderConfig) -> Result<RenderMetadata, RenderError> {
    todo!("render implementation")
}
