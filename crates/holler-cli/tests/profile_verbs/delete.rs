//! `holler profile delete` (story #662b, AC 3): refused while the profile has members;
//! `--keep-panes` detaches each member first (the panes keep running), then deletes, and
//! never re-attaches after a failed detach (Decision 8).

use holler_pane::{Pane, PaneError, PaneStore, Profile, ProfileName, ProfileStore};
use holler_pane_testkit::pane_store::PaneStoreOp;
use holler_pane_testkit::profile_store::ProfileStoreOp;
use serde_json::json;

use crate::list::rig::{
    assert_failure, assert_message_contains, matching_spec, member, pane, profile, run_both,
    run_both_seamed, run_both_with, NthPut, Rig,
};

const NAME: &str = "Some Profile";

fn stored(rig: &Rig, text: &str) -> Option<Profile> {
    rig.profiles
        .get(&ProfileName::parse(text).unwrap())
        .unwrap()
}

fn has_call<Op: PartialEq>(calls: &[Op], op: Op) -> bool {
    calls.contains(&op)
}

/// `Some Profile` with two members and a pane in no profile.
fn two_members() -> Rig {
    Rig::new(
        [
            member("demo-c1r1", NAME),
            member("demo-c2r1", NAME),
            pane("demo-c3r1"),
        ],
        [profile(
            NAME,
            vec![matching_spec("demo-c1r1"), matching_spec("demo-c2r1")],
        )],
    )
}

/// The profile of each pane in `rig`, by name, in `list()` order.
fn memberships(rig: &Rig) -> Vec<(String, Option<String>)> {
    rig.panes
        .list()
        .unwrap()
        .into_iter()
        .map(|p| (p.name.to_string(), p.profile.map(|n| n.as_str().to_owned())))
        .collect()
}

#[test]
fn delete_removes_a_profile_without_members_in_both_formats() {
    // A pane of another profile and a pane in none do not count.
    let seed = || {
        Rig::new(
            [member("demo-c1r1", "Other"), pane("demo-c2r1")],
            [
                profile("Alpha", vec![matching_spec("demo-c9r9")]),
                profile("Other", vec![]),
            ],
        )
    };
    for args in [
        &["profile", "delete", "Alpha"][..],
        // B6: --keep-panes with no members is a plain delete.
        &["profile", "delete", "Alpha", "--keep-panes"],
    ] {
        let ran = run_both_with(seed, args, Rig::run);
        let both = &ran.both;
        assert_eq!(both.text.code, 0, "{args:?}: {:?}", both.text);
        assert!(both.text.err.is_empty(), "{:?}", both.text);
        assert_eq!(both.text.out, "deleted profile Alpha (alpha)\n", "{args:?}");
        assert_eq!(
            both.envelope.data,
            json!({"name": "Alpha", "slug": "alpha", "detached": []})
        );
        assert!(
            both.json
                .out
                .contains(r#"{"name":"Alpha","slug":"alpha","detached":[]}"#),
            "a derived struct's key order: {}",
            both.json.out
        );
        for rig in ran.rigs() {
            assert_eq!(stored(rig, "Alpha"), None, "{args:?}: deleted");
            assert!(stored(rig, "Other").is_some());
            assert!(!has_call(&rig.panes.faults().calls(), PaneStoreOp::CasPut));
        }
    }
}

#[test]
fn delete_refuses_while_panes_are_live() {
    let ran = run_both_with(two_members, &["profile", "delete", NAME], Rig::run);
    assert_failure(&ran.both, "profile-has-live-panes", 3);
    for part in [
        r#""Some Profile""#,
        "2 live panes",
        "demo-c1r1, demo-c2r1",
        "holler profile delete 'Some Profile' --keep-panes",
    ] {
        assert_message_contains(&ran.both, part);
    }
    for rig in ran.rigs() {
        assert!(!has_call(&rig.panes.faults().calls(), PaneStoreOp::CasPut));
        assert!(!has_call(
            &rig.profiles.faults().calls(),
            ProfileStoreOp::Delete
        ));
        assert_eq!(stored(rig, NAME).unwrap().generation, 1);
    }
}

#[test]
fn delete_keep_panes_detaches_then_deletes() {
    let before = two_members().panes.list().unwrap();
    let ran = run_both_with(
        two_members,
        &["profile", "delete", NAME, "--keep-panes"],
        Rig::run,
    );
    let both = &ran.both;
    assert_eq!(both.text.code, 0, "{:?}", both.text);
    assert_eq!(
        both.text.out.lines().collect::<Vec<_>>(),
        vec![
            "deleted profile Some Profile (some-profile)",
            "detached, still running: demo-c1r1, demo-c2r1",
        ]
    );
    assert_eq!(
        both.envelope.data,
        json!({"name": "Some Profile", "slug": "some-profile", "detached": ["demo-c1r1", "demo-c2r1"]})
    );
    for rig in ran.rigs() {
        assert_eq!(stored(rig, NAME), None, "deleted");
        let after = rig.panes.list().unwrap();
        assert_eq!(after.len(), before.len(), "every pane record is kept");
        for (was, now) in before.iter().zip(&after) {
            let expected = if was.profile.is_some() {
                Pane {
                    profile: None,
                    generation: was.generation + 1,
                    ..was.clone()
                }
            } else {
                was.clone()
            };
            assert_eq!(now, &expected, "only profile and generation change");
        }
    }
}

#[test]
fn delete_of_a_missing_profile_is_profile_not_found() {
    let seed = || Rig::new([pane("demo-c1r1")], [profile("Other", vec![])]);
    let both = run_both(seed, &["profile", "delete", NAME]);
    assert_failure(&both, "profile-not-found", 3);
    assert_message_contains(&both, r#""Some Profile""#);
}

#[test]
fn delete_with_a_bad_name_is_usage_in_both_formats() {
    let both = run_both(two_members, &["profile", "delete", "   "]);
    assert_failure(&both, "usage", 2);
}

#[test]
fn delete_conflict_without_members_is_generation_conflict() {
    let seed = || {
        let rig = Rig::new([], [profile("Alpha", vec![])]);
        rig.profiles
            .faults()
            .fail_next(ProfileStoreOp::Delete, PaneError::Conflict);
        rig
    };
    for args in [
        &["profile", "delete", "Alpha"][..],
        &["profile", "delete", "Alpha", "--keep-panes"],
    ] {
        let ran = run_both_with(seed, args, Rig::run);
        assert_failure(&ran.both, "generation-conflict", 1);
        for rig in ran.rigs() {
            assert!(stored(rig, "Alpha").is_some(), "{args:?}");
        }
    }
}

#[test]
fn delete_conflict_after_a_detach_is_profile_conflict() {
    let seed = || {
        let rig = Rig::new(
            [member("demo-c1r1", NAME)],
            [profile(NAME, vec![matching_spec("demo-c1r1")])],
        );
        rig.profiles
            .faults()
            .fail_next(ProfileStoreOp::Delete, PaneError::Conflict);
        rig
    };
    let ran = run_both_with(seed, &["profile", "delete", NAME, "--keep-panes"], Rig::run);
    assert_failure(&ran.both, "profile-conflict", 1);
    assert_message_contains(&ran.both, "holler profile show 'Some Profile'");
    assert_message_contains(&ran.both, "demo-c1r1");
    for rig in ran.rigs() {
        assert_eq!(
            memberships(rig),
            vec![("demo-c1r1".to_owned(), None)],
            "the detach landed and is not undone"
        );
        assert!(stored(rig, NAME).is_some(), "the profile still exists");
    }
}

#[test]
fn delete_stops_at_a_failed_detach() {
    let seed = || {
        let rig = two_members();
        rig.panes
            .faults()
            .fail_next(PaneStoreOp::CasPut, PaneError::Conflict);
        rig
    };
    let ran = run_both_with(seed, &["profile", "delete", NAME, "--keep-panes"], Rig::run);
    assert_failure(&ran.both, "generation-conflict", 1);
    assert_message_contains(&ran.both, "detached so far: none");
    assert_message_contains(&ran.both, "the profile was not deleted");
    for rig in ran.rigs() {
        assert!(stored(rig, NAME).is_some(), "the profile still exists");
        assert_eq!(
            memberships(rig),
            memberships(&two_members()),
            "both still members"
        );
        assert!(!has_call(
            &rig.profiles.faults().calls(),
            ProfileStoreOp::Delete
        ));
    }
}

#[test]
fn delete_stops_at_a_failed_second_detach_naming_the_first() {
    // The first detach lands, the second fails: no re-attach (Decision 8), and the
    // message names what was detached.
    let ran = run_both_seamed(
        two_members,
        &[(2, NthPut::Fail(PaneError::Conflict))],
        &["profile", "delete", NAME, "--keep-panes"],
    );
    assert_failure(&ran.both, "generation-conflict", 1);
    assert_message_contains(&ran.both, "detached so far: demo-c1r1;");
    for rig in ran.rigs() {
        assert!(stored(rig, NAME).is_some(), "the profile still exists");
        assert_eq!(
            memberships(rig),
            vec![
                ("demo-c1r1".to_owned(), None),
                ("demo-c2r1".to_owned(), Some(NAME.to_owned())),
                ("demo-c3r1".to_owned(), None),
            ]
        );
        assert!(!has_call(
            &rig.profiles.faults().calls(),
            ProfileStoreOp::Delete
        ));
    }
}

#[test]
fn delete_counts_members_by_slug() {
    let seed = || {
        Rig::new(
            [member("demo-c1r1", "SOME-PROFILE")],
            [profile(NAME, vec![])],
        )
    };
    let both = run_both(seed, &["profile", "delete", NAME]);
    assert_failure(&both, "profile-has-live-panes", 3);
    assert_message_contains(&both, "1 live pane (demo-c1r1)");
}

#[test]
fn delete_quotes_a_name_with_a_quote_in_its_suggested_command() {
    // B1: the suggested command pastes into a shell as one word.
    let seed = || {
        Rig::new(
            [member("demo-c1r1", "Bob's Panes")],
            [profile("Bob's Panes", vec![])],
        )
    };
    let both = run_both(seed, &["profile", "delete", "Bob's Panes"]);
    assert_failure(&both, "profile-has-live-panes", 3);
    assert_message_contains(
        &both,
        r"holler profile delete 'Bob'\''s Panes' --keep-panes",
    );
}
