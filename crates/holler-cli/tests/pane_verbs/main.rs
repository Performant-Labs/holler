#![allow(clippy::unwrap_used)] // #670
#![allow(clippy::expect_used)] // #670
#![allow(clippy::panic)] // #670
#![allow(clippy::unreachable)] // #670
#![allow(dead_code)] // #670
//! In-process tests of the `holler pane` verbs (story #670, epic #633).
//!
//! One module per verb, so a verb story edits only `pane_verbs/<verb>.rs` and never
//! this file or the manifest. The shared pieces: the harness that runs a verb over
//! captured writers (`tests/verb_harness/mod.rs`), the output API (`output_api`), the
//! shared spec-flag validation (`spec_flags`) and the `say`/`interrupt`/`answer`
//! target accessors (`target_flags`).
//!
//! The process-level tests of the whole surface are a separate target,
//! `pane_cli_process`.

#[path = "../verb_harness/mod.rs"]
mod verb_harness;

mod output_api;
mod spec_flags;
mod target_flags;

mod close;
mod doctor;
mod driven;
mod get;
mod import;
mod launch;
mod list;
mod park;
mod relaunch;
mod reset;
mod switch;
mod unmanaged;
mod unpark;
mod watch;

/// The unwired port set is the one not-implemented port set (#649 builds the real wiring
/// and keeps `Unwired`): every port it hands a verb answers
/// `not-implemented`, so a verb that runs before its ports exist fails loudly instead of
/// acting. The harness's `run_verb` runs over it, and nothing here builds the real wiring.
#[test]
fn unwired_ports_answer_not_implemented() {
    let ports = verb_harness::unwired_ports();
    let codes = [
        ports.pane_store.list().err().map(|e| e.code().to_string()),
        ports.herdr.version().err().map(|e| e.code().to_string()),
        ports
            .harness
            .health(&holler_pane::HarnessEndpoint::Port(1))
            .err()
            .map(|e| e.code().to_string()),
    ];
    for code in codes {
        assert_eq!(code.as_deref(), Some("not-implemented"));
    }
    // #802: the amended methods are unwired too; and #831's four process methods, whose
    // defaults would answer `process-unsupported`, not `not-implemented`.
    let name = holler_pane::PaneName::parse("demo-c1r1").unwrap();
    let spec = holler_pane::ServeSpec::default();
    let argv = holler_pane::Argv::new(vec!["true".to_owned()]);
    let choice = holler_pane::ports::SessionChoice::Fresh("s-1".to_owned());
    let amended = [
        ports.harness.serve_command(&name, 1, &spec).map(drop),
        ports
            .harness
            .verify_serving(&holler_pane::HarnessEndpoint::Port(1), &spec),
        ports.host.run(&name, "/", &argv),
        ports
            .harness
            .process_command(&name, &spec, &choice)
            .map(drop),
        ports.harness.new_session_id().map(drop),
        ports.harness.locate_process(&name, "s-1").map(drop),
        ports.harness.login_status().map(drop),
        // #647 part 2: the two required reads.
        ports.host.attached_clients(&name).map(drop),
        ports
            .harness
            .agent_view(&holler_pane::HarnessEndpoint::Port(1), "ses_1", 3)
            .map(drop),
    ];
    for answer in amended {
        let code = answer.err().map(|e| e.code().to_string());
        assert_eq!(code.as_deref(), Some("not-implemented"));
    }
    // #839: the hub port is unwired too; a default would answer silently for a wrapper.
    let body = holler_pane::hub::Advertised {
        token: "tok_1".to_owned(),
        name: "kiwi/oc".to_owned(),
        harness_session_id: "ses_00000000000000000000000001".to_owned(),
        state: holler_pane::ports::Activity::Idle,
    };
    let check = holler_pane::hub::PromptMode::Check;
    let hub = [
        ports.hub.advertised().map(drop),
        ports.hub.rebind(&body, "ses_00000000000000000000000002").map(drop),
        ports.hub.prompt("ses_00000000000000000000000001", "hi", check),
    ];
    for answer in hub {
        let code = answer.err().map(|e| e.code().to_string());
        assert_eq!(code.as_deref(), Some("not-implemented"));
    }
}
