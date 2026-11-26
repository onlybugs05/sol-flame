use std::io::Cursor;
use std::path::Path;
use inferno::flamegraph::color::{BasicPalette, Palette};
use inferno::flamegraph::{Direction, Options};
use sol_flame_core::FoldedStackAggregator;
use crate::{FlameRenderError, Result};

/// Color palette theme for SVG flamegraph rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PaletteKind {
    /// Solana themed: Blue (Native/Math), Orange/Red (Borsh/Anchor Deserialization), Purple (CPI), Yellow (Heap)
    #[default]
    Solana,
    /// Inferno standard palettes
    Hot,
    Mem,
    Io,
    Red,
    Green,
    Blue,
    Aqua,
    Yellow,
    Purple,
}

/// Renders folded stack profiles into interactive SVG flamegraphs.
pub struct FlameRenderer {
    title: String,
    subtitle: Option<String>,
    palette: PaletteKind,
    count_name: String,
    min_width: f64,
}

impl Default for FlameRenderer {
    fn default() -> Self {
        Self::new("Solana Compute Unit Flamegraph")
    }
}

impl FlameRenderer {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            subtitle: Some("Interactive CU & Memory Profiler (sol-flame)".to_string()),
            palette: PaletteKind::Solana,
            count_name: "Compute Units (CU)".to_string(),
            min_width: 0.1,
        }
    }

    pub fn with_subtitle(mut self, sub: impl Into<String>) -> Self {
        self.subtitle = Some(sub.into());
        self
    }

    pub fn with_palette(mut self, pal: PaletteKind) -> Self {
        self.palette = pal;
        self
    }

    pub fn with_count_name(mut self, name: impl Into<String>) -> Self {
        self.count_name = name.into();
        self
    }

    /// Render folded stack text into an SVG string.
    pub fn render_folded_text(&self, folded_text: &str) -> Result<String> {
        let mut opt = Options::default();
        opt.title = self.title.clone();
        if let Some(ref sub) = self.subtitle {
            opt.subtitle = Some(sub.clone());
        }
        opt.count_name = self.count_name.clone();
        opt.direction = Direction::Straight;
        opt.min_width = self.min_width;

        // Configure palette mapping
        match self.palette {
            PaletteKind::Solana | PaletteKind::Hot => {
                opt.colors = Palette::Basic(BasicPalette::Hot);
            }
            PaletteKind::Mem => opt.colors = Palette::Basic(BasicPalette::Mem),
            PaletteKind::Io => opt.colors = Palette::Basic(BasicPalette::Io),
            PaletteKind::Red => opt.colors = Palette::Basic(BasicPalette::Red),
            PaletteKind::Green => opt.colors = Palette::Basic(BasicPalette::Green),
            PaletteKind::Blue => opt.colors = Palette::Basic(BasicPalette::Blue),
            PaletteKind::Aqua => opt.colors = Palette::Basic(BasicPalette::Aqua),
            PaletteKind::Yellow => opt.colors = Palette::Basic(BasicPalette::Yellow),
            PaletteKind::Purple => opt.colors = Palette::Basic(BasicPalette::Purple),
        }

        let mut svg_bytes = Vec::new();
        let cursor = Cursor::new(folded_text.as_bytes());
        inferno::flamegraph::from_reader(&mut opt, cursor, &mut svg_bytes)
            .map_err(|e| FlameRenderError::InfernoError(e.to_string()))?;

        let svg_str = String::from_utf8(svg_bytes)
            .map_err(|e| FlameRenderError::InfernoError(e.to_string()))?;

        if self.palette == PaletteKind::Solana {
            Ok(apply_solana_palette_styling(&svg_str))
        } else {
            Ok(svg_str)
        }
    }

    /// Render an aggregator directly to an SVG string.
    pub fn render_aggregator(&self, aggregator: &FoldedStackAggregator) -> Result<String> {
        self.render_folded_text(&aggregator.to_folded_text())
    }

    /// Render and save SVG directly to a file on disk.
    pub fn render_to_file<P: AsRef<Path>>(&self, aggregator: &FoldedStackAggregator, path: P) -> Result<()> {
        let svg = self.render_aggregator(aggregator)?;
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, svg)?;
        Ok(())
    }
}

/// Applies Solana-specific CSS coloring and tags to the generated SVG flamegraph.
/// - Borsh / Anchor Deserialization: Warm Red/Orange (#ef4444, #f97316)
/// - Cross-Program Invocations (CPI): Royal Purple (#9333ea, #a855f7)
/// - Heap / Bump Allocator: Warning Amber/Yellow (#eab308, #f59e0b)
/// - Native Program logic: Solana Cyan/Blue (#14F195, #3b82f6)
fn apply_solana_palette_styling(svg: &str) -> String {
    use sol_flame_core::FrameCategory;

    let solana_badge = r##"
    <defs>
      <linearGradient id="solanaGrad" x1="0%" y1="0%" x2="100%" y2="100%">
        <stop offset="0%" stop-color="#9945FF" />
        <stop offset="100%" stop-color="#14F195" />
      </linearGradient>
    </defs>
    "##;

    let mut output = String::with_capacity(svg.len() + 2048);

    // 1. Insert header gradient badge
    let body_start = if let Some(pos) = svg.find("<svg") {
        if let Some(tag_end) = svg[pos..].find('>') {
            let insert_point = pos + tag_end + 1;
            output.push_str(&svg[..insert_point]);
            output.push_str(solana_badge);
            insert_point
        } else {
            0
        }
    } else {
        0
    };

    let remainder = &svg[body_start..];

    // 2. Process each <g>...</g> frame block to apply semantic coloring
    let mut cursor = 0;
    while let Some(g_start) = remainder[cursor..].find("<g>") {
        let abs_g_start = cursor + g_start;
        output.push_str(&remainder[cursor..abs_g_start]);

        let block_slice = &remainder[abs_g_start..];
        if let Some(g_end) = block_slice.find("</g>") {
            let abs_g_end = abs_g_start + g_end + 4;
            let g_content = &remainder[abs_g_start..abs_g_end];

            // Extract function symbol from <title>...</title>
            let styled_g = if let Some(t_start) = g_content.find("<title>") {
                if let Some(t_end) = g_content[t_start..].find("</title>") {
                    let title_text = &g_content[t_start + 7..t_start + t_end];
                    // Strip the " (X Compute Units, Y%)" suffix
                    let symbol_name = if let Some(paren_idx) = title_text.rfind(" (") {
                        title_text[..paren_idx].trim()
                    } else {
                        title_text.trim()
                    };

                    let hex_color = if symbol_name.eq_ignore_ascii_case("all") {
                        "#64748b" // Neutral slate for root summary frame
                    } else {
                        let category = FrameCategory::classify(symbol_name);
                        match category {
                            FrameCategory::Native => "#3b82f6",         // Solana Blue
                            FrameCategory::Deserialization => "#f97316", // Warm Orange/Red
                            FrameCategory::Cpi => "#9333ea",             // Royal Purple
                            FrameCategory::Syscall => "#06b6d4",         // Cyan
                            FrameCategory::HeapAlloc => "#eab308",       // Warning Amber/Yellow
                        }
                    };

                    // Replace fill="rgb(...)" with fill="{hex_color}" in the rect
                    if let Some(rect_start) = g_content.find("<rect ") {
                        if let Some(fill_start) = g_content[rect_start..].find("fill=\"") {
                            let abs_fill_start = rect_start + fill_start + 6;
                            if let Some(fill_end) = g_content[abs_fill_start..].find('"') {
                                let abs_fill_end = abs_fill_start + fill_end;
                                let mut colored = String::with_capacity(g_content.len() + 16);
                                colored.push_str(&g_content[..abs_fill_start]);
                                colored.push_str(hex_color);
                                colored.push_str(&g_content[abs_fill_end..]);
                                colored
                            } else {
                                g_content.to_string()
                            }
                        } else {
                            g_content.to_string()
                        }
                    } else {
                        g_content.to_string()
                    }
                } else {
                    g_content.to_string()
                }
            } else {
                g_content.to_string()
            };

            output.push_str(&styled_g);
            cursor = abs_g_end;
        } else {
            output.push_str(&remainder[abs_g_start..]);
            cursor = remainder.len();
            break;
        }
    }

    if cursor < remainder.len() {
        output.push_str(&remainder[cursor..]);
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_svg_rendering() {
        let mut agg = FoldedStackAggregator::new();
        agg.add_folded_line("entrypoint;process_instruction;borsh_deser", 28540);
        agg.add_folded_line("entrypoint;process_instruction;cpi:spl_token", 18450);
        agg.add_folded_line("entrypoint;process_instruction;math", 1200);

        let renderer = FlameRenderer::new("Test Escrow Flamegraph");
        let svg = renderer.render_aggregator(&agg).expect("svg should render");

        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("Compute Units"));
        assert!(svg.contains("borsh_deser"));
        // Verify Solana semantic palette colors are applied
        assert!(svg.contains("#9333ea")); // CPI purple
        assert!(svg.contains("#f97316")); // Deser orange
        assert!(svg.contains("#3b82f6")); // Native blue
    }

    #[test]
    fn test_xss_escaping_in_title() {
        let mut agg = FoldedStackAggregator::new();
        agg.add_folded_line("entrypoint;math", 100);

        let renderer = FlameRenderer::new("<script>alert('xss')</script>");
        let svg = renderer.render_aggregator(&agg).expect("svg should render");

        // Must not contain raw script tag
        assert!(!svg.contains("<script>alert"));
        assert!(svg.contains("&lt;script&gt;alert"));
    }
}
