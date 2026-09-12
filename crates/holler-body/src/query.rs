//! Local `query/*` document builders (issue #185): `status`/`caps`/`support`/
//! `protocol`, built purely from this body's own local state — the persisted
//! identity (join), the connection-state file (issue #182), and the session
//! config (issue #187). **Never a model, never a live socket round trip**:
//! this is what makes `body status`/`caps`/`support`/`query` work without a
//! `body run` active on this box, and is exactly what a live `body run`
//! answers with when the hub forwards a `query/*` request from `hub query
//! TARGET …`.

use std::path::Path;

use holler_proto::{
    Caps, Code, HelloRole, ProtocolAnswer, SessionState as WireSessionState, Status,
    StatusSession, Support, SupportKind, WireError, FEATURES, HARNESS_IDS,
    PROTOCOL_MAX, PROTOCOL_MIN, PROTOCOL_VERSION,
};

use crate::config::{SessionConfig, SessionMode};
use crate::identity::BodyIdentity;

/// The capability ids (docs §9, §5.3's `kind: "capability"` subset) — a
/// finer-grained pair drawn from [`holler_proto::FEATURES`], distinguished
/// from the plain feature ids only by the `kind` reported in a `query/support`
/// answer.
const CAPABILITY_IDS: &[&str] = &["attach", "opencode-http"];

/// Classify a feature/harness/capability id into its `query/support` `kind`
/// (docs §5.3), or `None` when it is outside the whole v2 vocabulary
/// (`-32006 unknown_feature`).
fn classify(id: &str) -> Option<SupportKind> {
    if HARNESS_IDS.contains(&id) {
        Some(SupportKind::Harness)
    } else if CAPABILITY_IDS.contains(&id) {
        Some(SupportKind::Capability)
    } else if FEATURES.contains(&id) {
        Some(SupportKind::Feature)
    } else {
        None
    }
}

/// Build this body's `query/status` document.
pub fn local_status(state_root: &Path, identity: Option<&BodyIdentity>, configs: &[SessionConfig]) -> Status {
    let connected = crate::connection_state::read(state_root)
        .map(|c| matches!(c.state, crate::connection_state::ConnState::Connected))
        .unwrap_or(false);
    let hostname = identity.map(|i| i.hostname.clone()).unwrap_or_default();
    let token_id = identity.map(|i| i.token_id.clone());

    let mut harnesses: Vec<String> = configs.iter().map(|s| s.harness.clone()).collect();
    harnesses.sort();
    harnesses.dedup();

    let session_list = configs
        .iter()
        .map(|s| StatusSession {
            name: s.name.as_str().to_string(),
            harness: s.harness.clone(),
            state: WireSessionState::Idle,
            endpoint: s.endpoint.clone(),
        })
        .collect();

    Status {
        role: HelloRole::Body,
        protocol: PROTOCOL_VERSION,
        protocol_min: PROTOCOL_MIN,
        protocol_max: PROTOCOL_MAX,
        version: env!("CARGO_PKG_VERSION").to_string(),
        hostname,
        connected: Some(connected),
        token_id,
        listening: None,
        features: FEATURES.iter().map(|s| (*s).to_string()).collect(),
        harnesses: Some(harnesses),
        harnesses_known: None,
        harnesses_confirmed: None,
        bodies: None,
        sessions: None,
        session_list: Some(session_list),
    }
}

/// Build this body's `query/caps` document: the status doc plus a support
/// answer for every known id in the v2 vocabulary (docs §5.2).
pub fn local_caps(state_root: &Path, identity: Option<&BodyIdentity>, configs: &[SessionConfig]) -> Caps {
    let status = local_status(state_root, identity, configs);
    let mut caps = std::collections::BTreeMap::new();
    for id in FEATURES.iter().chain(HARNESS_IDS.iter()) {
        // Every id in FEATURES/HARNESS_IDS is by construction known to
        // `classify`, so `local_support` never returns `Err` here.
        if let Ok(s) = local_support(id, configs) {
            caps.insert((*id).to_string(), s);
        }
    }
    Caps { status, caps }
}

/// Answer `query/support {feature}` (docs §5.3) for this body: local probes
/// only, never a model — with one deliberate exception (issue #195): an
/// `attach`-mode harness's probe dials the real endpoint (see
/// [`probe_attach_session_live`]'s own doc comment for why that stays a
/// short, bounded, plain-sync TCP probe rather than growing an async
/// `reqwest` call into this function).
///
/// - A **harness** id is `ok:true` when some configured session names that
///   harness and either (a) it is `attach` mode and its endpoint answers
///   `GET /session/{session_id}` (bare, falling back to `/api/session/{id}`
///   — the same dual-prefix existence check
///   `http_attach_driver::check_exists` uses; issue #185 originally left
///   this an untested "presence in config is enough" placeholder, replaced
///   here with the issue #195 spec's real probe), or (b) it is `spawn` mode
///   and `command[0]` resolves on `PATH` or as an existing path.
/// - A **capability** id (`attach`, `opencode-http`) is always `ok:true`
///   (both are implemented body-side, independent of session config).
/// - Any other **feature** id is `ok:true` (this body speaks the whole
///   protocol machinery — a feature id names a wire capability, not a
///   per-session probe).
/// - An id outside the v2 vocabulary is `-32006 unknown_feature`.
pub fn local_support(feature: &str, configs: &[SessionConfig]) -> Result<Support, WireError> {
    let Some(kind) = classify(feature) else {
        return Err(WireError::new(
            Code::UnknownFeature,
            format!("unknown feature/harness id: {feature}"),
            None,
        ));
    };
    Ok(match kind {
        SupportKind::Harness => harness_support(feature, configs),
        SupportKind::Capability | SupportKind::Feature => Support {
            feature: feature.to_string(),
            kind,
            ok: true,
            how: Some("implemented".to_string()),
            reason: None,
        },
    })
}

fn harness_support(harness: &str, configs: &[SessionConfig]) -> Support {
    let sessions: Vec<&SessionConfig> = configs.iter().filter(|c| c.harness == harness).collect();
    if sessions.is_empty() {
        return Support {
            feature: harness.to_string(),
            kind: SupportKind::Harness,
            ok: false,
            how: None,
            reason: Some("not configured".to_string()),
        };
    }
    for s in &sessions {
        match s.mode {
            SessionMode::Attach => {
                let (endpoint, session_id) = (s.endpoint.as_deref(), s.session_id.as_deref());
                let ok = match (endpoint, session_id) {
                    (Some(endpoint), Some(session_id)) => {
                        probe_attach_session_live(endpoint, session_id)
                    }
                    // Defensive: `config::parse` already refuses an `attach`
                    // row missing either field, so this never happens for a
                    // config that reached this far — but a probe with
                    // nothing to dial is `ok:false`, not a fabricated `true`.
                    _ => false,
                };
                return Support {
                    feature: harness.to_string(),
                    kind: SupportKind::Harness,
                    ok,
                    how: ok.then(|| "attach".to_string()),
                    reason: (!ok).then(|| "attach endpoint did not answer".to_string()),
                };
            }
            SessionMode::Spawn => {
                if let Some(first) = s.command.as_ref().and_then(|c| c.first()) {
                    if resolvable(first) {
                        return Support {
                            feature: harness.to_string(),
                            kind: SupportKind::Harness,
                            ok: true,
                            how: Some(format!("spawn: {first}")),
                            reason: None,
                        };
                    }
                }
            }
        }
    }
    Support {
        feature: harness.to_string(),
        kind: SupportKind::Harness,
        ok: false,
        how: None,
        reason: Some("command[0] not found on PATH".to_string()),
    }
}

/// A short, bounded, **plain synchronous** existence probe against an
/// attach-mode harness's real endpoint (issue #195's `support opencode`
/// spec: "ok iff the endpoint answers `GET /session/{id}`"), hand-rolled
/// over `std::net::TcpStream` rather than an async `reqwest` call.
///
/// This module's callers are a mix of sync and async contexts: the CLI's
/// `body support`/`body caps`/`body query support` leaves call
/// [`local_support`]/[`local_caps`] directly from a plain, runtime-free
/// `fn main` (no tokio context at all), while a live `body run`'s
/// `dispatch::handle_query` calls the very same functions from inside an
/// already-running tokio runtime. An async probe would need either a nested
/// `tokio::runtime::Builder` per sync call site (workable, but see
/// `join.rs`'s own "a body is a short-lived CLI with no runtime in scope"
/// framing for why that pattern exists there but is unnecessary here) or —
/// far worse — a `.block_on()` from *inside* the already-running runtime,
/// which tokio does not allow at all. A tiny, bounded (≤ ~1.6s total: two
/// prefixes, ~800ms connect+read timeout each), hand-rolled blocking TCP
/// probe sidesteps the whole question: it is safe to call from either
/// context (the same trade-off this workspace's own hand-rolled fake test
/// server and `holler_hub::serve`'s hand-rolled WebSocket upgrade already
/// make — see `fake_server.rs`'s own module doc), at the cost of briefly
/// blocking whichever thread calls it. Acceptable here: `support`/`caps` are
/// low-frequency, human-triggered admin queries, not a hot path.
///
/// Tries the bare `GET {endpoint}/session/{id}` form first, falling back to
/// `/api/session/{id}` on any failure of the first — the same dual-prefix
/// fallback `http_attach_driver::check_exists` uses (both forms were
/// independently confirmed live to work as an existence probe; see that
/// function's own doc comment). A connect failure, a timeout, or any
/// non-2xx status from *both* prefixes is `false` — this is a "does the
/// harness live here" signal for `support`/`caps`, not a fail-closed attach
/// gate, so it fails open to `false` rather than erroring.
fn probe_attach_session_live(endpoint: &str, session_id: &str) -> bool {
    let path_bare = format!("/session/{session_id}");
    let path_api = format!("/api/session/{session_id}");
    probe_one(endpoint, &path_bare) || probe_one(endpoint, &path_api)
}

/// One bounded blocking `GET {endpoint}{path}` — `true` iff the response's
/// HTTP status line is in the `2xx` range. See
/// [`probe_attach_session_live`]'s own doc comment for why this is
/// hand-rolled sync TCP rather than `reqwest`.
fn probe_one(endpoint: &str, path: &str) -> bool {
    use std::io::{Read, Write};
    use std::net::{TcpStream, ToSocketAddrs};
    use std::time::Duration;

    const CONNECT_TIMEOUT: Duration = Duration::from_millis(800);
    const IO_TIMEOUT: Duration = Duration::from_millis(800);

    let Some((host, port)) = parse_host_port(endpoint) else { return false };
    let Ok(mut addrs) = (host.as_str(), port).to_socket_addrs() else { return false };
    let Some(addr) = addrs.next() else { return false };
    let Ok(mut stream) = TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT) else { return false };
    let _ = stream.set_read_timeout(Some(IO_TIMEOUT));
    let _ = stream.set_write_timeout(Some(IO_TIMEOUT));

    let request =
        format!("GET {path} HTTP/1.1\r\nHost: {host}:{port}\r\nConnection: close\r\n\r\n");
    if stream.write_all(request.as_bytes()).is_err() {
        return false;
    }

    let mut buf = [0u8; 128];
    let Ok(n) = stream.read(&mut buf) else { return false };
    let text = String::from_utf8_lossy(&buf[..n]);
    // The status line's second whitespace-separated token is the numeric
    // code (`"HTTP/1.1 200 OK"` → `"200"`).
    text.split_whitespace()
        .nth(1)
        .and_then(|code| code.parse::<u16>().ok())
        .is_some_and(|code| (200..300).contains(&code))
}

/// Split `"http://127.0.0.1:4096"` (or `https://…`, or a bare `"host:port"`)
/// into `(host, port)`. `None` for anything that does not carry an explicit
/// port — this probe never guesses a default (attach `endpoint`s are always
/// written with one; see `config.rs`'s own grammar doc).
fn parse_host_port(endpoint: &str) -> Option<(String, u16)> {
    let without_scheme = endpoint
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(endpoint);
    let authority = without_scheme.split('/').next().unwrap_or(without_scheme);
    let (host, port) = authority.rsplit_once(':')?;
    let port: u16 = port.parse().ok()?;
    Some((host.to_string(), port))
}

/// `true` iff `cmd` is directly runnable: an existing file at an absolute or
/// relative path, or a bare name resolvable on `PATH`.
fn resolvable(cmd: &str) -> bool {
    if cmd.contains('/') {
        return Path::new(cmd).is_file();
    }
    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path_var).any(|dir| dir.join(cmd).is_file())
}

/// Answer `query/protocol {version?}` (docs §5.4): the current session
/// protocol range, and — when `version` was asked — whether it falls in it.
pub fn local_protocol(version: Option<u32>) -> ProtocolAnswer {
    ProtocolAnswer {
        session: PROTOCOL_VERSION,
        min: PROTOCOL_MIN,
        max: PROTOCOL_MAX,
        asked: version,
        ok: version.map(|v| (PROTOCOL_MIN..=PROTOCOL_MAX).contains(&v)),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #185
mod tests {
    use super::*;
    use holler_proto::SessionName;

    fn spawn(name: &str, harness: &str, command: &[&str]) -> SessionConfig {
        SessionConfig {
            name: SessionName::parse(name).unwrap(),
            harness: harness.to_string(),
            mode: SessionMode::Spawn,
            command: Some(command.iter().map(|s| s.to_string()).collect()),
            cwd: None,
            env: None,
            interrupt: crate::config::Interrupt::Acp,
            endpoint: None,
            session_id: None,
        }
    }

    #[test]
    fn unknown_feature_is_32006() {
        let err = local_support("not-a-real-id", &[]).expect_err("unknown id must error");
        assert_eq!(err.code, Code::UnknownFeature.jsonrpc());
    }

    #[test]
    fn harness_true_when_command_on_path_false_when_not() {
        let configs = vec![spawn("alpha", "opencode", &["/bin/sh"])];
        let s = local_support("opencode", &configs).expect("known id");
        assert!(s.ok, "an absolute, existing command must be ok:true: {s:?}");

        let configs = vec![spawn("alpha", "opencode", &["/definitely/not/a/real/binary-xyz"])];
        let s = local_support("opencode", &configs).expect("known id");
        assert!(!s.ok, "a missing command must be ok:false: {s:?}");
    }

    #[test]
    fn harness_with_no_configured_session_is_not_ok() {
        let s = local_support("claude", &[]).expect("known id");
        assert!(!s.ok);
    }

    #[test]
    fn capability_ids_are_always_ok() {
        assert!(local_support("attach", &[]).expect("known id").ok);
        assert!(local_support("opencode-http", &[]).expect("known id").ok);
    }

    #[test]
    fn caps_contains_every_known_id() {
        let caps = local_caps(Path::new("/nonexistent"), None, &[]);
        for id in FEATURES.iter().chain(HARNESS_IDS.iter()) {
            assert!(caps.caps.contains_key(*id), "caps is missing {id}");
        }
    }

    #[test]
    fn protocol_with_version_1_is_ok_false() {
        let a = local_protocol(Some(1));
        assert_eq!(a.asked, Some(1));
        assert_eq!(a.ok, Some(false));
    }

    #[test]
    fn protocol_with_current_version_is_ok_true() {
        let a = local_protocol(Some(PROTOCOL_VERSION));
        assert_eq!(a.ok, Some(true));
    }
}
