//! `unattached` (#647 part 2, ADR-0021 section 12, brief D9 to D11): a registered pane whose
//! tmux session exists and has no tmux client attached, so no Herdr pane shows it. The rule
//! reads `HostPort::attached_clients` once `host.ps` answered, for every pane whatever its
//! hold, role or harness; it names no command, because no Holler verb attaches a client.
//!
//! The rig attaches one client per pane (`rig::make_live`), as a Herdr pane does; a test that
//! wants a pane unattached takes its client away with the fake host's `detach_clients`.

use holler_cli::output::Format;
use holler_pane::findings::{FindingKind as K, FixState};
use holler_pane::pane::Hold;
use holler_pane::{HostPort, PaneError};
use holler_pane_testkit::envelope::check_envelope;
use holler_pane_testkit::host::HostOp;

use super::rig::{json_data, json_findings, keys, kinds, of_kind, one, whole, Rig, Seed};
use super::{one_pane, PANE};

/// D11's words, which the message carries: what is wrong, and that no verb cures it.
const NO_CLIENT: &str = "no tmux client is attached";
const NO_VERB: &str = "no Holler verb attaches one";

/// Take every client away from `pane`'s tmux session.
fn detach(rig: &Rig, pane: &str) {
    rig.host
        .detach_clients(&rig.live(pane).name)
        .expect("the session exists");
}

/// The issue's acceptance: a running, registered session with no attached client raises
/// exactly one `unattached`, with no remedy and nothing `--fix` can do; the same pane attached
/// raises nothing.
#[test]
fn a_session_with_no_client_raises_one_unattached_and_attached_raises_nothing() {
    let rig = one_pane();
    detach(&rig, PANE);

    let report = rig.run(&whole(false));

    let finding = one(&report, K::Unattached, PANE);
    assert_eq!(report.findings.len(), 1, "{:?}", keys(&report));
    assert_eq!(finding.remedy, None, "{finding:?}");
    assert_eq!(finding.fix, FixState::NotFixable, "{finding:?}");
    assert_eq!(
        finding.grid,
        Some(rig.record(PANE).herdr.grid),
        "{finding:?}"
    );
    assert!(finding.message.contains(NO_CLIENT), "{:?}", finding.message);
    assert!(finding.message.contains(NO_VERB), "{:?}", finding.message);

    rig.host
        .attach_client(&rig.live(PANE).name)
        .expect("the session exists");
    let report = rig.run(&whole(false));
    assert!(report.findings.is_empty(), "{:?}", keys(&report));
}

/// The verb: the text line names the pane, its position and the stable code; the JSON
/// envelope is valid and carries the code with a null remedy; both formats exit alike.
#[test]
fn the_verb_reports_unattached_in_both_formats() {
    let rig = Rig::new(&[Seed::new("demo-c1r2", 2, 1)]);
    detach(&rig, "demo-c1r2");

    let text = rig.verb(&["pane", "doctor"], Format::Text);
    let json = rig.verb(&["pane", "doctor"], Format::Json);

    check_envelope(&json.out, json.code).unwrap_or_else(|e| panic!("a valid envelope: {e}"));
    assert_eq!(text.code, json.code, "text {text:?} vs json {json:?}");
    assert!(
        text.out
            .lines()
            .any(|l| l.starts_with("demo-c1r2 r2c1 unattached: ")),
        "{}",
        text.out
    );
    let data = json_data(&json);
    let found = json_findings(&data, "unattached");
    assert_eq!(found.len(), 1, "{data}");
    assert_eq!(found[0]["pane"], "demo-c1r2", "{data}");
    assert_eq!(found[0]["grid"]["pos"], "r2c1", "{data}");
    assert!(found[0]["remedy"].is_null(), "{data}");
    assert_eq!(found[0]["fix"], "not-fixable", "{data}");
}

/// A missing session is `tmux-session-missing` only: the count is never asked for it.
#[test]
fn a_missing_session_raises_tmux_session_missing_and_no_count_is_asked() {
    let rig = one_pane();
    rig.host
        .end_session(&rig.live(PANE).name)
        .expect("end the session");
    let mark = rig.mark();

    let report = rig.run(&whole(false));

    one(&report, K::TmuxSessionMissing, PANE);
    assert!(
        of_kind(&report, K::Unattached).is_empty(),
        "{:?}",
        keys(&report)
    );
    let calls = rig.calls_since(&mark);
    assert!(
        !calls.host.contains(&HostOp::AttachedClients),
        "{:?}",
        calls.host
    );
}

/// A failed count is `observe-failed` naming `host.attached_clients`, and no `unattached`:
/// nothing is concluded from a read that failed.
#[test]
fn a_failed_count_is_observe_failed_and_not_unattached() {
    let rig = one_pane();
    detach(&rig, PANE);
    rig.host.faults().fail_next(
        HostOp::AttachedClients,
        PaneError::Unavailable {
            what: "tmux list-panes printed a made-up line".to_owned(),
        },
    );

    let report = rig.run(&whole(false));

    let failed = one(&report, K::ObserveFailed, PANE);
    assert!(
        failed.message.contains("host.attached_clients"),
        "{:?}",
        failed.message
    );
    assert!(
        of_kind(&report, K::Unattached).is_empty(),
        "{:?}",
        keys(&report)
    );
}

/// The rule is not gated on the hold or the role: a parked pane and the orchestrator's pane
/// with no client raise it too.
#[test]
fn a_parked_pane_and_the_orchestrators_pane_raise_it_too() {
    let rig = Rig::new(&[
        Seed::new("demo-parked", 1, 1),
        Seed::new("demo-orch", 1, 2).orchestrator(),
    ]);
    rig.rewrite("demo-parked", |pane| {
        pane.hold = Hold::Parked {
            reason: "a test park".to_owned(),
            release_when: "never".to_owned(),
            since: 0,
        }
    });
    detach(&rig, "demo-parked");
    detach(&rig, "demo-orch");

    let report = rig.run(&whole(false));

    one(&report, K::Unattached, "demo-parked");
    one(&report, K::Unattached, "demo-orch");
}

/// The rig's attach is what keeps its panes clean, not the rule being off: the same world with
/// every client taken away raises `unattached` for every pane, and nothing else.
#[test]
fn a_world_built_without_a_client_raises_unattached_for_every_pane() {
    let rig = Rig::new(&[
        Seed::new("demo-c1r1", 1, 1),
        Seed::new("demo-c2r1", 1, 2),
        Seed::new("demo-c3r1", 1, 3).orchestrator(),
    ]);
    for pane in ["demo-c1r1", "demo-c2r1", "demo-c3r1"] {
        detach(&rig, pane);
    }

    let report = rig.run(&whole(false));

    assert_eq!(kinds(&report), vec!["unattached"; 3], "{:?}", keys(&report));
}

/// Once per pass, and nothing written for it: a second pass over the same world reports the
/// same finding once more and writes no record (the first pass recorded what it observed).
#[test]
fn unattached_is_reported_once_per_pass_and_writes_nothing() {
    let rig = one_pane();
    detach(&rig, PANE);
    let first = rig.run(&whole(false));
    let stored = rig.record(PANE);

    let second = rig.run(&whole(false));

    assert_eq!(keys(&first), keys(&second));
    assert_eq!(
        of_kind(&second, K::Unattached).len(),
        1,
        "{:?}",
        keys(&second)
    );
    assert_eq!(rig.record(PANE), stored, "the second pass wrote nothing");
}

/// `--fix` repairs nothing of it: the finding stays not fixable, the record is unchanged but
/// for what the pass observed, and no client is attached.
#[test]
fn fix_does_nothing_for_unattached() {
    let rig = one_pane();
    detach(&rig, PANE);
    rig.run(&whole(false));
    let before = rig.record(PANE);

    let report = rig.run(&whole(true));

    let finding = one(&report, K::Unattached, PANE);
    assert_eq!(finding.fix, FixState::NotFixable, "{finding:?}");
    assert_eq!(finding.remedy, None, "{finding:?}");
    assert_eq!(rig.record(PANE), before, "--fix wrote nothing for it");
    assert_eq!(rig.host.attached_clients(&rig.live(PANE).name), Ok(0));
}
