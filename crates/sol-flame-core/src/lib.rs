pub mod aggregator;
pub mod heap;
pub mod symbols;
pub mod trace;

pub use aggregator::{FoldedStackAggregator, Hotspot, ProfileSummary};
pub use heap::{HeapMonitor, HeapWarning, SOLANA_HEAP_START, SOLANA_HEAP_SIZE, SOLANA_HEAP_WARN_THRESHOLD};
pub use symbols::{DwarfSymbolResolver, FrameCategory, ResolvedFrame, SourceLocation, SymbolMap};
pub use trace::{CallFrame, ExecutionTrace, Opcode, StepEvent};

#[derive(Debug, thiserror::Error)]
pub enum FlameCoreError {
    #[error("Failed to parse ELF binary: {0}")]
    ElfParse(String),

    #[error("Failed to parse DWARF debug info: {0}")]
    DwarfParse(String),

    #[error("Symbol not found at address 0x{0:x}")]
    SymbolNotFound(u64),

    #[error("Invalid execution trace: {0}")]
    InvalidTrace(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, FlameCoreError>;
