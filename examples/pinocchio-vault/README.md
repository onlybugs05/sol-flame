# Pinocchio Vault Zero-Copy Benchmark

This example demonstrates zero-copy account access, reducing account parsing cost from 28,540 CU to under 620 CU and eliminating heap allocation entirely.

## Profile with `sol-flame`

```bash
cargo run -p cargo-cu-flame -- demo --scenario pinocchio
```

### Profile Breakdown
- **Zero-Copy Account View:** ~620 CU (vs 28,540 CU in Borsh)
- **Heap Memory:** 0 bytes (no bump allocator growth)
- **CPI Token-2022 Transfer:** ~18,450 CU
- **Constant Product Math:** ~1,480 CU
- **Total CU Reduction:** >57% overall savings!
