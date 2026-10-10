//! `holler pane launch --profile P` (brief AC 16a, 16c, 16e, 16g, 16h, 16j and 16k): the spec
//! edit and the live change as one transaction (I8), and the one reconcile step (decision 15).

use holler_cli::output::Format;
use holler_cli::pane::profile_scope::reconcile_step;
use holler_pane::{Actor, PaneError};
use holler_pane_testkit::fixture::sample_profile;
use holler_pane_testkit::harness::HarnessOp;
use holler_pane_testkit::pane_store::PaneStoreOp;

use super::rig::{
    assert_untouched, data_of, error_of, launch_spec, launch_with, launch_without, occurrences,
    profile_name, Rig, PANE,
};

/// The phrase every reconcile step starts with: it must occur once in a message.
const STEP_PHRASE: &str = "to reconcile, run";

/// The next attach fails with `session-not-found` (the act fails at A7).
fn fail_the_attach(rig: &Rig) {
    let missing = PaneError::SessionNotFound {
        what: "ses_gone".into(),
    };
    rig.fakes
        .harness
        .faults()
        .fail_next(HarnessOp::AttachTui, missing);
}

/// The fake scope's restore conflicts: another writer moves `profile` first.
fn conflict_the_restore(rig: &Rig, profile: &'static str) {
    let profiles = rig.fakes.profiles.clone();
    rig.fakes.scope.before_next_restore(move || {
        let theirs = sample_profile(profile, &["demo-c2r1"]).unwrap();
        let actor = Actor::parse("another writer").unwrap();
        profiles.concurrent_put(&theirs, &actor).unwrap();
    });
}

/// AC 16a: P gains exactly the effective spec, in one write, and the record names P.
#[test]
fn launch_with_profile_adds_the_spec_and_bumps_once() {
    let rig = Rig::new();
    let run = rig.run(&launch_with(&["--profile", "demo"]), Format::Json);
    data_of(&run);
    let profile = rig.profile("demo");
    assert_eq!(profile.generation, 2);
    assert_eq!(profile.panes, [launch_spec()]);
    let record = rig.record(PANE).unwrap();
    assert_eq!(record.profile, Some(profile_name("demo")));
    let log: Vec<u64> = rig
        .profile_log("demo")
        .iter()
        .map(|e| e.generation)
        .collect();
    assert_eq!(log, [1, 2], "one write: bumps once");
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 16c (G-8): a failed act puts P's specs back by a second write: the specs are equal, the
/// generation moved by two, and the log shows the edit and its reversal.
#[test]
fn a_failed_act_restores_the_profile_specs() {
    let rig = Rig::new();
    fail_the_attach(&rig);
    let run = rig.run(&launch_with(&["--profile", "demo"]), Format::Json);
    assert_eq!(run.code, 3, "{run:?}");
    assert_eq!(error_of(&run).0, "session-not-found");
    let profile = rig.profile("demo");
    assert!(
        profile.panes.is_empty(),
        "the seeded specs: {:?}",
        profile.panes
    );
    assert_eq!(profile.generation, 3);
    let log: Vec<u64> = rig
        .profile_log("demo")
        .iter()
        .map(|e| e.generation)
        .collect();
    assert_eq!(log, [1, 2, 3]);
    assert_eq!(rig.record(PANE), None);
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 16e: a profile that does not exist is refused before anything.
#[test]
fn a_missing_profile_is_refused() {
    let rig = Rig::new();
    let run = rig.run(&launch_with(&["--profile", "nope"]), Format::Json);
    assert_eq!(run.code, 3, "{run:?}");
    assert_eq!(error_of(&run).0, "profile-not-found");
    assert_untouched(&run.calls);
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 16g: `--spec-only` edits P's spec and changes nothing live; the only pane-store call is
/// the scope's own read of the record.
#[test]
fn spec_only_changes_the_profile_and_nothing_live() {
    let rig = Rig::new();
    let mut argv = launch_without("--herdr-session");
    argv.extend(["--profile", "demo", "--spec-only"]);
    let run = rig.run(&argv, Format::Text);
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.out.contains("nothing live changed"), "{run:?}");
    let profile = rig.profile("demo");
    assert_eq!(
        (profile.generation, profile.panes),
        (2, vec![launch_spec()])
    );
    let live = (&run.calls.herdr, &run.calls.host, &run.calls.harness);
    assert_eq!(
        (live.0.len(), live.1.len(), live.2.len()),
        (0, 0, 0),
        "{live:?}"
    );
    assert!(run.calls.probes.is_empty(), "no probe run");
    assert_eq!(run.calls.panes, [PaneStoreOp::Get], "the scope's read only");
    assert_eq!(rig.record(PANE), None);
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 16h: a restore that conflicts is `profile-conflict` with #663's step for P, printed
/// once.
#[test]
fn a_profile_conflict_after_the_act_fails_loudly() {
    let rig = Rig::new();
    conflict_the_restore(&rig, "demo");
    fail_the_attach(&rig);
    let run = rig.run(&launch_with(&["--profile", "demo"]), Format::Json);
    assert_eq!(run.code, 1, "{run:?}");
    let (code, message) = error_of(&run);
    assert_eq!(code, "profile-conflict");
    let step = reconcile_step(Some(&profile_name("demo")));
    assert!(message.contains(&step), "{message}");
    assert_eq!(occurrences(&message, STEP_PHRASE), 1, "{message}");
    assert_eq!(rig.record(PANE), None);
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 16j: the step quotes the profile's name with #663's quoting, once, and adds none of its
/// own.
#[test]
fn the_reconcile_step_quotes_the_profile() {
    let rig = Rig::with(Vec::new(), vec![sample_profile("it's", &[]).unwrap()]);
    conflict_the_restore(&rig, "it's");
    fail_the_attach(&rig);
    let run = rig.run(&launch_with(&["--profile", "it's"]), Format::Json);
    assert_eq!(run.code, 1, "{run:?}");
    let (code, message) = error_of(&run);
    assert_eq!(code, "profile-conflict");
    let doctor = r"holler pane doctor --profile 'it'\''s'";
    let show = r"holler profile show 'it'\''s'";
    assert_eq!(occurrences(&message, doctor), 1, "{message}");
    assert_eq!(occurrences(&message, show), 1, "{message}");
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 16k (C-15): over #663's real `StoreScope`, a step the scope already printed is not
/// printed again, and an act error the scope returns unchanged gets the verb's step, once.
#[test]
fn a_step_the_real_scope_printed_is_not_repeated() {
    let step = reconcile_step(Some(&profile_name("demo")));
    let rig = Rig::new().with_store_scope();
    rig.after(HarnessOp::Serve, |f, _| {
        let theirs = sample_profile("demo", &["demo-c2r1"]).unwrap();
        let actor = Actor::parse("another writer").unwrap();
        f.profiles.concurrent_put(&theirs, &actor).unwrap();
    });
    fail_the_attach(&rig);
    let run = rig.run(&launch_with(&["--profile", "demo"]), Format::Json);
    assert_eq!(run.code, 1, "conflict: {run:?}");
    let (code, message) = error_of(&run);
    assert_eq!(code, "profile-conflict");
    assert!(message.contains(&step), "{message}");
    assert_eq!(
        occurrences(&message, STEP_PHRASE),
        1,
        "the scope's step only: {message}"
    );
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();

    let rig = Rig::new().with_store_scope();
    fail_the_attach(&rig);
    let run = rig.run(&launch_with(&["--profile", "demo"]), Format::Json);
    assert_eq!(run.code, 3, "no conflict: {run:?}");
    let (code, message) = error_of(&run);
    assert_eq!(code, "session-not-found");
    assert!(message.contains(&step), "{message}");
    assert_eq!(
        occurrences(&message, STEP_PHRASE),
        1,
        "the verb's step only: {message}"
    );
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}
