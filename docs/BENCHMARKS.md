# Benchmarks & Case Study: Anchor vs. Pinocchio

To demonstrate `sol-flame`'s profiling fidelity, the repository includes two contrasting real-world Solana program architectures in the `examples/` directory.

---

## 1. Architecture Comparison

### A. Traditional Anchor Escrow (`examples/anchor-escrow`)
- Uses standard Borsh deserialization via `anchor_lang::AccountDeserialize`.
- Deserializes full account structs (`UserState`, `VaultState`) into dynamically allocated heap structures.
- Executes an SPL Token CPI transfer.

### B. Pinocchio Zero-Copy Vault (`examples/pinocchio-vault`)
- Uses `Pinocchio` zero-copy memory mapping (`VaultZeroCopy::load`).
- Directly references raw account byte slices without heap allocation.
- Executes an SPL Token-2022 CPI transfer.

---

## 2. Profiler Benchmark Results

The benchmark profiles were generated using:
```bash
cargo cu-flame demo --scenario anchor --json target/sol-flame/anchor.json
cargo cu-flame demo --scenario pinocchio --json target/sol-flame/pinocchio.json
cargo cu-flame compare target/sol-flame/anchor.json target/sol-flame/pinocchio.json
```

### Comparative Summary Table

| Metric | Anchor Escrow | Pinocchio Vault | Delta | Optimization |
| :--- | :--- | :--- | :--- | :--- |
| **Total Instruction CU** | **48,190 CU** | **20,550 CU** | **-27,640 CU** | **-57.36% Overall** |
| **Account Deserialization** | 28,540 CU (59.2%) | 620 CU (3.0%) | -27,920 CU | **-97.8% Account Overhead** |
| **Peak Heap Allocation** | **4,300 bytes** | **0 bytes** | -4,300 bytes | **100% Heap Free** |
| **CPI Token Transfer** | 18,450 CU (38.3%) | 18,450 CU (89.8%) | 0 CU | Identical CPI cost |
| **Math & Logic** | 1,200 CU (2.5%) | 1,480 CU (7.2%) | +280 CU | Constant product curve |

---

## 3. Analysis & Key Insights

1. **Borsh Deserialization is the #1 Culprit:** In the standard Anchor instruction, **59.23% of the entire transaction budget** was consumed simply reading the two accounts into memory!
2. **Zero-Copy Eliminates 97.8% of Deserialization Cost:** By using `Pinocchio`'s zero-copy reference mapping, account loading dropped from 28,540 CU to 620 CU.
3. **Heap Elimination:** Anchor required 4,300 bytes of bump allocator memory. Pinocchio required **0 bytes**, leaving the entire 32KB heap available for complex transaction logic.
4. **CPI Transparency:** `sol-flame` clearly separated the 18,450 CU consumed by the SPL Token CPI, allowing developers to see that CPI overhead accounted for 89.8% of the remaining Pinocchio instruction cost.
