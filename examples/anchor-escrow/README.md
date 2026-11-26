# Anchor Escrow CU Benchmark

This example demonstrates how traditional Anchor programs allocate memory on the heap and burn significant Compute Units on account deserialization.

## Profile with `sol-flame`

```bash
cargo run -p cargo-cu-flame -- demo --scenario anchor
```

### Profile Breakdown
- **Borsh Deserialization:** ~28,540 CU (~59.2% of total instruction cost)
- **CPI Token Transfer:** ~18,450 CU
- **Business Logic & Math:** ~1,200 CU
- **Peak Heap Memory:** 4,300 bytes
