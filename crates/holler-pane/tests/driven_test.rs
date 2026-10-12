#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #839
//! `driver_of` and `elsewhere` (#839, brief D3, P4, P6 and P8; Acceptance 6 and 20): the body
//! that drives a pane is accepted only on positive evidence.
//!
//! **The exclusion rule (D3).** A body that advertises another pane's session of record, or
//! another pane's recorded DRIVEN, is that pane's and is never a candidate for any rule, the
//! fallback included, unless it also advertises this pane's own session of record or recorded
//! DRIVEN. Of the bodies that remain, the first rule that matches decides: (i) the session of
//! record; (ii) the recorded DRIVEN; (iii) a switch's target; with no evidence, (iv) on a
//! listing shared with no other pane's session of record the listed bodies (one is `One`, more
//! `Ambiguous`), and (v) on a shared listing the listed bodies are `Unproven`.
//!
//! Also here: `activity_refusal`, the one mapping of an `Activity` to its refusal that
//! `check_idle` and the driver check share (Acceptance 8), and `switch_command`, the remedy
//! builder reconcile uses for a body that is behind (D3), and `session_held`, whose
//! `holler release` is built only from an advertised name that parses as a routable name.

mod common;

use holler_pane::driven::{driver_of, elsewhere, Driver};
use holler_pane::findings::switch_command;
use holler_pane::hub::{session_held, Advertised};
use holler_pane::pane::{Health, Hold, LastObserved};
use holler_pane::ports::Activity;
use holler_pane::tx_switch::{
    activity_refusal, SESSION_BUSY, SESSION_HOLDS_PERMISSION, SESSION_HOLDS_QUESTION,
};
use holler_pane::Pane;

/// `ses_`-shaped ids (D8), one per role in the cases.
const SOR1: &str = "ses_00000000000000000000000001";
const OLD1: &str = "ses_00000000000000000000000002";
const NEW1: &str = "ses_00000000000000000000000003";
const SOR2: &str = "ses_00000000000000000000000004";
const D2: &str = "ses_00000000000000000000000005";
const CFG: &str = "ses_00000000000000000000000006";
const OTHER: &str = "ses_00000000000000000000000007";

/// A healthy pane `name` with `sor` as its session of record and `driven` as its recorded
/// DRIVEN.
fn pane(name: &str, sor: Option<&str>, driven: Option<&str>) -> Pane {
    let mut pane = common::pane();
    pane.name = common::pane_name(name);
    pane.hold = Hold::None;
    pane.harness.health = Health::Healthy;
    pane.session_of_record = sor.map(str::to_owned);
    pane.last_observed = LastObserved {
        shown: sor.map(str::to_owned),
        driven: driven.map(str::to_owned),
        at: 1,
    };
    pane
}

/// A live, idle attach body `label/oc` advertising `id`.
fn body(label: &str, id: &str) -> Advertised {
    Advertised {
        token: format!("tok_{label}"),
        name: format!("{label}/oc"),
        harness_session_id: id.to_owned(),
        state: Activity::Idle,
    }
}

fn listed(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|id| (*id).to_owned()).collect()
}

/// The labels of the bodies a `Driver` names, in its order (`None` names none).
fn named(driver: &Driver<'_>) -> (&'static str, Vec<String>) {
    let labels = |bodies: &[&Advertised]| bodies.iter().map(|b| b.name.clone()).collect();
    match driver {
        Driver::None => ("none", Vec::new()),
        Driver::One(one) => ("one", vec![one.name.clone()]),
        Driver::Ambiguous(bodies) => ("ambiguous", labels(bodies)),
        Driver::Unproven(bodies) => ("unproven", labels(bodies)),
    }
}

fn one(label: &str) -> (&'static str, Vec<String>) {
    ("one", vec![format!("{label}/oc")])
}

// --- rules (i) to (iii): evidence -------------------------------------------------------

/// (i) before (ii): a body on the session of record is the driver even when another advertises
/// the recorded DRIVEN.
#[test]
fn the_session_of_record_is_preferred_to_the_recorded_driven() {
    let p1 = pane("demo-c1r1", Some(SOR1), Some(OLD1));
    let bodies = [body("behind", OLD1), body("onrec", SOR1)];
    let driver = driver_of(&p1, &bodies, &listed(&[SOR1, OLD1]), &[], None);
    assert_eq!(named(&driver), one("onrec"));
}

/// (ii): a body that is behind is found by the record's own memory, listed or not (a relaunch
/// onto a new session leaves the body on an id the restarted server no longer lists, E129).
#[test]
fn a_stale_body_is_found_by_the_recorded_driven_listed_or_not() {
    let p1 = pane("demo-c1r1", Some(NEW1), Some(OLD1));
    let bodies = [body("behind", OLD1)];
    for server in [listed(&[NEW1, OLD1]), listed(&[NEW1])] {
        let driver = driver_of(&p1, &bodies, &server, &[], None);
        assert_eq!(named(&driver), one("behind"), "server lists {server:?}");
    }
}

/// (iii): for a switch, a body that already advertises the target is the driver (no rebind is
/// needed), and a reset has no target.
#[test]
fn a_switch_target_names_the_body_that_already_drives_it() {
    let others = [pane("demo-c2r1", Some(SOR2), None)];
    let p1 = pane("demo-c1r1", Some(SOR1), None);
    let bodies = [body("pick", NEW1)];
    let shared = listed(&[SOR1, SOR2, NEW1]);
    let driver = driver_of(&p1, &bodies, &shared, &others, Some(NEW1));
    assert_eq!(named(&driver), one("pick"));
    let reset = driver_of(&p1, &bodies, &shared, &others, None);
    assert_eq!(
        named(&reset).0,
        "unproven",
        "no target: the listed body is unproven"
    );
}

#[test]
fn two_bodies_on_one_piece_of_evidence_are_ambiguous() {
    let p1 = pane("demo-c1r1", Some(SOR1), Some(OLD1));
    let cases = [
        (
            [body("a", SOR1), body("b", SOR1)],
            None,
            "the session of record",
        ),
        (
            [body("a", OLD1), body("b", OLD1)],
            None,
            "the recorded DRIVEN",
        ),
        ([body("a", NEW1), body("b", NEW1)], Some(NEW1), "the target"),
    ];
    for (bodies, target, what) in cases {
        let driver = driver_of(&p1, &bodies, &listed(&[]), &[], target);
        let (kind, mut labels) = named(&driver);
        labels.sort();
        assert_eq!(kind, "ambiguous", "two bodies on {what}");
        assert_eq!(labels, ["a/oc", "b/oc"], "both named: {what}");
    }
}

#[test]
fn no_body_is_none() {
    let p1 = pane("demo-c1r1", Some(SOR1), Some(OLD1));
    assert_eq!(
        named(&driver_of(&p1, &[], &listed(&[SOR1]), &[], Some(NEW1))).0,
        "none"
    );
}

// --- rules (iv) and (v): no evidence ----------------------------------------------------

/// (iv), Acceptance 20 (c): on a listing that names no other pane's session of record, the one
/// listed body is the pane's (a reverted body is found and re-pointed by one switch); two are
/// ambiguous; an unlisted one is not a candidate.
#[test]
fn the_listed_fallback_holds_only_on_an_unshared_listing() {
    let others = [pane("demo-c2r1", Some(SOR2), None)];
    let p1 = pane("demo-c1r1", Some(SOR1), None);
    let own = listed(&[SOR1, CFG]);
    let reverted = [body("rev", CFG)];
    assert_eq!(
        named(&driver_of(&p1, &reverted, &own, &others, None)),
        one("rev")
    );

    let two = [body("a", CFG), body("b", OTHER)];
    let both = listed(&[SOR1, CFG, OTHER]);
    assert_eq!(
        named(&driver_of(&p1, &two, &both, &others, None)).0,
        "ambiguous"
    );

    assert_eq!(
        named(&driver_of(&p1, &reverted, &listed(&[SOR1]), &others, None)).0,
        "none",
        "an unlisted body with no evidence is no candidate"
    );
}

/// (v), Acceptance 20 (a): on a shared listing (it holds another pane's session of record) the
/// listed bodies with no evidence are `Unproven`, all of them named; none listed is `None`.
#[test]
fn listed_bodies_on_a_shared_listing_are_unproven() {
    let others = [pane("demo-c2r1", Some(SOR2), None)];
    let p1 = pane("demo-c1r1", Some(SOR1), None);
    let bodies = [body("a", CFG), body("b", OTHER)];
    let shared = listed(&[SOR1, SOR2, CFG, OTHER]);
    let (kind, mut labels) = named(&driver_of(&p1, &bodies, &shared, &others, None));
    labels.sort();
    assert_eq!(
        (kind, labels),
        ("unproven", vec!["a/oc".to_owned(), "b/oc".to_owned()])
    );

    let unlisted = listed(&[SOR1, SOR2]);
    assert_eq!(
        named(&driver_of(&p1, &bodies, &unlisted, &others, None)).0,
        "none"
    );
}

/// Acceptance 20 (b), the second plan check's block: P1 has no body; P2's body is behind and
/// advertises its configured id, which the shared server lists and nobody's evidence names. It
/// is `Unproven` for P1, never `One`. With that body on P2's recorded DRIVEN instead, the
/// exclusion rule removes it for P1 (`None`), and P2 finds it by rule (ii).
#[test]
fn a_pane_with_no_body_never_claims_another_pane_s_behind_body() {
    let p2 = pane("demo-c2r1", Some(SOR2), None);
    let p1 = pane("demo-c1r1", Some(SOR1), None);
    let shared = listed(&[SOR1, SOR2, CFG, D2]);
    let reverted = [body("p2body", CFG)];
    let driver = driver_of(&p1, &reverted, &shared, std::slice::from_ref(&p2), None);
    assert_eq!(named(&driver), ("unproven", vec!["p2body/oc".to_owned()]));

    let p2 = pane("demo-c2r1", Some(SOR2), Some(D2));
    let behind = [body("p2body", D2)];
    let driver = driver_of(&p1, &behind, &shared, std::slice::from_ref(&p2), None);
    assert_eq!(
        named(&driver).0,
        "none",
        "the exclusion rule removes P2's body"
    );
    let driver = driver_of(&p2, &behind, &shared, &[p1], None);
    assert_eq!(
        named(&driver),
        one("p2body"),
        "P2 finds it by its recorded DRIVEN"
    );
}

// --- the exclusion rule, Acceptance 20 (d) ----------------------------------------------

/// Rows (1) and (2): a body on P2's session of record, or on P2's recorded DRIVEN, is never
/// P1's, under any rule: as a switch target (iii), listed on an unshared or a shared listing
/// (iv, v), or with no listing at all.
#[test]
fn a_body_another_pane_s_evidence_names_is_never_a_candidate() {
    let rows = [
        (
            "row 1, P2's session of record",
            pane("demo-c2r1", Some(OTHER), None),
        ),
        (
            "row 2, P2's recorded DRIVEN",
            pane("demo-c2r1", Some(SOR2), Some(OTHER)),
        ),
    ];
    let p1 = pane("demo-c1r1", Some(SOR1), Some(OLD1));
    let bodies = [body("theirs", OTHER)];
    for (row, p2) in rows {
        let others = [p2];
        let listings = [
            listed(&[]),
            listed(&[SOR1, OTHER]),
            listed(&[SOR1, SOR2, OTHER]),
        ];
        for server in &listings {
            for target in [None, Some(OTHER)] {
                let driver = driver_of(&p1, &bodies, server, &others, target);
                assert_eq!(
                    named(&driver).0,
                    "none",
                    "{row}: server {server:?}, target {target:?}"
                );
            }
        }
    }
}

/// Rows (3) and (4): a body that also advertises P1's own evidence is P1's; this pane's
/// evidence is checked first, so a stale DRIVEN on another pane never takes a pane's own body.
/// Row (4) gives `One` for P1 as decided (P8), though P2's record names the same id.
#[test]
fn a_body_on_this_pane_s_own_evidence_is_kept_by_the_exception() {
    let p1 = pane("demo-c1r1", Some(SOR1), Some(OLD1));
    let rows = [
        (
            "row 3, P1's record that P2's stale DRIVEN names",
            pane("demo-c2r1", Some(SOR2), Some(SOR1)),
            SOR1,
        ),
        (
            "row 4, P1's recorded DRIVEN that P2's record names",
            pane("demo-c2r1", Some(OLD1), None),
            OLD1,
        ),
    ];
    for (row, p2, id) in rows {
        let bodies = [body("mine", id)];
        for server in [listed(&[]), listed(&[SOR1, OLD1, SOR2])] {
            let driver = driver_of(&p1, &bodies, &server, std::slice::from_ref(&p2), None);
            assert_eq!(named(&driver), one("mine"), "{row}: server {server:?}");
        }
    }
}

/// The result does not depend on pane order: `others` is the records as read, in any order.
#[test]
fn the_join_does_not_depend_on_the_order_of_the_other_panes() {
    let p1 = pane("demo-c1r1", Some(SOR1), None);
    let a = pane("demo-c2r1", Some(SOR2), Some(OTHER));
    let b = pane("demo-c3r1", Some(D2), None);
    let bodies = [body("theirs", OTHER), body("mine", SOR1)];
    let server = listed(&[SOR1, SOR2, D2, OTHER]);
    let forward = driver_of(&p1, &bodies, &server, &[a.clone(), b.clone()], None);
    let backward = driver_of(&p1, &bodies, &server, &[b, a], None);
    assert_eq!(named(&forward), one("mine"));
    assert_eq!(named(&forward), named(&backward));
}

// --- elsewhere (P6) ---------------------------------------------------------------------

/// A switch's target advertised by a live body other than the driver is refused in the plan;
/// `elsewhere` names those bodies, the ones the exclusion rule removed included, and never the
/// driver itself (the idempotent re-run).
#[test]
fn elsewhere_names_the_other_bodies_on_the_target() {
    let p2 = pane("demo-c2r1", Some(NEW1), None);
    let p1 = pane("demo-c1r1", Some(SOR1), None);
    let bodies = [body("mine", SOR1), body("theirs", NEW1)];
    let server = listed(&[SOR1, NEW1]);
    let driver = driver_of(&p1, &bodies, &server, &[p2], Some(NEW1));
    let Driver::One(mine) = &driver else {
        panic!("P1's own body drives it: {:?}", named(&driver));
    };
    assert_eq!(mine.name, "mine/oc");
    let found: Vec<&str> = elsewhere(&bodies, Some(*mine), NEW1)
        .iter()
        .map(|b| b.name.as_str())
        .collect();
    assert_eq!(found, ["theirs/oc"], "the removed body counts");

    let already = [body("mine", NEW1)];
    assert!(
        elsewhere(&already, Some(&already[0]), NEW1).is_empty(),
        "the driver on the target is the idempotent re-run, not another body"
    );
    assert_eq!(
        elsewhere(&bodies, None, NEW1).len(),
        1,
        "no driver: every body on it"
    );
}

// --- activity_refusal (Acceptance 8) ----------------------------------------------------

/// The one mapping, a total match over the four variants: the driver check and `check_idle`
/// both call it (the verbs' side of the table is in `holler-cli`'s `pane_verbs/driven.rs`).
#[test]
fn activity_refusal_is_one_total_mapping() {
    assert_eq!(activity_refusal(Activity::Idle).map(|(code, _)| code), None);
    let table = [
        (Activity::Busy, SESSION_BUSY),
        (Activity::HoldingQuestion, SESSION_HOLDS_QUESTION),
        (Activity::WaitingPermission, SESSION_HOLDS_PERMISSION),
    ];
    for (activity, want) in table {
        let (code, why) = activity_refusal(activity).expect("refused");
        assert_eq!(code, want, "{activity:?}");
        assert!(!why.is_empty(), "{activity:?} says why");
    }
}

// --- the remedy builder ------------------------------------------------------------------

/// `holler pane switch PANE SESSION`, built for an id that parses, and `None` for one that does
/// not (advertised text is untrusted, so a command is never spelled from it).
#[test]
fn the_switch_remedy_is_built_only_for_a_valid_session_id() {
    let pane = common::pane_name("demo-c1r1");
    assert_eq!(
        switch_command(&pane, SOR1).as_deref(),
        Some(format!("holler pane switch demo-c1r1 {SOR1}").as_str())
    );
    let long = "s".repeat(65);
    for bad in ["", "ses x", "ses_1;rm -rf /", "ses_\n1", long.as_str()] {
        assert_eq!(switch_command(&pane, bad), None, "{bad:?}");
    }
}

/// D3's untrusted-text rule for `session-held`: the advertised name is quoted, and `holler
/// release NAME` is built only from a name that parses as a routable name; any other name, or
/// none, names the id (quoted) and spells no name.
#[test]
fn the_release_command_is_built_only_from_a_routable_name() {
    let held = session_held(SOR1, Some("kiwi/oc"), "operator", Some("night"));
    assert_eq!(held.code().to_string(), "session-held");
    let text = held.to_string();
    assert!(
        text.contains("\"kiwi/oc\" is held (operator hold, reason \"night\")"),
        "{text}"
    );
    assert!(
        text.contains("run holler release kiwi/oc to lift it"),
        "{text}"
    );
    for bad in [
        "kiwi/oc; rm -rf ~",
        "kiwi/a b",
        "kiwi/oc\n\u{1b}[2J",
        "kiwi/OC",
    ] {
        let text = session_held(SOR1, Some(bad), "default", None).to_string();
        assert!(!text.contains(bad), "{bad:?}: {text}");
        assert!(!text.contains("holler release kiwi"), "{bad:?}: {text}");
        assert!(
            text.contains(&format!("{SOR1:?} is held (default hold)")),
            "{bad:?}: {text}"
        );
    }
    let unnamed = session_held(SOR1, None, "operator", None).to_string();
    assert!(!unnamed.contains("holler release kiwi"), "{unnamed}");
}
