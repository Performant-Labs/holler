#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #460
//! Join held and the one-time release grant, CLI (issue #460, on #437; tag
//! `test-grp-invoc`): the real `holler` binary against a real hub started with
//! `--join-held`, a real body and `stub-acp` — `release --once` prints a grant
//! id, `say --grant` gets exactly one prompt through, and every refusal has its
//! own exit code, for text and `--json`. The `--json` shapes are pinned by
//! `tests/fixtures/hold/*.shape.json` (types only).

mod support;

use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use serde_json::Value;
use support::hold_rig::{Rig, SESSION};
use support::{holler_cmd, StateDir, STARTUP_WAIT};

const HELD_EXIT: i32 = 4;
const INVALID_GRANT_EXIT: i32 = 5;

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

/// `holler say ...`, retried while the hub answers `session_busy` (nothing is
/// delivered by a busy refusal, so a retry cannot double-deliver; the hub's
/// presence cache can trail the roster row by a moment after a turn ends).
fn say_retrying(state: &StateDir, args: &[&str]) -> Out {
    let deadline = std::time::Instant::now() + STARTUP_WAIT;
    loop {
        let o = run(state, args);
        if !o.stderr.contains("session_busy") || std::time::Instant::now() >= deadline {
            return o;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

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

fn joined_held_rig(sessions: &[(&str, &[&str])]) -> Rig {
    Rig::start_with_args(sessions, &[], &["--join-held"])
}

#[test]
fn release_once_prints_the_grant_and_say_grant_gets_exactly_one_prompt_through() {
    let rig = joined_held_rig(&[("alpha", &["--chunks", "1"])]);
    let state = &rig.hub_state;

    // A session that joined held refuses a plain say (exit 4), and the message says how to get in.
    let refused = run(state, &["say", SESSION, "hi"]);
    assert_eq!(refused.code, HELD_EXIT, "{}", refused.stderr);
    assert!(refused.stderr.contains("held on join") && refused.stderr.contains("release --once"), "{}", refused.stderr);

    // The roster shows the kind.
    let table = run(state, &["roster"]).stdout;
    let row = table.lines().find(|l| l.starts_with("b/alpha")).unwrap();
    assert!(row.contains("held (default)") && row.contains("held on join"), "{row}");

    // release --once prints the grant id, alone, on stdout.
    let minted = run(state, &["release", SESSION, "--once"]);
    assert_eq!(minted.code, 0, "{}", minted.stderr);
    let grant = minted.stdout.trim().to_string();
    assert!(grant.starts_with("gnt_") && !grant.contains(char::is_whitespace), "{:?}", minted.stdout);
    assert!(!minted.stderr.contains("warning"), "no warning for the normal case: {}", minted.stderr);

    // A plain say is still refused while the grant is live; the grant gets one prompt through.
    assert_eq!(run(state, &["say", SESSION, "hi"]).code, HELD_EXIT);
    let ok = say_retrying(state, &["say", SESSION, "hi", "--grant", &grant]);
    assert_eq!(ok.code, 0, "{}", ok.stderr);
    assert!(ok.stdout.contains("stub chunk"));
    // The second use is refused with its own exit code, and so is a plain say.
    let again = run(state, &["say", SESSION, "hi", "--grant", &grant]);
    assert_eq!(again.code, INVALID_GRANT_EXIT, "{}", again.stderr);
    assert!(again.stderr.contains("invalid_grant") && again.stderr.contains("already been used"), "{}", again.stderr);
    assert_eq!(run(state, &["say", SESSION, "hi"]).code, HELD_EXIT);
    // An unknown grant id.
    let unknown = run(state, &["say", SESSION, "hi", "--grant", "gnt_00000000000000000000000000000000"]);
    assert_eq!(unknown.code, INVALID_GRANT_EXIT);
    assert!(unknown.stderr.contains("unknown grant"), "{}", unknown.stderr);
}

#[test]
fn the_grant_json_shapes_are_pinned() {
    let rig = joined_held_rig(&[("alpha", &["--chunks", "1"])]);
    let state = &rig.hub_state;
    let minted = json_of(&run(state, &["release", SESSION, "--once", "--ttl", "5m", "--json"]));
    assert_eq!((minted["ttl_ms"].as_u64(), minted["default_held"].as_bool(), minted["operator_held"].as_bool()), (Some(300_000), Some(true), Some(false)));
    assert_shape("release_once", &minted);
    let grant = minted["grant"].as_str().unwrap().to_string();

    let held = run(state, &["say", SESSION, "hi", "--json"]);
    assert_eq!(held.code, HELD_EXIT);
    let held = json_of(&held);
    assert_eq!((held["hold_kind"].as_str(), held["reason"].as_str()), (Some("default"), Some("held on join")));
    assert_shape("say_held", &held);

    assert_eq!(say_retrying(state, &["say", SESSION, "hi", "--grant", &grant, "--json"]).code, 0);
    let used = run(state, &["say", SESSION, "hi", "--grant", &grant, "--json"]);
    assert_eq!(used.code, INVALID_GRANT_EXIT);
    let used = json_of(&used);
    assert_eq!((used["error"].as_str(), used["reason"].as_str()), (Some("invalid_grant"), Some("used")));
    assert_shape("say_invalid_grant", &used);

    // Roster --json: the kind is a wire field.
    let roster = json_of(&run(state, &["roster", "--json"]));
    let row = roster["rows"].as_array().unwrap().iter().find(|r| r["name"] == SESSION).unwrap().clone();
    let fields: Value = ["hold", "hold_reason", "held_since", "hold_kind"].iter().map(|k| (k.to_string(), row[*k].clone())).collect::<serde_json::Map<_, _>>().into();
    assert_shape("roster_row_default_hold_fields", &fields);

    // Plain release peels the default hold: shape of the `lifted` reply.
    let released = json_of(&run(state, &["release", SESSION, "--json"]));
    assert_eq!((released["lifted"].as_str(), released["still_held"].as_bool(), released["hold"].as_bool()), (Some("default"), Some(false), Some(false)));
    assert_shape("release", &released);
}

#[test]
fn an_operator_hold_makes_release_once_warn_and_beats_the_grant() {
    let rig = joined_held_rig(&[("alpha", &["--chunks", "1"])]);
    let state = &rig.hub_state;
    assert_eq!(run(state, &["hold", SESSION, "--reason", "drain"]).code, 0);
    let table = run(state, &["roster"]).stdout;
    let row = table.lines().find(|l| l.starts_with("b/alpha")).unwrap();
    assert!(row.contains("drain") && row.contains("[+default]"), "{row}");

    let minted = run(state, &["release", SESSION, "--once"]);
    assert_eq!(minted.code, 0);
    assert!(minted.stderr.contains("operator hold"), "{}", minted.stderr);
    let grant = minted.stdout.trim().to_string();
    let refused = run(state, &["say", SESSION, "hi", "--grant", &grant]);
    assert_eq!(refused.code, HELD_EXIT, "{}", refused.stderr);
    assert!(refused.stderr.contains("drain") && refused.stderr.contains("ask whoever holds it"), "the operator hold is the reason: {}", refused.stderr);

    // Plain release peels the operator hold, then the default hold.
    let first = run(state, &["release", SESSION]);
    assert!(first.stdout.contains("still held by default"), "{}", first.stdout);
    assert_eq!(say_retrying(state, &["say", SESSION, "hi", "--grant", &grant]).code, 0, "the grant was not spent by the refusal");
    let second = run(state, &["release", SESSION]);
    assert!(second.stdout.contains("released the default hold"), "{}", second.stdout);
    assert_eq!(say_retrying(state, &["say", SESSION, "hi"]).code, 0);
}

#[test]
fn argument_errors_have_their_own_exit_codes() {
    let rig = joined_held_rig(&[("alpha", &["--chunks", "1"])]);
    let state = &rig.hub_state;
    let bad_ttl = run(state, &["release", SESSION, "--once", "--ttl", "soon"]);
    assert_eq!(bad_ttl.code, 3, "{}", bad_ttl.stderr);
    assert_eq!(run(state, &["release", SESSION, "--ttl", "5m"]).code, 2, "--ttl needs --once");
    let unknown = run(state, &["release", "b/nope", "--once"]);
    assert_eq!(unknown.code, 1);
    assert!(unknown.stderr.contains("unknown session"), "{}", unknown.stderr);
}

#[test]
fn a_session_with_no_default_hold_says_a_grant_is_not_needed() {
    let rig = Rig::start(&[("alpha", &["--chunks", "1"])]); // no --join-held
    let state = &rig.hub_state;
    let minted = run(state, &["release", SESSION, "--once"]);
    assert_eq!(minted.code, 0);
    assert!(minted.stderr.contains("no default hold"), "{}", minted.stderr);
    // And with neither option in use the say path is exactly as before.
    let said = say_retrying(state, &["say", SESSION, "hi"]);
    assert_eq!(said.code, 0, "{}", said.stderr);
}

#[test]
fn the_grant_exit_code_and_verbs_are_documented() {
    let doc = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/protocol/v2.md")).unwrap();
    let section = &doc[doc.find("## 10. CLI mapping").unwrap()..];
    for needle in ["--join-held", "--once", "--grant", "exits **5**", "invalid_grant"] {
        assert!(section.contains(needle), "§10 must document {needle}");
    }
    assert_eq!(holler_cli::hold_cmd::INVALID_GRANT_EXIT_CODE, INVALID_GRANT_EXIT);
    assert_eq!(holler_cli::hold_cmd::HELD_EXIT_CODE, HELD_EXIT);
}
