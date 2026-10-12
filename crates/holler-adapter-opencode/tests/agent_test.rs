#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #642
//! The pane's OpenCode agent, applied and verified (#642 part 3, acceptance 1-6), with no
//! OpenCode: `serve_command` puts `OPENCODE_CONFIG_CONTENT={"default_agent":KEY}` on the
//! server's argv (D1, D2), and `verify_serving` asks the running server, here the stub of
//! `support/stub.rs`, whether it really runs that agent (D4), refusing with the adapter's own
//! `agent-unusable` (D5) when it does not.

#[path = "support/fixtures.rs"]
mod fixtures;
#[path = "support/stub.rs"]
#[allow(dead_code)] // #642: shared with hermetic_test.rs
mod stub;

use std::time::Instant;

use fixtures::*;
use holler_adapter_opencode::tui::tui_argv;
use holler_adapter_opencode::{ProcessEnv, AGENT_UNUSABLE};
use holler_pane::{HarnessEndpoint, HarnessPort, PaneError, ServeSpec};
use holler_pane_testkit::fault::PortOp;
use holler_pane_testkit::harness::HarnessOp;
use stub::{on_refused_port, Stub};

/// The variable that carries the agent (D2).
const VAR: &str = "OPENCODE_CONFIG_CONTENT";
/// A text placed in the stub's replies: a message that quotes a reply body carries it.
const REPLY_MARKER: &str = "zq642-reply-marker";

fn spec(key: &str) -> ServeSpec {
    ServeSpec::default().with_agent(agent(key))
}

/// The one element D2 adds for `key`: compact JSON, `default_agent` only.
fn pair(key: &str) -> String {
    format!(r#"{VAR}={{"default_agent":"{key}"}}"#)
}

/// A `GET /config` reply whose `default_agent` is `key`, shaped like E21's.
fn config_naming(key: &str) -> String {
    format!(
        r#"{{"command":{{}},"plugin":[],"default_agent":"{key}","username":"{REPLY_MARKER}","mode":{{}},"agent":{{}}}}"#
    )
}

/// A stub whose `/config` answers `config` and whose `/agent` answers `agents`.
fn server(config: &str, agents: &str) -> Stub {
    let stub = Stub::start();
    stub.json("GET", "/config", 200, config);
    stub.json("GET", "/agent", 200, agents);
    stub
}

/// The refusal's code and message, or a panic naming what was answered.
fn refused<T: std::fmt::Debug>(result: Result<T, PaneError>, what: &str) -> (String, String) {
    match result {
        Err(PaneError::Refused { code, message }) => (code.as_str().to_owned(), message),
        other => panic!("{what}: expected a refusal, got {other:?}"),
    }
}

// ---- Acceptance 1: the argv ----

/// D2: under `Inherit` the pair follows the prefix's own words, before the binary.
#[test]
fn ac1_an_agent_is_the_servers_default_agent_on_its_argv_under_inherit() {
    let argv = serving(ProcessEnv::Inherit)
        .serve_command(&pane_name(), 48123, &spec("plan"))
        .unwrap();
    assert_eq!(
        strings(&argv),
        [
            "env",
            "-u",
            "OPENCODE_DISABLE_TERMINAL_TITLE",
            r#"OPENCODE_CONFIG_CONTENT={"default_agent":"plan"}"#,
            BIN,
            "serve",
            "--port",
            "48123",
            "--hostname",
            "127.0.0.1",
            "--pure"
        ]
    );
}

/// D2: under `Isolated` the pair comes after the configured pairs.
#[test]
fn ac1_under_isolated_the_pair_follows_the_configured_pairs() {
    let argv = serving(isolated())
        .serve_command(&pane_name(), 48124, &spec("plan"))
        .unwrap();
    assert_eq!(
        strings(&argv),
        [
            "env",
            "-i",
            "HOME=/tmp/hlr802/home",
            "XDG_DATA_HOME=/tmp/hlr802/data",
            r#"OPENCODE_CONFIG_CONTENT={"default_agent":"plan"}"#,
            BIN,
            "serve",
            "--port",
            "48124",
            "--hostname",
            "127.0.0.1",
            "--pure"
        ]
    );
}

/// D2: `env` applies assignments in order, so the adapter's assignment, the later one,
/// replaces a same-named pair configured in `Isolated`.
#[test]
fn ac1_the_adapters_assignment_follows_a_configured_one_of_the_same_name() {
    let env = ProcessEnv::Isolated(vec![
        ("HOME".to_owned(), "/tmp/hlr802/home".to_owned()),
        (VAR.to_owned(), "x".to_owned()),
    ]);
    let argv = serving(env)
        .serve_command(&pane_name(), 48123, &spec("plan"))
        .unwrap();
    let parts = strings(&argv);
    let at = |text: &str| parts.iter().position(|e| *e == text);
    let configured = at(&format!("{VAR}=x")).expect("the configured pair stays");
    let adapters = at(&pair("plan")).expect("the adapter's pair");
    let bin = at(BIN).expect("the binary");
    assert!(configured < adapters && adapters < bin, "{parts:?}");
}

/// D2: each valid key is carried in exactly one element, that element is the pair, and its
/// value is JSON holding `default_agent` and nothing else.
#[test]
fn ac1_each_key_is_carried_in_exactly_one_element() {
    for key in ["plan", "orchestrator", "a-b_c"] {
        let argv = serving(ProcessEnv::Inherit)
            .serve_command(&pane_name(), 48123, &spec(key))
            .unwrap();
        let carrying: Vec<&str> = strings(&argv)
            .into_iter()
            .filter(|e| e.contains(key))
            .collect();
        assert_eq!(carrying, [pair(key).as_str()], "{key}");
        let value = carrying[0].strip_prefix(&format!("{VAR}=")).unwrap();
        let json: serde_json::Value = serde_json::from_str(value).unwrap();
        assert_eq!(json, serde_json::json!({ "default_agent": key }), "{key}");
    }
}

/// D2: with an agent, the server's argv is the TUI's prefix plus exactly the one pair, and
/// without that element it is the argv a spec without an agent gets.
#[test]
fn ac1_with_an_agent_the_server_prefix_is_the_tuis_plus_one_pair() {
    for env in [ProcessEnv::Inherit, isolated()] {
        let h = serving(env.clone());
        let with = h.serve_command(&pane_name(), 48123, &spec("plan")).unwrap();
        let without = h
            .serve_command(&pane_name(), 48123, &ServeSpec::default())
            .unwrap();
        let tui = tui_argv(BIN, &env, 48123, "/p", "ses_1");
        let at = tui.iter().position(|e| e == BIN).unwrap();
        let with = strings(&with);
        assert_eq!(with[..at], tui[..at], "{env:?}: the TUI's prefix");
        assert_eq!(with[at], pair("plan"), "{env:?}: then the one pair");
        assert_eq!(with[at + 1], BIN, "{env:?}: then the binary");
        let mut stripped = with.clone();
        stripped.remove(at);
        assert_eq!(stripped, strings(&without), "{env:?}");
    }
}

// ---- Acceptance 2 and 3: no agent, nothing applied, nothing sent ----

/// No agent: no pair on the argv (#802's byte-for-byte pin stays in `hermetic_test.rs`,
/// `ac8_serve_command_is_the_server_argv_behind_the_env_prefix`), and neither method sends
/// anything: `verify_serving` answers `Ok` from the argv alone (E3).
#[test]
fn ac2_ac3_without_an_agent_nothing_is_applied_and_nothing_is_sent() {
    let stub = Stub::start();
    let h = serving(ProcessEnv::Inherit);
    let argv = h
        .serve_command(&pane_name(), stub.port, &ServeSpec::default())
        .unwrap();
    assert!(strings(&argv).iter().all(|e| !e.contains(VAR)), "{argv:?}");
    assert_eq!(
        h.verify_serving(&HarnessEndpoint::Port(stub.port), &ServeSpec::default()),
        Ok(())
    );
    assert!(stub.lines().is_empty(), "sent: {:?}", stub.lines());
}

// ---- Acceptance 4: a server that runs the agent ----

/// D4: `/config` names the key, and `/agent` lists it as an agent a default may be: a
/// `mode` other than `subagent` (`primary`, or `all`, what a config agent without a `mode`
/// gets, E33) and `hidden` not `true` (`null`, or absent as OpenCode answers it, E21).
/// Exactly the two requests, in that order.
#[test]
fn ac4_a_server_that_runs_the_agent_verifies() {
    let e21 = r#"[{"name":"plan","mode":"primary","native":true,"hidden":null},{"name":"build","mode":"primary","native":true,"hidden":null},{"name":"explore","mode":"subagent","native":true,"hidden":null}]"#;
    let no_hidden = r#"[{"name":"build","mode":"primary","native":true},{"name":"plan","mode":"primary","native":true,"description":"p"}]"#;
    let all = r#"[{"name":"build","mode":"primary"},{"name":"plan","mode":"all","native":false}]"#;
    for (agents, what) in [
        (e21, "E21's list, hidden null"),
        (no_hidden, "no hidden key"),
        (all, "mode all"),
    ] {
        let stub = server(&config_naming("plan"), agents);
        assert_eq!(
            serving(ProcessEnv::Inherit)
                .verify_serving(&HarnessEndpoint::Port(stub.port), &spec("plan")),
            Ok(()),
            "{what}"
        );
        assert_eq!(
            stub.lines(),
            ["GET /config HTTP/1.1", "GET /agent HTTP/1.1"],
            "{what}"
        );
    }
}

// ---- Acceptance 5: a server that does not ----

/// D4, D5: each mismatch is `agent-unusable`, in one line naming the key and the reason, and
/// quoting no reply body.
#[test]
fn ac5_a_server_that_does_not_run_the_agent_is_refused_agent_unusable() {
    let good = format!(r#"[{{"name":"plan","mode":"primary","description":"{REPLY_MARKER}"}}]"#);
    let not_default = "the server's default agent is not plan";
    let cases = [
        (
            config_naming("build"),
            good.clone(),
            not_default,
            "default is build",
        ),
        (
            format!(r#"{{"username":"{REPLY_MARKER}"}}"#),
            good.clone(),
            not_default,
            "no default_agent",
        ),
        (
            config_naming("plan"),
            format!(r#"[{{"name":"build","mode":"primary","description":"{REPLY_MARKER}"}}]"#),
            "no agent named plan",
            "absent",
        ),
        (
            config_naming("plan"),
            format!(r#"[{{"name":"plan","mode":"subagent","description":"{REPLY_MARKER}"}}]"#),
            "agent plan is a subagent",
            "a subagent",
        ),
        (
            config_naming("plan"),
            format!(
                r#"[{{"name":"plan","mode":"primary","hidden":true,"description":"{REPLY_MARKER}"}}]"#
            ),
            "agent plan is hidden",
            "hidden",
        ),
    ];
    for (config, agents, reason, what) in cases {
        let stub = server(&config, &agents);
        let (code, message) = refused(
            serving(ProcessEnv::Inherit)
                .verify_serving(&HarnessEndpoint::Port(stub.port), &spec("plan")),
            what,
        );
        assert_eq!(code, AGENT_UNUSABLE.as_str(), "{what}");
        assert!(message.contains(reason), "{what}: {message:?}");
        assert!(!message.contains(['\n', '\r']), "{what}: one line");
        assert!(
            !message.contains(REPLY_MARKER),
            "{what}: no reply body: {message:?}"
        );
    }
}

// ---- Acceptance 6: a server that cannot answer ----

/// D4: a `200` that is not the JSON the step needs (the web app's page) is `unavailable`,
/// one line naming the route and the port.
#[test]
fn ac6_an_html_200_for_config_or_agent_is_unavailable() {
    let config_html = Stub::start();
    config_html.html("GET", "/config");
    let agent_html = Stub::start();
    agent_html.json("GET", "/config", 200, &config_naming("plan"));
    agent_html.html("GET", "/agent");
    for (stub, route) in [(&config_html, "/config"), (&agent_html, "/agent")] {
        let message = unavailable(
            serving(ProcessEnv::Inherit)
                .verify_serving(&HarnessEndpoint::Port(stub.port), &spec("plan")),
            route,
        );
        assert_one_line_naming(&message, route, route);
        assert!(message.contains(&stub.port.to_string()), "{message:?}");
    }
}

/// D4(b): an element named for the key whose `mode` is not a string is not the JSON the step
/// needs: `unavailable`, not a refusal.
#[test]
fn ac6_an_agent_without_a_string_mode_is_unavailable() {
    for agents in [
        r#"[{"name":"plan","native":true}]"#,
        r#"[{"name":"plan","mode":null}]"#,
        r#"[{"name":"plan","mode":7}]"#,
    ] {
        let stub = server(&config_naming("plan"), agents);
        unavailable(
            serving(ProcessEnv::Inherit)
                .verify_serving(&HarnessEndpoint::Port(stub.port), &spec("plan")),
            agents,
        );
    }
}

/// D4: a refused port is `unavailable`; a server that accepts and never answers is `timeout`
/// with `op` `harness.verify_serving` (`OP_VERIFY_SERVING`, D8), within the call's bound.
#[test]
fn ac6_a_refused_or_frozen_server_is_unavailable_or_timeout() {
    let h = serving(ProcessEnv::Inherit);
    let (_, answer) = on_refused_port(
        |port| h.verify_serving(&HarnessEndpoint::Port(port), &spec("plan")),
        |answer| matches!(answer, Err(PaneError::Unavailable { .. })),
    );
    unavailable(answer, "a refused port");
    let frozen = Stub::start();
    frozen.freeze();
    let start = Instant::now();
    let op = timeout_op(
        h.verify_serving(&HarnessEndpoint::Port(frozen.port), &spec("plan")),
        "a frozen server",
    );
    assert_eq!(op, HarnessOp::VerifyServing.as_str());
    assert_within(
        start,
        quick().call + SLACK,
        "verify_serving of a frozen server",
    );
}
