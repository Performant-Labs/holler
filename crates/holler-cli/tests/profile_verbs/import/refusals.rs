//! What `holler profile import` refuses (story #665, D8): each refusal answers its code in
//! both formats, writes nothing, and never prints what the file holds. The typed guards do
//! the validation (`EnvVarName`, `Argv`, `GridPos`), so there is no second scan of the text.

use serde_json::{json, Value};

use super::{assert_no_profile_write, full_document, path_text, write_document, write_file};
use crate::export::FULL_DOCUMENT;
use crate::list::rig::{assert_failure, profile, run_both_with, Both, Rig};

/// A made-up secret: it must never appear in any output.
const CANARY: &str = "t665-import-canary-not-a-secret-51c7";

fn empty() -> Rig {
    Rig::new([], [])
}

/// Import `document` into a rig from `seed`: the outcome is a refusal coded `code` at exit
/// `exit` that wrote nothing, and whose output never holds [`CANARY`].
fn refused(seed: impl Fn() -> Rig, document: &Value, extra: &[&str], code: &str, exit: i32) {
    let dir = tempfile::tempdir().unwrap();
    let file = write_document(dir.path(), document);
    refused_file(seed, &file, extra, code, exit);
}

fn refused_file(seed: impl Fn() -> Rig, file: &str, extra: &[&str], code: &str, exit: i32) {
    let mut argv = vec!["profile", "import", file];
    argv.extend_from_slice(extra);
    let ran = run_both_with(seed, &argv, Rig::run);
    assert_failure(&ran.both, code, exit);
    assert_never_printed(&ran.both, CANARY);
    for rig in ran.rigs() {
        assert_no_profile_write(rig);
        assert_eq!(
            rig.panes.faults().calls(),
            vec![],
            "no pane is read or written"
        );
    }
}

fn assert_never_printed(both: &Both, secret: &str) {
    for text in [
        &both.text.out,
        &both.text.err,
        &both.json.out,
        &both.json.err,
    ] {
        assert!(
            !text.contains(secret),
            "the file's content was printed: {text}"
        );
    }
}

/// [`full_document`] with `edit` applied to its one spec.
fn with_spec(edit: impl FnOnce(&mut Value)) -> Value {
    let mut document = full_document();
    edit(&mut document["panes"][0]);
    document
}

#[test]
fn an_environment_value_is_profile_secret_refused_and_never_echoed() {
    let in_list = with_spec(|spec| spec["env"] = json!([format!("DEMO_TOKEN={CANARY}")]));
    refused(empty, &in_list, &[], "profile-secret-refused", 3);
    let bare = with_spec(|spec| spec["env"] = json!(format!("DEMO_TOKEN={CANARY}")));
    refused(empty, &bare, &[], "profile-secret-refused", 3);
}

#[test]
fn a_blank_environment_name_is_env_name_invalid() {
    for blank in ["", " "] {
        let document = with_spec(|spec| spec["env"] = json!([blank]));
        refused(empty, &document, &[], "env-name-invalid", 3);
    }
}

#[test]
fn a_command_or_a_check_as_a_string_is_command_not_argv() {
    let command = with_spec(|spec| spec["command"] = json!(format!("opencode serve {CANARY}")));
    refused(empty, &command, &[], "command-not-argv", 3);
    let check = with_spec(|spec| spec["check"] = json!(format!("curl -s {CANARY}")));
    refused(empty, &check, &[], "command-not-argv", 3);
}

#[test]
fn a_bad_position_is_its_grid_code() {
    let ambiguous = with_spec(|spec| spec["herdr"]["grid"] = json!("hj-c1r2"));
    refused(empty, &ambiguous, &[], "grid-ambiguous", 3);
    let out_of_range = with_spec(|spec| spec["herdr"]["grid"] = json!("r0c1"));
    refused(empty, &out_of_range, &[], "grid-out-of-range", 3);
}

#[test]
fn a_taken_name_or_slug_is_profile_exists() {
    let fleet = || Rig::new([], [profile("Fleet", vec![])]);
    refused(fleet, &full_document(), &[], "profile-exists", 3);
    let shouted = || Rig::new([], [profile("FLEET", vec![])]);
    refused(shouted, &full_document(), &[], "profile-exists", 3);
    let copy = || Rig::new([], [profile("Copy", vec![])]);
    refused(
        copy,
        &full_document(),
        &["--name", "copy"],
        "profile-exists",
        3,
    );
}

#[test]
fn a_document_of_another_shape_is_usage_and_never_quoted() {
    let mut unknown_top = full_document();
    unknown_top[CANARY] = json!(CANARY);
    let mut wrong_format = full_document();
    wrong_format["format"] = json!(CANARY);
    let mut newer = full_document();
    newer["version"] = json!(2);
    let mut no_name = full_document();
    no_name.as_object_mut().unwrap().remove("name");
    let unknown_in_spec = with_spec(|spec| spec["colour"] = json!(CANARY));
    let record_field = with_spec(|spec| spec["generation"] = json!(3));
    for document in [
        unknown_top,
        wrong_format,
        newer,
        no_name,
        unknown_in_spec,
        record_field,
        json!([CANARY]),
    ] {
        refused(empty, &document, &[], "usage", 2);
    }
}

#[test]
fn a_file_that_is_not_a_document_is_usage() {
    let dir = tempfile::tempdir().unwrap();
    let not_json = path_text(&write_file(dir.path(), "a.json", &format!("{{ {CANARY}")));
    refused_file(empty, &not_json, &[], "usage", 2);
    let missing = path_text(&dir.path().join("missing.json"));
    refused_file(empty, &missing, &[], "usage", 2);
    let directory = path_text(dir.path());
    refused_file(empty, &directory, &[], "usage", 2);
    // Over 1 MiB, though a valid document: padded with whitespace after it.
    let padded = format!("{FULL_DOCUMENT}{}", " ".repeat(1024 * 1024));
    let big = path_text(&write_file(dir.path(), "big.json", &padded));
    refused_file(empty, &big, &[], "usage", 2);
}
