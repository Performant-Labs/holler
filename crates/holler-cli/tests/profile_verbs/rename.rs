//! `holler profile rename OLD NEW` (story #665, Scope A): the verb reads OLD, refuses a NEW
//! equal to OLD's stored name, and sends one `ProfileStore::rename` with the generation it
//! read and its own actor. Moving the member panes is the hub's half of the transaction
//! (`holler-hub/tests/profile_rename_test.rs`): the verb writes no pane.

use clap::error::ErrorKind;
use holler_cli::output::Format;
use holler_pane::{
    Actor, Cursor, PaneError, Ports, Profile, ProfileChange, ProfileEvent, ProfileLogEntry,
    ProfileName, ProfileStore, Watch,
};
use holler_pane_testkit::pane_store::PaneStoreOp;
use holler_pane_testkit::profile_store::{FakeProfileStore, ProfileStoreOp};
use serde_json::json;

use crate::list::rig::{
    assert_failure, assert_message_contains, matching_spec, member, pane, profile, run_both,
    run_both_with, Rig,
};
use crate::verb_harness::parse::try_parse;
use crate::verb_harness::{run_verb_with, Outcome};

/// The actor every rename by the verb is logged under.
const ACTOR: &str = "holler profile rename";

fn name(text: &str) -> ProfileName {
    ProfileName::parse(text).unwrap()
}

fn stored(rig: &Rig, text: &str) -> Option<Profile> {
    rig.profiles.get(&name(text)).unwrap()
}

/// The calls made through `rig`'s profile store.
fn profile_calls(rig: &Rig) -> Vec<ProfileStoreOp> {
    rig.profiles.faults().calls()
}

/// `Demo` with one member and a pane in no profile, moved to generation 2 by another writer,
/// so a verb that sent any generation but the one it read would be refused.
fn demo_at_two() -> Rig {
    let rig = Rig::new(
        [member("demo-c1r1", "Demo"), pane("demo-c2r1")],
        [profile("Demo", vec![matching_spec("demo-c1r1")])],
    );
    let current = stored(&rig, "Demo").unwrap();
    rig.profiles
        .concurrent_put(&current, &Actor::parse("another writer").unwrap())
        .unwrap();
    rig
}

#[test]
fn rename_renames_the_profile_in_both_formats() {
    let ran = run_both_with(
        demo_at_two,
        &["profile", "rename", "Demo", "Renamed"],
        Rig::run,
    );
    let both = &ran.both;
    assert_eq!(both.text.code, 0, "{:?}", both.text);
    assert!(both.text.err.is_empty(), "{:?}", both.text);
    assert_eq!(
        both.text.out,
        "renamed profile Demo (demo) to Renamed (renamed), generation 3\n"
    );
    let data = &both.envelope.data;
    assert_eq!(data["from"], json!({"name": "Demo", "slug": "demo"}));
    assert_eq!(
        data["profile"],
        serde_json::to_value(stored(&ran.json_rig, "Renamed").unwrap()).unwrap(),
        "data.profile is the renamed record as stored"
    );
    for rig in ran.rigs() {
        let renamed = stored(rig, "Renamed").expect("stored under the new name");
        assert_eq!(renamed.generation, 3, "the generation read (2), + 1");
        assert_eq!(renamed.panes, vec![matching_spec("demo-c1r1")]);
        assert_eq!(stored(rig, "Demo"), None, "gone under the old name");
        let log: Vec<ProfileLogEntry> = rig.profiles.log(&name("Renamed")).unwrap();
        let last = log.last().expect("a renamed entry");
        assert_eq!(last.actor, Actor::parse(ACTOR).unwrap(), "the verb's actor");
        assert_eq!(last.change, ProfileChange::Renamed { from: name("Demo") });
        let renames = profile_calls(rig)
            .into_iter()
            .filter(|op| *op == ProfileStoreOp::Rename)
            .count();
        assert_eq!(renames, 1, "one rename: {:?}", profile_calls(rig));
        assert!(
            !rig.panes.faults().calls().contains(&PaneStoreOp::CasPut),
            "the verb writes no pane: the hub moves the members"
        );
    }
}

#[test]
fn rename_takes_names_with_spaces() {
    let seed = || Rig::new([], [profile("Some Profile", vec![])]);
    let ran = run_both_with(
        seed,
        &["profile", "rename", "Some Profile", "Other Name"],
        Rig::run,
    );
    assert_eq!(
        ran.both.text.out,
        "renamed profile Some Profile (some-profile) to Other Name (other-name), generation 2\n"
    );
    assert_eq!(
        ran.both.envelope.data["from"],
        json!({"name": "Some Profile", "slug": "some-profile"})
    );
    for rig in ran.rigs() {
        assert!(stored(rig, "Other Name").is_some());
    }
}

#[test]
fn rename_of_only_the_spelling_is_accepted() {
    let seed = || Rig::new([], [profile("Demo", vec![])]);
    let ran = run_both_with(seed, &["profile", "rename", "Demo", "DEMO"], Rig::run);
    assert_eq!(
        ran.both.text.out,
        "renamed profile Demo (demo) to DEMO (demo), generation 2\n"
    );
    for rig in ran.rigs() {
        assert_eq!(stored(rig, "demo").unwrap().name.as_str(), "DEMO");
    }
}

#[test]
fn rename_of_a_missing_profile_is_profile_not_found_before_any_rename() {
    let seed = || Rig::new([], [profile("Other", vec![])]);
    let ran = run_both_with(seed, &["profile", "rename", "Demo", "Renamed"], Rig::run);
    assert_failure(&ran.both, "profile-not-found", 3);
    assert_message_contains(&ran.both, r#""Demo""#);
    for rig in ran.rigs() {
        assert!(!profile_calls(rig).contains(&ProfileStoreOp::Rename));
    }
}

#[test]
fn rename_onto_a_taken_name_is_profile_exists() {
    let seed = || Rig::new([], [profile("Demo", vec![]), profile("Taken", vec![])]);
    for to in ["Taken", "TAKEN"] {
        let ran = run_both_with(seed, &["profile", "rename", "Demo", to], Rig::run);
        assert_failure(&ran.both, "profile-exists", 3);
        for rig in ran.rigs() {
            assert_eq!(
                stored(rig, "Demo").unwrap().generation,
                1,
                "{to}: unchanged"
            );
            assert_eq!(stored(rig, "Taken").unwrap().name.as_str(), "Taken");
        }
    }
}

#[test]
fn rename_to_the_stored_name_is_usage_and_sends_no_rename() {
    let seed = || Rig::new([], [profile("Demo", vec![])]);
    for args in [
        &["profile", "rename", "Demo", "Demo"][..],
        &["profile", "rename", "demo", "Demo"],
    ] {
        let ran = run_both_with(seed, args, Rig::run);
        assert_failure(&ran.both, "usage", 2);
        for rig in ran.rigs() {
            assert!(
                !profile_calls(rig).contains(&ProfileStoreOp::Rename),
                "{args:?}: refused before the store is asked"
            );
            assert_eq!(stored(rig, "Demo").unwrap().generation, 1);
        }
    }
}

#[test]
fn rename_with_a_bad_name_is_usage() {
    let seed = || Rig::new([], [profile("Demo", vec![])]);
    for args in [
        &["profile", "rename", "Demo", "   "][..],
        &["profile", "rename", "   ", "Renamed"],
    ] {
        let both = run_both(seed, args);
        assert_failure(&both, "usage", 2);
    }
}

#[test]
fn rename_takes_exactly_two_positionals() {
    let error = try_parse(&["profile", "rename", "Demo"]).expect_err("NEW is required");
    assert_eq!(error.kind(), ErrorKind::MissingRequiredArgument);
    let error =
        try_parse(&["profile", "rename", "A", "B", "C"]).expect_err("three is one too many");
    assert_eq!(error.kind(), ErrorKind::UnknownArgument);
    assert!(try_parse(&["profile", "rename", "A", "B"]).is_ok());
}

/// A profile store that lets another writer move the profile between the verb's read and its
/// rename: the race `generation-conflict` reports.
struct MovedBeforeRename<'a> {
    inner: &'a FakeProfileStore,
}

impl ProfileStore for MovedBeforeRename<'_> {
    fn get(&self, name: &ProfileName) -> Result<Option<Profile>, PaneError> {
        self.inner.get(name)
    }

    fn list(&self) -> Result<Vec<Profile>, PaneError> {
        self.inner.list()
    }

    fn cas_put(&self, p: &Profile, expected: u64, actor: &Actor) -> Result<Profile, PaneError> {
        self.inner.cas_put(p, expected, actor)
    }

    fn delete(&self, name: &ProfileName, expected: u64, actor: &Actor) -> Result<(), PaneError> {
        self.inner.delete(name, expected, actor)
    }

    fn watch(&self, since: Cursor) -> Result<Watch<ProfileEvent>, PaneError> {
        self.inner.watch(since)
    }

    fn log(&self, name: &ProfileName) -> Result<Vec<ProfileLogEntry>, PaneError> {
        self.inner.log(name)
    }

    fn rename(
        &self,
        from: &ProfileName,
        to: &ProfileName,
        expected: u64,
        actor: &Actor,
    ) -> Result<Profile, PaneError> {
        if let Some(current) = self.inner.get(from)? {
            let other = Actor::parse("another writer").unwrap();
            self.inner.concurrent_put(&current, &other)?;
        }
        self.inner.rename(from, to, expected, actor)
    }
}

#[test]
fn rename_after_another_writer_moved_the_profile_is_generation_conflict() {
    let seed = || Rig::new([], [profile("Demo", vec![])]);
    let run = |rig: &Rig, argv: &[&str], format: Format| -> Outcome {
        let seam = MovedBeforeRename {
            inner: &rig.profiles,
        };
        run_verb_with(
            argv,
            format,
            Ports {
                profile_store: &seam,
                ..rig.ports()
            },
        )
    };
    let ran = run_both_with(seed, &["profile", "rename", "Demo", "Renamed"], run);
    assert_failure(&ran.both, "generation-conflict", 1);
    for rig in ran.rigs() {
        assert_eq!(stored(rig, "Renamed"), None);
        assert_eq!(
            stored(rig, "Demo").unwrap().generation,
            2,
            "the other write only"
        );
    }
}
