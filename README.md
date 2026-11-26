# sol-flame (`cargo-cu-flame`)
> **The Zero-Overhead Compute Unit & Memory Flamegraph Profiler for Solana Programs**

[![License](https://img.shields.io/badge/license-Apache%202.0%20%2F%20MIT-blue.svg)](LICENSE)
[![Solana](https://img.shields.io/badge/Solana-sBPF%20%2F%20SVM-14F195.svg)](https://solana.com)
[![Mollusk](https://img.shields.io/badge/Engine-mollusk--svm-purple.svg)](https://github.com/anza-xyz/mollusk)

---

## 🎯 The Motto & Vision
> *"Stop guessing CUs with `msg!()`. See every single micro-instruction on an interactive flamegraph."*

**The Problem:**
Solana programs have a hard Compute Unit limit (200,000 CU standard; 1.4M max with priority fees). When transactions fail with `Program failed to complete: exceeded maximum number of instructions allowed`, developers are left in the dark.
Currently, developers sprinkle `sol_log_compute_units()` across their code, compile, deploy to a local validator, and squint at terminal logs. This 1980s-style printf-debugging burns CUs, pollutes logs, and fails to identify the exact culprit—whether it is Borsh account deserialization, memory re-allocation, or an inner Cross-Program Invocation (CPI).

**The Solution:**
`sol-flame` is a native CLI (`cargo cu-flame`) and CI testing harness. It loads compiled sBPF shared objects (`.so`) into an in-process SVM harness (`mollusk-svm`), traces instruction execution at the bytecode level, maps program counters back to source lines via DWARF debug symbols, and generates **interactive SVG flamegraphs**.

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

1. **Sub-Millisecond Execution:**
   Runs directly in-process via `mollusk-svm`. No need to spin up `solana-test-validator`, no RPC port conflicts, and no hanging background processes.
2. **Line-by-Line DWARF Symbolication:**
   Translates raw virtual memory addresses directly back to your source code lines (`src/instructions/deposit.rs:114`) using `gimli`.
3. **Deep CPI Transparency:**
   Tracks nested Cross-Program Invocations (SPL Token, System Program, Token-2022) with distinct visual hierarchies.
4. **32KB Heap Monitor:**
   Profiles heap allocation peaks to warn developers when memory fragmentation threatens stack overflow boundaries.
5. **CI Regression Guard:**
   Integrates into GitHub Actions to fail PRs if a code change introduces more than $X\%$ Compute Unit regression.

---

## 🛠️ CLI Quickstart

### Installation
```bash
cargo install sol-flame
```

### Basic Profile
```bash
# Build with debug symbols
cargo build-sbf

# Generate interactive flamegraph for the 'deposit' instruction
cargo cu-flame --instruction deposit
```

### Profile Invariant Fuzzing
```bash
cargo cu-flame --instruction swap --fuzz 1000 --max-cu 120000
```

### Output Artifacts
- **Terminal Summary:** Detailed table of top CU-consuming functions.
- **Interactive SVG:** `target/sol-flame/flamegraph.svg` (automatically opens in browser).
- **Flamechart JSON:** Machine-readable trace for CI regression tracking.

---

## 📦 Project Structure

```text
sol-flame/
├── Cargo.toml
├── README.md                           # This document
├── ARCHITECTURE.md                     # Systems design & execution hooking spec
├── SOLO_GRANTS_APPLICATION.md          # Ready application for sologrants.com ($720)
├── SOLANA_FOUNDATION_GRANT_PROPOSAL.md # $18,000 public good proposal
├── crates/
│   ├── sol-flame-core/                 # DWARF parser, sBPF tracer, stack aggregator
│   ├── sol-flame-mollusk/              # In-memory execution hook & CU accounting
│   ├── sol-flame-render/               # SVG flamegraph generator (via inferno)
│   └── cargo-cu-flame/                 # Developer CLI binary
└── examples/
    ├── anchor-escrow/                  # Sample Anchor project with CU benchmarks
    └── pinocchio-vault/                # Pinocchio zero-copy comparison example
```
