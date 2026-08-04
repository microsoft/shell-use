//! Backend selection: which terminal emulator a session runs on.

use serde::{Deserialize, Serialize};

use crate::terminal::alacritty::AlacrittyEmu;
use crate::terminal::emu::Emulator;
use crate::terminal::xtermjs::XtermJsEmu;

/// Scrollback retained by every session, in rows.
pub const SCROLLBACK: usize = 5_000;

/// The emulator a session drives its PTY output through.
///
/// Both backends pass the same conformance suite, so this picks which
/// emulator's interpretation of an ambiguous sequence a session sees, not
/// which features it gets. It exists because "does my TUI look right in
/// VS Code's terminal" is a question only xterm.js can answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Backend {
    #[default]
    Alacritty,
    #[serde(rename = "xtermjs", alias = "xterm.js", alias = "xterm")]
    XtermJs,
}

impl Backend {
    pub fn as_str(self) -> &'static str {
        match self {
            Backend::Alacritty => "alacritty",
            Backend::XtermJs => "xtermjs",
        }
    }

    pub const ALL: [Backend; 2] = [Backend::Alacritty, Backend::XtermJs];

    pub fn build(self, cols: u16, rows: u16) -> anyhow::Result<Box<dyn Emulator>> {
        Ok(match self {
            Backend::Alacritty => Box::new(AlacrittyEmu::new(cols, rows, SCROLLBACK)),
            Backend::XtermJs => Box::new(XtermJsEmu::new(cols, rows, SCROLLBACK)?),
        })
    }
}

impl std::str::FromStr for Backend {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "alacritty" => Ok(Backend::Alacritty),
            "xtermjs" | "xterm.js" | "xterm" => Ok(Backend::XtermJs),
            other => Err(format!(
                "unknown backend {other:?}; expected one of: alacritty, xtermjs"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_names_round_trip() {
        for b in Backend::ALL {
            assert_eq!(b.as_str().parse::<Backend>(), Ok(b));
        }
    }

    #[test]
    fn xtermjs_accepts_the_spellings_people_actually_type() {
        for s in ["xtermjs", "xterm.js", "xterm", "XtermJS", "  xterm.js  "] {
            assert_eq!(s.parse::<Backend>(), Ok(Backend::XtermJs), "parsing {s:?}");
        }
    }

    #[test]
    fn an_unknown_backend_names_the_valid_ones() {
        let err = "ghostty".parse::<Backend>().unwrap_err();
        assert!(
            err.contains("alacritty") && err.contains("xtermjs"),
            "{err}"
        );
    }

    #[test]
    fn the_default_is_alacritty_so_existing_sessions_are_unchanged() {
        assert_eq!(Backend::default(), Backend::Alacritty);
    }

    /// The wire spelling is what clients send; a rename would silently break
    /// every already-published binding.
    #[test]
    fn backends_serialize_to_their_wire_names() {
        assert_eq!(
            serde_json::to_string(&Backend::XtermJs).unwrap(),
            "\"xtermjs\""
        );
        assert_eq!(
            serde_json::to_string(&Backend::Alacritty).unwrap(),
            "\"alacritty\""
        );
        assert_eq!(
            serde_json::from_str::<Backend>("\"xterm.js\"").unwrap(),
            Backend::XtermJs
        );
    }

    /// Both backends must actually construct; a typo in the JS shim would
    /// otherwise only surface when someone opened a session.
    #[test]
    fn every_backend_builds_a_working_emulator() {
        for b in Backend::ALL {
            let mut emu = b
                .build(20, 3)
                .unwrap_or_else(|e| panic!("{}: {e}", b.as_str()));
            emu.process(b"hi");
            assert_eq!(emu.size(), (20, 3), "{}", b.as_str());
            assert_eq!(emu.viewable_rows()[0][0].ch, "h", "{}", b.as_str());
        }
    }
}
