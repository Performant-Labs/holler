//! The fixtures `hermetic_test.rs`, `agent_test.rs` and `prompt_test.rs` share (#642 part 3,
//! D12): moved here from `hermetic_test.rs`, never copied. Each file includes it with
//! `#[path = "support/fixtures.rs"] mod fixtures;` beside `support/stub.rs`, and uses the part
//! it needs, so every item a file may leave unused carries its own `allow(dead_code)`.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use holler_adapter_opencode::{
    OpenCodeConfig, OpenCodeHarness, ProcessEnv, Timeouts, TmuxConfig, TmuxSocket,
};
use holler_pane::{AgentKey, Argv, PaneError, PaneId, PaneName};

use crate::stub::Stub;

/// A session id of OpenCode's shape: `ses_` and 26 of `[0-9A-Za-z]`.
#[allow(dead_code)] // #642: a subset per test crate (D12)
pub const ID: &str = "ses_0123456789abcdefABCDEFghij";
/// What a bound may overrun by on a loaded CI runner.
#[allow(dead_code)] // #642: a subset per test crate (D12)
pub const SLACK: Duration = Duration::from_millis(300);
/// The binary every `serve_command` case names.
#[allow(dead_code)] // #642: a subset per test crate (D12)
pub const BIN: &str = "/opt/oc/bin/opencode";

/// Short bounds so the frozen cases finish quickly.
#[allow(dead_code)] // #642: a subset per test crate (D12)
pub fn quick() -> Timeouts {
    Timeouts {
        call: Duration::from_secs(2),
        request: Duration::from_millis(800),
        health: Duration::from_millis(300),
        settle: Duration::from_millis(400),
    }
}

/// A config whose tmux half is never reached in 642a: a binary and socket that do not exist,
/// and a `tui_session` that knows no pane.
#[allow(dead_code)] // #642: a subset per test crate (D12)
pub fn config(opencode_bin: impl Into<PathBuf>, timeouts: Timeouts) -> OpenCodeConfig {
    OpenCodeConfig {
        opencode_bin: opencode_bin.into(),
        serve_args: Vec::new(),
        env: ProcessEnv::Inherit,
        tmux: TmuxConfig {
            tmux_bin: PathBuf::from("/nonexistent/hlr642-tmux"),
            socket: TmuxSocket::Path(PathBuf::from("/nonexistent/hlr642-tmux.sock")),
        },
        tui_session: Arc::new(|pane: &PaneId| {
            Err(PaneError::PaneNotFound {
                what: pane.as_str().to_string(),
            })
        }),
        timeouts,
    }
}

/// An adapter for the calls that never spawn.
#[allow(dead_code)] // #642: a subset per test crate (D12)
pub fn harness(timeouts: Timeouts) -> OpenCodeHarness {
    OpenCodeHarness::new(config("/nonexistent/hlr642-opencode", timeouts))
}

/// An adapter with `env` and `serve_args` `["--pure"]`, naming [`BIN`].
#[allow(dead_code)] // #642: a subset per test crate (D12)
pub fn serving(env: ProcessEnv) -> OpenCodeHarness {
    let mut config = config(BIN, quick());
    config.serve_args = vec!["--pure".to_owned()];
    config.env = env;
    OpenCodeHarness::new(config)
}

/// The isolated environment of the cases: a scratch `HOME` and one `XDG_*` dir.
#[allow(dead_code)] // #642: a subset per test crate (D12)
pub fn isolated() -> ProcessEnv {
    ProcessEnv::Isolated(vec![
        ("HOME".to_owned(), "/tmp/hlr802/home".to_owned()),
        ("XDG_DATA_HOME".to_owned(), "/tmp/hlr802/data".to_owned()),
    ])
}

#[allow(dead_code)] // #642: a subset per test crate (D12)
pub fn pane_name() -> PaneName {
    PaneName::parse("demo-c1r1").unwrap()
}

#[allow(dead_code)] // #642: a subset per test crate (D12)
pub fn agent(text: &str) -> AgentKey {
    AgentKey::parse(text).unwrap()
}

#[allow(dead_code)] // #642: a subset per test crate (D12)
pub fn strings(argv: &Argv) -> Vec<&str> {
    argv.as_slice().iter().map(String::as_str).collect()
}

/// `GET /session/<ID>` answers that session.
#[allow(dead_code)] // #642: a subset per test crate (D12)
pub fn known_session(stub: &Stub) {
    stub.json(
        "GET",
        &format!("/session/{ID}"),
        200,
        &format!(r#"{{"id":"{ID}","title":"{ID}"}}"#),
    );
}

#[allow(dead_code)] // #642: a subset per test crate (D12)
pub fn unavailable<T: std::fmt::Debug>(result: Result<T, PaneError>, what: &str) -> String {
    match result {
        Err(PaneError::Unavailable { what: message }) => message,
        other => panic!("{what}: expected unavailable, got {other:?}"),
    }
}

#[allow(dead_code)] // #642: a subset per test crate (D12)
pub fn timeout_op<T: std::fmt::Debug>(result: Result<T, PaneError>, what: &str) -> String {
    match result {
        Err(PaneError::Timeout { op }) => op,
        other => panic!("{what}: expected timeout, got {other:?}"),
    }
}

/// ADR-0021 section 9: one line, naming the route and the status, short.
#[allow(dead_code)] // #642: a subset per test crate (D12)
pub fn assert_one_line_naming(message: &str, route: &str, what: &str) {
    assert!(
        !message.contains(['\n', '\r']),
        "{what}: the message is one line: {message:?}"
    );
    assert!(
        message.len() <= 200,
        "{what}: at most 200 bytes, got {}",
        message.len()
    );
    assert!(
        message.contains(route),
        "{what}: names the route {route}: {message:?}"
    );
    assert!(
        message.contains("200"),
        "{what}: names the status 200: {message:?}"
    );
}

#[allow(dead_code)] // #642: a subset per test crate (D12)
pub fn assert_within(start: Instant, bound: Duration, what: &str) {
    let took = start.elapsed();
    assert!(took < bound, "{what}: took {took:?}, bound {bound:?}");
}
