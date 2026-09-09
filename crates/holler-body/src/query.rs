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
        })
        .collect();

    Status {
        role: HelloRole::Body,
        protocol: PROTOCOL_VERSION,
        protocol_min: PROTOCOL_MIN,
        protocol_max: PROTOCOL_MAX,
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
/// only, never a model.
///
/// - A **harness** id is `ok:true` when some configured session names that
///   harness and either (a) it is `attach` mode (issue #185: "does the
///   attach endpoint answer" — this probe does not dial out to verify that;
///   an `attach` session's mere presence in config is treated as `ok:true,
///   how:"attach"`, a decision documented in the PR and left for a follow-up
///   story to make a real probe), or (b) it is `spawn` mode and
///   `command[0]` resolves on `PATH` or as an existing path.
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
                return Support {
                    feature: harness.to_string(),
                    kind: SupportKind::Harness,
                    ok: true,
                    how: Some("attach".to_string()),
                    reason: None,
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
