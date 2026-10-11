//! A pane's OpenCode server: the argv that starts it (`serve_command`), whether the running
//! one honours its spec (`verify_serving`), and whether a server is healthy (`health`).
//!
//! The adapter starts no server (#802). The host adapter's `run` starts the argv in the
//! pane's tmux session, in the pane spec's directory, and its `stop_owned` stops it; the
//! launch engine waits for its health. So nothing here spawns, waits on or signals a
//! process.

use std::time::Duration;

use holler_pane::ports::AGENT_UNSUPPORTED;
use holler_pane::{Argv, PaneError, ServeSpec};
use serde_json::Value;

use crate::http::{self, Reply};
use crate::{json_of, OpenCodeConfig};

/// The route a health check asks.
const HEALTH: &str = "/global/health";

/// `HarnessPort::serve_command`: `<env prefix> <opencode_bin> serve --port P --hostname
/// 127.0.0.1 <serve_args>`, behind the `env` prefix the TUI's argv also starts with
/// ([`crate::ProcessEnv::prefix`]). The server listens on the loopback address only, and
/// the pane's name is not on the argv: the host runs it in the pane's own tmux session and
/// directory. Pure: it reads the configuration and nothing else. A spec with an agent is
/// refused ([`refuse_an_agent`]), so no argv starts a server without the agent asked for.
pub(crate) fn serve_command(
    config: &OpenCodeConfig,
    port: u16,
    spec: &ServeSpec,
) -> Result<Argv, PaneError> {
    refuse_an_agent(spec)?;
    let mut argv = config.env.prefix();
    argv.extend([
        config.bin()?.to_owned(),
        "serve".to_owned(),
        "--port".to_owned(),
        port.to_string(),
        "--hostname".to_owned(),
        "127.0.0.1".to_owned(),
    ]);
    argv.extend(config.serve_args.iter().cloned());
    Ok(Argv::new(argv))
}

/// `agent-unsupported` for a spec that asks for an agent, and `Ok` for one that does not,
/// with nothing sent. Until #642 part 3 the adapter applies no agent, so no argv it builds
/// could carry one (`serve_command`), and it cannot check a running server for one:
/// `verify_serving` answers this too, and fails closed.
pub(crate) fn refuse_an_agent(spec: &ServeSpec) -> Result<(), PaneError> {
    match spec.agent {
        None => Ok(()),
        Some(_) => Err(PaneError::Refused {
            code: AGENT_UNSUPPORTED,
            message: "the OpenCode adapter does not apply an agent yet (#642)".to_owned(),
        }),
    }
}

/// Whether the server on `port` answers `GET /global/health` with `{"healthy": true}`
/// within `within`. Refused, timed out, garbled or anything else is `false`.
pub(crate) fn healthy(port: u16, within: Duration) -> bool {
    matches!(
        http::request(port, "GET", HEALTH, None, within),
        Ok(reply) if is_healthy(&reply)
    )
}

/// A `200` whose JSON is an object with `"healthy": true`.
fn is_healthy(reply: &Reply) -> bool {
    json_of(reply)
        .as_ref()
        .and_then(|health| health.get("healthy"))
        .and_then(Value::as_bool)
        == Some(true)
}
