#![allow(clippy::unwrap_used)] // #670
#![allow(clippy::expect_used)] // #670
#![allow(clippy::panic)] // #670
#![allow(clippy::unreachable)] // #670
#![allow(dead_code)] // #670
//! Process-level tests of the `holler pane` / `holler profile` surface (story #670,
//! epic #633): the real `holler` binary, no hub, no body.
//!
//! What lives here is what only the binary can show: that `main.rs` dispatches `pane`
//! and `profile`, that `--format`/`--json` choose the output, that a usage error under
//! those two subcommands (and only those) is an envelope, that the flag matrix parses,
//! that `say`/`interrupt`/`answer`/`roster` refuse `--pane` and `--profile` before they
//! contact a hub, and that ADR 0003 and the CLI fixture carry the new rows. The
//! per-verb routing inside a stub is tested in-process by `pane_verbs` and
//! `profile_verbs`.
//!
//! Stderr is never asserted empty: the `logging_started` banner is always on it. What
//! is asserted is the refusal line, and that stdout holds exactly what the contract
//! says (nothing, or one envelope).

#[path = "../../support/mod.rs"]
mod support;

mod docs_rows;
mod flags;
mod legacy_verbs;
mod stub;
mod usage;

use serde_json::Value;

/// The verbs of `holler pane` and the story that owns each (epic #633).
pub const PANE_VERBS: &[(&str, u32)] = &[
    ("list", 643),
    ("get", 643),
    ("watch", 643),
    ("launch", 644),
    ("relaunch", 644),
    ("switch", 645),
    ("reset", 645),
    ("park", 646),
    ("unpark", 646),
    ("close", 646),
    ("doctor", 647),
    ("import", 650),
];

/// The verbs of `holler profile` and the story that owns each (#665 is proposed: the
/// operator confirms it).
pub const PROFILE_VERBS: &[(&str, u32)] = &[
    ("create", 662),
    ("delete", 662),
    ("list", 662),
    ("show", 662),
    ("apply", 664),
    ("rename", 665),
    ("export", 665),
    ("import", 665),
];

/// What one run of the binary produced.
#[derive(Debug)]
pub struct Out {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Run `holler <args>` against an empty state dir (no hub is running).
pub fn holler(args: &[&str]) -> Out {
    let state = support::StateDir::new();
    let output = support::holler_cmd(&state)
        .args(args)
        .output()
        .expect("run the holler binary");
    Out {
        code: output.status.code().expect("holler exited, not killed"),
        stdout: String::from_utf8(output.stdout).expect("stdout is UTF-8"),
        stderr: String::from_utf8(output.stderr).expect("stderr is UTF-8"),
    }
}

impl Out {
    /// Whether stderr has a line that is exactly `line` (the banner shares stderr).
    pub fn stderr_has_line(&self, line: &str) -> bool {
        self.stderr.lines().any(|l| l == line)
    }

    /// The one envelope on stdout: exactly one newline-terminated line, a JSON object.
    pub fn envelope(&self) -> Value {
        assert!(
            self.stdout.ends_with('\n'),
            "an envelope line ends with a newline: {self:?}"
        );
        let lines: Vec<&str> = self.stdout.lines().collect();
        assert_eq!(lines.len(), 1, "exactly one line on stdout: {self:?}");
        let value: Value =
            serde_json::from_str(lines[0]).unwrap_or_else(|e| panic!("{self:?} is JSON: {e}"));
        assert_eq!(value["schema_version"], 1, "{value}");
        value
    }
}

/// Collect failures instead of stopping at the first, so one run names every verb that
/// is wrong.
pub fn assert_no_failures(failures: Vec<String>) {
    assert!(
        failures.is_empty(),
        "{} failure(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}
