use sol_flame_core::heap::HeapMonitor;
use sol_flame_core::symbols::{FrameCategory, ResolvedFrame, SymbolMap};
use sol_flame_core::trace::{CallFrame, ExecutionTrace, Opcode, StepEvent};

/// State of an active call in the tracer call stack.
#[derive(Debug, Clone)]
struct ActiveFrame {
    call_frame: CallFrame,
    #[allow(dead_code)]
    pc: u64,
}

/// Compute meter and opcode execution tracer hooked into the SVM execution loop.
///
/// Hooks:
/// - Opcode 0x85 (CALL): Push stack frame, record entry Compute Meter
/// - Opcode 0x95 (EXIT): Pop stack frame, calculate ΔCompute Meter
/// - CPI boundary: Tag stack frame with `cpi:<PROGRAM_NAME>`
/// - Heap allocation: Track peak bump pointer against 32KB Solana limit
pub struct MolluskComputeTracer {
    instruction_name: String,
    initial_cu: u64,
    remaining_cu: u64,
    active_stack: Vec<ActiveFrame>,
    completed_roots: Vec<CallFrame>,
    events: Vec<StepEvent>,
    symbol_map: Option<SymbolMap>,
    heap_monitor: HeapMonitor,
    cpi_program_stack: Vec<String>,
}

impl MolluskComputeTracer {
    pub fn new(instruction_name: impl Into<String>, initial_cu: u64) -> Self {
        Self {
            instruction_name: instruction_name.into(),
            initial_cu,
            remaining_cu: initial_cu,
            active_stack: Vec::new(),
            completed_roots: Vec::new(),
            events: Vec::new(),
            symbol_map: None,
            heap_monitor: HeapMonitor::new(),
            cpi_program_stack: Vec::new(),
        }
    }

    pub fn with_symbol_map(mut self, symbol_map: SymbolMap) -> Self {
        self.symbol_map = Some(symbol_map);
        self
    }

    pub fn remaining_cu(&self) -> u64 {
        self.remaining_cu
    }

    pub fn set_remaining_cu(&mut self, cu: u64) {
        self.remaining_cu = cu;
    }

    pub fn consume_cu(&mut self, delta: u64) {
        if self.remaining_cu >= delta {
            self.remaining_cu -= delta;
        } else {
            self.remaining_cu = 0;
        }
    }

    pub fn heap_monitor(&self) -> &HeapMonitor {
        &self.heap_monitor
    }

    pub fn heap_monitor_mut(&mut self) -> &mut HeapMonitor {
        &mut self.heap_monitor
    }

    /// Step hook called on each executed instruction.
    pub fn on_step(&mut self, pc: u64, opcode_byte: u8, current_remaining_cu: u64) {
        self.remaining_cu = current_remaining_cu;
        let opcode = Opcode::from(opcode_byte);

        match opcode {
            Opcode::Call => {
                let resolved = self.resolve_or_synthetic(pc);
                self.push_frame(pc, resolved);
            }
            Opcode::Exit => {
                self.pop_frame(pc);
            }
            _ => {}
        }
    }

    /// Explicitly push a call frame with symbol information.
    pub fn push_frame(&mut self, pc: u64, frame: ResolvedFrame) {
        let depth = self.active_stack.len();
        let mut call_frame = CallFrame::from_resolved(&frame, self.remaining_cu, depth);
        call_frame.heap_offset_bytes = self.heap_monitor.current_bytes();

        self.events.push(StepEvent::PushFrame {
            pc,
            frame,
            remaining_cu: self.remaining_cu,
            heap_offset: self.heap_monitor.current_bytes(),
        });

        self.active_stack.push(ActiveFrame { call_frame, pc });
    }

    /// Intercept Cross-Program Invocation (CPI) entry.
    pub fn on_cpi_start(&mut self, program_name: &str, instruction_name: Option<&str>) {
        let sym_name = format!("cpi:{}", program_name);
        self.cpi_program_stack.push(program_name.to_string());

        let frame = ResolvedFrame {
            symbol: sym_name.clone(),
            demangled: sym_name,
            location: None,
            category: FrameCategory::Cpi,
            is_inlined: false,
        };

        self.events.push(StepEvent::CpiStart {
            target_program: program_name.to_string(),
            instruction_name: instruction_name.map(|s| s.to_string()),
            remaining_cu: self.remaining_cu,
        });

        self.push_frame(0, frame);
    }

    /// Intercept Cross-Program Invocation (CPI) exit.
    pub fn on_cpi_end(&mut self) {
        self.cpi_program_stack.pop();
        self.events.push(StepEvent::CpiEnd {
            remaining_cu: self.remaining_cu,
        });
        self.pop_frame(0);
    }

    /// Explicitly pop the top call frame upon return or EXIT opcode.
    pub fn pop_frame(&mut self, pc: u64) {
        if let Some(mut active) = self.active_stack.pop() {
            active.call_frame.complete(self.remaining_cu);

            self.events.push(StepEvent::PopFrame {
                pc,
                remaining_cu: self.remaining_cu,
                heap_offset: self.heap_monitor.current_bytes(),
            });

            if let Some(parent) = self.active_stack.last_mut() {
                parent.call_frame.children.push(active.call_frame);
            } else {
                self.completed_roots.push(active.call_frame);
            }
        }
    }

    /// Intercept bump heap allocation `sol_alloc_free_` syscall.
    pub fn on_heap_alloc(&mut self, bytes: u64) -> u64 {
        let addr = self.heap_monitor.allocate(bytes);
        self.events.push(StepEvent::HeapAlloc {
            size: bytes,
            new_offset: self.heap_monitor.current_bytes(),
        });
        addr
    }

    /// Finalize trace and return full `ExecutionTrace`.
    pub fn finish(mut self) -> ExecutionTrace {
        // Unwind any unpopped frames
        while !self.active_stack.is_empty() {
            self.pop_frame(0);
        }

        let total_consumed = self.initial_cu.saturating_sub(self.remaining_cu);

        ExecutionTrace {
            instruction_name: self.instruction_name,
            initial_cu: self.initial_cu,
            final_cu: self.remaining_cu,
            total_cu_consumed: total_consumed,
            peak_heap_bytes: self.heap_monitor.peak_bytes(),
            root_frames: self.completed_roots,
            events: self.events,
        }
    }

    fn resolve_or_synthetic(&self, pc: u64) -> ResolvedFrame {
        if let Some(ref map) = self.symbol_map {
            if let Some(frame) = map.resolve_pc(pc) {
                return frame;
            }
        }

        let sym = format!("func_0x{:x}", pc);
        ResolvedFrame {
            symbol: sym.clone(),
            demangled: sym,
            location: None,
            category: FrameCategory::Native,
            is_inlined: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_tracer_call_tree() {
        let mut tracer = MolluskComputeTracer::new("deposit", 200_000);

        // Entrypoint
        tracer.on_step(0x1000, 0x85, 200_000); // Push func_0x1000
        tracer.on_heap_alloc(1024);

        // Nested call
        tracer.on_step(0x2000, 0x85, 180_000); // Push func_0x2000
        tracer.on_step(0x2000, 0x95, 150_000); // Pop func_0x2000 (used 30k CU)

        // Exit entrypoint
        tracer.on_step(0x1000, 0x95, 120_000); // Pop func_0x1000 (total 80k CU)

        let trace = tracer.finish();
        assert_eq!(trace.total_cu_consumed, 80_000);
        assert_eq!(trace.peak_heap_bytes, 1024);
        assert_eq!(trace.root_frames.len(), 1);
        assert_eq!(trace.root_frames[0].children.len(), 1);
        assert_eq!(trace.root_frames[0].children[0].delta_cu, 30_000);
    }
}
