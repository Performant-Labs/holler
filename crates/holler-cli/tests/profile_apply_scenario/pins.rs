//! The target's default run (decision 5 of the #667 brief): pure pins, no hub, no process, no
//! Herdr. A12: every test here that starts a scratch set is `#[ignore]`d with the opt-in
//! variable named, so `cargo test` never starts a server. A14: the [`Applied`] reader takes
//! the guide's shape and refuses anything else. (A13 and A14's `launch_args` half are pinned
//! beside the scratch guards, in `pane_integration/guards.rs`.)

use std::path::Path;

use holler_pane::tx_apply::{Action, Outcome};
use serde_json::{json, Value};

use crate::plan::Applied;
use crate::scratch::GATE_VAR;

/// What a test calls to start a scratch set, written so this file does not contain it.
const STARTS_A_SCRATCH_SET: &str = concat!("Scratch", "::start");

/// The `#[test]` functions of `source` whose body starts a scratch set, each with the
/// attribute lines above it.
fn scratch_tests(source: &str) -> Vec<(String, Vec<String>)> {
    let mut found = Vec::new();
    let mut attributes: Vec<String> = Vec::new();
    let mut lines = source.lines();
    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("#[") {
            attributes.push(trimmed.to_owned());
            continue;
        }
        if trimmed.starts_with("///") || trimmed.starts_with("//") {
            continue;
        }
        let is_fn = trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ");
        if is_fn && attributes.iter().any(|a| a == "#[test]") {
            let name = trimmed
                .trim_start_matches("pub ")
                .trim_start_matches("fn ")
                .split('(')
                .next()
                .unwrap_or_default()
                .to_owned();
            let body: Vec<&str> = lines.by_ref().take_while(|l| *l != "}").collect();
            if body.iter().any(|l| l.contains(STARTS_A_SCRATCH_SET)) {
                found.push((name, std::mem::take(&mut attributes)));
            }
        }
        attributes.clear();
    }
    found
}

/// A12: every scratch test of this target is `#[ignore = "..."]` naming the opt-in variable.
/// It reads the target's own sources, so removing one `#[ignore]` fails it.
#[test]
fn every_scratch_test_is_ignored_and_names_the_opt_in_variable() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/profile_apply_scenario");
    let mut tests = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("the target's directory") {
        let path = entry.expect("an entry").path();
        if path.extension().is_some_and(|e| e == "rs") {
            let source = std::fs::read_to_string(&path).expect("a source file");
            tests.extend(scratch_tests(&source));
        }
    }
    assert!(
        tests.len() >= 2,
        "the two scenario tests are found: {tests:?}"
    );
    for (name, attributes) in &tests {
        let ignored = attributes
            .iter()
            .any(|a| a.starts_with("#[ignore = \"") && a.contains(GATE_VAR));
        assert!(
            ignored,
            "{name} starts a scratch set but is not #[ignore]d naming {GATE_VAR}: {attributes:?}"
        );
    }
}

/// The source pin's reader sees a test that starts a scratch set with no `#[ignore]`.
#[test]
fn the_source_pin_finds_a_scratch_test_and_its_attributes() {
    let start = STARTS_A_SCRATCH_SET;
    let source = format!(
        "#[test]\n#[ignore = \"opt-in: {GATE_VAR}=1\"]\nfn a() {{\n    let Some(s) = {start}() else {{ return }};\n}}\n\n\
         #[test]\nfn b() {{\n    let Some(s) = {start}() else {{ return }};\n}}\n\n\
         #[test]\nfn pure() {{\n    let _ = 1;\n}}\n"
    );
    let found = scratch_tests(&source);
    let names: Vec<&str> = found.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["a", "b"], "only the tests that start a scratch set");
    assert_eq!(found[1].1, ["#[test]"], "b carries no #[ignore]");
}

/// A fixed `data` in the guide's shape: a dry run of three steps.
fn guide_shaped() -> Value {
    json!({
        "profile": "hlr649-apply-p",
        "dry_run": true,
        "steps": [
            {"pane": "hlr649-ap1", "action": "adopt", "outcome": "planned",
             "code": null, "diff": null, "detail": null},
            {"pane": "hlr649-ap2", "action": "report", "outcome": "reported", "code": null,
             "diff": {"pane": "hlr649-ap2", "status": "differs", "differences": [
                 {"field": "opencode_agent", "spec": "plan", "live": null}]},
             "detail": null},
            {"pane": "hlr649-ap4", "action": "create", "outcome": "refused",
             "code": "probe-failed", "diff": null, "detail": "the health check failed"},
        ]
    })
}

/// A14: the reader takes the guide's shape, with production's names for actions and outcomes.
#[test]
fn the_applied_reader_reads_the_guides_shape() {
    let applied = Applied::parse(&guide_shaped()).expect("the guide's shape");
    assert_eq!(applied.profile, "hlr649-apply-p");
    assert!(applied.dry_run);
    assert_eq!(applied.panes(), ["hlr649-ap1", "hlr649-ap2", "hlr649-ap4"]);
    assert_eq!(
        applied.step("hlr649-ap1").row(),
        (Action::Adopt, Outcome::Planned)
    );
    let report = applied.step("hlr649-ap2");
    assert_eq!(report.row(), (Action::Report, Outcome::Reported));
    assert_eq!(report.diff["differences"][0]["field"], "opencode_agent");
    let refused = applied.step("hlr649-ap4");
    assert_eq!(refused.row(), (Action::Create, Outcome::Refused));
    assert_eq!(refused.code.as_deref(), Some("probe-failed"));
    assert_eq!(refused.detail.as_deref(), Some("the health check failed"));
}

/// A14: a missing key, at the top or in a step, is refused, even one whose value would be
/// `null` (the guide prints every key, `null` when absent).
#[test]
fn the_applied_reader_refuses_a_missing_key() {
    for key in ["profile", "dry_run", "steps"] {
        let mut data = guide_shaped();
        data.as_object_mut().unwrap().remove(key);
        assert!(Applied::parse(&data).is_err(), "no {key:?} is refused");
    }
    for key in ["pane", "action", "outcome", "code", "diff", "detail"] {
        let mut data = guide_shaped();
        data["steps"][0].as_object_mut().unwrap().remove(key);
        assert!(
            Applied::parse(&data).is_err(),
            "a step with no {key:?} is refused"
        );
    }
}

/// A14: an action or outcome name production does not print is refused.
#[test]
fn the_applied_reader_refuses_an_unknown_action_or_outcome() {
    for (key, name) in [
        ("action", "launch"),
        ("action", "Adopt"),
        ("action", ""),
        ("outcome", "done"),
        ("outcome", "Planned"),
        ("outcome", "adopt"),
    ] {
        let mut data = guide_shaped();
        data["steps"][0][key] = json!(name);
        assert!(Applied::parse(&data).is_err(), "{key} {name:?} is refused");
    }
}
