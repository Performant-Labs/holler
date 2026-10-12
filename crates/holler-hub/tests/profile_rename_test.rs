#![allow(clippy::unwrap_used, clippy::expect_used)] // #665
#![allow(clippy::panic, clippy::unreachable)] // #665
//! Issue #665: `profile/rename`, the one hub-side transaction that renames a profile and
//! moves every member pane to the new name (ADR-0021 section 8, "Rename as built").
//!
//! The request goes through `profile::dispatch` with both registries, as the control socket
//! sends it, and the reply line is parsed back the way a client does (a `PaneReply` in a
//! JSON-RPC result), so every code here is the code on the wire. The registries are real, on
//! a throwaway state dir; the setup writes through the ports. This file pins the transaction's
//! outcome (the record, the members, the logs, the feeds, the refusals, the membership hook);
//! `profile_rename_faults_test.rs` pins its failures, restart, retry and concurrency.

mod pane_support;

use std::sync::Arc;

use holler_hub::panes::PaneState;
use holler_hub::profile::ProfileState;
use holler_hub::state::HubState;
use holler_pane::{
    Actor, Pane, PaneError, PaneStore, Profile, ProfileChange, ProfileLogEntry, ProfileName,
    ProfileStore,
};
use holler_proto::clock::now_millis;
use pane_support::{
    drain, head, load, load_profiles, outcome, profile_head, profiles_file, registry_file,
    rename_params, request, sample_pane, temp_state,
};
use serde_json::{json, Value};

/// The actor of every rename here; the setup writes as `pane_support::actor()` (`t661`).
const RENAMER: &str = "t665-operator";

/// The two registries on a throwaway state dir.
struct Rig {
    panes: Arc<PaneState>,
    profiles: Arc<ProfileState>,
    state: HubState,
    _dir: tempfile::TempDir,
}

impl Rig {
    fn new() -> Self {
        let (dir, state) = temp_state();
        Self {
            panes: Arc::new(load(&state)),
            profiles: Arc::new(load_profiles(&state)),
            state,
            _dir: dir,
        }
    }

    async fn call(&self, method: &str, params: Option<Value>) -> Result<Option<Value>, PaneError> {
        outcome(
            &request(&self.panes, &self.profiles, method, params).await,
            method,
        )
    }

    /// `profile/rename` from `from` to `to` at `expected`, by [`RENAMER`].
    async fn rename(&self, from: &str, to: &str, expected: u64) -> Result<Profile, PaneError> {
        self.call(
            "profile/rename",
            Some(rename_params(from, to, expected, RENAMER)),
        )
        .await
        .map(|data| serde_json::from_value(data.expect("a rename answers data")).unwrap())
    }

    /// Create the profile `name` with one sample spec per entry of `specs`, at generation 1.
    fn profile(&self, name: &str, specs: &[&str]) -> Profile {
        self.profiles
            .cas_put(
                &pane_support::profile(name, specs),
                0,
                &pane_support::actor(),
            )
            .unwrap()
    }

    /// Create the sample pane `name`, naming `profile` when given, at generation 1.
    fn pane(&self, name: &str, profile: Option<&str>) -> Pane {
        self.panes.cas_put(&sample_pane(name, profile), 0).unwrap()
    }

    fn pane_record(&self, name: &str) -> Pane {
        self.panes
            .get(&pane_support::name(name))
            .unwrap()
            .expect("the pane exists")
    }

    fn stored(&self, name: &str) -> Option<Profile> {
        self.profiles.get(&pname(name)).unwrap()
    }

    fn log(&self, name: &str) -> Vec<ProfileLogEntry> {
        self.profiles.log(&pname(name)).unwrap()
    }
}

fn pname(text: &str) -> ProfileName {
    ProfileName::parse(text).unwrap()
}

fn renamer() -> Actor {
    Actor::parse(RENAMER).unwrap()
}

/// A pane record as JSON without the two fields a rename changes on a member.
fn untouched_part(pane: &Pane) -> Value {
    let mut value = serde_json::to_value(pane).unwrap();
    let object = value.as_object_mut().unwrap();
    object.remove("profile");
    object.remove("generation");
    value
}

fn profile_of(pane: &Pane) -> Option<&str> {
    pane.profile.as_ref().map(ProfileName::as_str)
}

/// `at` never decreases in `log`.
fn assert_non_decreasing(what: &str, log: &[ProfileLogEntry]) {
    let at: Vec<i64> = log.iter().map(|entry| entry.at).collect();
    assert!(
        at.windows(2).all(|pair| pair[0] <= pair[1]),
        "{what}: `at` decreases: {at:?}"
    );
}

// --- 1. The record --------------------------------------------------------------------

#[tokio::test]
async fn the_record_moves_to_the_new_name_and_the_reply_is_the_stored_record() {
    let rig = Rig::new();
    let old = rig.profile("Night Shift", &["hj-c1r1", "hj-c1r2"]);
    // The hub stamps by the wall clock in milliseconds: wait until it has passed the
    // create's stamp, so a rename that kept the old `updated` cannot equal its own stamp.
    while now_millis() <= old.updated {
        std::thread::yield_now();
    }

    let renamed = rig.rename("Night Shift", "Day Shift", 1).await.unwrap();

    let stored = rig.stored("Day Shift").expect("found by the new name");
    assert_eq!(renamed, stored, "the reply is the stored record");
    assert_eq!(rig.stored("DAY-SHIFT"), Some(stored.clone()), "by the slug");
    assert_eq!(rig.stored("Night Shift"), None, "gone under the old name");
    assert_eq!(stored.name.as_str(), "Day Shift");
    assert_eq!(stored.slug, "day-shift");
    assert_eq!(stored.generation, 2, "the generation + 1");
    assert_eq!(stored.created, old.created, "the old `created`");
    assert!(stored.updated >= old.updated, "stamped: {stored:?}");
    assert_eq!(
        Some(stored.updated),
        rig.log("Day Shift").first().map(|entry| entry.at),
        "`updated` is the rename's one stamp, the `at` of the new name's one entry"
    );
    assert_eq!(stored.panes, old.panes, "the same specs");
    assert_eq!(rig.profiles.list().unwrap(), vec![stored]);
}

// --- 2 and 3. Every member follows, by slug, and stays attached -----------------------

#[tokio::test]
async fn every_member_follows_and_nothing_else_of_a_pane_changes() {
    let rig = Rig::new();
    rig.profile("Night Shift", &["hj-c1r1"]);
    rig.profile("Other", &[]);
    let members = ["hj-c1r1", "hj-c1r2", "hj-c1r3"];
    for name in members {
        rig.pane(name, Some("Night Shift"));
    }
    rig.pane("hj-c2r1", Some("Other"));
    rig.pane("hj-c3r1", None);
    let before = rig.panes.list().unwrap();

    rig.rename("Night Shift", "Day Shift", 1).await.unwrap();

    for was in &before {
        let now = rig.pane_record(was.name.as_str());
        if members.contains(&was.name.as_str()) {
            assert_eq!(profile_of(&now), Some("Day Shift"), "{now:?}");
            assert_eq!(now.generation, was.generation + 1, "{now:?}");
            assert_eq!(
                untouched_part(&now),
                untouched_part(was),
                "a member is not detached, relaunched or otherwise touched"
            );
        } else {
            assert_eq!(&now, was, "a pane of no or another profile is untouched");
        }
    }
}

#[tokio::test]
async fn membership_is_by_slug() {
    let rig = Rig::new();
    rig.profile("Night Shift", &[]);
    rig.profile("Night Shifts", &[]);
    rig.pane("hj-c1r1", Some("NIGHT-SHIFT"));
    rig.pane("hj-c1r2", Some("Night Shift"));
    let other = rig.pane("hj-c1r3", Some("Night Shifts"));

    rig.rename("Night Shift", "Day Shift", 1).await.unwrap();

    for name in ["hj-c1r1", "hj-c1r2"] {
        let now = rig.pane_record(name);
        assert_eq!(profile_of(&now), Some("Day Shift"), "{name}: {now:?}");
        assert_eq!(now.generation, 2, "{name}");
    }
    assert_eq!(
        rig.pane_record("hj-c1r3"),
        other,
        "another slug is not a member"
    );
}

// --- 4. The logs ----------------------------------------------------------------------

#[tokio::test]
async fn the_old_log_is_closed_and_the_new_log_has_exactly_one_renamed_entry() {
    let rig = Rig::new();
    let p = rig.profile("Night Shift", &["hj-c1r1"]);
    let mut next = p.clone();
    next.panes[0].context.soft += 1;
    rig.profiles
        .cas_put(&next, 1, &pane_support::actor())
        .unwrap();
    let old_before = rig.log("Night Shift");

    // The request spells the old name differently: `from` is the stored display name.
    rig.rename("NIGHT SHIFT", "Day Shift", 2).await.unwrap();

    let old_after = rig.log("Night Shift");
    assert_eq!(old_after.len(), old_before.len() + 1, "{old_after:?}");
    assert_eq!(
        old_after[..old_before.len()],
        old_before[..],
        "the existing entries are untouched"
    );
    let closed = old_after.last().unwrap();
    assert_eq!(
        (closed.generation, &closed.actor, &closed.change),
        (3, &renamer(), &ProfileChange::Deleted)
    );
    let new_log = rig.log("Day Shift");
    assert_eq!(
        new_log.len(),
        1,
        "exactly one entry, nothing copied: {new_log:?}"
    );
    let opened = &new_log[0];
    assert_eq!(
        (opened.generation, &opened.actor, &opened.change),
        (
            3,
            &renamer(),
            &ProfileChange::Renamed {
                from: pname("Night Shift")
            }
        )
    );
    assert_eq!(opened.at, closed.at, "one stamp for both entries");
    assert_non_decreasing("the old log", &old_after);
}

#[tokio::test]
async fn a_deleted_profiles_log_under_the_new_slug_goes_on() {
    let rig = Rig::new();
    rig.profile("Day Shift", &[]);
    rig.profiles
        .delete(&pname("Day Shift"), 1, &pane_support::actor())
        .unwrap();
    let older = rig.log("Day Shift");
    rig.profile("Night Shift", &[]);

    rig.rename("Night Shift", "Day Shift", 1).await.unwrap();

    let log = rig.log("Day Shift");
    assert_eq!(log.len(), older.len() + 1, "{log:?}");
    assert_eq!(log[..older.len()], older[..], "the older log goes on");
    let last = log.last().unwrap();
    assert_eq!(
        (last.generation, &last.change),
        (
            2,
            &ProfileChange::Renamed {
                from: pname("Night Shift")
            }
        )
    );
    assert_non_decreasing("the reused name's log", &log);
}

// --- 5. The feeds ---------------------------------------------------------------------

#[tokio::test]
async fn the_feeds_carry_the_deletion_then_the_put_and_one_put_per_member() {
    let rig = Rig::new();
    rig.profile("Night Shift", &["hj-c1r1"]);
    rig.profile("Other", &[]);
    rig.pane("hj-c1r2", Some("Night Shift"));
    rig.pane("hj-c1r1", Some("Night Shift"));
    rig.pane("hj-c2r1", Some("Other"));
    rig.pane("hj-c3r1", None);
    let (profile_from, pane_from) = (profile_head(&rig.profiles), head(&rig.panes));

    let renamed = rig.rename("Night Shift", "Day Shift", 1).await.unwrap();

    let events = drain(&mut rig.profiles.watch(profile_from).unwrap());
    assert_eq!(events.len(), 2, "{events:?}");
    assert_eq!(events[0].name.slug(), "night-shift");
    assert_eq!(
        events[0].profile, None,
        "the deletion of the old name first"
    );
    assert_eq!(events[1].profile.as_deref(), Some(&renamed), "then the put");
    assert_eq!(events[1].cursor.0, events[0].cursor.0 + 1, "consecutive");

    let events = drain(&mut rig.panes.watch(pane_from).unwrap());
    let names: Vec<&str> = events.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(
        names,
        ["hj-c1r1", "hj-c1r2"],
        "one put per member, in name order"
    );
    for event in &events {
        let record = event.pane.as_deref().expect("a put");
        assert_eq!(record, &rig.pane_record(event.name.as_str()));
    }
    assert_eq!(events[1].cursor.0, events[0].cursor.0 + 1, "consecutive");
}

#[tokio::test]
async fn a_rename_of_one_slug_is_one_put_one_entry_and_skips_a_member_naming_it_already() {
    let rig = Rig::new();
    rig.profile("Night Shift", &[]);
    rig.pane("hj-c1r1", Some("Night Shift"));
    let already = rig.pane("hj-c1r2", Some("NIGHT SHIFT"));
    let (profile_from, pane_from) = (profile_head(&rig.profiles), head(&rig.panes));

    let renamed = rig.rename("Night Shift", "NIGHT SHIFT", 1).await.unwrap();

    assert_eq!(
        (renamed.name.as_str(), renamed.slug.as_str()),
        ("NIGHT SHIFT", "night-shift")
    );
    let events = drain(&mut rig.profiles.watch(profile_from).unwrap());
    assert_eq!(events.len(), 1, "one put: {events:?}");
    assert_eq!(events[0].profile.as_deref(), Some(&renamed));
    let log = rig.log("Night Shift");
    assert_eq!(log.len(), 2, "created, then one renamed entry: {log:?}");
    assert_eq!(
        log[1].change,
        ProfileChange::Renamed {
            from: pname("Night Shift")
        }
    );
    let events = drain(&mut rig.panes.watch(pane_from).unwrap());
    let names: Vec<&str> = events.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(
        names,
        ["hj-c1r1"],
        "the member naming NEW already yields none"
    );
    assert_eq!(rig.pane_record("hj-c1r2"), already, "and is not written");
    assert_eq!(profile_of(&rig.pane_record("hj-c1r1")), Some("NIGHT SHIFT"));
}

// --- 6. Refusals change nothing -------------------------------------------------------

/// Everything a refused rename must leave as it was: both files, both registries' memory,
/// both logs and both feeds' heads.
#[derive(Debug, PartialEq)]
struct Everything {
    pane_file: Vec<u8>,
    profile_file: Vec<u8>,
    panes: Vec<Pane>,
    profiles: Vec<Profile>,
    logs: Vec<Vec<ProfileLogEntry>>,
    heads: (u64, u64),
}

fn everything(rig: &Rig) -> Everything {
    Everything {
        pane_file: std::fs::read(registry_file(&rig.state)).unwrap(),
        profile_file: std::fs::read(profiles_file(&rig.state)).unwrap(),
        panes: rig.panes.list().unwrap(),
        profiles: rig.profiles.list().unwrap(),
        logs: vec![rig.log("Night Shift"), rig.log("Day Shift")],
        heads: (profile_head(&rig.profiles).0, head(&rig.panes).0),
    }
}

#[tokio::test]
async fn a_refused_rename_changes_nothing_and_answers_its_code_in_order() {
    let rig = Rig::new();
    let p = rig.profile("Night Shift", &["hj-c1r1"]);
    rig.profiles.cas_put(&p, 1, &pane_support::actor()).unwrap();
    rig.profile("Day Shift", &[]);
    rig.pane("hj-c1r1", Some("Night Shift"));
    let before = everything(&rig);
    // Night Shift is at 2, Day Shift at 1.
    let refusals: [(&str, &str, u64, &str); 13] = [
        ("No Such", "Fresh", 0, "profile-not-found"),
        ("No Such", "Fresh", 9, "profile-not-found"),
        ("Night Shift", "Night Shift", 2, "usage"),
        ("NIGHT-SHIFT", "Night Shift", 2, "usage"),
        ("Night Shift", "Night Shift", 1, "usage"),
        ("Night Shift", "Day Shift", 2, "profile-exists"),
        ("Night Shift", "DAY-SHIFT", 2, "profile-exists"),
        ("Night Shift", "day shift", 2, "profile-exists"),
        ("Night Shift", "Day Shift", 1, "profile-exists"),
        ("Night Shift", "Day Shift", 7, "profile-exists"),
        ("Night Shift", "Fresh", 1, "generation-conflict"),
        ("Night Shift", "Fresh", 3, "generation-conflict"),
        ("Night Shift", "Fresh", 0, "generation-conflict"),
    ];
    for (from, to, expected, code) in refusals {
        let err = rig.rename(from, to, expected).await.unwrap_err();
        assert_eq!(
            err.code(),
            code,
            "rename({from:?}, {to:?}, {expected}): {err}"
        );
        assert_eq!(
            everything(&rig),
            before,
            "rename({from:?}, {to:?}, {expected}) changed something"
        );
        assert_eq!(rig.stored("Fresh"), None);
    }
}

// --- 12. The membership hook after a rename -------------------------------------------

#[tokio::test]
async fn after_a_rename_a_pane_write_naming_the_old_name_is_refused_and_the_new_accepted() {
    let rig = Rig::new();
    rig.profile("Night Shift", &[]);
    rig.rename("Night Shift", "Day Shift", 1).await.unwrap();
    let put = |pane: Pane| json!({"pane": pane, "expected_generation": 0});

    let old = rig
        .call(
            "pane/cas_put",
            Some(put(sample_pane("hj-c1r1", Some("Night Shift")))),
        )
        .await
        .unwrap_err();
    assert_eq!(old.code(), "profile-not-found", "{old}");
    let new = rig
        .call(
            "pane/cas_put",
            Some(put(sample_pane("hj-c1r1", Some("Day Shift")))),
        )
        .await;
    assert!(new.is_ok(), "{new:?}");
}

// --- 13. The real handler answers, not the stub ---------------------------------------

#[tokio::test]
async fn profile_rename_without_params_is_a_usage_error_from_the_real_handler() {
    let rig = Rig::new();
    for params in [None, Some(json!({})), Some(json!({"from": "Night Shift"}))] {
        let err = rig
            .call("profile/rename", params.clone())
            .await
            .unwrap_err();
        assert!(
            matches!(err, PaneError::Usage { .. }),
            "{params:?}: {err:?}"
        );
    }
}
