use sol_flame_core::ProfileSummary;

/// Terminal presenter for pretty-printing CU profile results and flame summaries.
pub struct TerminalPresenter;

impl TerminalPresenter {
    /// Render a rich summary table to the console.
    pub fn print_summary(summary: &ProfileSummary, instruction_limit: Option<u64>) {
        let divider = "═".repeat(78);
        let thin_divider = "─".repeat(78);

        println!("\n╔{}╗", divider);
        println!(
            "║  ⚡ sol-flame Profiler: Instruction `{}`",
            pad_right(&summary.instruction, 44)
        );
        println!("╠{}╣", divider);

        let limit_str = instruction_limit
            .map(|l| format!(" / {} max CU", format_number(l)))
            .unwrap_or_default();

        let percent_str = instruction_limit
            .map(|l| format!(" ({:.1}% of budget)", (summary.total_cu as f64 / l as f64) * 100.0))
            .unwrap_or_default();

        println!(
            "║  Total Compute Units:  {:>10} CU{}{}",
            format_number(summary.total_cu),
            limit_str,
            pad_right(&percent_str, 30)
        );

        let heap_kb = summary.peak_heap_bytes as f64 / 1024.0;
        let heap_percent = (summary.peak_heap_bytes as f64 / 32768.0) * 100.0;
        println!(
            "║  Peak Heap Memory:     {:>10} bytes ({:.1} KB / 32 KB, {:.1}%){}",
            format_number(summary.peak_heap_bytes),
            heap_kb,
            heap_percent,
            pad_right("", 16)
        );

        if let Some(ref warn) = summary.heap_warning {
            println!("╟{}╢", thin_divider);
            println!("║  ⚠️  WARNING: {}", pad_right(warn, 63));
        }

        println!("╠{}╣", divider);
        println!(
            "║  {:<42} {:>10} {:>10} {:>8} ║",
            "HOTSPOT FUNCTION", "CU", "% TOTAL", "CATEGORY"
        );
        println!("╟{}╢", thin_divider);

        for (i, spot) in summary.hotspots.iter().enumerate() {
            let cat_tag = spot.category.color_palette_tag();
            let rank = format!("{}. ", i + 1);
            let display_name = truncate_str(&format!("{}{}", rank, spot.symbol), 40);

            println!(
                "║  {:<42} {:>10} {:>9.2}% {:>8} ║",
                display_name,
                format_number(spot.cu),
                spot.percent,
                cat_tag
            );

            if let Some(ref src) = spot.source {
                println!("║    ↳ {}", pad_right(src, 71));
            }
        }

        println!("╚{}╝\n", divider);
    }
}

fn pad_right(s: &str, width: usize) -> String {
    if s.len() >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - s.len()))
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len - 1])
    }
}

fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let mut count = 0;
    for c in s.chars().rev() {
        if count > 0 && count % 3 == 0 {
            result.push(',');
        }
        result.push(c);
        count += 1;
    }
    result.chars().rev().collect()
}
