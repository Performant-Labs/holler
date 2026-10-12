#![allow(clippy::unwrap_used, clippy::expect_used)] // #665
#![allow(clippy::panic, clippy::unreachable)] // #665
//! Issue #665: `profile/rename` when something goes wrong (ADR-0021 section 8, "Rename as
//! built"): a registry that failed to load, a save that fails, a restart, a retry after a
//! crash between the two saves, and two writers racing. The outcome of a rename that works
//! is pinned by `profile_rename_test.rs`.
//!
//! A save is made to fail by replacing its file with a directory after the registry loaded:
//! the atomic write's final rename onto a directory fails, and the other registry's file is
//! still writable. Every race runs on its own runtime in its own thread, bounded by
//! [`RACE_WITHIN`], so a lock-order bug fails the test instead of hanging it.

mod pane_support;

use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

use holler_hub::panes::PaneState;
use holler_hub::profile::ProfileState;
use holler_hub::state::HubState;
use holler_pane::{
    Pane, PaneError, PaneStore, Profile, ProfileLogEntry, ProfileName, ProfileStore,
};
use pane_support::{
    head, load, load_profiles, outcome, profile_head, profiles_file, registry_file, rename_params,
    request, sample_pane, temp_state,
};
use serde_json::{json, Value};

const RENAMER: &str = "t665-operator";

/// The bound on one whole race (both writers and the checks).
const RACE_WITHIN: Duration = Duration::from_secs(60);

/// How many times each race runs, each on fresh registries, to surface an interleaving.
const RACE_ROUNDS: usize = 20;

/// The members of `Night Shift` in every fixture here.
const MEMBERS: [&str; 3] = ["hj-c1r1", "hj-c1r2", "hj-c1r3"];

struct Rig {
    panes: Arc<PaneState>,
    profiles: Arc<ProfileState>,
    state: HubState,
    dir: tempfile::TempDir,
}

impl Rig {
    fn new() -> Self {
        let (dir, state) = temp_state();
        Self::on(dir, state)
    }

    /// Both registries loaded (again) from the files of `state`.
    fn on(dir: tempfile::TempDir, state: HubState) -> Self {
        Self {
            panes: Arc::new(load(&state)),
            profiles: Arc::new(load_profiles(&state)),
            state,
            dir,
        }
    }

    /// The same state dir, loaded again: a restart.
    fn restart(self) -> Self {
        let Rig { state, dir, .. } = self;
        Self::on(dir, state)
    }

    /// `Night Shift` at generation 1 with three members, `Other` with one, and a pane in
    /// no profile; both files exist.
    fn fleet() -> Self {
        let rig = Self::new();
        rig.profile("Night Shift");
        rig.profile("Other");
        for name in MEMBERS {
            rig.pane(name, Some("Night Shift"));
        }
        rig.pane("hj-c2r1", Some("Other"));
        rig.pane("hj-c3r1", None);
        rig
    }

    async fn rename(
        &self,
        from: &str,
        to: &str,
        expected: u64,
    ) -> Result<Option<Value>, PaneError> {
        rename_on(&self.panes, &self.profiles, from, to, expected).await
    }

    fn profile(&self, name: &str) -> Profile {
        self.profiles
            .cas_put(
                &pane_support::profile(name, &["hj-c1r1"]),
                0,
                &pane_support::actor(),
            )
            .unwrap()
    }

    fn pane(&self, name: &str, profile: Option<&str>) -> Pane {
        self.panes.cas_put(&sample_pane(name, profile), 0).unwrap()
    }

    fn named(&self, name: &str) -> Option<String> {
        self.panes
            .get(&pane_support::name(name))
            .unwrap()
            .expect("the pane exists")
            .profile
            .map(|p| p.to_string())
    }

    /// What a failed rename must leave in memory: both registries, both logs, both heads.
    fn memory(&self) -> (Vec<Pane>, Vec<Profile>, Vec<ProfileLogEntry>, u64, u64) {
        (
            self.panes.list().unwrap(),
            self.profiles.list().unwrap(),
            self.profiles.log(&pname("Night Shift")).unwrap(),
            head(&self.panes).0,
            profile_head(&self.profiles).0,
        )
    }
}

async fn rename_on(
    panes: &Arc<PaneState>,
    profiles: &Arc<ProfileState>,
    from: &str,
    to: &str,
    expected: u64,
) -> Result<Option<Value>, PaneError> {
    let params = Some(rename_params(from, to, expected, RENAMER));
    outcome(
        &request(panes, profiles, "profile/rename", params).await,
        "profile/rename",
    )
}

fn pname(text: &str) -> ProfileName {
    ProfileName::parse(text).unwrap()
}

/// Replace the file at `path` by an empty directory, so the next save of it fails.
fn block_save(path: &Path) {
    std::fs::remove_file(path).unwrap();
    std::fs::create_dir(path).unwrap();
}

fn unblock_save(path: &Path) {
    std::fs::remove_dir(path).unwrap();
}

// --- 7. Fail closed -------------------------------------------------------------------

#[tokio::test]
async fn a_pane_registry_that_failed_to_load_is_store_corrupt_and_nothing_is_written() {
    let (dir, state) = temp_state();
    std::fs::create_dir_all(&state.hub_dir).unwrap();
    std::fs::write(registry_file(&state), b"{ not json").unwrap();
    let rig = Rig::on(dir, state);
    rig.profile("Night Shift");
    let profiles_before = std::fs::read(profiles_file(&rig.state)).unwrap();

    // Before any profile check: a missing old name is still `store-corrupt`.
    for from in ["Night Shift", "No Such"] {
        let err = rig.rename(from, "Day Shift", 1).await.unwrap_err();
        assert_eq!(err.code(), "store-corrupt", "{from}: {err}");
    }
    assert_eq!(
        std::fs::read(profiles_file(&rig.state)).unwrap(),
        profiles_before
    );
    assert_eq!(
        std::fs::read(registry_file(&rig.state)).unwrap(),
        b"{ not json"
    );
    assert!(rig.profiles.get(&pname("Night Shift")).unwrap().is_some());
}

#[tokio::test]
async fn a_profile_registry_that_failed_to_load_is_store_corrupt_and_nothing_is_written() {
    let (dir, state) = temp_state();
    std::fs::create_dir_all(&state.hub_dir).unwrap();
    std::fs::write(profiles_file(&state), b"{ not json").unwrap();
    let rig = Rig::on(dir, state);
    rig.pane("hj-c1r1", None);
    let panes_before = std::fs::read(registry_file(&rig.state)).unwrap();

    let err = rig.rename("Night Shift", "Day Shift", 1).await.unwrap_err();

    assert_eq!(err.code(), "store-corrupt", "{err}");
    assert_eq!(
        std::fs::read(registry_file(&rig.state)).unwrap(),
        panes_before
    );
    assert_eq!(
        std::fs::read(profiles_file(&rig.state)).unwrap(),
        b"{ not json"
    );
}

// --- 8. Failed saves ------------------------------------------------------------------

#[tokio::test]
async fn a_failed_pane_save_is_unavailable_and_changes_nothing() {
    let rig = Rig::fleet();
    let profiles_before = std::fs::read(profiles_file(&rig.state)).unwrap();
    let memory = rig.memory();
    block_save(&registry_file(&rig.state));

    let err = rig.rename("Night Shift", "Day Shift", 1).await.unwrap_err();

    assert_eq!(err.code(), "unavailable", "{err}");
    assert_eq!(
        std::fs::read(profiles_file(&rig.state)).unwrap(),
        profiles_before
    );
    assert!(
        registry_file(&rig.state).is_dir(),
        "nothing replaced the blocked file"
    );
    assert_eq!(rig.memory(), memory, "memory, logs and cursors unchanged");
}

#[tokio::test]
async fn a_failed_profile_save_puts_the_pane_file_back_and_the_same_rename_then_succeeds() {
    let rig = Rig::fleet();
    let panes_file = registry_file(&rig.state);
    let panes_before = std::fs::read(&panes_file).unwrap();
    let inode_before = std::fs::metadata(&panes_file).unwrap().ino();
    let memory = rig.memory();
    block_save(&profiles_file(&rig.state));

    let err = rig.rename("Night Shift", "Day Shift", 1).await.unwrap_err();

    assert_eq!(err.code(), "unavailable", "{err}");
    assert_eq!(
        std::fs::read(&panes_file).unwrap(),
        panes_before,
        "the pane document has its previous bytes again"
    );
    // The panes are saved first (D2): the file was replaced, then saved back, so it is a
    // new file with the old bytes. A profile-first order never touches it here.
    assert_ne!(
        std::fs::metadata(&panes_file).unwrap().ino(),
        inode_before,
        "the pane document is saved before the profile document"
    );
    assert_eq!(
        rig.memory(),
        memory,
        "memory is committed only after both saves"
    );

    unblock_save(&profiles_file(&rig.state));
    rig.rename("Night Shift", "Day Shift", 1)
        .await
        .expect("the same rename at the same generation succeeds");
    for name in MEMBERS {
        assert_eq!(rig.named(name).as_deref(), Some("Day Shift"), "{name}");
    }
}

// --- 9. Restart -----------------------------------------------------------------------

#[tokio::test]
async fn a_rename_survives_a_restart_and_a_resumed_watcher_learns_of_the_deletion() {
    let rig = Rig::fleet();
    let since = profile_head(&rig.profiles);
    rig.rename("Night Shift", "Day Shift", 1).await.unwrap();
    let before = (
        rig.panes.list().unwrap(),
        rig.profiles.list().unwrap(),
        rig.profiles.log(&pname("Night Shift")).unwrap(),
        rig.profiles.log(&pname("Day Shift")).unwrap(),
    );

    let rig = rig.restart();

    let after = (
        rig.panes.list().unwrap(),
        rig.profiles.list().unwrap(),
        rig.profiles.log(&pname("Night Shift")).unwrap(),
        rig.profiles.log(&pname("Day Shift")).unwrap(),
    );
    assert_eq!(
        after, before,
        "the tombstone of the old name passes the load check"
    );
    let events = pane_support::drain(&mut rig.profiles.watch(since).unwrap());
    assert!(
        events
            .iter()
            .any(|e| e.name.slug() == "night-shift" && e.profile.is_none()),
        "the deletion of the old name: {events:?}"
    );
}

// --- 10. A retry completes an interrupted rename --------------------------------------

#[tokio::test]
async fn running_the_same_rename_again_completes_one_interrupted_between_the_saves() {
    // The state a crash between the two saves leaves (panes saved, profiles not): the
    // members name the new name, the old profile is live at its generation, the new name is
    // free. A profile-first order cannot leave it, so it is built by hand, then reloaded.
    let rig = Rig::new();
    rig.profile("Night Shift");
    for name in MEMBERS {
        rig.pane(name, Some("Day Shift"));
    }
    let rig = rig.restart();

    rig.rename("Night Shift", "Day Shift", 1)
        .await
        .expect("the rerun completes the rename");

    assert!(rig.profiles.get(&pname("Night Shift")).unwrap().is_none());
    assert_eq!(
        rig.profiles
            .get(&pname("Day Shift"))
            .unwrap()
            .map(|p| p.generation),
        Some(2)
    );
    for name in MEMBERS {
        assert_eq!(rig.named(name).as_deref(), Some("Day Shift"), "{name}");
    }
}

// --- 11. Concurrency ------------------------------------------------------------------

/// Run `race` on a multi-threaded runtime of its own, in a thread of its own, and fail if it
/// does not finish within [`RACE_WITHIN`] (a deadlock fails; it does not hang the suite).
fn bounded(race: impl FnOnce(&tokio::runtime::Runtime) + Send + 'static) {
    let (done, finished) = mpsc::channel();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .unwrap();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| race(&runtime)));
        runtime.shutdown_background();
        let _ = done.send(result);
    });
    match finished.recv_timeout(RACE_WITHIN) {
        Ok(Ok(())) => {}
        Ok(Err(panic)) => std::panic::resume_unwind(panic),
        Err(_) => panic!("the race did not finish within {RACE_WITHIN:?}: a lock-order bug?"),
    }
}

#[test]
fn two_renames_of_one_profile_racing_exactly_one_wins_and_every_member_names_it() {
    bounded(|runtime| {
        for round in 0..RACE_ROUNDS {
            let rig = Rig::fleet();
            let gate = Arc::new(tokio::sync::Barrier::new(2));
            let outcomes: Vec<(&str, Result<Option<Value>, PaneError>)> = runtime.block_on(async {
                let racers: Vec<_> = ["Day Shift", "Late Shift"]
                    .into_iter()
                    .map(|to| {
                        let (panes, profiles) = (rig.panes.clone(), rig.profiles.clone());
                        let gate = gate.clone();
                        tokio::spawn(async move {
                            gate.wait().await;
                            (to, rename_on(&panes, &profiles, "Night Shift", to, 1).await)
                        })
                    })
                    .collect();
                let mut outcomes = Vec::new();
                for racer in racers {
                    outcomes.push(racer.await.unwrap());
                }
                outcomes
            });
            let winners: Vec<&str> = outcomes
                .iter()
                .filter(|(_, result)| result.is_ok())
                .map(|&(to, _)| to)
                .collect();
            assert_eq!(winners.len(), 1, "round {round}: {outcomes:?}");
            for (_, result) in &outcomes {
                if let Err(err) = result {
                    assert!(
                        ["generation-conflict", "profile-not-found"].contains(&err.code()),
                        "round {round}: the loser's code: {err}"
                    );
                }
            }
            for name in MEMBERS {
                assert_eq!(
                    rig.named(name).as_deref(),
                    Some(winners[0]),
                    "round {round}"
                );
            }
        }
    });
}

#[test]
fn a_member_written_during_a_rename_is_moved_or_refused_never_lost() {
    bounded(|runtime| {
        for round in 0..RACE_ROUNDS {
            let rig = Rig::fleet();
            let mut edited = sample_pane("hj-c1r1", Some("Night Shift"));
            edited.context.soft = 4_242;
            let gate = Arc::new(tokio::sync::Barrier::new(2));
            let (write, rename) = runtime.block_on(async {
                let (panes, profiles, g) = (rig.panes.clone(), rig.profiles.clone(), gate.clone());
                let params = json!({"pane": edited, "expected_generation": 1});
                let writer = tokio::spawn(async move {
                    g.wait().await;
                    let line = request(&panes, &profiles, "pane/cas_put", Some(params)).await;
                    outcome(&line, "pane/cas_put")
                });
                let (panes, profiles, g) = (rig.panes.clone(), rig.profiles.clone(), gate.clone());
                let renamer = tokio::spawn(async move {
                    g.wait().await;
                    rename_on(&panes, &profiles, "Night Shift", "Day Shift", 1).await
                });
                (writer.await.unwrap(), renamer.await.unwrap())
            });
            assert!(
                rename.is_ok(),
                "round {round}: the rename succeeds: {rename:?}"
            );
            let record = rig
                .panes
                .get(&pane_support::name("hj-c1r1"))
                .unwrap()
                .unwrap();
            assert_eq!(
                record.profile.as_ref().map(ProfileName::as_str),
                Some("Day Shift"),
                "round {round}: the member names the new name: {write:?}"
            );
            match write {
                Ok(_) => assert_eq!(
                    record.context.soft, 4_242,
                    "round {round}: a write that landed is kept and moved"
                ),
                Err(err) => assert!(
                    ["generation-conflict", "profile-not-found"].contains(&err.code()),
                    "round {round}: a write after the rename is refused: {err}"
                ),
            }
        }
    });
}
