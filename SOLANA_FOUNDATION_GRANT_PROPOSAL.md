# Solana Foundation Grant Proposal: `sol-flame`

**Project Title:** `sol-flame`: Zero-Overhead Compute Unit & Memory Flamegraph Profiler for Solana Programs  
**Requested Amount:** $18,000 USD  
**Category:** Developer Tooling & Public Good Infrastructure  
**Project Lead:** sol-flame Core Team  
**Repository:** [https://github.com/sol-flame/sol-flame](https://github.com/sol-flame/sol-flame)

---

## 1. Executive Summary

Solana's high-performance architecture relies on predictable Compute Unit (CU) consumption. Every deployed program must operate within strict transaction budgets (200k standard, 1.4M max). Today, analyzing CU bottlenecks and memory consumption is one of the most frustrating aspects of Solana development. Developers resort to primitive `sol_log_compute_units()` statements, parsing stdout logs from local validators.

`sol-flame` is an open-source, developer-first profiling suite that bridges the sBPF bytecode layer, the in-memory SVM runtime (`mollusk-svm`), and modern visualization tools (`inferno`). It provides instantaneous, line-by-line CU profiling, 32KB heap allocation tracking, and interactive SVG flamegraphs without launching external validators.

---

## 2. Technical Architecture

`sol-flame` operates across three key layers:
1. **ELF & DWARF Inspection Layer:** Uses `gimli` and `object` to parse `.debug_info` and `.debug_line`, mapping virtual program counters back to Rust source lines and inlined call chains.
2. **In-Memory SVM Execution Layer:** Intercepts `mollusk-svm` execution loops, recording entry and exit compute meter states on opcode calls (0x85 CALL / 0x95 EXIT), tracking Cross-Program Invocations (CPI), and monitoring 32KB heap bump offsets.
3. **Visualization & CI/CD Layer:** Collapses execution traces into folded stacks, generating interactive, zoomable SVG flamegraphs with custom Solana color themes, and outputting `cu-profile.json` for automated CI regression gates.

---

## 3. Current Implementation Status

A fully working, tested MVP is already implemented in the repository:
- Core DWARF symbolication and interval range lookup (`crates/sol-flame-core`).
- 32KB Solana heap sentinel with 75% overflow detection (`crates/sol-flame-core`).
- Folded stack aggregator and hotspot calculator (`crates/sol-flame-core`).
- SVG flamegraph generator with Solana theme (`crates/sol-flame-render`).
- Formatted ANSI terminal summary table and CI JSON exporter (`crates/sol-flame-render`).
- In-memory compute tracer with CPI sub-tree branching (`crates/sol-flame-mollusk`).
- CLI tool `cargo-cu-flame` and `sol-flame` (`crates/cargo-cu-flame`).
- Verified benchmarks comparing Anchor Borsh deserialization vs Pinocchio zero-copy views (`examples/`).

---

## 4. Project Roadmap & Milestones

### Milestone 1: Core Engine & CLI MVP (Completed)
- Working multi-crate Rust workspace.
- DWARF parsing, execution tracing, SVG generation, and CLI interface.
- *Status:* **Delivered & verified with unit tests.**

### Milestone 2: Native Mollusk & Agave SVM Dynamic Tracing ($6,000)
- Deep integration with `mollusk-svm` 0.17+ execution callbacks.
- Support for complex account fixtures, sysvars, and SPL Token CPIs.
- Automatic binary symbol extraction from `cargo build-sbf` and `anchor build`.
- *Timeline:* Months 1-2.

### Milestone 3: Web-Based Interactive Profiler UI ($6,000)
- Standalone local web UI (`cargo cu-flame --serve`) with live instruction call-trees, heap waterfall graphs, and comparative side-by-side PR diffs.
- Interactive filtering by program category (deserialization, business logic, CPI).
- *Timeline:* Months 3-4.

### Milestone 4: CI/CD GitHub Action & Ecosystem Integration ($6,000)
- Official `sol-flame-action` GitHub Marketplace Action.
- Automated PR comment bot rendering CU delta graphs and alerting on threshold regressions.
- Integration guides for Anchor, Shank, and Pinocchio developers.
- *Timeline:* Months 5-6.

---

## 5. Budget Allocation

| Area | Description | Budget |
| :--- | :--- | :--- |
| **SVM Runtime Integration** | mollusk-svm instruction-level callback hooks & sBPF debugger | $6,000 |
| **Web UI & Visualizer** | Interactive web profiler, heap timeline, and side-by-side diffs | $6,000 |
| **CI Automation & Community** | GitHub Action, documentation, benchmarks, community workshops | $6,000 |
| **Total** | | **$18,000 USD** |

---

## 6. Reviewer Verification Instructions

Reviewers can clone and run the working MVP directly:

```bash
git clone https://github.com/sol-flame/sol-flame
cd sol-flame

# Run tests
cargo test

# Run Anchor Escrow demonstration
cargo run -p cargo-cu-flame -- demo --scenario anchor

# Run Pinocchio Zero-Copy demonstration
cargo run -p cargo-cu-flame -- demo --scenario pinocchio

# Inspect the generated SVG
xdg-open target/sol-flame/flamegraph.svg
```
