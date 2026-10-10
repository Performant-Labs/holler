//! `holler pane close PANE [--profile NAME] [--spec-only]` (story #646, part 2).
//!
//! Every case runs the verb in-process over the rig of fakes (`close/rig.rs`), whose
//! call-log span proves the order the verb works in: stop the pane's owned processes,
//! close its Herdr pane, then delete its record (`PaneStore::delete`, ruling 9) —
//! never moving a pane's Herdr position (no `ensure_pane`, no keystroke, I4). With
//! `--profile P` the spec removal and the live close are one transaction
//! (`ProfileScope::edit_spec`, I8): P's generation moves once on success, and a failed
//! close leaves P's specs as they were (the generation may move by two — the edit and
//! its reversal, the scope's own compensation). `--spec-only` removes the spec and
//! closes nothing; a spec-less pane is refused `pane-not-in-profile` before any write
//! (binding 4: spec presence, checked before `edit_spec`, whose no-op `Remove` would
//! still CAS-write a silent generation bump).
//!
//! Exit codes equal across formats (0 ok, 3 refusal, 2 usage, 1 failure) and every
//! JSON run passes the test kit's envelope helper — both held by `run_both`.

pub(crate) mod rig;

use clap::Parser;
use holler_cli::output::Format;
use holler_cli::{Cli, Command};
use holler_pane::{PaneError, ProfileChange};
use holler_pane_testkit::fault::Fault;
use holler_pane_testkit::herdr::HerdrOp;
use holler_pane_testkit::host::HostOp;
use holler_pane_testkit::pane_store::PaneStoreOp;
use holler_pane_testkit::profile_store::ProfileStoreOp;
use serde_json::json;

use rig::{alpha_world, assert_untouched, bare_step, profile, profile_step, run_both, Rig};

/// `pane close <name>` with the flags of the case appended.
fn close<'a>(name: &'a str, flags: &[&'a str]) -> Vec<&'a str> {
    let mut argv = vec!["pane", "close", name];
    argv.extend_from_slice(flags);
    argv
}

/// The profile-carrying flags of the `--profile` cases.
const IN_ALPHA: [&str; 2] = ["--profile", "Demo Alpha"];

/// `--profile "Demo Alpha" --spec-only`.
const SPEC_ONLY_ALPHA: [&str; 3] = ["--profile", "Demo Alpha", "--spec-only"];

/// The names of P's specs, in order.
fn spec_names(alpha: &holler_pane::Profile) -> Vec<&str> {
    alpha.panes.iter().map(|spec| spec.pane.as_str()).collect()
}

/// The close acceptance (issue #646): the verb parses with a PANE positional, and only
/// one positional.
#[test]
fn close_takes_one_pane_positional() {
    let parsed = Cli::try_parse_from(["holler", "pane", "close", "demo-c1r1"])
        .expect("`pane close PANE` parses (the verb owns its own clap Args)");
    assert!(matches!(parsed.command, Command::Pane(_)));
    assert!(
        Cli::try_parse_from(["holler", "pane", "close", "a", "b"]).is_err(),
        "only PANE, no second positional"
    );
}

/// AC (close leaves no owned process): one live pane is stopped, closed and deleted —
/// the host call log holds exactly `stop_owned`, the Herdr log exactly `close` (no
/// `ensure_pane`: a close never moves a pane's Herdr position; no keystroke, I4), and
/// the record is gone.
#[test]
fn close_stops_the_processes_closes_the_herdr_pane_and_deletes_the_record() {
    let rig = Rig::new();
    let seeded = rig.live("demo-c1r1", None);
    let run = rig.run(&close("demo-c1r1", &[]), Format::Text);
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.err, "", "{run:?}");
    assert_eq!(run.out, "closed demo-c1r1\n");
    assert_eq!(run.calls.host, vec![HostOp::StopOwned], "stop_owned, once");
    assert_eq!(run.calls.herdr, vec![HerdrOp::Close], "herdr close, once");
    let writes: Vec<PaneStoreOp> = run
        .calls
        .panes
        .iter()
        .filter(|op| matches!(op, PaneStoreOp::CasPut | PaneStoreOp::Delete))
        .copied()
        .collect();
    assert_eq!(writes, vec![PaneStoreOp::Delete], "one delete, no put");
    assert_eq!(run.calls.profiles, vec![], "no profile call at all");
    rig.assert_gone(&seeded);
}

/// The same close in JSON: one envelope (the helper checks it) with
/// `{"pane": <name>}` as its data, and the exit codes equal across formats.
#[test]
fn close_json_is_one_envelope_and_the_exits_agree() {
    let world = || {
        let rig = Rig::new();
        rig.live("demo-c1r1", None);
        rig
    };
    let both = run_both(world, &close("demo-c1r1", &[]));
    assert_eq!(both.text.out, "closed demo-c1r1\n");
    assert_eq!(both.envelope.data, json!({"pane": "demo-c1r1"}));
}

/// AC (`close --profile P`): the spec removal and the live close are one transaction —
/// P's spec for the pane is gone, its generation moved exactly once, its log carries
/// the one update — and the live close and the record delete are exactly the bare
/// verb's.
#[test]
fn close_with_a_profile_removes_the_spec_and_bumps_the_generation_once() {
    let both = run_both(alpha_world, &close("demo-c1r1", &IN_ALPHA));
    assert_eq!(
        both.text.out,
        "closed demo-c1r1 (removed from profile \"Demo Alpha\")\n"
    );
    assert_eq!(
        both.envelope.data,
        json!({"pane": "demo-c1r1", "generation": 2}),
        "{}",
        both.json.out
    );
    for (rig, run) in both.each() {
        let alpha = rig.profile("Demo Alpha");
        assert_eq!(spec_names(&alpha), ["demo-c2r1"], "the spec is removed");
        assert_eq!(alpha.generation, 2, "one write, one bump");
        let log = rig.profile_log("Demo Alpha");
        assert_eq!(log.len(), 2, "the seed's `created` plus one update");
        assert!(
            matches!(log[1].change, ProfileChange::Updated { .. }),
            "the one write is logged as an update: {:?}",
            log[1].change
        );
        assert_eq!(run.calls.host, vec![HostOp::StopOwned]);
        assert_eq!(run.calls.herdr, vec![HerdrOp::Close]);
        assert_eq!(
            run.calls
                .profiles
                .iter()
                .filter(|op| **op == ProfileStoreOp::CasPut)
                .count(),
            1,
            "exactly one profile write"
        );
        assert_eq!(
            run.calls
                .panes
                .iter()
                .filter(|op| matches!(op, PaneStoreOp::CasPut | PaneStoreOp::Delete))
                .count(),
            1,
            "the record delete is the one pane write"
        );
        assert!(rig.record("demo-c1r1").is_none(), "the record is gone");
    }
}

/// AC (a failed close leaves P unchanged): a live close that fails after the first
/// live call fails the run, restores P's specs (the generation moved by two — the edit
/// and its reversal, the scope's own compensation), leaves the record in place, and
/// its message ends with the profile's reconcile step (ruling 7).
#[test]
fn a_failed_live_close_restores_the_profile_specs() {
    let world = || {
        let rig = alpha_world();
        rig.herdr.faults().fail_next(
            HerdrOp::Close,
            PaneError::Unavailable {
                what: "the Herdr socket is gone".into(),
            },
        );
        rig
    };
    let both = run_both(world, &close("demo-c1r1", &IN_ALPHA));
    both.assert_error("unavailable", 1, Some(&profile_step("Demo Alpha")));
    assert!(
        both.text.err.contains("demo-c1r1"),
        "the message names the pane: {:?}",
        both.text.err
    );
    for (rig, run) in both.each() {
        let alpha = rig.profile("Demo Alpha");
        assert_eq!(
            spec_names(&alpha),
            ["demo-c1r1", "demo-c2r1"],
            "P's specs are unchanged"
        );
        assert_eq!(alpha.generation, 3, "the edit and its reversal");
        assert_eq!(
            rig.profile_log("Demo Alpha").len(),
            3,
            "created, the edit, the reversal"
        );
        assert!(
            rig.record("demo-c1r1").is_some(),
            "the record is not deleted"
        );
        assert_eq!(run.calls.host, vec![HostOp::StopOwned], "the act began");
        assert_eq!(
            run.calls.herdr,
            vec![HerdrOp::Close],
            "the close was attempted"
        );
    }
}

/// AC (`close --spec-only` changes P and nothing live): the spec is removed with no
/// adapter call at all and no record delete — the pane keeps its record, its Herdr
/// pane and its process.
#[test]
fn close_spec_only_removes_the_spec_and_touches_nothing_live() {
    let both = run_both(alpha_world, &close("demo-c1r1", &SPEC_ONLY_ALPHA));
    assert_eq!(
        both.text.out,
        "removed demo-c1r1 from profile \"Demo Alpha\"\n"
    );
    assert_eq!(
        both.envelope.data,
        json!({"pane": "demo-c1r1", "generation": 2}),
        "{}",
        both.json.out
    );
    for (rig, run) in both.each() {
        assert_eq!(
            spec_names(&rig.profile("Demo Alpha")),
            ["demo-c2r1"],
            "the spec is removed"
        );
        assert!(rig.record("demo-c1r1").is_some(), "the record stays");
        assert_eq!(run.calls.herdr, vec![], "no Herdr call");
        assert_eq!(run.calls.host, vec![], "no host call");
        assert_eq!(run.calls.harness, vec![], "no harness call");
        assert_eq!(run.calls.probes, vec![], "no probe run");
        assert_untouched(&run.calls);
    }
}

/// Binding 8 (`--spec-only` needs no record): a detached spec for a pane with no
/// record is exactly what `--spec-only` cleans up — the spec goes, the run succeeds.
#[test]
fn close_spec_only_removes_a_detached_spec_without_a_record() {
    let world = || {
        let rig = Rig::with_profiles([profile("Demo Alpha", &["demo-c1r1", "demo-c5r1"])]);
        rig.live("demo-c1r1", Some("Demo Alpha"));
        rig
    };
    let both = run_both(world, &close("demo-c5r1", &SPEC_ONLY_ALPHA));
    assert_eq!(both.text.code, 0, "{:?}", both.text);
    assert_eq!(
        both.text.out,
        "removed demo-c5r1 from profile \"Demo Alpha\"\n"
    );
    for (rig, _run) in both.each() {
        assert_eq!(
            spec_names(&rig.profile("Demo Alpha")),
            ["demo-c1r1"],
            "only the detached spec is gone"
        );
        assert!(rig.record("demo-c5r1").is_none(), "still no record");
    }
}

/// Binding 4 (the spec-presence pre-check): a member whose spec is absent from P is
/// refused `pane-not-in-profile` before any write — in the live `--profile` path and
/// the `--spec-only` path alike — so P's generation never moves and nothing live is
/// called.
#[test]
fn close_refuses_a_spec_less_pane_before_any_profile_write() {
    let world = || {
        let rig = Rig::with_profiles([profile("Demo Alpha", &["demo-c2r1"])]);
        rig.live("demo-c1r1", Some("Demo Alpha"));
        rig
    };
    for flags in [&IN_ALPHA[..], &SPEC_ONLY_ALPHA[..]] {
        let both = run_both(world, &close("demo-c1r1", flags));
        both.assert_error("pane-not-in-profile", 3, None);
        both.assert_untouched();
        for (rig, _run) in both.each() {
            assert_eq!(
                rig.profile("Demo Alpha").generation,
                1,
                "no silent generation bump"
            );
            assert!(
                rig.record("demo-c1r1").is_some(),
                "the record is not deleted"
            );
        }
    }
}

/// AC (a named pane with no record): `pane-not-found`, exit 3, nothing called, and no
/// reconcile step (a plan refusal, ruling 7).
#[test]
fn close_refuses_a_pane_with_no_record() {
    let both = run_both(Rig::new, &close("demo-c9r9", &[]));
    both.assert_error("pane-not-found", 3, None);
    assert!(
        both.text.err.contains("demo-c9r9"),
        "the message names the pane: {:?}",
        both.text.err
    );
    both.assert_untouched();
}

/// `pane close` with no PANE is a usage error (exit 2) refused by the verb, touching
/// nothing — park's own pattern (an optional positional the verb itself requires), so
/// the tail parses and the verb owns the message.
#[test]
fn close_needs_a_pane() {
    let both = run_both(Rig::new, &["pane", "close"]);
    both.assert_error("usage", 2, None);
    both.assert_untouched();
}

/// AC (without `--profile` no profile changes): the bare close makes no profile-store
/// call at all, not even a read.
#[test]
fn close_without_a_profile_never_touches_a_profile() {
    let world = || {
        let rig = Rig::new();
        rig.live("demo-c1r1", None);
        rig
    };
    let both = run_both(world, &close("demo-c1r1", &[]));
    assert_eq!(both.text.code, 0, "{:?}", both.text);
    for (_rig, run) in both.each() {
        assert_eq!(
            run.calls.profiles,
            vec![],
            "no profile-store call at all: {:?}",
            run.calls.profiles
        );
    }
}

/// Ruling 7 (the step follows failures from the first live call on): a failed
/// `stop_owned` sends nothing to Herdr and deletes nothing but carries the step; a
/// failed Herdr close has stopped the processes, keeps the record, and carries the
/// step too.
#[test]
fn a_live_failure_after_stop_owned_names_the_reconcile_step() {
    let stop_failed = || {
        let rig = Rig::new();
        rig.live("demo-c1r1", None);
        rig.host.faults().fail_next(
            HostOp::StopOwned,
            PaneError::Unavailable {
                what: "tmux is gone".into(),
            },
        );
        rig
    };
    let both = run_both(stop_failed, &close("demo-c1r1", &[]));
    both.assert_error("unavailable", 1, Some(&bare_step()));
    for (rig, run) in both.each() {
        assert_eq!(run.calls.herdr, vec![], "nothing was closed");
        assert!(rig.record("demo-c1r1").is_some(), "the record stays");
    }

    let close_failed = || {
        let rig = Rig::new();
        rig.live("demo-c1r1", None);
        rig.herdr.faults().fail_next(
            HerdrOp::Close,
            PaneError::Unavailable {
                what: "the Herdr socket is gone".into(),
            },
        );
        rig
    };
    let both = run_both(close_failed, &close("demo-c1r1", &[]));
    both.assert_error("unavailable", 1, Some(&bare_step()));
    for (rig, run) in both.each() {
        assert_eq!(run.calls.host, vec![HostOp::StopOwned], "the act began");
        assert_eq!(
            run.calls.herdr,
            vec![HerdrOp::Close],
            "the close was attempted"
        );
        assert!(
            rig.record("demo-c1r1").is_some(),
            "no delete after a failed act"
        );
    }
}

/// A record delete that conflicts (another writer moved the record during the act)
/// fails loudly with `generation-conflict` and the step — bare, and with `--profile`,
/// where the scope has already restored P's specs (ADR-0021 section 8, step 5).
#[test]
fn a_record_conflict_after_the_act_fails_loudly() {
    let bare = || {
        let rig = Rig::new();
        rig.live("demo-c1r1", None);
        rig.panes
            .faults()
            .fail_next(PaneStoreOp::Delete, PaneError::Conflict);
        rig
    };
    let both = run_both(bare, &close("demo-c1r1", &[]));
    both.assert_error("generation-conflict", 1, Some(&bare_step()));
    for (rig, run) in both.each() {
        assert_eq!(run.calls.host, vec![HostOp::StopOwned]);
        assert_eq!(run.calls.herdr, vec![HerdrOp::Close]);
        assert!(rig.record("demo-c1r1").is_some(), "the delete did not land");
    }

    let scoped = || {
        let rig = alpha_world();
        rig.panes
            .faults()
            .fail_next(PaneStoreOp::Delete, PaneError::Conflict);
        rig
    };
    let both = run_both(scoped, &close("demo-c1r1", &IN_ALPHA));
    both.assert_error("generation-conflict", 1, Some(&profile_step("Demo Alpha")));
    for (rig, _run) in both.each() {
        let alpha = rig.profile("Demo Alpha");
        assert_eq!(
            spec_names(&alpha),
            ["demo-c1r1", "demo-c2r1"],
            "P's specs are restored"
        );
        assert_eq!(alpha.generation, 3, "the edit and its reversal");
    }
}

/// A store failure before anything moves is a runtime failure (exit 1), touching
/// nothing live.
#[test]
fn a_store_failure_fails_the_run() {
    let world = || {
        let rig = Rig::new();
        rig.panes
            .faults()
            .set(Some(Fault::Fail(PaneError::Unavailable {
                what: "pane store".into(),
            })));
        rig.live("demo-c1r1", None);
        rig
    };
    let both = run_both(world, &close("demo-c1r1", &[]));
    both.assert_error("unavailable", 1, None);
    for (rig, run) in both.each() {
        rig.panes.faults().set(None);
        assert!(rig.record("demo-c1r1").is_some(), "the record stays");
        assert_eq!(run.calls.herdr, vec![], "nothing live was called");
        assert_eq!(run.calls.host, vec![]);
    }
}
