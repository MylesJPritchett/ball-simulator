//! Text drawing with the bundled Inter Bold font (SIL Open Font License, see
//! `assets/fonts/Inter-LICENSE.txt`), rasterized by fontdue and blended into
//! the frame.

use std::collections::HashMap;

use fontdue::{Font, FontSettings, Metrics};
use tiny_skia::Pixmap;

use super::layout::Region;
use crate::look::palette::Rgb;

const INTER_BOLD: &[u8] = include_bytes!("../../assets/fonts/Inter-Bold.ttf");

/// Line height as a multiple of the font size.
const LINE_HEIGHT: f32 = 1.15;
/// Inter's capital height as a multiple of the font size.
pub const CAP_HEIGHT: f32 = 0.727;

pub struct TextPainter {
    font: Font,
    glyphs: HashMap<(char, u32), (Metrics, Vec<u8>)>,
}

/// Text fitted into a region: the font size and the wrapped lines.
#[derive(Debug, Clone, PartialEq)]
pub struct FittedText {
    pub px: f32,
    pub lines: Vec<String>,
}

impl Default for TextPainter {
    fn default() -> Self {
        Self::new()
    }
}

impl TextPainter {
    pub fn new() -> Self {
        TextPainter {
            font: Font::from_bytes(INTER_BOLD, FontSettings::default())
                .expect("bundled font is valid"),
            glyphs: HashMap::new(),
        }
    }

    /// Width of one line of text at `px`, including kerning.
    pub fn line_width(&self, line: &str, px: f32) -> f32 {
        let mut width = 0.0;
        let mut prev = None;
        for ch in line.chars() {
            if let Some(p) = prev {
                width += self.font.horizontal_kern(p, ch, px).unwrap_or(0.0);
            }
            width += self.font.metrics(ch, px).advance_width;
            prev = Some(ch);
        }
        width
    }

    /// Greedy word wrap to `max_width`. A single word wider than the line is
    /// kept whole (the caller shrinks the font until it fits).
    fn wrap(&self, text: &str, px: f32, max_width: f32) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();
        for word in text.split_whitespace() {
            match lines.last_mut() {
                Some(line) if self.line_width(&format!("{line} {word}"), px) <= max_width => {
                    line.push(' ');
                    line.push_str(word);
                }
                _ => lines.push(word.to_string()),
            }
        }
        lines
    }

    /// Largest size up to `max_px` at which `text` wraps to fit `region`.
    pub fn fit(&self, text: &str, region: Region, max_px: f32) -> FittedText {
        let mut px = max_px;
        loop {
            let lines = self.wrap(text, px, region.width());
            let fits = lines.len() as f32 * px * LINE_HEIGHT <= region.height()
                && lines
                    .iter()
                    .all(|l| self.line_width(l, px) <= region.width());
            if fits || px <= 8.0 {
                return FittedText { px, lines };
            }
            px *= 0.95;
        }
    }

    /// Draws fitted text centred in `region`, each line centred horizontally.
    pub fn draw_centred(
        &mut self,
        pixmap: &mut Pixmap,
        text: &FittedText,
        region: Region,
        colour: Rgb,
    ) {
        let line_height = text.px * LINE_HEIGHT;
        let block = text.lines.len() as f32 * line_height;
        // Centre each line's capitals within its line box.
        let mut baseline =
            region.center_y() - block / 2.0 + (line_height + text.px * CAP_HEIGHT) / 2.0;
        for line in &text.lines {
            let x = region.center_x() - self.line_width(line, text.px) / 2.0;
            self.draw_line(pixmap, line, x, baseline, text.px, colour);
            baseline += line_height;
        }
    }

    /// Draws one line with its left edge at `x` and its baseline at `baseline`.
    pub fn draw_line(
        &mut self,
        pixmap: &mut Pixmap,
        line: &str,
        x: f32,
        baseline: f32,
        px: f32,
        colour: Rgb,
    ) {
        let mut pen = x;
        let mut prev = None;
        for ch in line.chars() {
            if let Some(p) = prev {
                pen += self.font.horizontal_kern(p, ch, px).unwrap_or(0.0);
            }
            let font = &self.font;
            let (metrics, coverage) = self
                .glyphs
                .entry((ch, px.to_bits()))
                .or_insert_with(|| font.rasterize(ch, px));
            let left = (pen + metrics.xmin as f32).round() as i32;
            let top = (baseline - (metrics.ymin + metrics.height as i32) as f32).round() as i32;
            blend_coverage(pixmap, left, top, metrics.width, coverage, colour);
            pen += metrics.advance_width;
            prev = Some(ch);
        }
    }
}

/// Source-over blend of a coverage mask in a solid colour onto an opaque,
/// premultiplied pixmap.
fn blend_coverage(
    pixmap: &mut Pixmap,
    left: i32,
    top: i32,
    width: usize,
    coverage: &[u8],
    colour: Rgb,
) {
    if width == 0 {
        return;
    }
    let (pw, ph) = (pixmap.width() as i32, pixmap.height() as i32);
    let data = pixmap.data_mut();
    for (row, cov_row) in coverage.chunks_exact(width).enumerate() {
        let y = top + row as i32;
        if !(0..ph).contains(&y) {
            continue;
        }
        for (col, &a) in cov_row.iter().enumerate() {
            let x = left + col as i32;
            if a == 0 || !(0..pw).contains(&x) {
                continue;
            }
            let i = ((y * pw + x) * 4) as usize;
            let a = a as u32;
            for c in 0..3 {
                let dst = data[i + c] as u32;
                data[i + c] = ((colour[c] as u32 * a + dst * (255 - a) + 127) / 255) as u8;
            }
        }
    }
}
