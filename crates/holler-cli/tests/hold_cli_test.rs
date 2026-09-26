#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #443
//! Session hold, CLI (issue #443, umbrella #437; tag `test-grp-invoc`): the
//! real `holler` binary against a real hub, a real body and `stub-acp` —
//! `hold`, `roster` showing it, `say` refused with the reason, `release`,
//! `say` works — for both text and `--json`.
//!
//! The `--json` shapes are pinned by the files in `tests/fixtures/hold/`
//! (each value replaced by its JSON type, so a renamed, added or removed key
//! fails here). The exit code for a held session is pinned by
//! [`HELD_EXIT`] and by the docs conformance test at the bottom.

mod support;

use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use serde_json::Value;
use support::{holler_cmd, join, mint_token, roster_json, wait_for, write_sessions_toml, Body, Hub, StateDir, STARTUP_WAIT};

/// The documented exit code of a `say` refused because the session is held.
const HELD_EXIT: i32 = 4;

struct Out {
    code: i32,
    stdout: String,
    stderr: String,
}

fn run(state: &StateDir, args: &[&str]) -> Out {
    let o = holler_cmd(state)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn holler")
        .wait_with_output()
        .expect("wait on holler");
    Out {
        code: o.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&o.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
    }
}

/// One hub, one body labelled `b` hosting `sessions`, all in one state dir.
fn rig(sessions: &[(&str, &[&str])]) -> (StateDir, Hub, Body) {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let (token_id, secret) = mint_token(&state, "b");
    join(&state, &state, &hub.ws_url(), &token_id, &secret);
    let config = write_sessions_toml(&state, sessions);
    let body = Body::start(&state, &config);
    for (name, _) in sessions {
        let want = format!("b/{name}");
        wait_for(STARTUP_WAIT, || {
            roster_json(&state)["rows"].as_array()?.iter().any(|r| r["name"] == want.as_str() && r["state"] == "idle").then_some(())
        })
        .unwrap_or_else(|| panic!("session {want} never came up idle"));
    }
    (state, hub, body)
}

/// Replace every scalar with its type name, recursively.
fn shape(v: &Value) -> Value {
    match v {
        Value::Object(m) => Value::Object(m.iter().map(|(k, v)| (k.clone(), shape(v))).collect()),
        Value::Array(a) => Value::Array(a.iter().map(shape).collect()),
        Value::String(_) => "string".into(),
        Value::Number(_) => "number".into(),
        Value::Bool(_) => "bool".into(),
        Value::Null => "null".into(),
    }
}

fn assert_shape(name: &str, actual: &Value) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/hold").join(format!("{name}.shape.json"));
    let got = shape(actual);
    if std::env::var("BLESS_SHAPES").is_ok() {
        std::fs::write(&path, format!("{}\n", serde_json::to_string_pretty(&got).unwrap())).unwrap();
        return;
    }
    let want: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("missing {}", path.display()))).unwrap();
    assert_eq!(got, want, "the --json shape of `{name}` changed (BLESS_SHAPES=1 to rewrite {} after review)", path.display());
}

fn json_of(o: &Out) -> Value {
    serde_json::from_str(o.stdout.trim()).unwrap_or_else(|e| panic!("stdout is not one JSON object ({e}): {:?}", o.stdout))
}

/// `say` in each delivery variant, and the redirect form of `interrupt`, is
/// refused with the reason and the since-time; plain `interrupt` is not.
fn assert_say_refused_everywhere(state: &StateDir) {
    for args in [&["say", "b/alpha", "hi"][..], &["say", "b/alpha", "hi", "--queue"], &["say", "alpha", "hi"]] {
        let o = run(state, args);
        assert_eq!(o.code, HELD_EXIT, "{args:?}: {}", o.stderr);
        assert!(o.stderr.contains("session_held") && o.stderr.contains("deploy freeze"), "{}", o.stderr);
        assert!(o.stderr.contains("since 20"), "the since-time is shown: {}", o.stderr);
        assert!(o.stdout.is_empty());
    }
    let redirect = run(state, &["interrupt", "b/alpha", "do this instead"]);
    assert_eq!(redirect.code, HELD_EXIT, "{}", redirect.stderr);
    assert_eq!(run(state, &["interrupt", "b/alpha"]).code, 0, "interrupt still works on a held session");
}

#[test]
fn hold_roster_say_refused_release_say_works_text() {
    let (state, _hub, body) = rig(&[("alpha", &["--chunks", "1"])]);

    let held = run(&state, &["hold", "b/alpha", "--reason", "deploy freeze"]);
    assert_eq!(held.code, 0, "{}", held.stderr);
    assert!(held.stdout.contains("held b/alpha since "), "{}", held.stdout);
    assert!(held.stdout.contains("deploy freeze"), "{}", held.stdout);

    // Repeating a hold is a success and says so, keeping the reason.
    let again = run(&state, &["hold", "b/alpha", "--reason", "something else"]);
    assert_eq!(again.code, 0);
    assert!(again.stdout.starts_with("already held b/alpha"), "{}", again.stdout);
    assert!(again.stdout.contains("deploy freeze") && !again.stdout.contains("something else"), "{}", again.stdout);

    // The roster table shows the hold and its reason.
    let table = run(&state, &["roster"]);
    assert_eq!(table.code, 0);
    assert!(table.stdout.contains("HOLD"), "{}", table.stdout);
    let row = table.stdout.lines().find(|l| l.starts_with("b/alpha")).unwrap();
    assert!(row.contains("held ") && row.contains("deploy freeze"), "{row}");

    assert_say_refused_everywhere(&state);

    // release restores say; repeating it is a success no-op.
    let released = run(&state, &["release", "b/alpha"]);
    assert_eq!((released.code, released.stdout.trim()), (0, "released b/alpha"));
    let again = run(&state, &["release", "b/alpha"]);
    assert_eq!((again.code, again.stdout.trim()), (0, "b/alpha was not held"));
    let said = run(&state, &["say", "b/alpha", "hi"]);
    assert_eq!(said.code, 0, "{}", said.stderr);
    assert!(said.stdout.contains("stub chunk"));
    body.stop(&state, Duration::from_secs(10));
}

#[test]
fn hold_release_roster_and_say_json_shapes_are_pinned() {
    let (state, _hub, body) = rig(&[("alpha", &["--chunks", "1"]), ("beta", &["--chunks", "1"])]);

    let held = run(&state, &["hold", "b/alpha", "--reason", "deploy freeze", "--json"]);
    assert_eq!(held.code, 0, "{}", held.stderr);
    let held = json_of(&held);
    assert_eq!((held["hold"].as_bool(), held["newly_held"].as_bool(), held["reason"].as_str()), (Some(true), Some(true), Some("deploy freeze")));
    assert_shape("hold", &held);
    // A hold without a reason has no `reason` key at all... (null on the control reply)
    let bare = json_of(&run(&state, &["hold", "b/beta", "--json"]));
    assert_shape("hold_no_reason", &bare);

    // Roster --json: held rows carry the wire fields, unheld rows carry none.
    let roster = roster_json(&state);
    let rows = roster["rows"].as_array().unwrap();
    let alpha = rows.iter().find(|r| r["name"] == "b/alpha").unwrap();
    assert_eq!((alpha["hold"].as_bool(), alpha["hold_reason"].as_str()), (Some(true), Some("deploy freeze")));
    assert!(alpha["held_since"].as_str().is_some());
    let hold_fields: Value = ["hold", "hold_reason", "held_since"].iter().map(|k| (k.to_string(), alpha[*k].clone())).collect::<serde_json::Map<_, _>>().into();
    assert_shape("roster_row_hold_fields", &hold_fields);
    let unheld_after_release = {
        run(&state, &["release", "b/beta"]);
        roster_json(&state)["rows"].as_array().unwrap().iter().find(|r| r["name"] == "b/beta").cloned().unwrap()
    };
    for k in ["hold", "hold_reason", "held_since"] {
        assert!(unheld_after_release.get(k).is_none(), "{k} must be absent on an unheld row");
    }

    // say --json to a held session: the refusal is one JSON object on stdout, same exit code.
    let refused = run(&state, &["say", "b/alpha", "hi", "--json"]);
    assert_eq!(refused.code, HELD_EXIT);
    let refused = json_of(&refused);
    assert_eq!((refused["error"].as_str(), refused["session"].as_str(), refused["reason"].as_str()), (Some("session_held"), Some("b/alpha"), Some("deploy freeze")));
    assert_shape("say_held", &refused);

    let released = json_of(&run(&state, &["release", "b/alpha", "--json"]));
    assert_eq!((released["hold"].as_bool(), released["was_held"].as_bool()), (Some(false), Some(true)));
    assert_shape("release", &released);
    // Unheld session, --json: unaffected.
    assert_eq!(run(&state, &["say", "b/alpha", "hi", "--json"]).code, 0);
    body.stop(&state, Duration::from_secs(10));
}

#[test]
fn unknown_and_ambiguous_sessions_use_the_control_command_conventions() {
    let (state, _hub, body) = rig(&[("alpha", &["--chunks", "1"])]);
    for args in [&["hold", "b/nope"][..], &["release", "b/nope"], &["hold", "nope", "--reason", "x"]] {
        let o = run(&state, args);
        assert_eq!(o.code, 1, "{args:?}");
        assert!(o.stderr.contains("unknown session"), "{}", o.stderr);
    }
    assert!(roster_json(&state)["rows"].as_array().unwrap().iter().all(|r| r.get("hold").is_none()), "no phantom hold");
    body.stop(&state, Duration::from_secs(10));
}

#[test]
fn an_ambiguous_bare_name_lists_the_candidates_and_exits_2() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let mut bodies = Vec::new();
    let mut body_states = Vec::new();
    for label in ["one", "two"] {
        let bs = StateDir::new();
        let (token_id, secret) = mint_token(&state, label);
        join(&bs, &state, &hub.ws_url(), &token_id, &secret);
        let config = write_sessions_toml(&bs, &[("alpha", &["--chunks", "1"])]);
        bodies.push(Body::start(&bs, &config));
        body_states.push(bs);
    }
    wait_for(STARTUP_WAIT, || (roster_json(&state)["rows"].as_array()?.len() == 2).then_some(())).expect("two rows");
    let o = run(&state, &["hold", "alpha"]);
    assert_eq!(o.code, 2, "{}", o.stderr);
    assert!(o.stderr.contains("one/alpha") && o.stderr.contains("two/alpha"), "candidates are listed: {}", o.stderr);
    for (b, bs) in bodies.into_iter().zip(&body_states) {
        b.stop(bs, Duration::from_secs(10));
    }
}

#[test]
fn a_held_session_does_not_affect_another_over_the_cli() {
    let (state, _hub, body) = rig(&[("alpha", &["--chunks", "1"]), ("beta", &["--chunks", "1"])]);
    assert_eq!(run(&state, &["hold", "b/alpha"]).code, 0);
    assert_eq!(run(&state, &["say", "b/alpha", "hi"]).code, HELD_EXIT);
    assert_eq!(run(&state, &["say", "b/beta", "hi"]).code, 0);
    body.stop(&state, Duration::from_secs(10));
}

// --- docs conformance -----------------------------------------------------------

#[test]
fn the_held_exit_code_and_the_new_verbs_are_documented() {
    let doc = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/protocol/v2.md")).unwrap();
    let section = &doc[doc.find("## 10. CLI mapping").unwrap()..];
    assert!(section.contains("holler hold SESSION") && section.contains("holler release SESSION"));
    assert!(
        section.contains(&format!("exits {HELD_EXIT}")) && section.contains("session_held"),
        "§10 must document that a say refused with session_held exits {HELD_EXIT}"
    );
    assert_eq!(holler_cli::hold_cmd::HELD_EXIT_CODE, HELD_EXIT);
}
