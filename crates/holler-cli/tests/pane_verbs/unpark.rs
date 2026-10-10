//! `holler pane unpark` (story #646 part 1): its own cases. The cases it shares with park
//! run in `park.rs`, which covers both verbs (AC 1-7); this file holds unpark's store
//! failures (AC 8d) and its named-member case (AC 5), over park's rig.

use holler_pane::pane::Hold;

use crate::park::failures::assert_failures_name_the_pane_and_stop;
use crate::park::rig::{held, member, old_park, profile, run_both, Rig, Verb};

/// AC 8d: unpark's store failures, the same cases as park's, in unpark's words.
#[test]
fn unpark_failures_name_the_pane_and_stop() {
    assert_failures_name_the_pane_and_stop(Verb::Unpark);
}

/// AC 5 for unpark: a named member with its profile is the one pane unparked.
#[test]
fn unpark_of_a_member_named_with_its_profile_changes_only_that_pane() {
    let parked_members = || {
        Rig::new(
            [
                held(member("demo-c1r1", "Demo Alpha"), old_park()),
                held(member("demo-c2r1", "Demo Alpha"), old_park()),
            ],
            [profile("Demo Alpha", &["demo-c1r1", "demo-c2r1"])],
        )
    };
    let both = run_both(
        parked_members,
        &Verb::Unpark.argv(&["demo-c1r1", "--profile", "Demo Alpha"]),
    );
    assert_eq!(both.text.code, 0, "{:?}", both.text);
    assert_eq!(both.text.out, "demo-c1r1: unparked\n");
    for rig in [&both.text_rig, &both.json_rig] {
        let record = rig.record("demo-c1r1");
        assert_eq!(record.hold, Hold::None);
        assert_eq!(record.generation, 2);
        assert_eq!(rig.cas_puts(), 1, "only demo-c1r1 is written");
        assert_eq!(rig.record("demo-c2r1"), rig.seeded("demo-c2r1"));
    }
}
