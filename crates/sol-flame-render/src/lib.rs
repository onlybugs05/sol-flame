pub mod flame;
pub mod profile;
pub mod terminal;

pub use flame::{FlameRenderer, PaletteKind};
pub use profile::{CiRegressionReport, CuHotspot, CuProfile};
pub use terminal::TerminalPresenter;

#[derive(Debug, thiserror::Error)]
pub enum FlameRenderError {
    #[error("Flamegraph generation failed: {0}")]
    InfernoError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, FlameRenderError>;
