use sol_flame_core::heap::{HeapMonitor, HeapWarning};

/// Mollusk runtime 32KB heap hook adapter.
pub struct MolluskHeapSentinel {
    inner: HeapMonitor,
}

impl Default for MolluskHeapSentinel {
    fn default() -> Self {
        Self::new()
    }
}

impl MolluskHeapSentinel {
    pub fn new() -> Self {
        Self {
            inner: HeapMonitor::new(),
        }
    }

    pub fn monitor(&self) -> &HeapMonitor {
        &self.inner
    }

    pub fn monitor_mut(&mut self) -> &mut HeapMonitor {
        &mut self.inner
    }

    pub fn on_syscall_alloc(&mut self, size: u64) -> u64 {
        self.inner.allocate(size)
    }

    pub fn peak_bytes(&self) -> u64 {
        self.inner.peak_bytes()
    }

    pub fn check_warning(&self) -> Option<HeapWarning> {
        self.inner.check_warning()
    }
}
