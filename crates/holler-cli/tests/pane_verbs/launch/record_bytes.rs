//! #808 constraint (a) and clarification 4: the record `launch` writes is byte for byte the
//! record it wrote before `HerdrPane` carried a label, and that pre-#808 record, and the same
//! record with a label, both still read.
//!
//! The literal below is hard-coded, never regenerated: a launch of `LAUNCH`'s spec through the
//! engine, with the constant clock [`AT`], on the code of `main` at
//! `92fbc6e4186acc717f4aa2b6465c55a266b9043c` (#808's base), stored and written back with
//! `serde_json::to_string`. The two tests share it, so it lives in this one file.

use holler_pane::tx_launch::{self, LaunchRequest, TxOptions};
use holler_pane::Pane;
use serde_json::Value;

use super::rig::{launch_spec, name, Rig, PANE};

/// The clock the engine stamps into `last_observed.at`.
const AT: i64 = 1_760_000_000_000;

/// The record `launch` wrote at #808's base (see the module docs). It has no `label` key.
const PRE_808_RECORD: &str = r#"{"name":"demo-c1r1","generation":1,"herdr":{"session":"scratch","workspace":"main","pane_id":"w1:p1","grid":{"row":2,"col":1,"pos":"r2c1"}},"host":{"name":"localhost","tmux":"demo-c1r1","cwd":"/srv/demo","herdr_api_version":"0.9.1-preview.2026-09-21-0ff0f27e2226"},"harness":{"kind":"opencode","port":48100,"pid":20000,"health":"healthy"},"session_of_record":"ses_00000000000000000000000001","role":"agent","hold":"none","last_observed":{"shown":"ses_00000000000000000000000001","at":1760000000000},"model":{"provider":"demo-provider","model_id":"demo-model","effort":"medium"},"env":[],"context":{"soft":100000,"hard":150000},"probe":{"expect":[]}}"#;

/// The record a launch of `LAUNCH`'s spec stores, through the engine at [`AT`].
fn launched_record(rig: &Rig) -> Pane {
    let request = LaunchRequest {
        name: name(PANE),
        herdr_session: Some("scratch".to_owned()),
        spec: launch_spec(),
        profile: None,
        spec_only: false,
    };
    let options = TxOptions {
        now_ms: || AT,
        ..TxOptions::default()
    };
    let (answer, _) = rig.engine(|ports| tx_launch::launch(ports, &request, &options));
    answer.expect("the launch succeeds");
    rig.record(PANE).expect("a record")
}

/// A `launch` writes the same bytes as before #808: no `label` key, nothing else moved. A
/// guard on constraint (a): it holds on #808's base and fails if the field is written when
/// `None` (`"label":null`) or the record changes in any other way.
#[test]
fn a_launch_writes_the_record_bytes_it_wrote_before_808() {
    let rig = Rig::new();
    let record = launched_record(&rig);
    assert_eq!(serde_json::to_string(&record).unwrap(), PRE_808_RECORD);
}

/// The pre-#808 record reads, with no label, and writes back to the same bytes; the same
/// record with `"label":"x"` on its `herdr` reads too, with that label, and round-trips (the
/// hub reads `panes.json` once and writes every record back on each write).
#[test]
fn the_pre_808_record_and_a_labelled_one_both_read_and_round_trip() {
    let old: Pane = serde_json::from_str(PRE_808_RECORD).expect("the pre-#808 record reads");
    assert_eq!(old.herdr.label, None, "a record with no label key has none");
    assert_eq!(serde_json::to_string(&old).unwrap(), PRE_808_RECORD);
    let again: Pane = serde_json::from_str(&serde_json::to_string(&old).unwrap()).unwrap();
    assert_eq!(again, old);

    let mut labelled: Value = serde_json::from_str(PRE_808_RECORD).unwrap();
    labelled["herdr"]["label"] = Value::from("x");
    let read: Pane = serde_json::from_value(labelled.clone()).expect("a labelled record reads");
    assert_eq!(read.herdr.label.as_deref(), Some("x"));
    assert_eq!(
        Pane {
            herdr: holler_pane::HerdrPane {
                label: None,
                ..read.herdr.clone()
            },
            ..read.clone()
        },
        old,
        "the label is the only difference"
    );
    let written = serde_json::to_value(&read).unwrap();
    assert_eq!(written, labelled, "the labelled record writes back as read");
    let reread: Pane = serde_json::from_value(written).unwrap();
    assert_eq!(reread, read);
}
