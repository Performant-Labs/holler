//! The join, per pane (#650 acceptance 1, 4, 5, 6, 8, 11, 13): the label finds the Herdr
//! pane, the snapshot gives the position, the files and a healthy server's list give the
//! session of record, the flags give the role.

use holler_cli::output::Format;
use holler_pane::findings::FindingKind as K;
use holler_pane::pane::{Health, PaneRole};
use holler_pane::{GridPos, HerdrPane, PaneError, PaneId, PaneName};
use holler_pane_testkit::harness::HarnessOp;
use serde_json::Value;

use super::rig::{
    data_of, error_of, message, one_finding, owned_sessions, pane_names, pane_row, sessions_toml,
    text_line, toml_array, ImportRig, HERDR_SESSION,
};
use crate::doctor::rig::Seed;

const A: &str = "hj-c1r1";
const B: &str = "hj-c1r2";
const C: &str = "hj-c2r1";

/// The pinned refusal of a real run with problems.
fn blocked(n: usize) -> String {
    format!("import blocked: {n} pane(s) have problems; run with --dry-run for the report")
}

/// One pane, `A`, at `r1c1`, with agreeing files.
fn one_pane() -> ImportRig {
    ImportRig::new(&[Seed::new(A, 1, 1)])
}

/// The dry run's JSON data.
fn dry(rig: &ImportRig) -> Value {
    data_of(&rig.run(&["--dry-run"], Format::Json))
}

/// A real run refused as `import-blocked` for `n` panes, in both formats, writing nothing.
fn assert_real_run_blocked(rig: &ImportRig, n: usize) {
    let run = rig.run(&[], Format::Json);
    assert_eq!(run.code, 3, "{run:?}");
    assert_eq!(error_of(&run), ("import-blocked".to_owned(), blocked(n)));
    let text = rig.run(&[], Format::Text);
    assert_eq!(text.code, 3, "{text:?}");
    assert!(text.out.is_empty(), "nothing on stdout: {text:?}");
    assert_eq!(text.err.lines().count(), 1, "one line: {text:?}");
    assert!(text.err.contains(&blocked(n)), "{text:?}");
    assert!(rig.records().is_empty(), "no record: {:?}", rig.records());
    assert!(rig.fleet().is_none(), "no profile");
}

// --- Acceptance 1: agreement --------------------------------------------------------------

/// Three panes made in another order than their names', `B` placed at row 2, column 1.
fn three_panes() -> ImportRig {
    ImportRig::new(&[Seed::new(C, 1, 2), Seed::new(B, 2, 1), Seed::new(A, 1, 1)])
}

#[test]
fn agreeing_files_import_every_pane_with_its_session_of_record() {
    let rig = three_panes();
    for name in [A, B, C] {
        // The port is the launcher entry's, never `47000 + col * 10 + row` of the name.
        let cell = GridPos::parse(&name[3..]).expect("the name spells a cell");
        assert_ne!(rig.launcher[name].port, 47000 + cell.col * 10 + cell.row);
    }
    let run = rig.run(&["--orchestrator", A], Format::Json);
    let data = data_of(&run);
    assert_eq!(data["findings"], Value::Array(vec![]), "{data}");
    assert_eq!(data["recorded"], 3, "{data}");
    assert_eq!(data["dry_run"], false, "{data}");
    assert_eq!(pane_names(&data), [A, B, C], "ascending PaneName order");

    let mut records = rig.records();
    records.sort_by(|x, y| x.name.cmp(&y.name));
    let expected = vec![
        rig.expected_record(A, PaneRole::Orchestrator),
        rig.expected_record(B, PaneRole::Agent),
        rig.expected_record(C, PaneRole::Agent),
    ];
    assert_eq!(records, expected);

    let row = pane_row(&data, B);
    assert_eq!(row["action"], "record", "{row}");
    assert_eq!(row["role"], "agent", "{row}");
    assert_eq!(row["session_of_record"], rig.world.live(B).session.as_str());
    assert_eq!(pane_row(&data, A)["role"], "orchestrator");
    assert!(
        run.out.contains(r#""pos":{"row":2,"col":1,"pos":"r2c1"}"#),
        "B's position is GridPos's JSON, row first: {}",
        run.out
    );
}

/// Acceptance 1, text form: one line per pane in name order, `NAME ACTION ROLE POS SESSION`.
#[test]
fn the_text_form_lists_each_pane_in_name_order() {
    let rig = three_panes();
    let run = rig.run(&["--dry-run", "--orchestrator", A], Format::Text);
    assert_eq!(run.code, 0, "{run:?}");
    let session = rig.world.live(B).session.clone();
    assert_eq!(
        text_line(&run.out, B),
        [B, "record", "agent", "r2c1", session.as_str()]
    );
    assert_eq!(text_line(&run.out, A)[2], "orchestrator");
    let first_word = |l: &str| l.split_whitespace().next().map(str::to_owned);
    let order: Vec<String> = run
        .out
        .lines()
        .filter_map(first_word)
        .filter(|w| [A, B, C].contains(&w.as_str()))
        .collect();
    assert_eq!(order, [A, B, C], "{}", run.out);
}

// --- Acceptance 4 and 13: the label ---------------------------------------------------

#[test]
fn a_pane_with_no_labelled_herdr_pane_is_unlabelled_and_blocks() {
    let rig = ImportRig::new(&[Seed::new(A, 1, 1), Seed::new(B, 2, 1)]);
    rig.label(A, "someone-else");
    let data = dry(&rig);
    one_finding(&data, "pane-unlabelled", A, "problem");
    assert_eq!(pane_row(&data, A)["action"], "blocked");
    assert_eq!(pane_row(&data, A)["pos"], Value::Null);
    assert_eq!(
        pane_row(&data, B)["action"],
        "record",
        "the other pane still plans"
    );
    let text = rig.run(&["--dry-run"], Format::Text);
    assert_eq!(text.code, 0, "{text:?}");
    assert!(
        text.out
            .lines()
            .any(|l| l.starts_with("  problem pane-unlabelled: ")),
        "{}",
        text.out
    );
    assert_real_run_blocked(&rig, 1);
}

#[test]
fn two_herdr_panes_with_the_label_are_ambiguous_and_block() {
    let rig = one_pane();
    let other = rig.world.unregistered_herdr_pane(3, 3);
    rig.world.herdr.rename(&other.pane_id, A).expect("label");
    let data = dry(&rig);
    let finding = one_finding(&data, "pane-label-ambiguous", A, "problem");
    for id in [rig.world.pane_id(A), other.pane_id] {
        assert!(message(finding).contains(id.as_str()), "{finding}");
    }
    let text = rig.run(&["--dry-run"], Format::Text);
    assert!(
        text.out
            .lines()
            .any(|l| l.starts_with("  problem pane-label-ambiguous: ")),
        "{}",
        text.out
    );
    assert_real_run_blocked(&rig, 1);
}

/// A same-labelled pane in another Herdr session reported beside the rig's own.
fn elsewhere(label: &str) -> HerdrPane {
    HerdrPane {
        session: "elsewhere".to_owned(),
        workspace: HERDR_SESSION.to_owned(),
        pane_id: PaneId::new("elsewhere:9"),
        grid: GridPos { row: 3, col: 3 },
        label: Some(label.to_owned()),
    }
}

/// Acceptance 13: only the pane in `--herdr-session` is a candidate.
#[test]
fn the_join_reads_only_the_named_herdr_session() {
    let mut rig = one_pane();
    rig.extra_herdr.push(elsewhere(A));
    let data = data_of(&rig.run(&[], Format::Json));
    assert_eq!(
        data["findings"],
        Value::Array(vec![]),
        "no ambiguity: {data}"
    );
    let record = rig.record(A).expect("A is recorded");
    assert_eq!(record.herdr.session, HERDR_SESSION);
    assert_eq!(record.herdr.pane_id, rig.world.pane_id(A));

    let mut only_elsewhere = one_pane();
    only_elsewhere.label(A, "");
    only_elsewhere.extra_herdr.push(elsewhere(A));
    let data = dry(&only_elsewhere);
    one_finding(&data, "pane-unlabelled", A, "problem");
}

/// Acceptance 11: the position is the snapshot's; a name that spells another cell is only
/// information.
#[test]
fn a_name_that_spells_another_cell_is_information_and_the_snapshot_wins() {
    let rig = ImportRig::new(&[Seed::new(B, 1, 1)]);
    let data = dry(&rig);
    let finding = one_finding(&data, "name-cell-differs", B, "info");
    for cell in ["r2c1", "r1c1"] {
        assert!(message(finding).contains(cell), "{finding}");
    }
    let run = rig.run(&[], Format::Json);
    assert_eq!(data_of(&run)["recorded"], 1);
    let record = rig.record(B).expect("B is recorded");
    assert_eq!(record.herdr.grid, GridPos { row: 1, col: 1 });
}

// --- Acceptance 5: the files disagree -------------------------------------------------

/// A change to the rig's files, given the live session `a` and another session `b`.
type Edit = fn(&ImportRig, &str, &str);

/// Each disagreement: a `sources-disagree` problem, no record planned.
#[test]
fn files_that_name_different_sessions_disagree_and_block() {
    let cases: [(&str, Edit); 3] = [
        ("last-session A, sessions.toml B", |rig, _a, b| {
            let port = rig.world.live(A).port;
            rig.write(&format!("{A}/sessions.toml"), &sessions_toml(A, port, b));
        }),
        ("owned-sessions with no id", |rig, _a, _b| {
            rig.write(&format!("{A}/owned-sessions"), &owned_sessions(&[]));
        }),
        ("owned-sessions lists only B", |rig, _a, b| {
            rig.write(&format!("{A}/owned-sessions"), &owned_sessions(&[b]));
        }),
    ];
    for (case, edit) in cases {
        let rig = one_pane();
        let a = rig.world.live(A).session.clone();
        let b = rig.world.harness.seed_session(rig.world.live(A).port);
        edit(&rig, &a, &b);
        let data = dry(&rig);
        let finding = one_finding(&data, "sources-disagree", A, "problem");
        assert_eq!(pane_row(&data, A)["action"], "blocked", "{case}: {data}");
        assert_eq!(
            pane_row(&data, A)["session_of_record"],
            Value::Null,
            "{case}"
        );
        assert!(
            message(finding).contains("last-session"),
            "{case}: {finding}"
        );
    }
    let rig = one_pane();
    let b = rig.world.harness.seed_session(rig.world.live(A).port);
    let port = rig.world.live(A).port;
    rig.write(&format!("{A}/sessions.toml"), &sessions_toml(A, port, &b));
    assert_real_run_blocked(&rig, 1);
}

// --- Acceptance 6: the server --------------------------------------------------------------

/// A real run of `rig`'s one pane `A`: it exits 0 and records `A` with no session of record.
fn recorded_without_session(rig: &ImportRig) -> (Value, holler_pane::Pane) {
    let data = data_of(&rig.run(&[], Format::Json));
    assert_eq!(data["recorded"], 1, "{data}");
    let record = rig.record(A).expect("A is recorded");
    assert_eq!(record.session_of_record, None, "{record:?}");
    (data, record)
}

/// The files agree on a stale session that the healthy server does not hold: the record has
/// no session of record, and the stale id is in no record (I6).
#[test]
fn agreeing_files_naming_a_session_the_server_lacks_record_no_session() {
    let rig = one_pane();
    let stale = "ses_made_up_stale_session_0001";
    let port = rig.world.live(A).port;
    rig.write(&format!("{A}/last-session"), &format!("{stale}\n"));
    rig.write(
        &format!("{A}/sessions.toml"),
        &sessions_toml(A, port, stale),
    );
    rig.write(&format!("{A}/owned-sessions"), &owned_sessions(&[stale]));
    let (data, record) = recorded_without_session(&rig);
    one_finding(&data, "session-not-found", A, "warning");
    assert_eq!(record.harness.health, Health::Healthy);
    let stored = serde_json::to_string(&rig.records()).expect("records serialize");
    assert!(
        !stored.contains(stale),
        "no record holds the stale id: {stored}"
    );
}

#[test]
fn a_failed_health_check_is_observe_failed_and_health_unknown() {
    let rig = one_pane();
    let unavailable = PaneError::Unavailable {
        what: "made-up".to_owned(),
    };
    rig.world
        .harness
        .faults()
        .fail_next(HarnessOp::Health, unavailable);
    let (data, record) = recorded_without_session(&rig);
    let finding = one_finding(&data, K::ObserveFailed.code(), A, "warning");
    assert!(message(finding).contains("harness.health"), "{finding}");
    assert_eq!(record.harness.health, Health::Unknown);
}

/// After `health` answered `Ok(true)`, a failed list is `observe-failed` naming the list.
#[test]
fn a_failed_session_list_is_observe_failed() {
    let rig = one_pane();
    let unavailable = PaneError::Unavailable {
        what: "made-up".to_owned(),
    };
    rig.world
        .harness
        .faults()
        .fail_next(HarnessOp::ListSessions, unavailable);
    let (data, record) = recorded_without_session(&rig);
    let finding = one_finding(&data, K::ObserveFailed.code(), A, "warning");
    assert!(
        message(finding).contains("harness.list_sessions"),
        "{finding}"
    );
    assert_eq!(record.harness.health, Health::Unknown, "acceptance 6");
}

/// A change to the server on a port.
type Act = fn(&ImportRig, u16);

/// A server that answers and is not healthy is classified as reconcile does, never as
/// `observe-failed`.
#[test]
fn an_unhealthy_server_is_down_or_wedged_as_reconcile_says() {
    let cases: [(&str, Act, &str); 2] = [
        (
            K::ServerDown.code(),
            |rig, port| rig.world.harness.kill(port).expect("kill"),
            "does not pass its health check",
        ),
        (
            K::ServerWedged.code(),
            |rig, port| rig.world.harness.freeze(port).expect("freeze"),
            "accepts a connection and does not answer",
        ),
    ];
    for (code, act, state) in cases {
        let rig = one_pane();
        let port = rig.world.live(A).port;
        act(&rig, port);
        let (data, record) = recorded_without_session(&rig);
        let finding = one_finding(&data, code, A, "warning");
        let want = format!("the harness server on port {port} {state}");
        assert!(message(finding).contains(&want), "{finding}");
        assert_eq!(record.harness.health, Health::Unhealthy(code.to_owned()));
        assert!(
            super::rig::findings(&data, K::ObserveFailed.code()).is_empty(),
            "{code}: not observe-failed: {data}"
        );
    }
}

#[test]
fn no_recorded_session_is_a_warning_and_prints_a_dash() {
    let rig = one_pane();
    rig.remove(&format!("{A}/last-session"));
    let text = rig.run(&["--dry-run"], Format::Text);
    assert_eq!(text.code, 0, "{text:?}");
    assert_eq!(text_line(&text.out, A)[4], "-", "{}", text.out);
    let (data, _) = recorded_without_session(&rig);
    one_finding(&data, "no-session-recorded", A, "warning");

    let empty = one_pane();
    empty.write(&format!("{A}/last-session"), "  \n");
    let (data, _) = recorded_without_session(&empty);
    one_finding(&data, "no-session-recorded", A, "warning");
}

// --- The tmux session (join item 3) -------------------------------------------------------

#[test]
fn a_missing_tmux_session_is_reconciles_problem_and_blocks() {
    let rig = one_pane();
    let name = PaneName::parse(A).expect("a name");
    rig.world.host.end_session(&name).expect("end the session");
    let data = dry(&rig);
    one_finding(&data, K::TmuxSessionMissing.code(), A, "problem");
    assert_real_run_blocked(&rig, 1);
}

// A failed `ps` was `observe-failed` here, and the pane imported (#650). #835 (brief R2-6)
// reversed that: it is `harness-unreadable` and blocks, pinned in `harness.rs` as
// `a_failed_ps_is_harness_unreadable_and_blocks`.

// --- Acceptance 8: not OpenCode, command shapes, other directories ------------------------

#[test]
fn a_pane_that_is_not_opencode_is_skipped_and_the_others_import() {
    let rig = ImportRig::new(&[Seed::new(A, 1, 1), Seed::new(B, 2, 1)]);
    let live = rig.world.live(A);
    // Any other harness id: since #850 a `claude` attach row with an `endpoint` is refused when the
    // file is parsed (ADR 0014 point 7), so it would block the import as an invalid file instead
    // of being skipped as not OpenCode.
    let row = sessions_toml(A, live.port, &live.session).replace("\"opencode\"", "\"codex\"");
    rig.write(&format!("{A}/sessions.toml"), &row);
    let data = data_of(&rig.run(&[], Format::Json));
    one_finding(&data, "harness-not-opencode", A, "skipped");
    assert_eq!(pane_row(&data, A)["action"], "skipped");
    assert_eq!(data["recorded"], 1, "{data}");
    assert!(rig.record(A).is_none(), "no record for the skipped pane");
    assert!(rig.record(B).is_some(), "the other pane imports");
}

#[test]
fn a_command_array_is_the_argv_and_a_string_is_command_not_argv() {
    let mut rig = ImportRig::new(&[Seed::new(A, 1, 1), Seed::new(B, 2, 1)]);
    let argv = vec![
        "opencode".to_owned(),
        "--model".to_owned(),
        "x y".to_owned(),
    ];
    rig.launcher.get_mut(A).expect("A's entry").command = toml_array(&argv);
    rig.launcher.get_mut(B).expect("B's entry").command = "\"opencode --port 1\"".to_owned();
    rig.write_launcher();
    let data = data_of(&rig.run(&[], Format::Json));
    one_finding(&data, "command-not-argv", B, "warning");
    assert_eq!(data["recorded"], 2, "a warning does not block: {data}");
    let a = rig.record(A).expect("A is recorded");
    assert_eq!(a.command.map(|c| c.as_slice().to_vec()), Some(argv));
    assert_eq!(rig.record(B).expect("B is recorded").command, None);
}

#[test]
fn a_directory_without_pane_files_is_ignored_and_blocks_nothing() {
    let rig = one_pane();
    rig.write("opencode-mo/opencode.json", "{}\n");
    let data = data_of(&rig.run(&[], Format::Json));
    let ignored = super::rig::findings(&data, "ignored-entry");
    assert_eq!(ignored.len(), 1, "{data}");
    assert_eq!(ignored[0]["level"], "info");
    assert!(
        message(ignored[0]).contains("opencode-mo"),
        "{}",
        ignored[0]
    );
    assert_eq!(data["recorded"], 1, "{data}");
    assert!(rig.record(A).is_some());
}
