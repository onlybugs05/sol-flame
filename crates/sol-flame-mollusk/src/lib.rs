pub mod harness;
pub mod heap;
pub mod simulator;
pub mod tracer;

pub use harness::MolluskHarness;
pub use heap::MolluskHeapSentinel;
pub use simulator::{ProfileScenario, SvmInstructionSimulator};
pub use tracer::MolluskComputeTracer;

#[derive(Debug, thiserror::Error)]
pub enum MolluskFlameError {
    #[error("Core flame error: {0}")]
    Core(#[from] sol_flame_core::FlameCoreError),

    #[error("Execution failed: {0}")]
    Execution(String),

    #[error("SVM out of compute units: {0}")]
    ComputeExceeded(String),

    #[error("Heap overflow: {0}")]
    HeapOverflow(String),
}

pub type Result<T> = std::result::Result<T, MolluskFlameError>;
