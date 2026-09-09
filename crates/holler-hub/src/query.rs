//! Local `query/*` document builders for the **hub** role (issue #185):
//! `status`/`caps`/`support`/`protocol`, built from the live registry
//! ([`crate::live::Registry`]) — never a live probe of its own; a harness is
//! `ok:true` here iff the confirmation pass ([`crate::circuit`]) has already
//! proven it against a real body.

use holler_proto::{
    Caps, Code, HelloRole, ProtocolAnswer, Status, Support, SupportKind, WireError, FEATURES,
    HARNESS_IDS, PROTOCOL_MAX, PROTOCOL_MIN, PROTOCOL_VERSION,
};

use crate::live::Registry;

/// The two capability ids (docs §9, §5.3's `kind: "capability"` subset) —
/// the same pair the body side distinguishes (`crate::query` there); kept as
/// a private duplicate rather than a shared crate, since the two roles'
/// `ok`/`reason` semantics differ (a body probes itself; the hub reports on
/// its bodies).
const CAPABILITY_IDS: &[&str] = &["attach", "opencode-http"];

/// The protocol feature ids this hub build actually answers tonight (every
/// other id in [`holler_proto::FEATURES`] is a real, catalogued v2 feature
/// whose hub-side wiring is a **different**, concurrently-landing story —
/// `interrupt` (issue #190) and `roster` (issue #186) — so `ok:false` here is
/// an honest "not yet", not a spec gap).
const HUB_IMPLEMENTED_FEATURES: &[&str] = &["presence", "ping", "query", "token"];

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

fn hub_hostname() -> String {
    hostname::get()
        .map(|h| h.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "unknown".to_string())
}

/// Build the hub's `query/status` document from the live registry.
pub async fn local_status(registry: &Registry, listening: Vec<String>) -> Status {
    Status {
        role: HelloRole::Hub,
        protocol: PROTOCOL_VERSION,
        protocol_min: PROTOCOL_MIN,
        protocol_max: PROTOCOL_MAX,
        hostname: hub_hostname(),
        connected: None,
        token_id: None,
        listening: listening.first().cloned(),
        features: FEATURES.iter().map(|s| (*s).to_string()).collect(),
        harnesses: None,
        harnesses_known: Some(registry.harnesses_known().await),
        harnesses_confirmed: Some(registry.harnesses_confirmed().await),
        bodies: Some(registry.len().await as u32),
        sessions: Some(registry.total_sessions().await),
        session_list: None,
    }
}

/// Build the hub's `query/caps` document: the status doc plus a support
/// answer for every known id in the v2 vocabulary (docs §5.2).
pub async fn local_caps(registry: &Registry, listening: Vec<String>) -> Caps {
    let status = local_status(registry, listening).await;
    let confirmed: std::collections::BTreeSet<String> =
        status.harnesses_confirmed.iter().flatten().map(|c| c.id.clone()).collect();
    let mut caps = std::collections::BTreeMap::new();
    for id in FEATURES.iter().chain(HARNESS_IDS.iter()) {
        caps.insert((*id).to_string(), support_for(id, &confirmed));
    }
    Caps { status, caps }
}

/// Answer `query/support {feature}` for the hub: a harness is `ok:true` iff
/// the confirmation pass has proven it against a live body; a capability id
/// is `ok:false` (the hub does not itself terminate `attach`/`opencode-http`
/// transports — that is body-side); every other feature id is `ok:true` iff
/// this hub build already wires it ([`HUB_IMPLEMENTED_FEATURES`]). An id
/// outside the v2 vocabulary is `-32006 unknown_feature`.
pub async fn local_support(feature: &str, registry: &Registry) -> Result<Support, WireError> {
    if classify(feature).is_none() {
        return Err(WireError::new(
            Code::UnknownFeature,
            format!("unknown feature/harness id: {feature}"),
            None,
        ));
    }
    let confirmed: std::collections::BTreeSet<String> =
        registry.harnesses_confirmed().await.into_iter().map(|c| c.id).collect();
    Ok(support_for(feature, &confirmed))
}

fn support_for(id: &str, confirmed: &std::collections::BTreeSet<String>) -> Support {
    // `classify` is total over FEATURES ∪ HARNESS_IDS (the only ids this
    // function is ever called with — `local_caps` iterates exactly that
    // union, and `local_support` already rejected anything else above), so
    // the `unwrap_or` here is a defensive default, never actually hit.
    let kind = classify(id).unwrap_or(SupportKind::Feature);
    match kind {
        SupportKind::Harness => Support {
            feature: id.to_string(),
            kind,
            ok: confirmed.contains(id),
            how: confirmed.contains(id).then(|| "confirmed".to_string()),
            reason: (!confirmed.contains(id)).then(|| "not confirmed by any live body".to_string()),
        },
        SupportKind::Capability => Support {
            feature: id.to_string(),
            kind,
            ok: false,
            how: None,
            reason: Some("not implemented at the hub (body-side transport)".to_string()),
        },
        SupportKind::Feature => {
            let ok = HUB_IMPLEMENTED_FEATURES.contains(&id);
            Support {
                feature: id.to_string(),
                kind,
                ok,
                how: ok.then(|| "implemented".to_string()),
                reason: (!ok).then(|| "not yet implemented".to_string()),
            }
        }
    }
}

/// Answer `query/protocol {version?}` (docs §5.4).
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

    #[tokio::test]
    async fn unknown_feature_is_32006() {
        let registry = Registry::new();
        let err = local_support("not-a-real-id", &registry).await.expect_err("unknown id must error");
        assert_eq!(err.code, Code::UnknownFeature.jsonrpc());
    }

    #[tokio::test]
    async fn caps_contains_every_known_id() {
        let registry = Registry::new();
        let caps = local_caps(&registry, vec![]).await;
        for id in FEATURES.iter().chain(HARNESS_IDS.iter()) {
            assert!(caps.caps.contains_key(*id), "caps is missing {id}");
        }
    }

    #[tokio::test]
    async fn unconfirmed_harness_is_not_ok() {
        let registry = Registry::new();
        let s = local_support("opencode", &registry).await.expect("known id");
        assert!(!s.ok);
    }

    #[tokio::test]
    async fn confirmed_harness_reflects_real_probe() {
        let registry = Registry::new();
        let mut rx = registry.insert("cli_1", "kiwi", "tok_1").await;
        registry.set_harnesses_advertised("cli_1", vec!["opencode".to_string()]).await;
        // Not yet confirmed: advertising alone is not proof.
        let s = local_support("opencode", &registry).await.expect("known id");
        assert!(!s.ok, "advertised-but-unconfirmed must stay ok:false: {s:?}");

        registry.confirm_harness("cli_1", "opencode").await;
        let s = local_support("opencode", &registry).await.expect("known id");
        assert!(s.ok, "a confirmed harness must be ok:true: {s:?}");

        rx.close();
    }

    #[test]
    fn protocol_with_version_1_is_ok_false() {
        let a = local_protocol(Some(1));
        assert_eq!(a.ok, Some(false));
    }
}
