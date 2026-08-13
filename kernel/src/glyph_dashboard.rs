//! glyph_dashboard.rs — pixel-glyph rendering for observability modules.
//!
//! Phase C of the glyph-geometry rewrite law: route numeric series through
//! sparkline, matrices through heatmap, lattice positions through scatter,
//! and byte buffers through braille — all via `pixel_snapshot`.
//!
//! This is the bridge module: existing observers (sys_dashboard, telemetry,
//! fdr, event_log) call into these functions to produce glyph output instead
//! of raw ASCII bars or hex dumps.

use crate::pixel_snapshot;

// ─── Series → sparkline ────────────────────────────────────────────────

/// Render a `f64` series as a sparkline (Unicode block chars ▁–█).
/// Uses a default width of 40 glyphs.
pub fn render_sparkline(series: &[f64]) -> String {
    if series.is_empty() {
        return String::new();
    }
    pixel_snapshot::sparkline(series, 40)
}

/// Render a histogram of `f64` values as a sparkline, bucketing into
/// `bins` equally-spaced intervals.
pub fn render_histogram_sparkline(values: &[f64], bins: usize) -> String {
    if values.is_empty() || bins == 0 {
        return String::new();
    }
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let span = if max > min { max - min } else { 1.0 };
    let step = span / bins as f64;
    let mut counts = vec![0u64; bins];
    for &v in values {
        if v.is_finite() {
            let idx = ((v - min) / step).floor() as usize;
            if idx < bins {
                counts[idx] += 1;
            }
        }
    }
    let max_count = counts.iter().copied().max().unwrap_or(1);
    let scaled: Vec<f64> = counts
        .iter()
        .map(|&c| if max_count > 0 { c as f64 / max_count as f64 } else { 0.0 })
        .collect();
    pixel_snapshot::sparkline(&scaled, bins.min(80))
}

// ─── Matrices → heatmap ─────────────────────────────────────────────────

/// Render a dense `f64` matrix (row-major, `rows × cols`) as a braille heatmap.
/// Each braille cell encodes 8 values (2×4 grid) as dot intensities.
pub fn render_heatmap(data: &[f64], rows: usize, cols: usize) -> String {
    if data.is_empty() || rows == 0 || cols == 0 {
        return String::new();
    }
    pixel_snapshot::heatmap(data, cols, 40)
}

// ─── 2D points → scatter ────────────────────────────────────────────────

/// Render (x, y) point pairs as a braille scatter plot.
/// Returns a multi-line string of braille glyphs.
pub fn render_scatter(points: &[(f64, f64)], width: usize, height: usize) -> String {
    if points.is_empty() || width == 0 || height == 0 {
        return String::new();
    }
    pixel_snapshot::scatter(points, width, height)
}

// ─── Byte buffers → braille ─────────────────────────────────────────────

/// Render a byte buffer as braille glyphs — 8 bits per glyph.
/// This replaces hex dumps for "snapshot" views of raw buffer content.
/// Token savings: ~8 bytes per braille char vs ~5 per hex pair.
pub fn render_bytes_braille(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return "(empty)".to_string();
    }
    pixel_snapshot::braille(bytes, 40)
}

/// Render a byte buffer as half-block glyphs — 2 bits (4 levels) per glyph.
/// Less dense than braille but visually recognizable for binary patterns.
pub fn render_bytes_halfblock(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return "(empty)".to_string();
    }
    pixel_snapshot::half_block(bytes, 40)
}

// ─── Combined glyph dashboard header ────────────────────────────────────

/// Render a compact glyph dashboard: sparkline + heatmap + braille in one block.
/// Useful for embedding in `sys_dashboard::render` or `telemetry_aggregator` reports.
pub fn render_glyph_dashboard(
    series: Option<&[f64]>,
    matrix: Option<(&[f64], usize, usize)>,
    buffer: Option<&[u8]>,
) -> String {
    let mut out = String::with_capacity(512);

    if let Some(s) = series {
        if !s.is_empty() {
            out.push_str("sparkline: ");
            out.push_str(&render_sparkline(s));
            out.push('\n');
        }
    }

    if let Some((data, rows, cols)) = matrix {
        if !data.is_empty() && rows > 0 && cols > 0 {
            out.push_str("heatmap:\n");
            out.push_str(&render_heatmap(data, rows, cols));
            out.push('\n');
        }
    }

    if let Some(buf) = buffer {
        if !buf.is_empty() {
            out.push_str("braille: ");
            out.push_str(&render_bytes_braille(buf));
            out.push('\n');
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sparkline_empty() {
        assert_eq!(render_sparkline(&[]), "");
    }

    #[test]
    fn sparkline_non_empty() {
        let vals = [0.0, 0.5, 1.0, 0.5, 0.0];
        let s = render_sparkline(&vals);
        assert!(!s.is_empty());
        assert!(s.chars().any(|c| ('\u{2581}'..='\u{2588}').contains(&c)));
    }

    #[test]
    fn histogram_sparkline() {
        let vals: Vec<f64> = (0..100).map(|i| (i % 10) as f64).collect();
        let s = render_histogram_sparkline(&vals, 10);
        assert!(!s.is_empty());
    }

    #[test]
    fn braille_non_empty() {
        let bytes = [0x00, 0xFF, 0x55, 0xAA, 0x00, 0xFF, 0x55, 0xAA];
        let s = render_bytes_braille(&bytes);
        assert!(!s.is_empty());
        assert!(s.chars().any(|c| ('\u{2800}'..='\u{28FF}').contains(&c)));
    }

    #[test]
    fn braille_empty() {
        assert_eq!(render_bytes_braille(&[]), "(empty)");
    }

    #[test]
    fn heatmap_non_empty() {
        let data: Vec<f64> = (0..64).map(|i| i as f64 / 64.0).collect();
        let s = render_heatmap(&data, 8, 8);
        assert!(!s.is_empty());
    }

    #[test]
    fn heatmap_empty() {
        assert_eq!(render_heatmap(&[], 0, 0), "");
    }

    #[test]
    fn scatter_non_empty() {
        let pts = [(0.5, 0.5), (0.1, 0.9), (0.9, 0.1)];
        let s = render_scatter(&pts, 8, 8);
        assert!(!s.is_empty());
    }

    #[test]
    fn dashboard_combines() {
        let series = [0.1, 0.5, 0.9, 0.3];
        let data: Vec<f64> = (0..16).map(|i| i as f64 / 16.0).collect();
        let buffer = [0x00, 0xFF, 0x00];
        let s = render_glyph_dashboard(Some(&series), Some((&data, 4, 4)), Some(&buffer));
        assert!(s.contains("sparkline"));
        assert!(s.contains("heatmap"));
        assert!(s.contains("braille"));
    }
}