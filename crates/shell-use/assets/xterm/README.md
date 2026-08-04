# Vendored xterm.js assets

Compiled into the `shell-use` binary by `crates/shell-use/src/terminal/xtermjs.rs`
so the xterm.js backend needs no Node.js at runtime.

| File | Source | Version | License |
| ---- | ------ | ------- | ------- |
| `xterm-headless.js` | [`@xterm/headless`](https://www.npmjs.com/package/@xterm/headless) | 6.0.0 | MIT |
| `addon-unicode11.js` | [`@xterm/addon-unicode11`](https://www.npmjs.com/package/@xterm/addon-unicode11) | 0.9.0 | MIT |
| `LICENSE` | xterm.js | — | MIT |

`shim.js` is shell-use's own code, not vendored.

## Why the unicode11 addon

The headless bundle ships only the Unicode 6 width tables, which measure astral
emoji as one column. alacritty measures them as two, so without this a line
containing an emoji reports every following cell in a different column on the
two backends. Unicode 11 restores the pair.

Newer is not better here. Measured cursor column after each sequence:

| Input | alacritty | v6 | **v11** | v15 | v15-graphemes |
| ----- | --------- | -- | ------- | --- | ------------- |
| `🙂X` | 3 | 2 | **3** | 3 | 3 |
| `👨‍👩X` | 5 | 3 | **5** | 6 | 3 |
| `👍🏽X` | 5 | 3 | **5** | 5 | 3 |
| `🇺🇸X` | 3 | 3 | **3** | 3 | 3 |
| `你X` | 3 | 3 | **3** | 3 | 3 |

Only v11 agrees with alacritty on every case. `@xterm/addon-unicode-graphemes`
(v15 / v15-graphemes) is also marked experimental by its own package
description, and needs an `atob` the QuickJS host does not have.

## Updating

Re-download the bundle and the addon at the pinned versions and drop them in
unchanged. Then run `cargo test -p shell-use conformance`, which checks both
backends against the same contract, and re-measure the table above before
changing a version.
