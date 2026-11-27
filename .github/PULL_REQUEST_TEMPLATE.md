## Summary of Changes

A concise description of what this pull request introduces or fixes.

## Motivation & Context

Why is this change necessary? What issue does it solve?
Closes #(issue_number)

## Type of Change

- [ ] 🐛 Bug fix (non-breaking change which fixes an issue)
- [ ] ✨ New feature (non-breaking change adding functionality)
- [ ] ⚡ Performance optimization (CU reduction, faster symbolication)
- [ ] 📝 Documentation update
- [ ] 🧪 Testing & CI enhancement

## Verification & Testing

Describe how the changes were verified:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all
cargo run --bin sol-flame -- demo
```

- [ ] Unit tests pass cleanly
- [ ] All CI checks pass
- [ ] Flamegraph SVG generation tested and inspected
- [ ] No unintended Compute Unit regression

## Checklist

- [ ] My code adheres to the project's coding guidelines
- [ ] I have commented complex sections (especially low-level DWARF/sBPF logic)
- [ ] I have added corresponding tests for new features or bug fixes
- [ ] Related documentation has been updated
