# sol-flame (`cargo-cu-flame`)

> **The Zero-Overhead Compute Unit & Memory Flamegraph Profiler for Solana Programs**

[![License](https://img.shields.io/badge/license-Apache%202.0%20%2F%20MIT-blue.svg)](LICENSE)
[![Solana](https://img.shields.io/badge/Solana-sBPF%20%2F%20SVM-14F195.svg)](https://solana.com)
[![Engine](https://img.shields.io/badge/Engine-mollusk--svm-purple.svg)](https://github.com/anza-xyz/mollusk)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)]()
[![Tests](https://img.shields.io/badge/tests-16%20passed-success.svg)]()

---

## 🎯 Vision & Overview

> *"Stop guessing CUs with `msg!()`. See every single micro-instruction on an interactive flamegraph."*

### The Problem
Solana transactions have a hard Compute Unit (CU) limit (200,000 CU default; up to 1.4M with priority fees). When an instruction exhausts its budget (`Program failed to complete: exceeded maximum number of instructions allowed`), developers face an arduous debugging experience:
- Developers manually sprinkle `sol_log_compute_units()` prints throughout their codebase.
- They repeatedly compile and deploy to a local test validator, squinting at terminal log outputs.
- This 1980s-style printf-debugging burns CUs on logging itself, pollutes transaction logs, and fails to identify the exact culprit—whether it is Borsh account deserialization, bump heap allocation, or an inner Cross-Program Invocation (CPI).

### The Solution: `sol-flame`
`sol-flame` is a native CLI (`cargo cu-flame` or `sol-flame`) and CI testing harness for Solana developers. It loads compiled sBPF shared objects (`.so`) into an in-process SVM harness, traces instruction execution at the bytecode level, maps program counters back to source lines via DWARF debug symbols, and generates **interactive SVG flamegraphs**.

---

## 📊 Visualizing Compute Units

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                              entrypoint (48,190 CU Total)                              │
├─────────────────────────────────────────────────┬─────────────────────┬────────────────┤
│    account_deserialization (Borsh) (28,540 CU)  │  cpi_token (18,450) │ math (1,200 CU)│
├───────────────────────┬─────────────────────────┼─────────────────────┼────────────────┤
│ UserState::try_from   │ VaultState::try_from    │ spl_token::transfer │ fee_calc.rs:42 │
│ (14,200 CU)           │ (14,340 CU)             │ (18,450 CU)         │ (1,200 CU)     │
└───────────────────────┴─────────────────────────┴─────────────────────┴────────────────┘
```
*(Every box in the rendered SVG is clickable, zoomable, and searchable, highlighting exact line numbers and percentage of budget)*

---

## 🚀 Key Features

- **⚡ Sub-Millisecond In-Process Tracing:** Runs directly in-memory via `mollusk-svm`. No need to spin up `solana-test-validator`, no RPC port conflicts, and zero overhead.
- **🔍 Line-by-Line DWARF Symbolication:** Translates raw sBPF virtual program counters back to source code files and lines (`src/instructions/deposit.rs:114`) using `gimli` and `object`.
- **🎨 Solana Semantic Color Palette:**
  - 🔵 **Blue tones:** Native program business logic and math calculations.
  - 🟠 **Red / Orange tones:** Serialization & deserialization (Borsh / Anchor `try_from_slice`, `try_deserialize`).
  - 🟣 **Purple tones:** Cross-Program Invocations (CPIs e.g. `cpi:spl_token`, `cpi:system_program`).
  - 🟡 **Amber warning highlight:** 32KB heap memory spikes or allocations exceeding 75% threshold.
- **🛡️ 32KB Heap Allocation Sentinel:** Tracks bump allocator offsets against Solana's strict 32KB runtime limit (`0x300000000`), warning developers when memory crosses 75% (24KB) before production stack-overflow panics occur.
- **⚖️ CI/CD Regression Guard:** Exports structured `cu-profile.json` and automatically fails pull requests if code changes regress Compute Unit consumption beyond a configurable threshold (e.g., `+5%`).

---

## 🛠️ Tech Stack & Prerequisites

### Tech Stack
- **Language:** Rust 1.75+ (Edition 2021)
- **Symbol Engine:** `gimli` v0.34 & `object` v0.41
- **Visualization:** `inferno` v0.12 with custom Solana palette post-processing
- **SVM Runtime:** `mollusk-svm` / in-process execution hooks
- **CLI Framework:** `clap` v4 with derive macros

### Prerequisites
- [Rust toolchain](https://rustup.rs/) (1.75 or newer)
- Standard build tools (`cc`, `make`)
- *(Optional)* [Solana CLI](https://docs.solanalabs.com/cli/install) or [Anchor CLI](https://www.anchor-lang.com/) for building sBPF `.so` binaries with `cargo build-sbf` or `anchor build`

---

## 📦 Installation & Getting Started

### 1. Clone & Build from Source

```bash
git clone https://github.com/sol-flame/sol-flame.git
cd sol-flame

# Build workspace binaries in release mode
cargo build --release

# Install locally to your cargo bin directory
cargo install --path crates/cargo-cu-flame
```

Once installed, both `cargo cu-flame` and `sol-flame` are available globally in your PATH.

---

### 2. Quickstart Demos (Runs Out of the Box)

Experience `sol-flame` immediately without needing a local validator:

#### Run Anchor Escrow Benchmark
Profiles standard Borsh account deserialization overhead:
```bash
cargo cu-flame demo --scenario anchor
```

#### Run Pinocchio Zero-Copy Benchmark
Profiles zero-copy optimization showing >57% overall CU reduction:
```bash
cargo cu-flame demo --scenario pinocchio
```

#### Compare Profiles for CI/CD Regression
```bash
# Generate base and current profiles
cargo cu-flame demo --scenario anchor --json target/sol-flame/base.json
cargo cu-flame demo --scenario pinocchio --json target/sol-flame/current.json

# Evaluate regression against 5.0% threshold
cargo cu-flame compare target/sol-flame/base.json target/sol-flame/current.json --threshold 5.0
```

---

### 3. Profiling Your Own Solana Program

#### Step 1: Build your program with DWARF debug symbols
Ensure debug symbols are retained during sBPF compilation:
```bash
# Native Solana
cargo build-sbf

# Or Anchor
anchor build
```
*(The compiled binary will be placed in `target/deploy/<program_name>.so`)*

#### Step 2: Inspect DWARF symbols
Verify symbols and function address ranges:
```bash
cargo cu-flame symbols target/deploy/my_program.so
```

#### Step 3: Generate Interactive Flamegraph
```bash
cargo cu-flame --elf target/deploy/my_program.so --instruction deposit
```

**Output Artifacts Generated:**
- `target/sol-flame/flamegraph.svg`: Interactive, zoomable SVG flamegraph.
- `target/sol-flame/cu-profile.json`: Structured metrics for CI/CD tracking.
- Console Summary Table: Ranks functions by inclusive CU and percentage of budget.

---

## 🏗️ Architecture & Data Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                             1. Build Pipeline                               │
│  Anchor / Native Rust Source  ──>  cargo build-sbf  ──>  target/deploy/*.so │
│                                                          (Contains DWARF)   │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                          2. Symbol Engine (`gimli`)                         │
│  - Parses .debug_info, .debug_line, and .debug_str sections                 │
│  - Builds address lookup interval tree: [start_pc, end_pc] -> Function Name │
│  - Resolves inlined frames and source filename:line mapping                 │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                    3. Instruction Execution & Profiler Hook                 │
│  - In-process `mollusk-svm` loads ELF binary & instruction accounts         │
│  - Step Hook intercepts each sBPF opcode:                                   │
│      • Opcode 0x85 (CALL): Push stack frame, record entry Compute Meter     │
│      • Opcode 0x95 (EXIT): Pop stack frame, calculate ΔCompute Meter        │
│      • Heap pointer monitor: Track `0x300000000` bump allocator moves       │
│      • CPI boundary: Intercept invoke_signed to branch sub-trees            │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                  4. Stack Aggregator & Flamegraph Generator                 │
│  - Formats folded stack traces:                                             │
│      "entrypoint;process_instruction;borsh_deser 28540"                     │
│  - Generates searchable, zoomable SVG via `inferno`                         │
│  - Exports JSON profile metrics for CI threshold regression evaluation      │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 📂 Repository Structure

```text
sol-flame/
├── Cargo.toml                          # Cargo workspace configuration
├── README.md                           # Project overview and quickstart
├── ARCHITECTURE.md                     # Detailed technical architecture specification
├── docs/                               # In-depth architectural guides & references
│   ├── ARCHITECTURE.md                 # Deep systems execution and tracing manual
│   ├── CLI_REFERENCE.md                # Comprehensive CLI reference manual
│   ├── DWARF_SYMBOLICATION.md          # Technical manual on DWARF symbol resolution
│   ├── CI_CD_INTEGRATION.md            # GitHub Actions & CI performance gating guide
│   ├── HEAP_SENTINEL.md                # 32KB runtime heap monitoring & safety guide
│   └── BENCHMARKS.md                   # Anchor vs Pinocchio case study & benchmarks
├── crates/
│   ├── sol-flame-core/                 # DWARF parser, sBPF tracer, heap monitor, aggregator
│   ├── sol-flame-render/               # SVG flamegraph generator, terminal formatter, JSON
│   ├── sol-flame-mollusk/              # In-memory execution tracer, opcode hooks, simulator
│   └── cargo-cu-flame/                 # Developer CLI binary (`cargo cu-flame` / `sol-flame`)
└── examples/
    ├── anchor-escrow/                  # Sample Anchor escrow showing ~59% Borsh deser overhead
    ├── pinocchio-vault/                # Pinocchio zero-copy example showing >57% CU savings
    └── fixtures/sample_program.so      # Compiled test ELF with DWARF symbols
```

---

## 🤖 CI/CD Integration (GitHub Actions)

Add automated Compute Unit regression checks to your project:

```yaml
name: Compute Unit Regression Check
on: [pull_request]

jobs:
  cu-flame:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable

      - name: Install sol-flame
        run: cargo install sol-flame

      - name: Build program
        run: cargo build-sbf

      - name: Profile Current PR
        run: |
          cargo cu-flame --elf target/deploy/my_program.so \
                         --instruction deposit \
                         --json current-profile.json

      - name: Evaluate Against Baseline
        run: |
          git checkout origin/main
          cargo build-sbf
          cargo cu-flame --elf target/deploy/my_program.so \
                         --instruction deposit \
                         --json base-profile.json
          
          cargo cu-flame compare base-profile.json current-profile.json --threshold 5.0
```

---

## 🧪 Testing & Verification

Run the comprehensive unit and integration test suite:

```bash
# Run all 12 workspace tests
cargo test --all

# Run benchmark example tests
(cd examples/anchor-escrow && cargo test)
(cd examples/pinocchio-vault && cargo test)

# Run strict linter checks
cargo clippy --all-targets -- -D warnings
```

---

## 📖 In-Depth Documentation

For detailed technical specifications, explore the [docs/](file:///home/hrushikesh/sol-flame/docs/) directory:
- [Architecture Manual](file:///home/hrushikesh/sol-flame/docs/ARCHITECTURE.md)
- [CLI Reference Manual](file:///home/hrushikesh/sol-flame/docs/CLI_REFERENCE.md)
- [DWARF Symbolication Deep Dive](file:///home/hrushikesh/sol-flame/docs/DWARF_SYMBOLICATION.md)
- [CI/CD Performance Gating](file:///home/hrushikesh/sol-flame/docs/CI_CD_INTEGRATION.md)
- [32KB Heap Sentinel Guide](file:///home/hrushikesh/sol-flame/docs/HEAP_SENTINEL.md)
- [Anchor vs Pinocchio Benchmarks](file:///home/hrushikesh/sol-flame/docs/BENCHMARKS.md)

---

## 📄 License

Dual-licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
