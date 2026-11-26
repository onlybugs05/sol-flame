use crate::tracer::MolluskComputeTracer;

/// Integration adapter for `mollusk-svm`.
pub struct MolluskHarness {
    pub program_id: [u8; 32],
}

impl Default for MolluskHarness {
    fn default() -> Self {
        Self::new([0u8; 32])
    }
}

impl MolluskHarness {
    pub fn new(program_id: [u8; 32]) -> Self {
        Self { program_id }
    }

    /// Create a tracer bound to an execution run.
    pub fn create_tracer(&self, instruction_name: &str, compute_budget: u64) -> MolluskComputeTracer {
        MolluskComputeTracer::new(instruction_name, compute_budget)
    }
}
