//! Crisp, full-color SVG screenshot of a terminal grid, styled after
//! `svg-term-cli`: a rounded window panel with macOS-style controls.
//!
//! Vector output renders sharply at any zoom and needs no bundled font (the
//! viewer supplies a monospace face). Colors, bold/italic/underline/strike,
//! inverse, and dim are all preserved. Each run of same-styled cells is forced
//! to its exact column width via `textLength`, so alignment is independent of
//! the rendering font's metrics.

use std::fmt::Write;

use crate::terminal::emu::{Color, EmuCell};

const CELL_W: f32 = 10.0;
const CELL_H: f32 = 21.0;
const FONT_SIZE: f32 = 17.0;
const MARGIN_X: f32 = 15.0;
const HEADER_H: f32 = 38.0;
const MARGIN_BOTTOM: f32 = 14.0;
const DOT_R: f32 = 7.0;
const FONT_STACK: &str =
    "'Cascadia Code','JetBrains Mono','Fira Code',Menlo,Consolas,'DejaVu Sans Mono',monospace";

struct Theme {
    palette: [(u8, u8, u8); 16],
    default_fg: (u8, u8, u8),
    default_bg: (u8, u8, u8),
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            palette: [
                (40, 45, 53),
                (232, 131, 136),
                (168, 204, 140),
                (219, 171, 121),
                (113, 190, 242),
                (210, 144, 228),
                (102, 194, 205),
                (185, 191, 202),
                (111, 119, 131),
                (232, 131, 136),
                (168, 204, 140),
                (219, 171, 121),
                (115, 190, 243),
                (210, 144, 227),
                (102, 194, 205),
                (255, 255, 255),
            ],
            default_fg: (185, 191, 202),
            default_bg: (40, 45, 53),
        }
    }
}

impl Theme {
    fn resolve(&self, color: Color, is_fg: bool) -> (u8, u8, u8) {
        match color {
            Color::Default => {
                if is_fg {
                    self.default_fg
                } else {
                    self.default_bg
                }
            }
            Color::Idx(i) if i < 16 => self.palette[i as usize],
            Color::Idx(i) => crate::assert::color::ansi256_to_rgb(i),
            Color::Rgb(r, g, b) => (r, g, b),
        }
    }
}

fn hex((r, g, b): (u8, u8, u8)) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}

fn dim((r, g, b): (u8, u8, u8)) -> (u8, u8, u8) {
    let s = |v: u8| (v as f32 * 0.6) as u8;
    (s(r), s(g), s(b))
}

static BLANK: EmuCell = EmuCell {
    ch: String::new(),
    fg: Color::Default,
    bg: Color::Default,
    bold: false,
    dim: false,
    italic: false,
    underline: false,
    inverse: false,
    invisible: false,
    strike: false,
};

fn cell_at(row: &[EmuCell], x: usize) -> &EmuCell {
    row.get(x).unwrap_or(&BLANK)
}

/// Resolved background color for a cell (honoring inverse).
fn bg_of(cell: &EmuCell, theme: &Theme) -> (u8, u8, u8) {
    let bg = theme.resolve(cell.bg, false);
    let fg = theme.resolve(cell.fg, true);
    if cell.inverse {
        fg
    } else {
        bg
    }
}

#[derive(PartialEq)]
struct Style {
    fg: (u8, u8, u8),
    bold: bool,
    italic: bool,
    underline: bool,
    strike: bool,
    invisible: bool,
}

fn style_of(cell: &EmuCell, theme: &Theme) -> Style {
    let mut fg = theme.resolve(cell.fg, true);
    let bg = theme.resolve(cell.bg, false);
    if cell.inverse {
        fg = bg;
    }
    if cell.dim {
        fg = dim(fg);
    }
    Style {
        fg,
        bold: cell.bold,
        italic: cell.italic,
        underline: cell.underline,
        strike: cell.strike,
        invisible: cell.invisible,
    }
}

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}

/// Render a grid to a standalone SVG document.
pub fn render_svg(rows: &[Vec<EmuCell>], cols: u16) -> String {
    let theme = Theme::default();
    let cols = cols as usize;
    let x0 = MARGIN_X;
    let y0 = HEADER_H;
    let width = MARGIN_X * 2.0 + cols as f32 * CELL_W;
    let height = HEADER_H + MARGIN_BOTTOM + rows.len().max(1) as f32 * CELL_H;

    let mut out = String::new();
    let _ = write!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width:.0}" height="{height:.0}" viewBox="0 0 {width:.0} {height:.0}" font-family="{FONT_STACK}" font-size="{FONT_SIZE}px">"#
    );
    let _ = write!(
        out,
        r#"<rect width="{width:.0}" height="{height:.0}" rx="8" fill="{}"/>"#,
        hex(theme.default_bg)
    );
    for (i, dot) in ["#ff5f56", "#ffbd2e", "#27c93f"].iter().enumerate() {
        let cx = MARGIN_X + 5.0 + i as f32 * 20.0;
        let _ = write!(
            out,
            r#"<circle cx="{cx:.1}" cy="{cy:.1}" r="{DOT_R:.1}" fill="{dot}"/>"#,
            cy = HEADER_H / 2.0,
        );
    }

    for (y, row) in rows.iter().enumerate() {
        let mut x = 0;
        while x < cols {
            let bg = bg_of(cell_at(row, x), &theme);
            let mut run = 1;
            while x + run < cols && bg_of(cell_at(row, x + run), &theme) == bg {
                run += 1;
            }
            if bg != theme.default_bg {
                let rx = x0 + x as f32 * CELL_W;
                let ry = y0 + y as f32 * CELL_H;
                let rw = run as f32 * CELL_W;
                let _ = write!(
                    out,
                    r#"<rect x="{rx:.2}" y="{ry:.2}" width="{rw:.2}" height="{CELL_H:.2}" fill="{}"/>"#,
                    hex(bg)
                );
            }
            x += run;
        }
    }

    for (y, row) in rows.iter().enumerate() {
        let baseline = y0 + y as f32 * CELL_H + FONT_SIZE * 0.78;
        let mut x = 0;
        while x < cols {
            let style = style_of(cell_at(row, x), &theme);
            let mut run = 1;
            while x + run < cols && style_of(cell_at(row, x + run), &theme) == style {
                run += 1;
            }
            if !style.invisible {
                let text: String = (x..x + run)
                    .map(|i| {
                        let ch = &cell_at(row, i).ch;
                        if ch.is_empty() {
                            " ".to_string()
                        } else {
                            ch.clone()
                        }
                    })
                    .collect();
                if !text.trim().is_empty() {
                    let tx = x0 + x as f32 * CELL_W;
                    let tl = run as f32 * CELL_W;
                    let weight = if style.bold {
                        r#" font-weight="bold""#
                    } else {
                        ""
                    };
                    let italic = if style.italic {
                        r#" font-style="italic""#
                    } else {
                        ""
                    };
                    let deco = match (style.underline, style.strike) {
                        (true, true) => r#" text-decoration="underline line-through""#,
                        (true, false) => r#" text-decoration="underline""#,
                        (false, true) => r#" text-decoration="line-through""#,
                        (false, false) => "",
                    };
                    let _ = write!(
                        out,
                        r#"<text x="{tx:.2}" y="{baseline:.2}" fill="{fg}"{weight}{italic}{deco} textLength="{tl:.2}" lengthAdjust="spacingAndGlyphs" xml:space="preserve">{esc}</text>"#,
                        fg = hex(style.fg),
                        esc = escape(&text)
                    );
                }
            }
            x += run;
        }
    }

    out.push_str("</svg>");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(ch: &str, fg: Color, bg: Color) -> EmuCell {
        let mut c = BLANK.clone();
        c.ch = ch.to_string();
        c.fg = fg;
        c.bg = bg;
        c
    }

    #[test]
    fn emits_valid_svg_with_text_and_color() {
        let rows = vec![vec![
            cell("h", Color::Idx(1), Color::Default),
            cell("i", Color::Idx(1), Color::Default),
        ]];
        let svg = render_svg(&rows, 2);
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
        assert!(svg.contains("textLength"));
        assert!(svg.contains(&hex((232, 131, 136))));
        assert!(svg.contains(">hi</text>"));
    }

    #[test]
    fn emits_window_chrome() {
        let svg = render_svg(&[vec![cell(" ", Color::Default, Color::Default)]], 1);
        assert!(svg.contains("<circle"));
        assert!(svg.contains("#ff5f56"));
        assert!(svg.contains("#ffbd2e"));
        assert!(svg.contains("#27c93f"));
    }

    #[test]
    fn escapes_markup_characters() {
        let rows = vec![vec![cell("<", Color::Default, Color::Default)]];
        let svg = render_svg(&rows, 1);
        assert!(svg.contains("&lt;"));
        assert!(!svg.contains("><</text>"));
    }

    #[test]
    fn background_run_emitted_for_non_default_bg() {
        let rows = vec![vec![cell(" ", Color::Default, Color::Idx(4))]];
        let svg = render_svg(&rows, 1);
        assert!(svg.contains(&hex((113, 190, 242))));
    }
}
