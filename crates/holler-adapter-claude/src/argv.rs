//! The launch argv of a Claude Code pane (ADR-0021 section 15 items 5, 7, 10 and 11):
//! `HarnessPort::process_command`'s work, pure. It reads one fact of the disk, whether a
//! session's transcript file exists, and never its contents.
//!
//! The argv, an argv array and never a shell string (B2), carrying no secret:
//!
//! ```text
//! env -u ANTHROPIC_API_KEY ... -u CLAUDE_CODE_USE_FOUNDRY HOLLER_CLAUDE_SOCKET_DIR=<dir>
//!     <claude> --session-id|--resume <id> --settings <json> [--model <id> --effort <effort>]
//!     --disallowed-tools AskUserQuestion
//! ```
//!
//! - `env -u NAME` for each variable that silently changes the login
//!   (`HarnessKind::login_variables`), so the process keeps its subscription login whatever the
//!   tmux server's environment holds; no variable is given a value but the socket directory.
//! - The session: `--resume` when its transcript exists, `--session-id` when it does not, the
//!   rule the spike verified; a resume-only choice with no transcript is `session-not-found`.
//! - `--settings`, inline JSON: `crossSessionInbound: accept` (a cross-session message is
//!   delivered, never held behind a dialog) and a `SessionStart` hook whose command is
//!   [`SOCKET_HOOK`].
//! - `--model` and `--effort` from the spec's model; its provider is recorded, not passed.
//! - `--disallowed-tools AskUserQuestion`: a question is the one wait the hub cannot answer.
//!
//! **The socket hook (plan review W8).** Claude Code exports the path of the process's inbox
//! socket to its hooks as `CLAUDE_CODE_MESSAGING_SOCKET` before `SessionStart` runs. The hook
//! command is fixed text: nothing of the pane, the session or a path is put into it, so no text
//! of Holler's becomes shell code. Its data comes through the environment: the socket's path
//! from Claude Code, the directory it writes to from the argv's `HOLLER_CLAUDE_SOCKET_DIR`. It
//! writes the path to `<dir>/<pid>`, the pid of the process that runs the hook (its `$PPID`),
//! through a temporary file renamed into place, under `umask 077` (the directory `0700`, the
//! file `0600`). Every `SessionStart`, a person's `/clear` included, rewrites the same file, and
//! the reader trusts it only for a process `claude agents --json` lists (`inbox`).

use std::path::Path;

use holler_pane::pane::HarnessKind;
use holler_pane::ports::{SessionChoice, AGENT_UNSUPPORTED};
use holler_pane::{Argv, PaneError, PaneName, ServeSpec};
use serde_json::json;

use crate::ClaudeConfig;

/// The variable the launch argv sets for the socket hook: the directory it writes to.
pub(crate) const SOCKET_DIR_VARIABLE: &str = "HOLLER_CLAUDE_SOCKET_DIR";

/// The `SessionStart` hook command: fixed text, run by Claude Code through a shell, whose data
/// comes only through `HOLLER_CLAUDE_SOCKET_DIR` (the argv's) and `CLAUDE_CODE_MESSAGING_SOCKET`
/// (Claude Code's). A process with no socket writes nothing.
pub(crate) const SOCKET_HOOK: &str = concat!(
    "umask 077 && [ -n \"$CLAUDE_CODE_MESSAGING_SOCKET\" ] && [ -n \"$HOLLER_CLAUDE_SOCKET_DIR\" ] ",
    "&& d=\"$HOLLER_CLAUDE_SOCKET_DIR\" && mkdir -p \"$d\" && chmod 700 \"$d\" ",
    "&& t=\"$d/.$PPID.tmp\" && printf '%s\\n' \"$CLAUDE_CODE_MESSAGING_SOCKET\" > \"$t\" ",
    "&& mv -f \"$t\" \"$d/$PPID\" || true"
);

/// The tool every Claude pane launches without: a question is the one wait the hub cannot
/// answer (the spike, section 4).
const QUESTION_TOOL: &str = "AskUserQuestion";

/// The longest session id the adapter puts on an argv or into a path.
const SESSION_ID_MAX: usize = 64;

/// `process_command`'s work (see the module docs).
pub(crate) fn process_command(
    config: &ClaudeConfig,
    name: &PaneName,
    spec: &ServeSpec,
    choice: &SessionChoice,
) -> Result<Argv, PaneError> {
    if spec.agent.is_some() {
        return Err(PaneError::Refused {
            code: AGENT_UNSUPPORTED,
            message: format!(
                "{name}: a Claude Code pane takes no agent in this story (applying and checking \
                 Claude's --agent is a later story)"
            ),
        });
    }
    let cwd = spec.cwd.as_deref().ok_or_else(|| PaneError::Usage {
        message: format!("{name}: a Claude Code process needs the pane's directory"),
    })?;
    let session = checked_session(choice.session())?;
    let flag = session_flag(config, cwd, choice)?;
    let mut argv = vec!["env".to_owned()];
    for variable in HarnessKind::Claude.login_variables() {
        argv.extend(["-u".to_owned(), (*variable).to_owned()]);
    }
    argv.push(format!(
        "{SOCKET_DIR_VARIABLE}={}",
        utf8(&config.socket_dir, "the socket directory")?
    ));
    argv.push(utf8(&config.bin, "the claude program")?.to_owned());
    argv.extend([flag.to_owned(), session.to_owned()]);
    argv.extend(["--settings".to_owned(), settings()]);
    if let Some(model) = &spec.model {
        argv.extend(["--model".to_owned(), model.model_id.clone()]);
        argv.extend(["--effort".to_owned(), model.effort.clone()]);
    }
    argv.extend(["--disallowed-tools".to_owned(), QUESTION_TOOL.to_owned()]);
    Ok(Argv::new(argv))
}

/// The flag that starts the process holding the choice's session: `--resume` when its
/// transcript exists, `--session-id` when it does not; `session-not-found` for a resume-only
/// choice with no transcript.
fn session_flag(
    config: &ClaudeConfig,
    cwd: &str,
    choice: &SessionChoice,
) -> Result<&'static str, PaneError> {
    let talked = transcript(&config.config_dir, cwd, choice.session()).exists();
    match choice {
        SessionChoice::Fresh(_) => Ok("--session-id"),
        SessionChoice::Keep(_) if talked => Ok("--resume"),
        SessionChoice::Keep(_) => Ok("--session-id"),
        SessionChoice::Resume(_) if talked => Ok("--resume"),
        SessionChoice::Resume(session) => Err(PaneError::SessionNotFound {
            what: format!("{session:?} (it has no conversation to resume)"),
        }),
    }
}

/// Where Claude Code keeps the transcript of `session` for a process working in `cwd`:
/// `<config dir>/projects/<cwd, every character but an ASCII letter or digit as "-">/<session>.jsonl`
/// (the documented rule). Only its existence is ever read.
pub(crate) fn transcript(config_dir: &Path, cwd: &str, session: &str) -> std::path::PathBuf {
    let project: String = cwd
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    config_dir
        .join("projects")
        .join(project)
        .join(format!("{session}.jsonl"))
}

/// The `--settings` JSON: accept a cross-session message, and the socket hook at
/// `SessionStart`.
fn settings() -> String {
    json!({
        "crossSessionInbound": "accept",
        "hooks": {
            "SessionStart": [
                {"hooks": [{"type": "command", "command": SOCKET_HOOK}]}
            ]
        }
    })
    .to_string()
}

/// `session` when it is fit for an argv and a file name: 1 to 64 ASCII letters, digits and
/// `-` (a UUID is). Anything else is `usage`, quoting it debug-escaped.
fn checked_session(session: &str) -> Result<&str, PaneError> {
    let fits = (1..=SESSION_ID_MAX).contains(&session.len())
        && session
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-');
    if fits {
        return Ok(session);
    }
    let shown: String = session.chars().take(SESSION_ID_MAX).collect();
    Err(PaneError::Usage {
        message: format!(
            "{shown:?} is not a Claude Code session id: 1 to {SESSION_ID_MAX} ASCII letters, digits \
             and '-'"
        ),
    })
}

/// `path` as an argv's text; `unavailable` naming `what` when it is not UTF-8.
fn utf8<'a>(path: &'a Path, what: &str) -> Result<&'a str, PaneError> {
    path.to_str().ok_or_else(|| PaneError::Unavailable {
        what: format!("the path of {what} is not valid UTF-8"),
    })
}

/// A fresh session id: a version 4 UUID from the operating system's randomness.
pub(crate) fn new_session_id() -> Result<String, PaneError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|error| PaneError::Unavailable {
        what: format!("no randomness for a session id: {error}"),
    })?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    Ok(format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    ))
}
