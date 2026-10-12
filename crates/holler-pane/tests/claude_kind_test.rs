#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #831
//! The Claude Code harness kind and the endpoint a record becomes (#831, ADR-0021 section 15
//! items 1, 3, 5 and 8; plan review W9 and W13).
//!
//! - item 1: `HarnessKind::Claude` (serde `claude`), and the two facts of a kind, `shape()` and
//!   `abort_style()`, as pure methods of the kind (the one place they live);
//! - item 3: `HarnessInfo.port` is optional, a record is valid only as `opencode` with a port or
//!   `claude` with a pid and no port, an `opencode` record keeps its exact bytes, and
//!   `HarnessInfo::endpoint` is the one place a record becomes a `HarnessEndpoint`;
//! - item 5 and W9: the four process methods are default-armed and each default refuses
//!   `process-unsupported` (exit 3), `login_status` included, so a wrapper that forgets a forward
//!   fails loudly instead of answering "not applicable";
//! - item 8: `Activity::WaitingPermission`;
//! - W13 / item 12: the snapshot of a `claude` record names the port policy `none`, so a spec
//!   round-trips.

use holler_pane::error::{class_of, ErrorClass};
use holler_pane::pane::{AbortStyle, HarnessInfo, HarnessKind, Health, Shape};
use holler_pane::ports::{Activity, SessionChoice, PROCESS_UNSUPPORTED};
use holler_pane::profile_snapshot::spec_from_pane;
use holler_pane::{
    Argv, HarnessEndpoint, HarnessPort, Pane, PaneError, PaneId, PaneName, ServeSpec,
};
use serde_json::json;

mod common;

/// `common::pane()` (an opencode record) as a `claude` record with the pid `pid`.
fn sample_claude_pane(pid: u32) -> Pane {
    Pane {
        harness: HarnessInfo {
            kind: HarnessKind::Claude,
            port: None,
            pid: Some(pid),
            health: Health::Healthy,
        },
        ..common::pane()
    }
}

fn name(text: &str) -> PaneName {
    PaneName::parse(text).unwrap()
}

// --- item 1: the kind -------------------------------------------------------------------

#[test]
fn the_claude_kind_is_serde_claude_and_in_the_protocol_vocabulary() {
    assert_eq!(
        serde_json::to_value(HarnessKind::Claude).unwrap(),
        json!("claude")
    );
    let back: HarnessKind = serde_json::from_value(json!("claude")).unwrap();
    assert_eq!(back, HarnessKind::Claude);
    assert!(holler_proto::HARNESS_IDS.contains(&"claude"));
}

#[test]
fn a_kind_knows_its_shape_and_its_abort_style() {
    assert_eq!(HarnessKind::Opencode.shape(), Shape::ServerAndTui);
    assert_eq!(HarnessKind::Claude.shape(), Shape::SingleProcess);
    assert_eq!(HarnessKind::Opencode.abort_style(), AbortStyle::InPlace);
    assert_eq!(
        HarnessKind::Claude.abort_style(),
        AbortStyle::Relaunch,
        "the operator's decision: abort for a Claude Code pane is stop and relaunch"
    );
}

// --- item 3: the record and its endpoint ---------------------------------------------------

#[test]
fn an_opencode_record_keeps_its_exact_bytes() {
    let stored = json!({"kind": "opencode", "port": 48100, "health": "healthy"});
    let info: HarnessInfo = serde_json::from_value(stored.clone()).unwrap();
    assert_eq!(info.port, Some(48100));
    assert_eq!(info.pid, None);
    assert_eq!(
        serde_json::to_value(&info).unwrap(),
        stored,
        "the same bytes back"
    );
}

#[test]
fn a_claude_record_has_a_pid_and_no_port() {
    let stored = json!({"kind": "claude", "pid": 4242, "health": "unknown"});
    let info: HarnessInfo = serde_json::from_value(stored.clone()).unwrap();
    assert_eq!(info.kind, HarnessKind::Claude);
    assert_eq!(info.port, None);
    assert_eq!(info.pid, Some(4242));
    let written = serde_json::to_value(&info).unwrap();
    assert_eq!(written, stored);
    assert!(written.get("port").is_none(), "no port key: {written}");
}

#[test]
fn a_record_whose_harness_does_not_fit_its_kind_does_not_load() {
    for (why, harness) in [
        (
            "opencode without a port",
            json!({"kind": "opencode", "health": "unknown"}),
        ),
        (
            "claude with a port",
            json!({"kind": "claude", "port": 48100, "pid": 4242, "health": "unknown"}),
        ),
        (
            "claude without a pid",
            json!({"kind": "claude", "health": "unknown"}),
        ),
    ] {
        assert!(
            serde_json::from_value::<HarnessInfo>(harness.clone()).is_err(),
            "{why} must not load: {harness}"
        );
        let mut pane = common::pane_json();
        pane["harness"] = harness;
        assert!(
            serde_json::from_value::<Pane>(pane).is_err(),
            "a pane record with {why} must not load (fail closed)"
        );
    }
}

#[test]
fn the_endpoint_of_an_opencode_record_is_its_port() {
    let info: HarnessInfo =
        serde_json::from_value(json!({"kind": "opencode", "port": 48100, "health": "healthy"}))
            .unwrap();
    assert_eq!(
        info.endpoint(&name("demo-c1r1")).unwrap(),
        HarnessEndpoint::Port(48100)
    );
}

#[test]
fn the_endpoint_of_a_claude_record_is_its_pane_and_pid() {
    let info: HarnessInfo =
        serde_json::from_value(json!({"kind": "claude", "pid": 4242, "health": "healthy"}))
            .unwrap();
    assert_eq!(
        info.endpoint(&name("demo-c1r1")).unwrap(),
        HarnessEndpoint::Process {
            pane: name("demo-c1r1"),
            pid: 4242
        }
    );
}

#[test]
fn a_harness_built_in_code_that_does_not_fit_its_kind_has_no_endpoint() {
    for info in [
        HarnessInfo {
            kind: HarnessKind::Opencode,
            port: None,
            pid: None,
            health: Health::Unknown,
        },
        HarnessInfo {
            kind: HarnessKind::Claude,
            port: None,
            pid: None,
            health: Health::Unknown,
        },
    ] {
        assert!(
            info.endpoint(&name("demo-c1r1")).is_err(),
            "no endpoint is guessed for {info:?}"
        );
    }
}

// --- W13 / item 12: the snapshot's port policy -------------------------------------------

#[test]
fn the_spec_of_a_claude_record_names_the_port_policy_none() {
    let spec = spec_from_pane(&sample_claude_pane(4242));
    assert_eq!(spec.harness.kind, HarnessKind::Claude);
    assert_eq!(spec.harness.port_policy, "none");
}

#[test]
fn the_spec_of_an_opencode_record_still_names_its_fixed_port() {
    let pane = common::pane();
    let port = pane
        .harness
        .port
        .expect("the sample is an opencode record with a port");
    assert_eq!(
        spec_from_pane(&pane).harness.port_policy,
        format!("fixed:{port}")
    );
}

// --- item 5 and W9: the default-armed process methods -------------------------------------

/// A harness that implements only the required methods, as every server harness does.
struct ServerOnly;

impl HarnessPort for ServerOnly {
    fn serve_command(&self, _: &PaneName, _: u16, _: &ServeSpec) -> Result<Argv, PaneError> {
        Ok(Argv::new(vec!["serve".to_owned()]))
    }
    fn verify_serving(&self, _: &HarnessEndpoint, _: &ServeSpec) -> Result<(), PaneError> {
        Ok(())
    }
    fn health(&self, _: &HarnessEndpoint) -> Result<bool, PaneError> {
        Ok(true)
    }
    fn create_session(&self, _: &HarnessEndpoint) -> Result<String, PaneError> {
        Ok("ses_1".to_owned())
    }
    fn list_sessions(&self, _: &HarnessEndpoint) -> Result<Vec<String>, PaneError> {
        Ok(Vec::new())
    }
    fn abort(&self, _: &HarnessEndpoint, _: &str) -> Result<(), PaneError> {
        Ok(())
    }
    fn attach_tui(&self, _: &PaneId, _: &HarnessEndpoint, _: &str) -> Result<(), PaneError> {
        Ok(())
    }
    fn select_session(&self, _: &PaneId, _: &str) -> Result<(), PaneError> {
        Ok(())
    }
    fn shown_session(&self, _: &PaneId, _: &HarnessEndpoint) -> Result<Option<String>, PaneError> {
        Ok(None)
    }
}

/// `answer` is the refusal `process-unsupported`, class refusal (exit 3).
fn assert_process_unsupported<T: std::fmt::Debug>(call: &str, answer: Result<T, PaneError>) {
    let error = answer.expect_err(&format!("{call}: the default refuses"));
    assert_eq!(error.code(), "process-unsupported", "{call}: {error}");
    assert_eq!(class_of(error.code()), ErrorClass::Refusal, "{call}");
    assert_eq!(ErrorClass::Refusal.exit_code(), 3);
}

#[test]
fn process_unsupported_is_a_refusal_with_exit_3() {
    assert_eq!(PROCESS_UNSUPPORTED.as_str(), "process-unsupported");
    assert_eq!(class_of(PROCESS_UNSUPPORTED.as_str()), ErrorClass::Refusal);
}

#[test]
fn every_process_method_defaults_to_process_unsupported_login_status_included() {
    let h = ServerOnly;
    let pane = name("demo-c1r1");
    let spec = ServeSpec::default();
    assert_process_unsupported(
        "process_command",
        h.process_command(&pane, &spec, &SessionChoice::Fresh("s-1".to_owned())),
    );
    assert_process_unsupported("new_session_id", h.new_session_id());
    assert_process_unsupported("locate_process", h.locate_process(&pane, "s-1"));
    assert_process_unsupported(
        "login_status (W9: never a silent NotApplicable)",
        h.login_status(),
    );
}

#[test]
fn a_session_choice_names_its_session() {
    for (choice, id) in [
        (SessionChoice::Fresh("a".to_owned()), "a"),
        (SessionChoice::Keep("b".to_owned()), "b"),
        (SessionChoice::Resume("c".to_owned()), "c"),
    ] {
        assert_eq!(choice.session(), id, "{choice:?}");
    }
}

#[test]
fn a_serve_spec_carries_the_model_and_the_directory_only_when_asked() {
    let bare = ServeSpec::default();
    assert_eq!(
        bare.model, None,
        "OpenCode's spec keeps no model until #829"
    );
    assert_eq!(bare.cwd, None);
    let model = holler_pane::pane::ModelSpec {
        provider: "anthropic".to_owned(),
        model_id: "demo-model".to_owned(),
        effort: "high".to_owned(),
    };
    let spec = ServeSpec::default()
        .with_model(model.clone())
        .with_cwd("/srv/demo");
    assert_eq!(spec.model, Some(model));
    assert_eq!(spec.cwd.as_deref(), Some("/srv/demo"));
}

// --- item 8: the permission wait ----------------------------------------------------------

#[test]
fn activity_tells_a_permission_wait_from_a_question() {
    assert_ne!(Activity::WaitingPermission, Activity::HoldingQuestion);
    assert_ne!(Activity::WaitingPermission, Activity::Busy);
    assert_ne!(Activity::WaitingPermission, Activity::Idle);
}
