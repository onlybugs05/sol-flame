use serde::{Deserialize, Serialize};
use crate::symbols::{FrameCategory, ResolvedFrame};

/// Common sBPF opcodes tracked during execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Opcode {
    /// CALL instruction: 0x85
    Call = 0x85,
    /// EXIT instruction: 0x95
    Exit = 0x95,
    /// JMP / Branch instruction
    Jmp = 0x05,
    /// Memory load
    Load = 0x61,
    /// Memory store
    Store = 0x63,
    /// Other instructions
    Other = 0x00,
}

impl From<u8> for Opcode {
    fn from(byte: u8) -> Self {
        match byte {
            0x85 => Opcode::Call,
            0x95 => Opcode::Exit,
            0x05 => Opcode::Jmp,
            0x61 => Opcode::Load,
            0x63 => Opcode::Store,
            _ => Opcode::Other,
        }
    }
}

/// An active or recorded call frame in the execution call tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallFrame {
    pub symbol: String,
    pub demangled: String,
    pub source_location: Option<String>,
    pub category: FrameCategory,
    pub entry_cu: u64,
    pub exit_cu: Option<u64>,
    pub delta_cu: u64,
    pub heap_offset_bytes: u64,
    pub depth: usize,
    pub children: Vec<CallFrame>,
}

impl CallFrame {
    pub fn new(symbol: String, demangled: String, category: FrameCategory, entry_cu: u64, depth: usize) -> Self {
        Self {
            symbol,
            demangled,
            source_location: None,
            category,
            entry_cu,
            exit_cu: None,
            delta_cu: 0,
            heap_offset_bytes: 0,
            depth,
            children: Vec::new(),
        }
    }

    pub fn from_resolved(frame: &ResolvedFrame, entry_cu: u64, depth: usize) -> Self {
        Self {
            symbol: frame.symbol.clone(),
            demangled: frame.demangled.clone(),
            source_location: frame.location.as_ref().map(|loc| loc.to_string()),
            category: frame.category,
            entry_cu,
            exit_cu: None,
            delta_cu: 0,
            heap_offset_bytes: 0,
            depth,
            children: Vec::new(),
        }
    }

    pub fn complete(&mut self, exit_cu: u64) {
        self.exit_cu = Some(exit_cu);
        if self.entry_cu >= exit_cu {
            self.delta_cu = self.entry_cu - exit_cu;
        } else {
            // If meter counts up instead of down
            self.delta_cu = exit_cu - self.entry_cu;
        }
    }
}

/// Event captured by the instruction tracer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepEvent {
    PushFrame {
        pc: u64,
        frame: ResolvedFrame,
        remaining_cu: u64,
        heap_offset: u64,
    },
    PopFrame {
        pc: u64,
        remaining_cu: u64,
        heap_offset: u64,
    },
    CpiStart {
        target_program: String,
        instruction_name: Option<String>,
        remaining_cu: u64,
    },
    CpiEnd {
        remaining_cu: u64,
    },
    HeapAlloc {
        size: u64,
        new_offset: u64,
    },
}

/// Full execution trace collected across an instruction call
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionTrace {
    pub instruction_name: String,
    pub initial_cu: u64,
    pub final_cu: u64,
    pub total_cu_consumed: u64,
    pub peak_heap_bytes: u64,
    pub root_frames: Vec<CallFrame>,
    pub events: Vec<StepEvent>,
}

impl ExecutionTrace {
    pub fn new(instruction_name: String, initial_cu: u64) -> Self {
        Self {
            instruction_name,
            initial_cu,
            final_cu: initial_cu,
            total_cu_consumed: 0,
            peak_heap_bytes: 0,
            root_frames: Vec::new(),
            events: Vec::new(),
        }
    }

    pub fn record_event(&mut self, event: StepEvent) {
        self.events.push(event);
    }
}
