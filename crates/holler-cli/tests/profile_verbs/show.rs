//! `holler profile show` (story #662, AC 5): the spec, its live panes (the members), and
//! for each member every field where it differs from its spec; the last probe result,
//! read and never run.

use holler_pane::{
    Argv, GridPos, HerdrPane, Pane, PaneError, ProbeResult, ProfileName, ProfileSpec, ProfileStore,
};
use holler_pane_testkit::fault::Fault;
use serde_json::{json, Value};

use crate::list::rig::{assert_failure, matching_spec, member, profile, run_both, Rig};

const NAME: &str = "Some Profile";

fn argv(parts: &[&str]) -> Argv {
    Argv::new(parts.iter().map(|p| (*p).to_owned()).collect())
}

/// The row of `pane` in the JSON comparison.
fn row<'a>(data: &'a Value, pane: &str) -> &'a Value {
    data["comparison"]
        .as_array()
        .expect("comparison is an array")
        .iter()
        .find(|r| r["pane"] == pane)
        .unwrap_or_else(|| panic!("a comparison row for {pane}: {data}"))
}

/// The indented lines under the text line `head`, up to the next unindented line.
fn block<'a>(text: &'a str, head: &str) -> Vec<&'a str> {
    let mut lines = text.lines().skip_while(|l| *l != head);
    assert!(lines.next().is_some(), "a line {head:?} in {text:?}");
    lines.take_while(|l| l.starts_with("  ")).collect()
}

/// One profile `NAME` with `specs`, over `panes`.
fn rig_of(panes: Vec<Pane>, specs: Vec<ProfileSpec>) -> Rig {
    Rig::new(panes, [profile(NAME, specs)])
}

#[test]
fn show_reports_differences_field_by_field_in_both_formats() {
    let seed = || {
        let moved = Pane {
            herdr: HerdrPane {
                grid: GridPos { row: 2, col: 1 },
                ..member("demo-c1r1", NAME).herdr
            },
            ..member("demo-c1r1", NAME)
        };
        rig_of(vec![moved], vec![matching_spec("demo-c1r1")])
    };
    let both = run_both(seed, &["profile", "show", NAME]);
    assert_eq!(both.text.code, 0, "{:?}", both.text);

    assert_eq!(
        block(&both.text.out, "pane demo-c1r1: differs"),
        vec!["  herdr.grid: spec r1c1, live r2c1", "  probe: none"]
    );

    let data = &both.envelope.data;
    let stored = seed().profiles.get(&ProfileName::parse(NAME).unwrap());
    assert_eq!(
        data["profile"],
        serde_json::to_value(stored.unwrap().unwrap()).unwrap(),
        "the profile as stored"
    );
    assert_eq!(
        row(data, "demo-c1r1"),
        &json!({
            "pane": "demo-c1r1",
            "status": "differs",
            "differences": [{
                "field": "herdr.grid",
                "spec": {"row": 1, "col": 1, "pos": "r1c1"},
                "live": {"row": 2, "col": 1, "pos": "r2c1"},
            }],
            "probe": null,
        })
    );
    assert!(
        both.json
            .out
            .contains(r#""live":{"row":2,"col":1,"pos":"r2c1"}"#),
        "{}",
        both.json.out
    );
}

#[test]
fn show_reports_nothing_for_a_matching_pane() {
    let seed = || {
        rig_of(
            vec![member("demo-c1r1", NAME)],
            vec![matching_spec("demo-c1r1")],
        )
    };
    let both = run_both(seed, &["profile", "show", NAME]);
    assert_eq!(both.text.code, 0, "{:?}", both.text);
    assert_eq!(
        block(&both.text.out, "pane demo-c1r1: matches"),
        vec!["  probe: none"]
    );
    assert!(!both.text.out.contains(", live "), "{}", both.text.out);
    let r = row(&both.envelope.data, "demo-c1r1");
    assert_eq!(r["status"], "matches");
    assert_eq!(r["differences"], json!([]));
}

#[test]
fn show_reports_the_last_probe_result_without_running_one() {
    let probed = |name: &str, last: ProbeResult| {
        let mut p = member(name, NAME);
        p.probe.check = Some(argv(&["curl", "-s", "http://127.0.0.1:48100/v1/models"]));
        p.probe.expect = vec!["qwen38".to_owned()];
        p.probe.last = Some(last);
        p
    };
    let spec = |name: &str| {
        let mut s = matching_spec(name);
        s.check = Some(argv(&["curl", "-s", "http://127.0.0.1:48100/v1/models"]));
        s.expect = vec!["qwen38".to_owned()];
        s
    };
    let seed = || {
        rig_of(
            vec![
                probed("demo-c1r1", ProbeResult::Ok),
                probed(
                    "demo-c2r1",
                    ProbeResult::Failed {
                        missing: vec!["qwen38".to_owned()],
                    },
                ),
            ],
            vec![spec("demo-c1r1"), spec("demo-c2r1")],
        )
    };
    // `run_both` asserts the prober (and every adapter) was never called.
    let both = run_both(seed, &["profile", "show", NAME]);
    assert_eq!(both.text.code, 0, "{:?}", both.text);
    assert_eq!(
        block(&both.text.out, "pane demo-c1r1: matches"),
        vec!["  probe: ok"]
    );
    assert_eq!(
        block(&both.text.out, "pane demo-c2r1: matches"),
        vec![r#"  probe: failed (missing "qwen38")"#]
    );
    let data = &both.envelope.data;
    assert_eq!(row(data, "demo-c1r1")["probe"], json!("ok"));
    assert_eq!(
        row(data, "demo-c2r1")["probe"],
        json!({"failed": {"missing": ["qwen38"]}})
    );
}

#[test]
fn show_prints_command_and_check_as_argv_arrays() {
    let seed = || {
        let mut spec = matching_spec("demo-c1r1");
        spec.command = Some(argv(&["opencode", "serve"]));
        spec.check = Some(argv(&["curl", "-s", "http://127.0.0.1:48100/v1/models"]));
        let mut live = member("demo-c1r1", NAME);
        live.command = Some(argv(&["opencode", "serve", "--port 1"]));
        live.probe.check = spec.check.clone();
        rig_of(vec![live], vec![spec])
    };
    let both = run_both(seed, &["profile", "show", NAME]);
    assert_eq!(both.text.code, 0, "{:?}", both.text);

    let spec_lines = block(&both.text.out, "spec demo-c1r1");
    assert!(
        spec_lines.contains(&r#"  command: ["opencode","serve"]"#),
        "{spec_lines:?}"
    );
    assert!(
        spec_lines.contains(&r#"  check: ["curl","-s","http://127.0.0.1:48100/v1/models"]"#),
        "{spec_lines:?}"
    );
    assert_eq!(
        block(&both.text.out, "pane demo-c1r1: differs"),
        vec![
            r#"  command: spec ["opencode","serve"], live ["opencode","serve","--port 1"]"#,
            "  probe: none",
        ]
    );

    let data = &both.envelope.data;
    assert_eq!(
        data["profile"]["panes"][0]["command"],
        json!(["opencode", "serve"])
    );
    assert_eq!(
        row(data, "demo-c1r1")["differences"][0],
        json!({
            "field": "command",
            "spec": ["opencode", "serve"],
            "live": ["opencode", "serve", "--port 1"],
        })
    );
}

#[test]
fn show_lists_missing_and_extra_panes() {
    // A spec pane is unchecked text: one that carries an ESC sequence must print escaped.
    let hostile = "demo\u{1b}[31mx";
    let seed = || {
        rig_of(
            vec![
                member("demo-c1r1", NAME),
                member("demo-c4r1", NAME),
                // In another profile: a spec naming it here is detached, so it is missing.
                member("demo-c3r1", "Other"),
            ],
            vec![
                matching_spec("demo-c1r1"),
                matching_spec("demo-c3r1"),
                matching_spec(hostile),
            ],
        )
    };
    let both = run_both(seed, &["profile", "show", NAME]);
    assert_eq!(both.text.code, 0, "{:?}", both.text);
    let out = &both.text.out;

    assert_eq!(
        out.lines().next(),
        Some("profile Some Profile (some-profile): generation 1, 3 specs, 2 live")
    );
    let lines: Vec<&str> = out.lines().collect();
    for want in [
        "pane demo-c1r1: matches",
        "pane demo-c3r1: missing (no live pane)",
        r"pane demo\u{1b}[31mx: missing (no live pane)",
        "pane demo-c4r1: extra (no spec)",
    ] {
        assert!(lines.contains(&want), "a line {want:?} in {out:?}");
    }
    assert_eq!(
        block(out, "pane demo-c4r1: extra (no spec)"),
        vec!["  probe: none"]
    );
    assert!(block(out, "pane demo-c3r1: missing (no live pane)").is_empty());
    assert!(
        !out.contains('\u{1b}'),
        "no raw ESC reaches the terminal: {out:?}"
    );

    let data = &both.envelope.data;
    let statuses: Vec<(&str, &str)> = data["comparison"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| (r["pane"].as_str().unwrap(), r["status"].as_str().unwrap()))
        .collect();
    assert_eq!(
        statuses,
        vec![
            ("demo-c1r1", "matches"),
            ("demo-c3r1", "missing"),
            (hostile, "missing"),
            ("demo-c4r1", "extra"),
        ]
    );
    assert_eq!(row(data, "demo-c3r1")["probe"], Value::Null);
}

#[test]
fn show_of_a_missing_profile_is_profile_not_found_in_both_formats() {
    let seed = || Rig::new([member("demo-c1r1", "Other")], [profile("Other", vec![])]);
    let both = run_both(seed, &["profile", "show", NAME]);
    assert_failure(&both, "profile-not-found", 3);
    assert!(both.text.err.contains("Some Profile"), "{:?}", both.text);
}

#[test]
fn show_passes_a_store_failure_through() {
    let seed = || {
        let rig = rig_of(
            vec![member("demo-c1r1", NAME)],
            vec![matching_spec("demo-c1r1")],
        );
        rig.panes.faults().set(Some(Fault::Wedged));
        rig
    };
    let both = run_both(seed, &["profile", "show", NAME]);
    assert_failure(&both, "timeout", 1);
}

#[test]
fn show_with_a_bad_name_is_usage_in_both_formats() {
    let both = run_both(|| rig_of(vec![], vec![]), &["profile", "show", "   "]);
    assert_failure(&both, "usage", 2);
}

#[test]
fn show_passes_a_profile_store_failure_through() {
    // A failing profile store is its own error, never read as "no such profile".
    let seed = || {
        let rig = rig_of(vec![], vec![]);
        rig.profiles
            .faults()
            .set(Some(Fault::Fail(PaneError::Unavailable {
                what: "profile store".to_owned(),
            })));
        rig
    };
    let both = run_both(seed, &["profile", "show", NAME]);
    assert_failure(&both, "unavailable", 1);
}
