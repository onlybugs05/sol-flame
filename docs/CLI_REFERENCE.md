# CLI Reference Manual: `cargo-cu-flame` & `sol-flame`

The `sol-flame` profiler is packaged as both a Cargo subcommand (`cargo cu-flame`) and a standalone executable (`sol-flame`).

---

## 1. Command Syntax

```bash
cargo cu-flame [OPTIONS] [COMMAND]
# or
sol-flame [OPTIONS] [COMMAND]
```

When invoked via `cargo cu-flame`, Cargo automatically passes `cu-flame` as the first argument; `sol-flame` handles this transparently.

---

## 2. Global Options & Flags

| Flag / Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `-i, --instruction <NAME>` | String | `deposit` | Solana instruction entrypoint name to profile. |
| `-e, --elf <PATH>` | Path | *(None)* | Path to compiled sBPF ELF binary (`.so`). If omitted, runs the benchmark scenario. |
| `-o, --output <PATH>` | Path | `target/sol-flame/flamegraph.svg` | Destination path for interactive SVG flamegraph. |
| `-j, --json <PATH>` | Path | `target/sol-flame/cu-profile.json` | Destination path for machine-readable JSON profile. |
| `--max-cu <LIMIT>` | Integer | `200000` | Max Compute Unit budget (e.g. 200,000 standard, 1,400,000 max). |
| `--fuzz <N>` | Integer | *(None)* | Number of invariant fuzzing iterations to execute. |
| `--ci-check <PATH>` | Path | *(None)* | Baseline JSON profile path for CI regression verification. |
| `--threshold <PERCENT>` | Float | `5.0` | Maximum allowable percentage CU regression before failing with exit code 1. |
| `--demo` | Boolean | `false` | Run out-of-the-box demonstration profile. |
| `-h, --help` | Flag | - | Display help information and exit. |
| `-V, --version` | Flag | - | Display version information and exit. |

---

## 3. Subcommands

### `demo`
Run an instant, self-contained demonstration profile without compiling external contracts.

```bash
cargo cu-flame demo [OPTIONS]
```

**Options:**
- `-s, --scenario <SCENARIO>`: Choose either `anchor` (default, Borsh deserialization heavy) or `pinocchio` (zero-copy optimized).
- `-o, --output <PATH>`: Custom output path for SVG flamegraph.
- `-j, --json <PATH>`: Custom output path for JSON profile.

**Example:**
```bash
cargo cu-flame demo --scenario pinocchio -o vault.svg -j vault.json
```

---

### `symbols`
Inspect and dump DWARF debug symbols and function PC intervals from a compiled sBPF `.so` binary.

```bash
cargo cu-flame symbols <ELF_PATH>
```

**Example:**
```bash
cargo cu-flame symbols target/deploy/my_program.so
```

**Output:**
```text
🔍 Inspecting ELF & DWARF symbols in: target/deploy/my_program.so
   Total indexed symbols: 943
```

---

### `compare`
Compare two `cu-profile.json` files for continuous integration regression analysis.

```bash
cargo cu-flame compare <BASE_JSON> <CURRENT_JSON> [--threshold <PERCENT>]
```

**Arguments:**
- `<BASE_JSON>`: Path to baseline JSON profile from main/master branch.
- `<CURRENT_JSON>`: Path to JSON profile from current pull request.
- `-t, --threshold <FLOAT>`: Maximum allowed percentage regression (default: `5.0` for 5%).

**Exit Codes:**
- `0`: PASS — Change is within acceptable threshold or represents a performance improvement.
- `1`: FAIL — Compute units increased beyond the allowed threshold.

**Example:**
```bash
cargo cu-flame compare base.json current.json --threshold 3.0
```

---

## 4. Output Formats

### Structured JSON Profile (`cu-profile.json`)
```json
{
  "instruction": "deposit",
  "total_cu": 48190,
  "peak_heap_bytes": 4300,
  "hotspots": [
    {
      "symbol": "anchor_lang::AccountDeserialize::try_deserialize (VaultState)",
      "cu": 14340,
      "percent": 29.76,
      "source": "anchor-lang/src/account.rs:18:4"
    },
    {
      "symbol": "cpi:spl_token",
      "cu": 18450,
      "percent": 38.29
    }
  ]
}
```
