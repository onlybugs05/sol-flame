# Solo Grants Application: `sol-flame`

**Applicant:** sol-flame Core Team  
**Project:** `sol-flame` (`cargo-cu-flame`)  
**Target Grant Tier:** Solo Grant ($720 USD / SOL equivalent)  
**Track:** Solana Developer Tooling & Infrastructure  
**Repository:** [https://github.com/sol-flame/sol-flame](https://github.com/sol-flame/sol-flame)

---

## 1. Project Summary & The Problem It Solves

### The Problem
Solana transactions have a hard 200,000 Compute Unit (CU) default limit (up to 1.4M with priority fees). When an instruction exhausts its budget (`Program failed to complete: exceeded maximum number of instructions allowed`), developers face an arduous debugging experience:
- Developers manually sprinkle `sol_log_compute_units()` prints throughout their codebase.
- They repeatedly build and deploy to a local validator, squinting at terminal log outputs.
- This approach burns extra CUs on logging itself, pollutes logs, and fails to pinpoint whether the bottleneck is Borsh deserialization, bump heap re-allocation, or an inner Cross-Program Invocation (CPI).

### The Solution: `sol-flame`
`sol-flame` (`cargo-cu-flame`) is a zero-overhead, in-process compute unit and memory flamegraph profiler for Solana sBPF programs:
1. **In-Process Tracing:** Uses `mollusk-svm` to execute instructions in-memory in milliseconds without running a local validator.
2. **DWARF Symbolication:** Resolves instruction pointer addresses back to exact source file and line numbers using `gimli`.
3. **Interactive Visual Flamegraphs:** Generates clickable, zoomable SVG flamegraphs with custom Solana color coding (Native logic, Borsh/Anchor deserialization, CPI sub-trees, and Heap warnings).
4. **32KB Heap Sentinel:** Traces bump allocator offsets against Solana's strict 32KB runtime limit, warning developers when memory crosses 75% (24KB).
5. **CI/CD Regression Guard:** Exports structured `cu-profile.json` and fails GitHub Actions pull requests if CU consumption regresses beyond a configurable threshold.

---

## 2. Working Prototype & Current Progress

We have built a fully functional working foundation in this repository:
- **`crates/sol-flame-core`**: DWARF v4/v5 symbol resolver, symbol interval map, Rust symbol demangler, execution trace models, 32KB heap sentinel, and folded stack aggregator.
- **`crates/sol-flame-render`**: SVG flamegraph generation via `inferno`, custom Solana palette post-processing, terminal summary tables, structured `cu-profile.json` export, and CI regression diff engine.
- **`crates/sol-flame-mollusk`**: In-process compute meter tracer, opcode step hooks (0x85 CALL / 0x95 EXIT), CPI boundary interceptor, and simulated scenario runner.
- **`crates/cargo-cu-flame`**: Developer CLI binary supporting `cargo cu-flame demo`, `--elf <path>`, `--instruction <name>`, `--ci-check <base>`, and `compare`.
- **`examples/`**: Working Anchor Escrow and Pinocchio zero-copy benchmark programs demonstrating a 57% CU reduction on flamegraphs.

---

## 3. Scope of Work & Deliverables for Solo Grant

With this grant, we will complete:
1. **Packaging & Release:** Publish `sol-flame` on crates.io and provide pre-built binaries for Linux, macOS (Apple Silicon/Intel), and Windows.
2. **Anchor & Solana CLI Integration:** Seamless detection of `target/deploy/*.so` binaries built by `anchor build` and `cargo build-sbf`.
3. **Automated GitHub Action (`sol-flame-action`):** A turnkey GitHub Action that comments flamegraph diffs directly onto GitHub pull requests.
4. **Documentation & Video Walkthrough:** Interactive documentation, quickstart tutorials, and sample CI workflows for Solana builders.

---

## 4. Milestone Schedule & Budget

| Milestone | Deliverable | Timeline | Amount |
| :--- | :--- | :--- | :--- |
| **M1: Working MVP & CLI (Completed)** | Working workspace, DWARF parser, SVG flamegraph renderer, in-process tracer, examples | Day 0 | Complete |
| **M2: Full Release & CI Action** | Crates.io publication, GitHub Action, and comprehensive documentation | 2 Weeks | $720 |
| **Total** | | | **$720 USD** |

---

## 5. Reviewer Verification Guide

Grant reviewers can verify this working implementation immediately:

```bash
# Clone and run test suite
cargo test

# Run Anchor Escrow demonstration
cargo run -p cargo-cu-flame -- demo --scenario anchor

# Run Pinocchio Zero-Copy demonstration
cargo run -p cargo-cu-flame -- demo --scenario pinocchio

# Run CI regression check between Anchor and Pinocchio
cargo run -p cargo-cu-flame -- compare target/sol-flame/base.json target/sol-flame/current.json

# Inspect generated interactive SVG in browser
open target/sol-flame/flamegraph.svg
```
