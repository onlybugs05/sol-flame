# Security Policy

The `sol-flame` team takes software security seriously. We appreciate responsible disclosure to help protect developers profiling Solana smart contracts.

---

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |
| < 0.1   | :x:                |

---

## Reporting a Vulnerability

If you discover a potential vulnerability in `sol-flame` (such as improper ELF parsing, DWARF memory corruption, out-of-bounds reads during symbolication, or unsafe zero-copy casting):

1. **Do not open a public issue.**
2. Use **[GitHub Private Vulnerability Reporting](https://github.com/onlybugs05/sol-flame/security/advisories/new)** to submit your report confidentially.
3. Alternatively, email the maintainer directly at: **`onlybugs05@gmail.com`**.

### What to Include

Please provide:
- A description of the issue and potential security impact.
- Step-by-step reproduction instructions or a minimal Proof of Concept (PoC) `.so` binary.
- Your environment details (OS, Rust version, architecture).

---

## Response Timeline

- **Initial Response:** Within 48 hours acknowledging receipt.
- **Triage & Assessment:** Within 5 business days with a severity evaluation.
- **Patch & Advisory:** Coordinated release once verified. Contributors who responsibly report issues will be credited in release notes.
