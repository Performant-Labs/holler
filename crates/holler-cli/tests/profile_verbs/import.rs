//! `holler profile import FILE [--name NAME]` (story #665, Scope B, D7 and D8): a profile
//! read from one JSON document, validated by the typed guards, and created **detached** at
//! generation 1 (no pane is read or written). This file pins what an import stores; the
//! refusals are `import/refusals.rs`.

mod refusals;

use std::path::{Path, PathBuf};

use holler_pane::{Actor, GridPos, PaneStore, Profile, ProfileChange, ProfileName, ProfileStore};
use holler_pane_testkit::profile_store::ProfileStoreOp;
use serde_json::Value;

use crate::export::{full_spec, FULL_DOCUMENT};
use crate::list::rig::{assert_failure, member, pane, profile, run_both, run_both_with, Rig};
use crate::verb_harness::parse::try_parse;

/// The actor every import is logged under.
const ACTOR: &str = "holler profile import";

fn name(text: &str) -> ProfileName {
    ProfileName::parse(text).unwrap()
}

pub(crate) fn stored(rig: &Rig, text: &str) -> Option<Profile> {
    rig.profiles.get(&name(text)).unwrap()
}

/// The export of `export.rs`'s `fleet` as a JSON value, to edit.
pub(crate) fn full_document() -> Value {
    serde_json::from_str(FULL_DOCUMENT).unwrap()
}

/// Write `text` as `file` in `dir`; its path.
pub(crate) fn write_file(dir: &Path, file: &str, text: &str) -> PathBuf {
    let path = dir.join(file);
    std::fs::write(&path, text).unwrap();
    path
}

/// Write `document` (pretty) as `fleet.json` in `dir`; its path as text.
pub(crate) fn write_document(dir: &Path, document: &Value) -> String {
    let text = serde_json::to_string_pretty(document).unwrap();
    path_text(&write_file(dir, "fleet.json", &text))
}

pub(crate) fn path_text(path: &Path) -> String {
    path.to_str().unwrap().to_owned()
}

/// No profile was written through `rig`'s store.
pub(crate) fn assert_no_profile_write(rig: &Rig) {
    let calls = rig.profiles.faults().calls();
    assert!(
        !calls.contains(&ProfileStoreOp::CasPut),
        "nothing is written: {calls:?}"
    );
}

/// A rig whose pane `demo-c3r2` (the full spec's pane) is a member of `Other`, and whose
/// `demo-c1r1` is in no profile.
fn others() -> Rig {
    Rig::new(
        [member("demo-c3r2", "Other"), pane("demo-c1r1")],
        [profile("Other", vec![])],
    )
}

#[test]
fn import_creates_a_detached_profile_at_generation_one_in_both_formats() {
    let dir = tempfile::tempdir().unwrap();
    let file = path_text(&write_file(dir.path(), "fleet.json", FULL_DOCUMENT));
    let before = others().panes.list().unwrap();
    let ran = run_both_with(others, &["profile", "import", &file], Rig::run);
    let both = &ran.both;
    assert_eq!(both.text.code, 0, "{:?}", both.text);
    assert!(both.text.err.is_empty(), "{:?}", both.text);
    assert_eq!(
        both.text.out,
        "imported profile Fleet (fleet): 1 spec, generation 1\n"
    );
    assert_eq!(
        both.envelope.data["profile"],
        serde_json::to_value(stored(&ran.json_rig, "Fleet").unwrap()).unwrap(),
        "data.profile is the record as stored"
    );
    for rig in ran.rigs() {
        let fleet = stored(rig, "Fleet").expect("imported");
        assert_eq!(fleet.generation, 1);
        assert_eq!(fleet.panes, vec![full_spec("demo-c3r2", 2, 3)]);
        let log = rig.profiles.log(&name("Fleet")).unwrap();
        assert_eq!(log.len(), 1, "{log:?}");
        assert_eq!(log[0].actor, Actor::parse(ACTOR).unwrap());
        assert_eq!(log[0].change, ProfileChange::Created);
        assert_eq!(
            rig.panes.faults().calls(),
            vec![],
            "detached: no pane is read or written"
        );
        assert_eq!(
            rig.panes.list().unwrap(),
            before,
            "the spec's pane stays in Other"
        );
    }
}

#[test]
fn import_accepts_a_spec_naming_no_pane() {
    let dir = tempfile::tempdir().unwrap();
    let mut document = full_document();
    document["panes"][0]["pane"] = Value::from("demo-c9r9");
    let file = write_document(dir.path(), &document);
    let ran = run_both_with(others, &["profile", "import", &file], Rig::run);
    assert_eq!(ran.both.text.code, 0, "{:?}", ran.both.text);
    for rig in ran.rigs() {
        assert_eq!(stored(rig, "Fleet").unwrap().panes[0].pane, "demo-c9r9");
    }
}

#[test]
fn import_reads_a_position_in_every_form_the_grid_parser_accepts() {
    let forms = [
        Value::from("r2c1"),
        Value::from("c1r2"),
        Value::from("2,1"),
        Value::from("R2C1"),
        serde_json::json!({"row": 2, "col": 1}),
        serde_json::json!({"row": 2, "col": 1, "pos": "r2c1"}),
    ];
    for form in forms {
        let dir = tempfile::tempdir().unwrap();
        let mut document = full_document();
        document["panes"][0]["herdr"]["grid"] = form.clone();
        let file = write_document(dir.path(), &document);
        let ran = run_both_with(others, &["profile", "import", &file], Rig::run);
        assert_eq!(ran.both.text.code, 0, "{form}: {:?}", ran.both.text);
        for rig in ran.rigs() {
            assert_eq!(
                stored(rig, "Fleet").unwrap().panes[0].herdr.grid,
                GridPos { row: 2, col: 1 },
                "{form}"
            );
        }
    }
}

#[test]
fn import_name_overrides_the_files_name_and_is_parsed() {
    let dir = tempfile::tempdir().unwrap();
    let file = path_text(&write_file(dir.path(), "fleet.json", FULL_DOCUMENT));
    let ran = run_both_with(
        others,
        &["profile", "import", &file, "--name", "Fleet Copy"],
        Rig::run,
    );
    assert_eq!(
        ran.both.text.out,
        "imported profile Fleet Copy (fleet-copy): 1 spec, generation 1\n"
    );
    for rig in ran.rigs() {
        assert!(stored(rig, "Fleet Copy").is_some());
        assert_eq!(stored(rig, "Fleet"), None, "the file's name is not used");
    }
    let both = run_both(others, &["profile", "import", &file, "--name", "   "]);
    assert_failure(&both, "usage", 2);
}

#[test]
fn import_takes_one_file_positional() {
    let error = try_parse(&["profile", "import"]).expect_err("FILE is required");
    assert_eq!(
        error.kind(),
        clap::error::ErrorKind::MissingRequiredArgument
    );
    assert!(try_parse(&["profile", "import", "f.json", "--name", "Copy"]).is_ok());
}

#[test]
fn an_exported_profile_imports_back_with_equal_specs() {
    let dir = tempfile::tempdir().unwrap();
    let out = path_text(&dir.path().join("fleet.json"));
    let exporter = crate::export::fleet();
    let exported = exporter.run(
        &["profile", "export", "Fleet", "--out", &out],
        holler_cli::output::Format::Text,
    );
    assert_eq!(exported.code, 0, "{exported:?}");

    let ran = run_both_with(
        || Rig::new([], []),
        &["profile", "import", &out, "--name", "Copy"],
        Rig::run,
    );
    assert_eq!(ran.both.text.code, 0, "{:?}", ran.both.text);
    let original = stored(&exporter, "Fleet").unwrap();
    for rig in ran.rigs() {
        let copy = stored(rig, "Copy").expect("imported");
        assert_eq!(
            copy.panes, original.panes,
            "the specs survive the round trip"
        );
        assert_eq!(copy.name.as_str(), "Copy");
    }
}
