//! [`Emulator`] backend built on `@xterm/headless` running in QuickJS.
//!
//! The bundle and its host shim are embedded in the binary and evaluated into
//! a fresh QuickJS context per session, so this backend adds no runtime
//! dependency on Node or on anything installed on the machine.
//!
//! # Why the grid crosses the boundary packed
//!
//! Reading a cell means a call into JS, and an 80x30 screen is 2,400 of them
//! with ten property reads each. Walking the grid that way costs milliseconds
//! per poll. Instead [`shim.js`](../../../assets/xterm/shim.js) flattens a row
//! span into one string and one integer array, so a whole screen crosses in
//! two values and this module's job is decoding rather than traversal.
//!
//! # Threading
//!
//! [`Emulator`] is `Send` and the daemon moves the emulator between its reader
//! and request threads. `rquickjs`'s `parallel` feature makes `Runtime` and
//! `Context` `Send + Sync`, which is what lets this type be `Send` without
//! confining the interpreter to a thread of its own. It is emphatically not
//! `Sync`-in-spirit: every entry point below takes `&mut self`, so the daemon's
//! existing mutex is still what serializes access.

use compact_str::{CompactString, ToCompactString};
use rquickjs::{Context, Function, Object, Runtime};

use crate::terminal::cell::{Attrs, Color, EmuCell, UnderlineStyle, CONTINUATION};
use crate::terminal::emu::Emulator;

const XTERM_BUNDLE: &str = include_str!("../../assets/xterm/xterm-headless.js");
const UNICODE11: &str = include_str!("../../assets/xterm/addon-unicode11.js");
const SHIM: &str = include_str!("../../assets/xterm/shim.js");

/// The unicode11 addon is UMD and publishes itself by *replacing*
/// `module.exports`, so it is lifted onto a global the shim can find. Reading
/// it back before `__boot` runs also leaves `exports.Terminal`, which the shim
/// set up, untouched.
const UNICODE11_CAPTURE: &str = "globalThis.__unicode11 = module.exports.Unicode11Addon;";

/// Ints per cell in the packed `meta` array, mirroring `pack()` in the shim.
const STRIDE: usize = 6;

/// Rows decoded per `pack` call. Bounds the size of the temporary JS array a
/// full-scrollback read builds; see [`XtermJsEmu::rows_in_range`].
const PACK_BATCH_ROWS: usize = 256;

/// Color-mode bits, packed alongside the SGR booleans in the `flags` int.
const FG_PALETTE: i32 = 256;
const FG_RGB: i32 = 512;
const BG_PALETTE: i32 = 1024;
const BG_RGB: i32 = 2048;
const UL_PALETTE: i32 = 4096;
const UL_RGB: i32 = 8192;

/// Decode one color slot. `mode` is the pair of bits that says how to read
/// `raw`; with neither set the cell uses the terminal default, which the cell
/// vocabulary spells as `None`.
fn color(raw: i32, flags: i32, palette_bit: i32, rgb_bit: i32) -> Option<Color> {
    if flags & palette_bit != 0 {
        Some(Color::from_index(raw as u8))
    } else if flags & rgb_bit != 0 {
        Some(Color::Rgb(
            ((raw >> 16) & 0xff) as u8,
            ((raw >> 8) & 0xff) as u8,
            (raw & 0xff) as u8,
        ))
    } else {
        None
    }
}

/// xterm.js's `UnderlineStyle`, which already folds "not underlined" into
/// `NONE` and a bare `SGR 4` into `SINGLE`, so no separate underline flag has
/// to be consulted here.
fn underline(raw: i32) -> UnderlineStyle {
    match raw {
        1 => UnderlineStyle::Single,
        2 => UnderlineStyle::Double,
        3 => UnderlineStyle::Curly,
        4 => UnderlineStyle::Dotted,
        5 => UnderlineStyle::Dashed,
        _ => UnderlineStyle::None,
    }
}

fn attrs(flags: i32) -> Attrs {
    let mut a = Attrs::empty();
    for (bit, attr) in [
        (1, Attrs::BOLD),
        (2, Attrs::DIM),
        (4, Attrs::ITALIC),
        (8, Attrs::INVERSE),
        (16, Attrs::INVISIBLE),
        (32, Attrs::STRIKE),
        (64, Attrs::BLINK),
    ] {
        a.set(attr, flags & bit != 0);
    }
    a
}

pub struct XtermJsEmu {
    // Held to keep the interpreter alive for as long as the context that runs
    // in it; nothing calls through it directly.
    _runtime: Runtime,
    ctx: Context,
    /// The size xterm.js actually applied, which is not always the size that
    /// was asked for: it clamps to a 2x1 minimum. Caching the *requested* size
    /// sheared the grid, because `pack` emits `term.cols` cells per row while
    /// the decoder chunks by this value.
    cols: u16,
    rows: u16,
}

impl XtermJsEmu {
    pub fn new(cols: u16, rows: u16, scrollback: usize) -> anyhow::Result<Self> {
        let runtime = Runtime::new()?;
        let ctx = Context::full(&runtime)?;

        ctx.with(|ctx| -> anyhow::Result<()> {
            // Shim first: the bundle reads `process`/`exports` while it
            // evaluates, not just when the terminal is constructed.
            ctx.eval::<(), _>(SHIM)?;
            ctx.eval::<(), _>(XTERM_BUNDLE)?;
            ctx.eval::<(), _>(UNICODE11)?;
            ctx.eval::<(), _>(UNICODE11_CAPTURE)?;
            let boot: Function = ctx.globals().get("__boot")?;
            let emu: Object = boot.call((cols, rows, scrollback as u32))?;
            ctx.globals().set("__emu", emu)?;
            Ok(())
        })?;

        let mut emu = XtermJsEmu {
            _runtime: runtime,
            ctx,
            cols,
            rows,
        };
        emu.sync_size();
        Ok(emu)
    }

    /// Adopt the size xterm.js settled on.
    fn sync_size(&mut self) {
        self.cols = self.call::<i32>("cols").clamp(0, u16::MAX as i32) as u16;
        self.rows = self.call::<i32>("rows").clamp(0, u16::MAX as i32) as u16;
    }

    /// Call a zero-argument method on the shim's emulator object.
    ///
    /// No `this` is threaded through: every method the shim returns is a
    /// closure over its own `term`, so the receiver is unused, and rquickjs
    /// would otherwise pass a `This` wrapper as the first positional argument.
    fn call<R>(&self, method: &str) -> R
    where
        R: for<'js> rquickjs::FromJs<'js> + Default,
    {
        self.ctx
            .with(|ctx| -> rquickjs::Result<R> {
                let emu: Object = ctx.globals().get("__emu")?;
                emu.get::<_, Function>(method)?.call(())
            })
            .unwrap_or_default()
    }

    /// Decode one packed row span.
    ///
    /// Rows are read in batches rather than all at once. A full-scrollback
    /// grid is 5,000 rows, and packing it in one call builds a JS array of six
    /// boxed numbers per cell — 2.4 million of them — which lands above what
    /// QuickJS reclaims eagerly and below what makes it collect, so a poll loop
    /// calling `full_rows` grew the daemon by tens of megabytes per call.
    /// Batching keeps each allocation small enough to be collected between
    /// calls.
    fn rows_in_range(&self, full: bool) -> Vec<Vec<EmuCell>> {
        let (cols, _) = self.size();
        let cols = cols as usize;
        if cols == 0 {
            return Vec::new();
        }

        let span = self.ctx.with(|ctx| -> rquickjs::Result<(i32, i32)> {
            let emu: Object = ctx.globals().get("__emu")?;
            let start: i32 = emu.get::<_, Function>("start")?.call((full,))?;
            let end: i32 = emu.get::<_, Function>("end")?.call((full,))?;
            Ok((start, end))
        });
        let (start, end) = match span {
            Ok(span) => span,
            Err(_) => return Vec::new(),
        };

        let mut out = Vec::with_capacity((end - start).max(0) as usize);
        for batch in (start..end).step_by(PACK_BATCH_ROWS) {
            let batch_end = (batch + PACK_BATCH_ROWS as i32).min(end);
            let packed = self
                .ctx
                .with(|ctx| -> rquickjs::Result<(String, Vec<i32>)> {
                    let emu: Object = ctx.globals().get("__emu")?;
                    let packed: rquickjs::Array =
                        emu.get::<_, Function>("pack")?.call((batch, batch_end))?;
                    Ok((packed.get(0)?, packed.get(1)?))
                });
            let (chars, meta) = match packed {
                Ok(p) => p,
                Err(_) => return Vec::new(),
            };
            decode_into(&mut out, &chars, &meta, cols);
        }
        out
    }
}

/// Decode a packed batch into whole rows, appending to `out`.
fn decode_into(out: &mut Vec<Vec<EmuCell>>, chars: &str, meta: &[i32], cols: usize) {
    let mut cells = chars.split('\0');
    let mut row = Vec::with_capacity(cols);
    for m in meta.chunks_exact(STRIDE) {
        let ch = cells.next().unwrap_or("");
        let (width, fg, bg, ul_color, ul_style, flags) = (m[0], m[1], m[2], m[3], m[4], m[5]);

        // Width alone does not identify a continuation. xterm.js also reports
        // width 0 for a genuine zero-width grapheme that had no base character
        // to combine with (a lone combining mark, ZWSP, ZWJ, or a variation
        // selector at the start of a row): that cell owns its column and holds
        // real text. Only an *empty* zero-width cell is the second column of a
        // double-width character. Reading width alone dropped the grapheme and
        // left the row one column short of the grid.
        let ch = if !ch.is_empty() {
            ch.to_compact_string()
        } else if width == 0 {
            CompactString::const_new(CONTINUATION)
        } else {
            CompactString::const_new(" ")
        };

        row.push(EmuCell {
            ch,
            fg: color(fg, flags, FG_PALETTE, FG_RGB),
            bg: color(bg, flags, BG_PALETTE, BG_RGB),
            underline: underline(ul_style),
            underline_color: color(ul_color, flags, UL_PALETTE, UL_RGB),
            attrs: attrs(flags),
        });

        if row.len() == cols {
            out.push(std::mem::replace(&mut row, Vec::with_capacity(cols)));
        }
    }
}

impl Emulator for XtermJsEmu {
    fn process(&mut self, bytes: &[u8]) {
        // Fed as bytes rather than as a string on purpose: xterm.js runs its
        // own incremental UTF-8 decoder over a byte array and carries a
        // partial sequence across calls, which is what keeps a multi-byte
        // character split across two PTY reads from being corrupted.
        let _ = self.ctx.with(|ctx| -> rquickjs::Result<()> {
            let emu: Object = ctx.globals().get("__emu")?;
            let buf = rquickjs::TypedArray::<u8>::new(ctx.clone(), bytes)?;
            emu.get::<_, Function>("feed")?.call((buf,))
        });
    }

    fn take_pending_writes(&mut self) -> Vec<u8> {
        self.call::<String>("takeReplies").into_bytes()
    }

    fn resize(&mut self, cols: u16, rows: u16) {
        let (cols, rows) = crate::terminal::backend::clamp_size(cols, rows);
        let _ = self.ctx.with(|ctx| -> rquickjs::Result<()> {
            let emu: Object = ctx.globals().get("__emu")?;
            emu.get::<_, Function>("resize")?.call((cols, rows))
        });
        // Read the size back rather than trusting the request: xterm.js clamps
        // to its 2x1 minimum, and recording a smaller size than the grid it
        // actually holds makes every later decode mis-chunk the rows.
        self.sync_size();
    }

    fn size(&self) -> (u16, u16) {
        (self.cols, self.rows)
    }

    fn cursor(&self) -> (u16, u16) {
        let x = self.call::<i32>("cursorX").max(0) as u16;
        let y = self.call::<i32>("cursorY").max(0) as u16;
        (
            x.min(self.cols.saturating_sub(1)),
            y.min(self.rows.saturating_sub(1)),
        )
    }

    fn viewable_rows(&self) -> Vec<Vec<EmuCell>> {
        self.rows_in_range(false)
    }

    fn full_rows(&self) -> Vec<Vec<EmuCell>> {
        self.rows_in_range(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::emulator_conformance_tests!(|c, r, s| Box::new(XtermJsEmu::new(c, r, s).unwrap()));
}
