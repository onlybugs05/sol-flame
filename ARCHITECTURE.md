# Systems Architecture & Technical Specification: `sol-flame`

## 1. System Overview & Data Flow

`sol-flame` bridges three distinct layers:
1. **The Solana sBPF Binary & DWARF Layer:** Static inspection of the ELF binary.
2. **The In-Memory Execution Layer:** Hooked `mollusk-svm` / `rbpf` interpreter step loop.
3. **The Call-Stack & Visualizer Layer:** Folded stack frame aggregation and SVG rendering via `inferno`.

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
│      • Heap pointer monitor: Track `HEAP_START` offset moves                │
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

## 2. Low-Level Component Breakdown

### A. DWARF Symbol Resolver (`sol-flame-core/src/symbols.rs`)
Solana sBPF code compiled with standard toolchains embeds standard DWARF v4/v5 debugging symbols.
- **Challenge:** Rust heavily inlines functions in release mode (`opt-level = 3`). A single virtual PC address might represent 3 nested inlined function calls.
- **Solution:** We use `gimli::read::Dwarf` to walk the subprogram DIEs (Debugging Information Entries). When resolving an instruction pointer:
  1. We locate the contiguous address range enclosing the PC.
  2. We unwind the `DW_TAG_inlined_subroutine` ancestry chain.
  3. We return a full vector of inlined stack frames: `vec![OuterCaller, InlinedHelper, Callee]`.

### B. In-Memory Execution & CU Metering (`sol-flame-mollusk/src/tracer.rs`)
Instead of parsing log strings from RPC, `sol-flame` hooks the execution loop directly inside the in-memory SVM runtime:
- **Compute Meter Tap:** Solana VM tracks remaining instructions through `ComputeMeter`.
  At the start of instruction:
  $$\text{Initial CU} = \text{meter.get_remaining()}$$
- **Instruction Cycle Accounting:**
  For each executed basic block or call frame $i$:
  $$\Delta\text{CU}_i = \text{meter.remaining}_{t} - \text{meter.remaining}_{t+1}$$
- **Cross-Program Invocations (CPI):**
  When a program executes `sol_invoke` or `sol_invoke_signed`, the SVM transfers control to a separate loaded program ID (e.g., `TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA`). `sol-flame` captures this event, tagging the call-stack boundary with `cpi:<PROGRAM_NAME>`, providing clear visual distinction between native program computation and third-party CPI overhead.

### C. 32KB Heap Allocation Sentinel (`sol-flame-mollusk/src/heap.rs`)
Solana programs possess a strict 32KB runtime heap starting at virtual address `0x300000000`.
- Programs using standard Borsh deserialization or dynamic vectors allocate from this bump allocator.
- `sol-flame` intercepts `sol_alloc_free_` syscalls and traces the peak high-water mark of the bump pointer.
- If heap usage exceeds 24KB (75% threshold), `sol-flame` tags the flamegraph with a visual yellow/red memory alert to warn developers before production stack-overflow panics occur.

### D. Flamegraph Visualizer (`sol-flame-render/src/flame.rs`)
- Uses `inferno::flamegraph` with custom color palettes tailored for Solana:
  - **Blue tones:** Native program business logic and math.
  - **Red / Orange tones:** Serialization & deserialization (Borsh / Anchor `try_from_slice`).
  - **Purple tones:** Cross-Program Invocations (CPI).
  - **Yellow warning highlight:** High-water heap memory spikes.

---

## 3. CI/CD Integration Architecture (`sol-flame-action`)

For continuous integration, `cargo cu-flame` outputs a structured `cu-profile.json`:
```json
{
  "instruction": "deposit",
  "total_cu": 48190,
  "peak_heap_bytes": 4300,
  "hotspots": [
    {
      "symbol": "anchor_lang::AccountDeserialize::try_deserialize",
      "cu": 28540,
      "percent": 59.23,
      "source": "anchor-lang/src/lib.rs:18"
    }
  ]
}
```
The GitHub Action compares the profile against the target base branch (`main`). If total CU consumption increases by more than the user-specified tolerance (e.g. `5%`), the Action fails and uploads the comparative flamegraph diff as an artifact.
