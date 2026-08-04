// Host shim + Rust-facing glue for the vendored `@xterm/headless` bundle.
//
// Two jobs:
//
// 1. Stand in for the handful of browser/Node globals the bundle reaches for.
//    `process.title` is the important one: xterm.js branches on
//    `typeof process !== 'undefined' && 'title' in process` to set `isNode`,
//    and every `navigator` access sits on the other side of that branch, so
//    defining it means no user-agent sniffing ever runs.
//
// 2. Make writing synchronous. `Terminal.write()` is async: it queues the
//    chunk and drains it from a `setTimeout` callback, so the grid is still
//    empty when it returns. `Emulator::process` has to have the grid ready
//    when it returns, so timers are collected into a queue that `feed()`
//    drains to empty before it hands control back to Rust. The drain is
//    ordinary FIFO, which is all xterm.js needs: it only ever schedules its
//    own continuation.
//
// `performance.now()` returning a constant is deliberate, not a stub: the
// write loop yields to a fresh timer once it has spent 12ms on a chunk, and a
// frozen clock means it never does, so one `feed()` always consumes the whole
// chunk instead of leaving a tail for the next drain.

globalThis.process = { title: 'shell-use' };
globalThis.performance = { now: function () { return 0; } };
globalThis.console = {
  log: function () {}, warn: function () {}, error: function () {},
  debug: function () {}, info: function () {}, trace: function () {},
};

var __timers = [];
globalThis.setTimeout = function (fn) { return __timers.push(fn); };
globalThis.clearTimeout = function () {};
globalThis.setInterval = function () { return 0; };
globalThis.clearInterval = function () {};
globalThis.queueMicrotask = function (fn) { __timers.push(fn); };

// The bundle is UMD and assigns to `exports`.
globalThis.exports = {};
globalThis.module = { exports: globalThis.exports };

globalThis.__boot = function (cols, rows, scrollback) {
  var term = new exports.Terminal({
    cols: cols,
    rows: rows,
    scrollback: scrollback,
    // `getUnderlineStyle`, `getUnderlineColor` and `getNullCell` are proposed
    // API; without this every one of them throws.
    allowProposedApi: true,
  });

  // The headless bundle ships only the Unicode 6 width tables, which call
  // every astral emoji one column wide. alacritty measures them as two, so
  // without this a line containing an emoji puts every following cell in a
  // different column on the two backends, moving what `cells`, the locator,
  // and the SVG renderer report. The Unicode 11 provider restores the pair.
  if (typeof globalThis.__unicode11 === 'function') {
    term.loadAddon(new globalThis.__unicode11());
    term.unicode.activeVersion = '11';
  }

  // Replies the terminal wants sent back up the PTY (DA, CPR, and friends).
  var replies = [];
  term.onData(function (d) { replies.push(d); });

  function drain() {
    // `_timers` grows while draining, so re-check rather than snapshotting.
    // The cap turns a hypothetical self-rescheduling timer into an error
    // instead of a hung reader thread.
    var guard = 0;
    while (__timers.length) {
      __timers.shift()();
      if (++guard > 1000000) { throw new Error('xterm.js timer queue did not settle'); }
    }
  }

  // One reused cell object across the whole grid walk. `getCell(x, cell)`
  // fills it in place; the allocating form costs roughly twice as much.
  var CELL = term.buffer.active.getNullCell();

  return {
    feed: function (bytes) { term.write(bytes); drain(); },

    // Joined rather than returned as an array: one string crossing the
    // boundary beats one call per pending reply.
    takeReplies: function () { var s = replies.join(''); replies.length = 0; return s; },

    resize: function (cols, rows) { term.resize(cols, rows); drain(); },
    cols: function () { return term.cols; },
    rows: function () { return term.rows; },
    cursorX: function () { return term.buffer.active.cursorX; },
    cursorY: function () { return term.buffer.active.cursorY; },

    // Row span of the visible screen; `full` prepends the scrollback.
    start: function (full) { return full ? 0 : term.buffer.active.baseY; },
    end: function (full) {
      var b = term.buffer.active;
      return full ? b.length : b.baseY + term.rows;
    },

    // The grid crosses the boundary as exactly two values: every cell's text
    // in one NUL-joined string, and six ints per cell in one flat array. NUL
    // is safe as a separator because xterm.js reports an empty string, never
    // a NUL, for a cell holding nothing.
    //
    // Per-cell ints are `[width, fg, bg, ulColor, ulStyle, flags]`, with the
    // color *modes* packed into `flags` alongside the SGR booleans: a raw
    // color of 1 is palette slot 1 or the RGB triple #000001 depending on its
    // mode, so the mode has to travel with it.
    pack: function (start, end) {
      var buf = term.buffer.active, cols = term.cols;
      var chars = [], meta = [];
      for (var y = start; y < end; y++) {
        var line = buf.getLine(y);
        for (var x = 0; x < cols; x++) {
          if (!line) { chars.push(' '); meta.push(1, -1, -1, -1, 0, 0); continue; }
          var c = line.getCell(x, CELL);
          chars.push(c.getChars());

          // Reading a cell costs a JS call per getter, and a full-scrollback
          // dump is hundreds of thousands of cells. Most of them are ordinary
          // unstyled text, and for those one call answers all nineteen: -1 is
          // the "no color" value every getter returns, and no attribute bit is
          // set. Verified to produce byte-identical output to the long form
          // across every SGR in the cell vocabulary.
          if (c.isAttributeDefault()) { meta.push(c.getWidth(), -1, -1, -1, 0, 0); continue; }

          var fg = c.getFgColor();
          var fgMode = c.isFgPalette() ? 1 : (c.isFgRGB() ? 2 : 0);
          var ulColor = c.getUnderlineColor();
          var ulMode = c.isUnderlineColorPalette() ? 1 : (c.isUnderlineColorRGB() ? 2 : 0);

          // xterm.js keeps the underline color in an extended-attribute
          // record that it drops whenever the underline style is NONE, and
          // both underline-color getters then fall back to reporting the
          // foreground. Left alone that shows up as every colored cell
          // claiming an underline color it was never given. Collapsing the
          // case where the two are identical back to "unset" is exactly the
          // vocabulary's own spelling for it: `underline_color: None` already
          // means the underline takes the foreground. A cell that really did
          // set SGR 58 to its own foreground color lands here too, and draws
          // the same either way.
          if (ulColor === fg && ulMode === fgMode) { ulMode = 0; }

          // SGR 59 (reset underline color) does not clear the record: it
          // stores a sentinel that reads back through the public getters as
          // RGB #ffffff, so an ordinary reset produced a white underline where
          // there should be none. The sentinel is indistinguishable from a
          // real `58;2;255;255;255` at this layer -- both report RGB with
          // value 0xffffff -- so one of the two has to be wrong. Resetting is
          // overwhelmingly the more common of the two, and getting it wrong
          // paints a color the terminal never asked for, so it wins.
          if (ulMode === 2 && ulColor === 0xffffff) { ulMode = 0; }

          var flags =
            (c.isBold() ? 1 : 0) |
            (c.isDim() ? 2 : 0) |
            (c.isItalic() ? 4 : 0) |
            (c.isInverse() ? 8 : 0) |
            (c.isInvisible() ? 16 : 0) |
            (c.isStrikethrough() ? 32 : 0) |
            (c.isBlink() ? 64 : 0) |
            (fgMode === 1 ? 256 : (fgMode === 2 ? 512 : 0)) |
            (c.isBgPalette() ? 1024 : (c.isBgRGB() ? 2048 : 0)) |
            (ulMode === 1 ? 4096 : (ulMode === 2 ? 8192 : 0));

          meta.push(c.getWidth(), fg, c.getBgColor(), ulColor, c.getUnderlineStyle(), flags);
        }
      }
      return [chars.join('\0'), meta];
    },
  };
};
