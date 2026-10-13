#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #850
//! The body's config row for a Claude Code process (#850, Acceptance 1; ADR 0014 point 7, the row
//! rule): an attach row whose `harness` is `claude` names `session_id` (a lower-case UUID in its
//! 36-character form) and no `endpoint`, and is refused when the file is parsed, naming the key,
//! if it names an `endpoint`, sets `interrupt = "http"`, or has no `session_id` or one that is not
//! a UUID. The id `claude` still names the ACP bridge on a spawn row, and every attach row of any
//! other harness id is an OpenCode HTTP row, as before.
//!
//! `config_test.rs` (#187) stays green unedited; the two cases here that restate an old behaviour
//! (`a_claude_spawn_row_is_still_the_acp_bridge`, `an_opencode_attach_row_is_unchanged`) pass
//! before the story and are the regression pins of the rule's "no other row changes" half.

use std::path::{Path, PathBuf};

use holler_body::config::{self, ConfigError, Interrupt, SessionMode};
use holler_proto::SessionName;

/// A placeholder UUID in its 36-character lower-case form (the pane's session of record).
const UUID: &str = "0b6f8f2e-5c1d-4a43-9d51-2f1f6a7f0a11";

fn claude_attach(extra: &str) -> String {
    format!("[[session]]\nname = \"c1r1\"\nharness = \"claude\"\nmode = \"attach\"\n{extra}")
}

/// The `(row, name, field)` of a refusal, or a panic with what the parse did instead.
fn refused_field(toml: &str) -> &'static str {
    match config::parse(toml) {
        Err(ConfigError::Invalid { row: 0, name, field, .. }) => {
            assert_eq!(name, "c1r1", "the refusal names the row");
            field
        }
        other => panic!("expected the row refused with a named key, got {other:?}\n{toml}"),
    }
}

#[test]
fn a_claude_attach_row_with_a_uuid_session_id_parses() {
    let parsed = config::parse(&claude_attach(&format!("session_id = \"{UUID}\"\n")))
        .expect("a Claude attach row with a UUID session_id and no endpoint parses");
    assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
    let [row] = &parsed.sessions[..] else {
        panic!("one row expected: {:?}", parsed.sessions)
    };
    assert_eq!(row.name.as_str(), "c1r1");
    assert_eq!(row.harness, "claude");
    assert_eq!(row.mode, SessionMode::Attach);
    assert_eq!(row.session_id.as_deref(), Some(UUID));
    assert_eq!(row.endpoint, None, "a Claude row names no endpoint");
    assert_eq!(row.command, None);
    assert_eq!(row.interrupt, Interrupt::Acp, "the default interrupt, not http");
}

#[test]
fn a_claude_attach_row_with_an_endpoint_is_refused_naming_endpoint() {
    let toml = claude_attach(&format!(
        "endpoint = \"http://127.0.0.1:4096\"\nsession_id = \"{UUID}\"\n"
    ));
    assert_eq!(refused_field(&toml), "endpoint");
}

#[test]
fn a_claude_attach_row_with_interrupt_http_is_refused_naming_interrupt() {
    let toml = claude_attach(&format!("interrupt = \"http\"\nsession_id = \"{UUID}\"\n"));
    assert_eq!(refused_field(&toml), "interrupt");
}

#[test]
fn a_claude_attach_row_without_a_session_id_is_refused_naming_session_id() {
    assert_eq!(refused_field(&claude_attach("")), "session_id");
}

/// A name-shaped id, an OpenCode `ses_` id, an upper-case UUID, a UUID without its hyphens, one a
/// digit short, and one with a non-hexadecimal digit: each is refused naming `session_id`.
#[test]
fn a_session_id_that_is_not_a_lowercase_uuid_is_refused() {
    let bad = [
        "c1r1",
        "ses_000000000000000000000000a1",
        "0B6F8F2E-5C1D-4A43-9D51-2F1F6A7F0A11",
        "0b6f8f2e5c1d4a439d512f1f6a7f0a11",
        "0b6f8f2e-5c1d-4a43-9d51-2f1f6a7f0a1",
        "0b6f8f2e-5c1d-4a43-9d51-2f1f6a7f0a1g",
        "",
    ];
    for id in bad {
        let toml = claude_attach(&format!("session_id = \"{id}\"\n"));
        assert_eq!(refused_field(&toml), "session_id", "id {id:?}");
    }
}

/// F10, the fact the design rests on: a UUID is 36 characters and a name segment is at most 32,
/// so a pane's session id never parses as a routable session name, and `holler say <uuid>` is
/// refused as an unknown session rather than routed. Passes before the story (a pinned invariant).
#[test]
fn a_claude_uuid_never_parses_as_a_routable_name() {
    assert_eq!(UUID.len(), 36);
    assert!(
        SessionName::parse(UUID).is_err(),
        "a 36-character UUID must not be a valid session name"
    );
    assert!(SessionName::parse("c1r1").is_ok(), "the row's own name is");
}

/// `claude` is still the ACP bridge's harness id on a spawn row (ADR 0012): a `command`, no
/// `session_id`, no UUID rule. Passes before the story (a regression pin).
#[test]
fn a_claude_spawn_row_is_still_the_acp_bridge() {
    let toml = "[[session]]\nname = \"reviewer\"\nharness = \"claude\"\n\
        command = [\"npx\", \"-y\", \"@agentclientprotocol/claude-agent-acp@0.1.5\"]\n";
    let parsed = config::parse(toml).expect("the ACP bridge row parses");
    let [row] = &parsed.sessions[..] else {
        panic!("one row expected")
    };
    assert_eq!((row.harness.as_str(), row.mode), ("claude", SessionMode::Spawn));
    assert_eq!(row.command.as_ref().map(Vec::len), Some(3));
    assert_eq!((row.endpoint.as_deref(), row.session_id.as_deref()), (None, None));
}

/// Every attach row of another harness id is an OpenCode HTTP row, as before: an endpoint is
/// required, and a non-UUID session id is fine. Passes before the story (a regression pin).
#[test]
fn an_opencode_attach_row_is_unchanged() {
    let ok = "[[session]]\nname = \"oc\"\nharness = \"opencode\"\nmode = \"attach\"\n\
        endpoint = \"http://127.0.0.1:4096\"\nsession_id = \"ses_123\"\n";
    let parsed = config::parse(ok).expect("an OpenCode attach row parses");
    assert_eq!(parsed.sessions[0].endpoint.as_deref(), Some("http://127.0.0.1:4096"));
    assert_eq!(parsed.sessions[0].session_id.as_deref(), Some("ses_123"));

    let no_endpoint = "[[session]]\nname = \"oc\"\nharness = \"opencode\"\nmode = \"attach\"\n\
        session_id = \"ses_123\"\n";
    match config::parse(no_endpoint) {
        Err(ConfigError::Invalid { field: "endpoint", .. }) => {}
        other => panic!("an OpenCode attach row still requires an endpoint: {other:?}"),
    }
}

// --- the one function that compares the harness id with `claude` ------------------------------

/// Every `.rs` file under `dir`, recursively.
fn sources(dir: &Path, into: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources(&path, into);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            into.push(path);
        }
    }
}

/// `text` with every `//` comment dropped (a `//` inside a string literal is kept) and cut at its
/// first `#[cfg(test)]`, so only the code that ships is scanned.
fn shipped_code(text: &str) -> String {
    let mut out = String::new();
    for line in text.lines() {
        if line.trim_start().starts_with("#[cfg(test)]") {
            break;
        }
        let (mut in_string, mut escaped, mut cut) = (false, false, line.len());
        let bytes = line.as_bytes();
        for (at, byte) in bytes.iter().enumerate() {
            match (*byte, in_string, escaped) {
                (_, true, true) => escaped = false,
                (b'\\', true, false) => escaped = true,
                (b'"', _, _) => in_string = !in_string,
                (b'/', false, _) if bytes.get(at + 1) == Some(&b'/') => {
                    cut = at;
                    break;
                }
                _ => {}
            }
        }
        out.push_str(&line[..cut]);
        out.push('\n');
    }
    out
}

/// D2: `SessionConfig::attaches_to_claude_code` is the one place the body compares a harness id
/// with `claude`. Config validation, the driver choice and the support probe all call it, and no
/// other shipped code of the crate holds the literal.
#[test]
fn only_one_function_compares_the_harness_id_with_claude() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    sources(&root, &mut files);
    assert!(files.len() > 10, "the crate's sources are found under {root:?}");
    let mut literals = Vec::new();
    let mut callers = Vec::new();
    for file in &files {
        let code = shipped_code(&std::fs::read_to_string(file).unwrap());
        let rel = file.strip_prefix(&root).unwrap().to_string_lossy().into_owned();
        for (n, line) in code.lines().enumerate() {
            if line.contains("\"claude\"") {
                literals.push(format!("{rel}:{}: {}", n + 1, line.trim()));
            }
        }
        if code.contains("attaches_to_claude_code") {
            callers.push(rel);
        }
    }
    let config = std::fs::read_to_string(root.join("config.rs")).unwrap();
    assert!(
        shipped_code(&config).contains("fn attaches_to_claude_code"),
        "config.rs defines `fn attaches_to_claude_code`"
    );
    assert_eq!(
        literals.len(),
        1,
        "exactly one shipped place holds the literal \"claude\": {literals:?}"
    );
    assert!(literals[0].starts_with("config.rs:"), "and it is config.rs: {literals:?}");
    for user in ["config.rs", "session_manager/driver.rs", "query.rs"] {
        assert!(
            callers.iter().any(|c| c == user),
            "{user} uses attaches_to_claude_code (validation, driver choice, support probe): {callers:?}"
        );
    }
}
