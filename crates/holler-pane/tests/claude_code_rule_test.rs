#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #835
//! The one rule that tells a Claude Code pane by its foreground command name (#835, brief
//! section 2 and R2-7): public in `holler_pane::pane`, beside `HarnessKind`, so #831 and
//! #832 reuse it, and nothing else compares a command name with `claude`.
//!
//! The name is a process name as tmux reports it (`#{pane_current_command}`, already the
//! basename of the program), not the protocol's harness id: it matches exactly, and only
//! `claude` is classified. Every other name, a shell, OpenCode, an empty one, a wrapper, is
//! not Claude Code (and the import then takes the pane for OpenCode, as before).

use holler_pane::pane::{is_claude_code_command, CLAUDE_CODE_COMMAND};

#[test]
fn the_rule_is_the_exact_command_name_claude() {
    assert_eq!(CLAUDE_CODE_COMMAND, "claude");
    assert!(is_claude_code_command(CLAUDE_CODE_COMMAND));
}

#[test]
fn no_other_command_name_is_claude_code() {
    for other in [
        "bash",
        "sh",
        "opencode",
        "node",
        "env",
        "",
        "Claude",
        "CLAUDE",
        "claude-code",
        "claude ",
        " claude",
        "xclaude",
        "/usr/local/bin/claude",
    ] {
        assert!(!is_claude_code_command(other), "{other:?} is not Claude Code");
    }
}
