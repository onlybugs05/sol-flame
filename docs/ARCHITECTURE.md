# sol-flame: Comprehensive Systems Architecture & Technical Manual

## 1. System Overview

`sol-flame` is an in-process Compute Unit and memory flamegraph profiler for Solana smart contracts compiled to the Solana BPF (sBPF) architecture.

Unlike standard tracing techniques that rely on string logging (`sol_log_compute_units()`) or querying external RPC logs after validator execution, `sol-flame` hooks directly into the sBPF instruction execution and compute meter layer.

### System Boundaries & Execution Topology

```mermaid
flowchart TD
    Source[Rust Source Code / Anchor / Pinocchio] -->|cargo build-sbf| ELF[Compiled sBPF ELF Binary (*.so)]
    
    subgraph Host Process [In-Process sol-flame Execution]
        ELF --> DWARF[DWARF Symbol Engine<br/>gimli / object]
        DWARF --> IntervalTree[PC Interval Index<br/>Address -> Symbol + Line]
        
        ELF --> SVM[In-Memory SVM Execution<br/>mollusk-svm / tracer]
        
        SVM -->|0x85 CALL| StepHook[Opcode Step Hook]
        SVM -->|0x95 EXIT| StepHook
        SVM -->|Heap Bump 0x300000000| HeapSent[32KB Heap Sentinel]
        SVM -->|CPI invoke_signed| CpiTracker[CPI Sub-tree Tracker]
        
        StepHook --> CallTree[Call Frame Tree]
        HeapSent --> CallTree
        CpiTracker --> CallTree
        IntervalTree --> CallTree
        
        CallTree --> Aggregator[Folded Stack Aggregator]
        Aggregator --> SVG[Interactive SVG Flamegraph<br/>inferno + Solana Theme]
        Aggregator --> Terminal[Terminal Hotspot Table]
        Aggregator --> JSON[cu-profile.json Export]
    end
```

---

## 2. Core Execution Phases

### Phase 1: Binary & DWARF Debug Symbol Extraction
1. The developer compiles their Solana contract with debug symbols enabled (`cargo build-sbf` or `anchor build`).
2. `sol-flame-core` opens the ELF `.so` file via `object::File::parse`.
3. It loads the following DWARF sections into endian-aware slices:
   - `.debug_info`: Subprograms (`DW_TAG_subprogram`) and inlined call sites (`DW_TAG_inlined_subroutine`).
   - `.debug_line`: State machine parsing file names, line numbers, and column offsets.
   - `.debug_str`: Demangled and mangled symbol names (`DW_AT_linkage_name`, `DW_AT_name`).
4. An in-memory interval index (`BTreeMap<u64, SymbolRange>`) is constructed for sub-microsecond PC address lookups.

### Phase 2: In-Memory SVM Execution & Tracing
1. The program binary is loaded into the in-memory SVM runtime.
2. The initial compute budget is recorded:
   $$\text{Remaining CU}_0 = \text{meter.get_remaining()}$$
3. For each sBPF opcode executed:
   - **Opcode 0x85 (`CALL`):** A new `CallFrame` is pushed onto the active call stack, recording the current PC, resolved symbol, and entry compute meter value.
   - **Opcode 0x95 (`EXIT`):** The top `CallFrame` is popped, calculating the exact delta:
     $$\Delta\text{CU} = \text{Frame.entry\_cu} - \text{meter.get_remaining()}$$
   - **Heap Allocations:** Bump pointer adjustments starting at `0x300000000` are tracked. If peak memory exceeds 24,576 bytes (75% of 32KB), a high-memory warning is flagged.
   - **Cross-Program Invocations (CPI):** Transfers to external programs (e.g. `spl_token`, `system_program`) branch into dedicated `cpi:<PROGRAM_NAME>` sub-trees.

### Phase 3: Folded Stack Aggregation & Visualization
1. The hierarchical `CallFrame` tree is collapsed into folded stack lines:
   ```text
   entrypoint;anchor_escrow::process_instruction;borsh_deser 28540
   entrypoint;anchor_escrow::process_instruction;cpi:spl_token 18450
   entrypoint;anchor_escrow::process_instruction;math::calculate_deposit_fee 1200
   ```
2. The folded traces are piped into `inferno::flamegraph` to render an interactive, searchable SVG.
3. The SVG is enhanced with custom Solana color coding:
   - 🔵 **Blue:** Native business logic and math calculations.
   - 🟠 **Orange / Red:** Serialization & deserialization (`AccountDeserialize`, `try_from_slice`, Borsh).
   - 🟣 **Purple:** Cross-Program Invocations (`cpi:spl_token`, `cpi:system_program`).
   - 🟡 **Amber:** Memory allocation hotspots and high-water heap spikes.

---

## 3. Crate Architecture

The codebase is organized into four modular, decoupled crates:

```text
crates/
├── sol-flame-core/     # Foundation: DWARF symbol parsing, trace models, heap monitor
├── sol-flame-render/   # Presentation: SVG generation, terminal table, JSON export
├── sol-flame-mollusk/  # Runtime: SVM execution hooks, opcode tracer, scenario simulator
└── cargo-cu-flame/     # Interface: CLI binary, Cargo subcommand, CI comparison engine
```

### Component Responsibilities Matrix

| Crate | Responsibilities | Key Types |
| :--- | :--- | :--- |
| `sol-flame-core` | Low-level ELF parsing, DWARF traversal, symbol demangling, heap offset sentinel, folded stack aggregation. | `DwarfSymbolResolver`, `SymbolMap`, `HeapMonitor`, `FoldedStackAggregator`, `CallFrame` |
| `sol-flame-render` | SVG flamegraph generation with inferno, Solana palette mapping, ANSI terminal tables, JSON profile serialization. | `FlameRenderer`, `TerminalPresenter`, `CuProfile`, `CiRegressionReport` |
| `sol-flame-mollusk` | SVM execution step hooks, opcode CALL/EXIT interception, CPI sub-tree branching, scenario simulations. | `MolluskComputeTracer`, `SvmInstructionSimulator`, `MolluskHarness` |
| `cargo-cu-flame` | Developer CLI tool (`cargo cu-flame` & `sol-flame`), argument parsing, subcommand routing, CI regression evaluation. | `Cli`, `Commands`, `run_profile`, `run_demo`, `run_compare` |
