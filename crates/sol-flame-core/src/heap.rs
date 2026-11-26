use serde::{Deserialize, Serialize};

/// Solana standard sBPF 32KB heap starting virtual address: 0x300000000
pub const SOLANA_HEAP_START: u64 = 0x300000000;

/// Solana runtime max heap size: 32KB = 32,768 bytes
pub const SOLANA_HEAP_SIZE: u64 = 32 * 1024;

/// 75% warning threshold: 24KB = 24,576 bytes
pub const SOLANA_HEAP_WARN_THRESHOLD: u64 = 24 * 1024;

/// Alert raised when program heap memory exceeds safety bounds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeapWarning {
    pub current_bytes: u64,
    pub threshold_bytes: u64,
    pub percent_used: u32,
    pub message: String,
}

/// Solana 32KB Heap Sentinel.
///
/// Monitors bump allocator offsets and detects impending stack/heap collision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeapMonitor {
    start_addr: u64,
    max_size: u64,
    current_offset: u64,
    peak_offset: u64,
    allocation_count: u64,
}

impl Default for HeapMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl HeapMonitor {
    pub fn new() -> Self {
        Self {
            start_addr: SOLANA_HEAP_START,
            max_size: SOLANA_HEAP_SIZE,
            current_offset: 0,
            peak_offset: 0,
            allocation_count: 0,
        }
    }

    /// Record a bump allocation of `bytes` bytes.
    pub fn allocate(&mut self, bytes: u64) -> u64 {
        let addr = self.start_addr + self.current_offset;
        self.current_offset += bytes;
        if self.current_offset > self.peak_offset {
            self.peak_offset = self.current_offset;
        }
        self.allocation_count += 1;
        addr
    }

    /// Reset or set current bump offset (e.g. from syscall trace).
    pub fn record_offset(&mut self, offset: u64) {
        self.current_offset = offset;
        if offset > self.peak_offset {
            self.peak_offset = offset;
        }
    }

    pub fn current_bytes(&self) -> u64 {
        self.current_offset
    }

    pub fn peak_bytes(&self) -> u64 {
        self.peak_offset
    }

    pub fn allocation_count(&self) -> u64 {
        self.allocation_count
    }

    pub fn percent_used(&self) -> f64 {
        (self.peak_offset as f64 / self.max_size as f64) * 100.0
    }

    /// Check if peak memory crossed the 75% heap threshold (24KB).
    pub fn check_warning(&self) -> Option<HeapWarning> {
        if self.peak_offset >= SOLANA_HEAP_WARN_THRESHOLD {
            let percent = (self.percent_used().round()) as u32;
            Some(HeapWarning {
                current_bytes: self.peak_offset,
                threshold_bytes: SOLANA_HEAP_WARN_THRESHOLD,
                percent_used: percent,
                message: format!(
                    "Heap peak reached {} bytes ({}% of 32KB limit). High risk of memory overflow!",
                    self.peak_offset, percent
                ),
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heap_monitor_allocations() {
        let mut heap = HeapMonitor::new();
        assert_eq!(heap.peak_bytes(), 0);

        let ptr1 = heap.allocate(4096);
        assert_eq!(ptr1, SOLANA_HEAP_START);
        assert_eq!(heap.current_bytes(), 4096);
        assert_eq!(heap.peak_bytes(), 4096);
        assert!(heap.check_warning().is_none());

        // Allocate up to 26KB (crossing 24KB threshold)
        heap.allocate(22 * 1024);
        assert_eq!(heap.peak_bytes(), 26 * 1024);
        assert!(heap.check_warning().is_some());

        let warning = heap.check_warning().unwrap();
        assert!(warning.percent_used >= 80);
        assert!(warning.message.contains("High risk of memory overflow"));
    }
}
