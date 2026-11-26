use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::trace::{CallFrame, ExecutionTrace};
use crate::symbols::FrameCategory;

/// Aggregated hotspot function for terminal tables and JSON exports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hotspot {
    pub symbol: String,
    pub cu: u64,
    pub percent: f64,
    pub source: Option<String>,
    pub category: FrameCategory,
}

/// Machine-readable summary of a profile run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSummary {
    pub instruction: String,
    pub total_cu: u64,
    pub peak_heap_bytes: u64,
    pub heap_warning: Option<String>,
    pub hotspots: Vec<Hotspot>,
}

/// Aggregates hierarchical execution traces into folded stack lines
/// suitable for inferno flamegraph generation.
#[derive(Debug, Default)]
pub struct FoldedStackAggregator {
    /// folded stack string -> total CU consumed
    folded_counts: HashMap<String, u64>,
    /// symbol -> cumulative CU
    symbol_cu_map: HashMap<String, (u64, Option<String>, FrameCategory)>,
    total_cu: u64,
    peak_heap: u64,
}

impl FoldedStackAggregator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ingest a completed `ExecutionTrace`
    pub fn aggregate_trace(&mut self, trace: &ExecutionTrace) {
        self.total_cu += trace.total_cu_consumed;
        if trace.peak_heap_bytes > self.peak_heap {
            self.peak_heap = trace.peak_heap_bytes;
        }

        for root in &trace.root_frames {
            let mut stack = Vec::new();
            self.visit_frame(root, &mut stack);
        }
    }

    fn visit_frame(&mut self, frame: &CallFrame, stack: &mut Vec<String>) {
        stack.push(frame.demangled.clone());

        // Update symbol map
        let entry = self
            .symbol_cu_map
            .entry(frame.demangled.clone())
            .or_insert((0, frame.source_location.clone(), frame.category));
        entry.0 += frame.delta_cu;

        // If this is a leaf or has self CU, record folded line
        let children_cu: u64 = frame.children.iter().map(|c| c.delta_cu).sum();
        let self_cu = if frame.delta_cu >= children_cu {
            frame.delta_cu - children_cu
        } else {
            frame.delta_cu
        };

        if self_cu > 0 {
            let folded_key = stack.join(";");
            *self.folded_counts.entry(folded_key).or_insert(0) += self_cu;
        }

        for child in &frame.children {
            self.visit_frame(child, stack);
        }

        stack.pop();
    }

    /// Add an explicit folded stack trace line (e.g. "entrypoint;borsh_deser 28540")
    pub fn add_folded_line(&mut self, stack_str: &str, count: u64) {
        *self.folded_counts.entry(stack_str.to_string()).or_insert(0) += count;
        self.total_cu += count;

        let parts: Vec<&str> = stack_str.split(';').collect();
        if let Some(leaf) = parts.last() {
            let cat = FrameCategory::classify(leaf);
            let entry = self.symbol_cu_map.entry(leaf.to_string()).or_insert((0, None, cat));
            entry.0 += count;
        }
    }

    /// Output lines in standard folded flamegraph format:
    /// `frame1;frame2;frame3 count`
    pub fn to_folded_lines(&self) -> Vec<String> {
        let mut lines: Vec<String> = self
            .folded_counts
            .iter()
            .map(|(stack, count)| format!("{} {}", stack, count))
            .collect();
        lines.sort();
        lines
    }

    /// Format folded lines into a single text block
    pub fn to_folded_text(&self) -> String {
        self.to_folded_lines().join("\n")
    }

    /// Calculate top hotspots sorted by CU descending
    pub fn hotspots(&self, limit: usize) -> Vec<Hotspot> {
        let mut list: Vec<Hotspot> = self
            .symbol_cu_map
            .iter()
            .map(|(sym, (cu, source, cat))| {
                let percent = if self.total_cu > 0 {
                    (*cu as f64 / self.total_cu as f64) * 100.0
                } else {
                    0.0
                };
                Hotspot {
                    symbol: sym.clone(),
                    cu: *cu,
                    percent: (percent * 100.0).round() / 100.0,
                    source: source.clone(),
                    category: *cat,
                }
            })
            .collect();

        list.sort_by(|a, b| b.cu.cmp(&a.cu));
        list.truncate(limit);
        list
    }

    /// Produce `ProfileSummary` matching `ARCHITECTURE.md`
    pub fn summary(&self, instruction: &str) -> ProfileSummary {
        let heap_warn = if self.peak_heap >= crate::heap::SOLANA_HEAP_WARN_THRESHOLD {
            Some(format!(
                "Heap usage reached {} bytes (>= 24KB limit).",
                self.peak_heap
            ))
        } else {
            None
        };

        ProfileSummary {
            instruction: instruction.to_string(),
            total_cu: self.total_cu,
            peak_heap_bytes: self.peak_heap,
            heap_warning: heap_warn,
            hotspots: self.hotspots(10),
        }
    }

    pub fn total_cu(&self) -> u64 {
        self.total_cu
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_folded_aggregation() {
        let mut agg = FoldedStackAggregator::new();
        agg.add_folded_line("entrypoint;process_instruction;borsh_deser", 28540);
        agg.add_folded_line("entrypoint;process_instruction;cpi:spl_token", 18450);
        agg.add_folded_line("entrypoint;process_instruction;math", 1200);

        let lines = agg.to_folded_lines();
        assert_eq!(lines.len(), 3);
        assert_eq!(agg.total_cu(), 48190);

        let hotspots = agg.hotspots(5);
        assert_eq!(hotspots[0].symbol, "borsh_deser");
        assert_eq!(hotspots[0].cu, 28540);
        assert!(hotspots[0].percent > 59.0);
    }
}
