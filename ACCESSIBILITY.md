# Accessibility Statement for sol-flame

`sol-flame` is committed to ensuring digital accessibility for all Solana developers, including those using assistive technologies, keyboard navigation, or high-contrast displays.

---

## 🎨 Web Profiler & Interactive Flamegraph

The web-based flamegraph viewer and GitHub Pages showcase (`https://onlybugs05.github.io/sol-flame/`) incorporate accessibility best practices:

- **High-Contrast Semantic Palettes:** Frame colors (Solana cyan/blue, orange for Borsh deserialization, purple for CPIs) have been verified for readability against dark slate backgrounds.
- **Detailed SVG Text Nodes:** Every stack frame element includes explicit `<title>` tooltips and descriptive labels specifying function names, inclusive Compute Units (CU), and exact budget percentages.
- **Keyboard Navigation & Reset:** The web interface provides dedicated search and reset controls accessible via browser navigation.
- **Responsive Layout:** The profiler UI scales fluidly to any display zoom level up to 400% without loss of content or functionality.

---

## 💻 CLI & Terminal Output

- ANSI color outputs adhere to standard terminal color palettes.
- All metrics are simultaneously rendered as clean ASCII tables for screen readers and terminal logs.
- Machine-readable `--json` output is available for automated consumption without ANSI styling codes.

---

## 📬 Feedback & Reporting Barriers

If you encounter accessibility barriers or have suggestions for improvement, please open an issue on GitHub or reach out to `onlybugs05@gmail.com`.
