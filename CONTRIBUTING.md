# Contributing to sol-flame

Thank you for your interest in improving `sol-flame`! We welcome contributions from developers, security researchers, and Solana ecosystem builders.

---

## 🧭 Code of Conduct

All contributors and maintainers are expected to adhere to our [Code of Conduct](CODE_OF_CONDUCT.md). Please report unacceptable behavior to `onlybugs05@gmail.com`.

---

## 🛠️ Development Setup

### Prerequisites
- **Rust Toolchain:** Version 1.75 or later (`rustup default stable`)
- **Components:** `clippy`, `rustfmt` (`rustup component add clippy rustfmt`)
- **Solana Toolchain (Optional):** Required only if compiling new `.so` test fixtures with `cargo build-sbf`.

### Building from Source

Clone your fork and build the entire workspace:

```bash
git clone https://github.com/onlybugs05/sol-flame.git
cd sol-flame
cargo build
```

### Running Tests

Execute the full suite of unit and integration tests across all workspace crates:

```bash
cargo test --all --verbose
```

Run test suites for the example contracts:

```bash
cargo test -p anchor-escrow
cargo test -p pinocchio-vault
```

---

## 📐 Coding Guidelines

- **Formatting:** Keep code clean with `cargo fmt --all --check`.
- **Linting:** Ensure no warnings are triggered with `cargo clippy --all-targets -- -D warnings`.
- **Errors:** Use typed errors via `thiserror` rather than raw strings or panics.
- **Safety:** Avoid unsafe blocks. Any zero-copy casts must explicitly verify memory pointer alignment.

---

## 🔄 Pull Request Workflow

1. **Fork & Branch:** Create a branch prefixed with your feature/fix type (e.g. `feat/dwarf-inlining`, `fix/heap-boundary`).
2. **Commit Messages:** Follow Conventional Commits:
   - `feat(scope): ...`
   - `fix(scope): ...`
   - `docs(scope): ...`
   - `perf(scope): ...`
   - `test(scope): ...`
3. **Verify Locally:**
   ```bash
   cargo fmt --check
   cargo clippy --all-targets -- -D warnings
   cargo test --all
   cargo run --bin sol-flame -- demo
   ```
4. **Submit PR:** Fill out the [Pull Request Template](.github/PULL_REQUEST_TEMPLATE.md) completely, including motivation, test evidence, and any Compute Unit impact.
