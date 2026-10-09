#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #637
//! The ports (#637 AC 6): every trait is implemented here by a test double, so a later
//! change to a frozen signature breaks THIS file, not a wave-3 story mid-flight.
//!
//! The test also builds the `Ports` bundle and drives it from a verb-shaped function
//! (`&Ports` + two `&mut dyn Write`, an `edit_spec` whose `act` uses another port):
//! the shape #670's verbs use.

mod common;

use std::io::Write;
use std::time::Duration;

use common::{pane, pane_name, profile, profile_name, spec, MemPaneStore};
use holler_pane::{
    run_probe, Actor, Argv, Cursor, HarnessPort, HerdrPane, HerdrPort, HerdrSnapshot, HerdrSpec,
    HostPort, Key, Pane, PaneError, PaneId, PaneName, PaneReply, PaneStore, Ports, ProbeResult,
    Prober, Profile, ProfileEvent, ProfileLogEntry, ProfileName, ProfileScope, ProfileSpec,
    ProfileStore, ResolvedScope, SpecEdit, SystemProber, Watch,
};
use serde_json::json;

/// A closed-code error built the way a client gets it (shape-agnostic).
fn coded(code: &str, message: &str) -> PaneError {
    let reply: PaneReply = serde_json::from_value(
        json!({"ok": false, "data": null, "error": {"code": code, "message": message}}),
    )
    .unwrap();
    reply.into_result().unwrap_err()
}

fn argv(parts: &[&str]) -> Argv {
    serde_json::from_value(json!(parts)).unwrap()
}

fn herdr_pane_id() -> PaneId {
    serde_json::from_value(json!("p_12")).unwrap()
}

// ---------------------------------------------------------------------------
// The doubles: one per port
// ---------------------------------------------------------------------------

// `PaneStore` is `common::MemPaneStore`.

struct MemProfileStore {
    profile: Profile,
}

impl ProfileStore for MemProfileStore {
    fn get(&self, name: &ProfileName) -> Result<Option<Profile>, PaneError> {
        Ok((*name == self.profile.name).then(|| self.profile.clone()))
    }
    fn list(&self) -> Result<Vec<Profile>, PaneError> {
        Ok(vec![self.profile.clone()])
    }
    fn cas_put(
        &self,
        profile: &Profile,
        expected_generation: u64,
        _actor: &Actor,
    ) -> Result<Profile, PaneError> {
        if expected_generation != self.profile.generation {
            return Err(PaneError::Conflict);
        }
        let mut next = profile.clone();
        next.generation = expected_generation + 1;
        Ok(next)
    }
    fn delete(
        &self,
        _name: &ProfileName,
        expected_generation: u64,
        _actor: &Actor,
    ) -> Result<(), PaneError> {
        if expected_generation != self.profile.generation {
            return Err(PaneError::Conflict);
        }
        Ok(())
    }
    fn watch(&self, _since: Cursor) -> Result<Watch<ProfileEvent>, PaneError> {
        Ok(Box::new(std::iter::empty::<
            Result<Option<ProfileEvent>, PaneError>,
        >()))
    }
    fn log(&self, _name: &ProfileName) -> Result<Vec<ProfileLogEntry>, PaneError> {
        Ok(Vec::new())
    }
    /// `rename` is PROPOSED (#665): the double answers `not-implemented`.
    fn rename(
        &self,
        _from: &ProfileName,
        _to: &ProfileName,
        _expected_generation: u64,
        _actor: &Actor,
    ) -> Result<Profile, PaneError> {
        Err(PaneError::NotImplemented)
    }
}

/// Resolves against one profile and a fixed set of panes.
struct MemScope {
    profile: Profile,
    panes: Vec<Pane>,
}

impl ProfileScope for MemScope {
    fn resolve(
        &self,
        profile: &ProfileName,
        pane: Option<&PaneName>,
    ) -> Result<ResolvedScope, PaneError> {
        if *profile != self.profile.name {
            return Err(coded("profile-not-found", "no such profile"));
        }
        let members: Vec<Pane> = self
            .panes
            .iter()
            .filter(|p| p.profile.as_ref() == Some(profile))
            .cloned()
            .collect();
        let panes = match pane {
            None => members,
            Some(name) => {
                let found: Vec<Pane> = members.into_iter().filter(|p| p.name == *name).collect();
                if found.is_empty() {
                    return Err(coded("pane-not-in-profile", "not a member"));
                }
                found
            }
        };
        Ok(ResolvedScope {
            profile: self.profile.clone(),
            panes,
        })
    }

    fn edit_spec(
        &self,
        profile: Option<&ProfileName>,
        pane: &PaneName,
        edit: &SpecEdit,
        act: &mut dyn FnMut() -> Result<(), PaneError>,
    ) -> Result<Option<Profile>, PaneError> {
        // `None` runs only `act` and touches no profile.
        let Some(_profile) = profile else {
            act()?;
            return Ok(None);
        };
        let mut next = self.profile.clone();
        next.panes.retain(|s| s.pane != pane.as_str());
        if let SpecEdit::Set(spec) = edit {
            next.panes.push(ProfileSpec::clone(spec));
        }
        // I8: the profile write first, then the live act; a failed act records nothing.
        act()?;
        next.generation += 1;
        Ok(Some(next))
    }
}

struct TestHerdr {
    up: bool,
}

impl HerdrPort for TestHerdr {
    fn ensure_pane(&self, _spec: &HerdrSpec) -> Result<HerdrPane, PaneError> {
        Err(PaneError::NotImplemented)
    }
    fn send_text(&self, _pane: &PaneId, _text: &str) -> Result<(), PaneError> {
        Ok(())
    }
    fn send_keys(&self, _pane: &PaneId, _keys: &[Key]) -> Result<(), PaneError> {
        Ok(())
    }
    fn read(&self, _pane: &PaneId, max_lines: usize) -> Result<String, PaneError> {
        Ok("line\n".repeat(max_lines))
    }
    fn close(&self, _pane: &PaneId) -> Result<(), PaneError> {
        Ok(())
    }
    fn snapshot(&self) -> Result<HerdrSnapshot, PaneError> {
        Err(PaneError::NotImplemented)
    }
    fn version(&self) -> Result<String, PaneError> {
        if self.up {
            Ok("0.9.1".to_string())
        } else {
            Err(PaneError::Unavailable {
                what: "herdr socket".to_string(),
            })
        }
    }
}

struct TestHost;

impl HostPort for TestHost {
    fn ensure_session(&self, _name: &PaneName, _cwd: &str) -> Result<(), PaneError> {
        Ok(())
    }
    fn run(&self, _name: &PaneName, _argv: &Argv) -> Result<(), PaneError> {
        Ok(())
    }
    fn stop_owned(&self, _name: &PaneName) -> Result<(), PaneError> {
        Ok(())
    }
    fn ps(&self, _name: &PaneName) -> Result<Vec<u32>, PaneError> {
        Ok(vec![4242])
    }
}

struct TestHarness;

impl HarnessPort for TestHarness {
    fn serve(&self, _name: &PaneName, _port: u16) -> Result<u32, PaneError> {
        Ok(4242)
    }
    fn health(&self, _port: u16) -> Result<bool, PaneError> {
        Ok(true)
    }
    fn create_session(&self, _port: u16) -> Result<String, PaneError> {
        Ok("ses_new".to_string())
    }
    fn list_sessions(&self, _port: u16) -> Result<Vec<String>, PaneError> {
        Ok(vec!["ses_abc".to_string()])
    }
    fn abort(&self, _port: u16, _session: &str) -> Result<(), PaneError> {
        Ok(())
    }
    fn attach_tui(&self, _pane: &PaneId, _port: u16, _session: &str) -> Result<(), PaneError> {
        Ok(())
    }
    fn select_session(&self, _pane: &PaneId, _session: &str) -> Result<(), PaneError> {
        Ok(())
    }
    fn shown_session(&self, _pane: &PaneId) -> Result<Option<String>, PaneError> {
        Ok(Some("ses_abc".to_string()))
    }
}

/// Reports every expected string as missing: a probe that never passes.
struct FailingProber;

impl Prober for FailingProber {
    fn run_probe(&self, _argv: &Argv, expect: &[String], _timeout: Duration) -> ProbeResult {
        if expect.is_empty() {
            ProbeResult::Ok
        } else {
            ProbeResult::Failed {
                missing: expect.to_vec(),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The tests
// ---------------------------------------------------------------------------

#[test]
fn ports_are_send_sync_trait_objects() {
    fn assert_send_sync<T: ?Sized + Send + Sync>() {}
    assert_send_sync::<dyn PaneStore>();
    assert_send_sync::<dyn ProfileStore>();
    assert_send_sync::<dyn ProfileScope>();
    assert_send_sync::<dyn HerdrPort>();
    assert_send_sync::<dyn HostPort>();
    assert_send_sync::<dyn HarnessPort>();
    assert_send_sync::<dyn Prober>();
    assert_send_sync::<Ports<'static>>();
}

#[test]
fn ports_compile_in_test_impl() {
    exercise_stores();
    exercise_herdr();
    exercise_host_and_harness();
}

fn exercise_stores() {
    // PaneStore
    let panes = MemPaneStore::with(vec![pane()]);
    assert_eq!(panes.get(&pane_name("hj-c1r1")).unwrap(), Some(pane()));
    assert_eq!(panes.list().unwrap().len(), 1);
    let mut watch = panes.watch(Cursor(0)).unwrap();
    assert!(watch.next().is_none());

    // ProfileStore (rename is PROPOSED: the double says not-implemented)
    let profiles = MemProfileStore { profile: profile() };
    let actor = Actor::parse("test").unwrap();
    let name = profile_name("Some Profile");
    assert_eq!(profiles.get(&name).unwrap(), Some(profile()));
    assert_eq!(profiles.list().unwrap().len(), 1);
    assert_eq!(
        profiles.cas_put(&profile(), 3, &actor).unwrap().generation,
        4
    );
    assert_eq!(
        profiles.cas_put(&profile(), 2, &actor).unwrap_err().code(),
        "generation-conflict"
    );
    profiles.delete(&name, 3, &actor).unwrap();
    assert!(profiles.watch(Cursor(7)).unwrap().next().is_none());
    assert!(profiles.log(&name).unwrap().is_empty());
    let renamed = profiles.rename(&name, &profile_name("Other"), 3, &actor);
    assert_eq!(renamed.unwrap_err().code(), "not-implemented");
}

fn exercise_herdr() {
    let herdr = TestHerdr { up: true };
    assert_eq!(herdr.version().unwrap(), "0.9.1");
    let id = herdr_pane_id();
    herdr.send_text(&id, "hello").unwrap();
    herdr.send_keys(&id, &[]).unwrap();
    assert_eq!(herdr.read(&id, 2).unwrap(), "line\nline\n");
    herdr.close(&id).unwrap();
    let snapshot = herdr.snapshot();
    assert_eq!(snapshot.unwrap_err().code(), "not-implemented");
}

fn exercise_host_and_harness() {
    let name = pane_name("hj-c1r1");
    let host = TestHost;
    host.ensure_session(&name, "/work/holler").unwrap();
    host.run(&name, &argv(&["opencode", "--port", "8095"]))
        .unwrap();
    assert_eq!(host.ps(&name).unwrap(), vec![4242]);
    host.stop_owned(&name).unwrap();

    let id = herdr_pane_id();
    let harness = TestHarness;
    assert_eq!(harness.serve(&name, 8095).unwrap(), 4242);
    assert!(harness.health(8095).unwrap());
    assert_eq!(harness.create_session(8095).unwrap(), "ses_new");
    assert_eq!(harness.list_sessions(8095).unwrap(), vec!["ses_abc"]);
    harness.abort(8095, "ses_abc").unwrap();
    harness.attach_tui(&id, 8095, "ses_abc").unwrap();
    harness.select_session(&id, "ses_abc").unwrap();
    assert_eq!(
        harness.shown_session(&id).unwrap().as_deref(),
        Some("ses_abc")
    );
}

#[test]
fn scope_resolve_with_and_without_a_pane() {
    let mut other = pane();
    other.name = pane_name("hj-c2r1");
    let mut stranger = pane();
    stranger.name = pane_name("hj-c3r1");
    stranger.profile = None;
    let scope = MemScope {
        profile: profile(),
        panes: vec![pane(), other, stranger],
    };
    let p = profile_name("Some Profile");

    // No pane means every pane of the profile.
    let all = scope.resolve(&p, None).unwrap();
    assert_eq!(all.profile, profile());
    let names: Vec<&str> = all.panes.iter().map(|x| x.name.as_str()).collect();
    assert_eq!(names, vec!["hj-c1r1", "hj-c2r1"]);

    // A named member resolves to just that pane.
    let one = scope.resolve(&p, Some(&pane_name("hj-c2r1"))).unwrap();
    assert_eq!(one.panes.len(), 1);
    assert_eq!(one.panes[0].name.as_str(), "hj-c2r1");

    // A named pane outside the profile and a missing profile have their own codes.
    let outside = scope.resolve(&p, Some(&pane_name("hj-c3r1"))).unwrap_err();
    assert_eq!(outside.code(), "pane-not-in-profile");
    let missing = scope.resolve(&profile_name("Nope"), None).unwrap_err();
    assert_eq!(missing.code(), "profile-not-found");
}

#[test]
fn scope_edit_spec_with_and_without_a_profile() {
    let scope = MemScope {
        profile: profile(),
        panes: vec![pane()],
    };
    let target = pane_name("hj-c1r1");

    // Without a profile: only `act` runs, and no profile is touched.
    let mut ran = 0;
    let out = scope
        .edit_spec(None, &target, &SpecEdit::Remove, &mut || {
            ran += 1;
            Ok(())
        })
        .unwrap();
    assert_eq!((ran, out), (1, None));

    // With a profile: `act` runs and the edited profile comes back (generation + 1).
    let p = profile_name("Some Profile");
    let mut ran = 0;
    let removed = scope
        .edit_spec(Some(&p), &target, &SpecEdit::Remove, &mut || {
            ran += 1;
            Ok(())
        })
        .unwrap()
        .expect("a profile edit returns the edited profile");
    assert_eq!(ran, 1);
    assert_eq!(removed.generation, profile().generation + 1);
    assert!(removed.panes.iter().all(|s| s.pane != "hj-c1r1"));

    let set = scope
        .edit_spec(
            Some(&p),
            &target,
            &SpecEdit::Set(Box::new(spec())),
            &mut || Ok(()),
        )
        .unwrap()
        .unwrap();
    assert!(set.panes.iter().any(|s| s.pane == "hj-c1r1"));

    // A failing act fails the whole edit and records nothing.
    let failed = scope.edit_spec(Some(&p), &target, &SpecEdit::Remove, &mut || {
        Err(coded("probe-failed", "qwen38 missing"))
    });
    assert_eq!(failed.unwrap_err().code(), "probe-failed");
}

/// The shape #670's verbs use: borrowed ports, two writers, an `act` closure that
/// calls other ports while `edit_spec` runs.
fn verb_shaped(
    ports: &Ports<'_>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<(), PaneError> {
    let name = pane_name("hj-c1r1");
    let p = profile_name("Some Profile");
    let mut version = String::new();
    let edited = ports
        .scope
        .edit_spec(Some(&p), &name, &SpecEdit::Remove, &mut || {
            version = ports.herdr.version()?;
            ports.pane_store.get(&name)?;
            Ok(())
        });
    match edited {
        Ok(profile) => {
            let generation = profile.map_or(0, |pr| pr.generation);
            let _ = writeln!(out, "herdr {version} profile generation {generation}");
            Ok(())
        }
        Err(e) => {
            let _ = writeln!(err, "{}: {e}", e.code());
            Err(e)
        }
    }
}

#[test]
fn ports_bundle_drives_a_verb_shaped_function() {
    let pane_store = MemPaneStore::with(vec![pane()]);
    let profile_store = MemProfileStore { profile: profile() };
    let scope = MemScope {
        profile: profile(),
        panes: vec![pane()],
    };
    let (up, down) = (TestHerdr { up: true }, TestHerdr { up: false });
    let (host, harness, prober) = (TestHost, TestHarness, FailingProber);

    let ports = Ports {
        pane_store: &pane_store,
        profile_store: &profile_store,
        herdr: &up,
        host: &host,
        harness: &harness,
        scope: &scope,
        prober: &prober,
    };
    let (mut out, mut err) = (Vec::new(), Vec::new());
    verb_shaped(&ports, &mut out, &mut err).unwrap();
    assert_eq!(
        String::from_utf8(out).unwrap(),
        "herdr 0.9.1 profile generation 4\n"
    );
    assert!(err.is_empty());

    // The same verb against an unreachable Herdr: the act's error aborts the edit,
    // reaches stderr with its code, and nothing is written to stdout.
    let ports = Ports {
        herdr: &down,
        ..ports
    };
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let failure = verb_shaped(&ports, &mut out, &mut err).unwrap_err();
    assert_eq!(failure.code(), "unavailable");
    assert!(out.is_empty());
    assert!(String::from_utf8(err).unwrap().starts_with("unavailable: "));

    // The bundle reaches every port, including the prober.
    let result = ports.prober.run_probe(
        &argv(&["true"]),
        &["qwen38".to_string()],
        Duration::from_secs(1),
    );
    assert_eq!(
        result,
        ProbeResult::Failed {
            missing: vec!["qwen38".to_string()]
        }
    );
}

#[test]
fn run_probe_stub_never_reports_success() {
    // The free `run_probe` is a stub until #663 builds it: it must not say Ok, or a
    // launch that should refuse with probe-failed would sail through.
    let missing_binary = argv(&["/nonexistent/holler-probe-binary"]);
    let expect = ["qwen38".to_string()];
    let timeout = Duration::from_secs(1);

    assert!(!matches!(
        run_probe(&missing_binary, &expect, timeout),
        ProbeResult::Ok
    ));
    assert!(!matches!(
        SystemProber.run_probe(&missing_binary, &expect, timeout),
        ProbeResult::Ok
    ));

    // ProbeResult's three arms are all nameable and comparable.
    let arms = [
        ProbeResult::Ok,
        ProbeResult::Failed {
            missing: vec!["x".to_string()],
        },
        ProbeResult::Error("timed out".to_string()),
    ];
    assert_eq!(arms.len(), 3);
    assert_ne!(arms[0], arms[1]);
}
