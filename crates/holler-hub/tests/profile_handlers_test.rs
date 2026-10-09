#![allow(clippy::unwrap_used, clippy::expect_used)] // #661
#![allow(clippy::panic, clippy::unreachable)] // #661
//! Issue #661: the `profile/*` wire handlers and the membership hook (epic #633).
//! `profile::dispatch` (and `panes::dispatch` for the pane half) is called directly with
//! the whole request frame, and the reply line is parsed back the way a client does (a
//! `PaneReply` in a JSON-RPC result). The socket forwarding is pinned by
//! `pane_dispatch_test.rs` (#669).
//!
//! The registries are loaded with the short long-poll window, so no test waits the real
//! one. Every wait on a reply is a bounded `tokio::time::timeout`.

mod pane_support;

use std::sync::Arc;
use std::time::Duration;

use holler_hub::panes::{self, PaneState};
use holler_hub::profile::{self, ProfileState};
use holler_hub::state::HubState;
use holler_pane::{
    Cursor, Pane, PaneError, PaneStore, Profile, ProfileEvent, ProfileLogEntry, WatchReply,
};
use holler_proto::CorrelationId;
use pane_support::{
    dir_listing, load_profiles, outcome, profiles_file, registry_file, sample_pane, short_opts,
    temp_state,
};
use serde_json::{json, Value};

const REPLY_WITHIN: Duration = Duration::from_secs(10);
const SENTINEL: &str = "SENTINEL-661";

/// A pane registry and a profile registry on a throwaway state dir.
struct Rig {
    panes: Arc<PaneState>,
    profiles: Arc<ProfileState>,
    state: HubState,
    _dir: tempfile::TempDir,
}

impl Rig {
    fn new() -> Self {
        let (dir, state) = temp_state();
        Self::on(dir, state)
    }

    fn on(dir: tempfile::TempDir, state: HubState) -> Self {
        Self {
            panes: Arc::new(PaneState::load_with(&state, short_opts())),
            profiles: Arc::new(load_profiles(&state)),
            state,
            _dir: dir,
        }
    }

    /// Send one request and return the reply line. `params: None` leaves the member out.
    async fn line(&self, method: &str, params: Option<Value>) -> String {
        line(&self.panes, &self.profiles, method, params).await
    }

    /// Send one request and parse the reply back.
    async fn call(&self, method: &str, params: Option<Value>) -> Result<Option<Value>, PaneError> {
        outcome(&self.line(method, params).await, method)
    }
}

async fn line(
    panes: &Arc<PaneState>,
    profiles: &Arc<ProfileState>,
    method: &str,
    params: Option<Value>,
) -> String {
    let cid = CorrelationId::parse("h-profile-1").unwrap();
    let mut frame = json!({"jsonrpc": "2.0", "id": "h-profile-1", "method": method});
    if let Some(params) = params {
        frame["params"] = params;
    }
    let work = async {
        if method.starts_with("profile/") {
            profile::dispatch(method, &cid, &frame, profiles, panes).await
        } else {
            panes::dispatch(method, &cid, &frame, panes, profiles).await
        }
    };
    tokio::time::timeout(REPLY_WITHIN, work)
        .await
        .unwrap_or_else(|_| panic!("no reply to {method} within {REPLY_WITHIN:?}"))
}

fn sample(name: &str, panes: &[&str]) -> Profile {
    pane_support::profile(name, panes)
}

fn put_params(profile: &Profile, expected_generation: u64, actor: &str) -> Value {
    json!({"profile": profile, "expected_generation": expected_generation, "actor": actor})
}

fn as_profile(data: Option<Value>) -> Profile {
    serde_json::from_value(data.expect("data")).expect("data is a Profile")
}

fn pane_put_params(pane: &Pane, expected_generation: u64) -> Value {
    json!({"pane": pane, "expected_generation": expected_generation})
}

async fn create(rig: &Rig, name: &str) -> Profile {
    as_profile(
        rig.call(
            "profile/cas_put",
            Some(put_params(&sample(name, &["hj-c1r1"]), 0, "alice")),
        )
        .await
        .unwrap(),
    )
}

// --- AC 31: two verbs racing ---------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_cas_put_requests_racing_exactly_one_wins() {
    let rig = Rig::new();
    create(&rig, "Night Shift").await;
    let gate = Arc::new(tokio::sync::Barrier::new(2));
    let racers: Vec<_> = (0..2_u32)
        .map(|i| {
            let (panes, profiles, gate) = (rig.panes.clone(), rig.profiles.clone(), gate.clone());
            tokio::spawn(async move {
                let mut mine = sample("Night Shift", &["hj-c1r1"]);
                mine.panes[0].context.soft = 5_000 + i;
                gate.wait().await;
                let params = Some(put_params(&mine, 1, "racer"));
                outcome(
                    &line(&panes, &profiles, "profile/cas_put", params).await,
                    "profile/cas_put",
                )
            })
        })
        .collect();
    let mut outcomes = Vec::new();
    for racer in racers {
        outcomes.push(racer.await.unwrap());
    }
    let wins = outcomes.iter().filter(|o| o.is_ok()).count();
    let conflicts = outcomes
        .iter()
        .filter(|o| matches!(o, Err(PaneError::Conflict)))
        .count();
    assert_eq!((wins, conflicts), (1, 1), "outcomes: {outcomes:?}");
    let stored = as_profile(
        rig.call("profile/get", Some(json!({"name": "Night Shift"})))
            .await
            .unwrap(),
    );
    assert_eq!(stored.generation, 2);
}

// --- AC 26: the six verbs, and the shape of their data ------------------------------

#[tokio::test]
async fn get_list_cas_put_delete_log_round_trip() {
    let rig = Rig::new();
    // Created out of order, so the list is checked as a set (it has no pinned order).
    let b = sample("Day Shift", &["hj-c1r2"]);
    let created_b = as_profile(
        rig.call("profile/cas_put", Some(put_params(&b, 0, "alice")))
            .await
            .unwrap(),
    );
    assert_eq!(created_b.generation, 1, "cas_put returns the stored record");
    assert_eq!(created_b.slug, "day-shift");
    assert_eq!(created_b.panes, b.panes);
    create(&rig, "Night Shift").await;

    let got = rig
        .call("profile/get", Some(json!({"name": "Day Shift"})))
        .await
        .unwrap();
    assert_eq!(as_profile(got), created_b);
    let unknown = rig
        .call("profile/get", Some(json!({"name": "Unknown"})))
        .await;
    assert_eq!(
        unknown,
        Ok(None),
        "an unknown name is data: null, not an error"
    );

    let list = rig
        .call("profile/list", Some(json!({})))
        .await
        .unwrap()
        .expect("data");
    let mut names: Vec<&str> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["name"].as_str().unwrap())
        .collect();
    names.sort_unstable();
    assert_eq!(names, ["Day Shift", "Night Shift"]);

    let mut next = created_b.clone();
    next.panes[0].context.soft += 1;
    rig.call("profile/cas_put", Some(put_params(&next, 1, "bob")))
        .await
        .unwrap();
    let deleted = rig
        .call(
            "profile/delete",
            Some(json!({"name": "Day Shift", "expected_generation": 2, "actor": "carol"})),
        )
        .await;
    assert_eq!(deleted, Ok(None), "profile/delete answers data: null");
    assert_eq!(
        rig.call("profile/get", Some(json!({"name": "Day Shift"})))
            .await,
        Ok(None)
    );

    let data = rig
        .call("profile/log", Some(json!({"name": "Day Shift"})))
        .await
        .unwrap()
        .unwrap();
    let log: Vec<ProfileLogEntry> = serde_json::from_value(data).expect("an array of log entries");
    let seen: Vec<(u64, &str)> = log
        .iter()
        .map(|e| (e.generation, e.actor.as_str()))
        .collect();
    assert_eq!(
        seen,
        [(1, "alice"), (2, "bob"), (3, "carol")],
        "oldest first, each with its actor"
    );
    assert_eq!(
        rig.call("profile/list", Some(json!({})))
            .await
            .unwrap()
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn a_request_without_params_works_for_list_and_watch() {
    let rig = Rig::new();
    assert_eq!(rig.call("profile/list", None).await, Ok(Some(json!([]))));
    assert_eq!(
        rig.call("profile/list", Some(Value::Null)).await,
        Ok(Some(json!([])))
    );
    let batch = rig.call("profile/watch", None).await.unwrap().unwrap();
    assert_eq!(batch, json!({"events": [], "cursor": 0}));
}

// --- AC 27: bad params answer their code ---------------------------------------------

#[tokio::test]
async fn bad_params_answer_their_code() {
    let rig = Rig::new();
    let night = sample("Night Shift", &["hj-c1r1"]);
    let usage = [
        ("profile/get", json!({})),
        ("profile/get", json!({"name": "Night Shift", "x": 1})),
        ("profile/log", json!({})),
        ("profile/list", json!({"x": 1})),
        (
            "profile/delete",
            json!({"name": "Night Shift", "expected_generation": 1}),
        ),
        (
            "profile/cas_put",
            json!({"profile": night, "expected_generation": 0}),
        ),
        (
            "profile/cas_put",
            json!({"profile": night, "expected_generation": 0, "actor": "a", "x": 1}),
        ),
    ];
    for (method, params) in usage {
        let err = rig.call(method, Some(params.clone())).await.unwrap_err();
        assert!(
            matches!(err, PaneError::Usage { .. }),
            "{method} {params}: {err:?}"
        );
        assert_eq!(err.code(), "usage");
    }
    assert_eq!(
        rig.call("profile/list", None).await,
        Ok(Some(json!([]))),
        "nothing was stored"
    );

    for method in ["profile/delete", "profile/log"] {
        let params = json!({"name": "Night Shift", "expected_generation": 1, "actor": "a"});
        let params = if method == "profile/log" {
            json!({"name": "Night Shift"})
        } else {
            params
        };
        let gone = rig.call(method, Some(params)).await;
        assert!(
            matches!(gone, Err(PaneError::ProfileNotFound { .. })),
            "{method}: {gone:?}"
        );
    }

    create(&rig, "Night Shift").await;
    let stale = rig
        .call("profile/cas_put", Some(put_params(&night, 0, "a")))
        .await;
    assert_eq!(stale, Err(PaneError::Conflict));
    let stale_delete = rig
        .call(
            "profile/delete",
            Some(json!({"name": "Night Shift", "expected_generation": 9, "actor": "a"})),
        )
        .await;
    assert_eq!(stale_delete, Err(PaneError::Conflict));
    let same_slug = rig
        .call(
            "profile/cas_put",
            Some(put_params(&sample("night shift", &[]), 0, "a")),
        )
        .await;
    assert!(
        matches!(same_slug, Err(PaneError::ProfileExists { .. })),
        "{same_slug:?}"
    );
}

// --- AC 28: the secret guard (I7) ----------------------------------------------------

#[tokio::test]
async fn a_secret_value_is_refused_over_the_wire_and_never_echoed() {
    let rig = Rig::new();
    let mut with_env = serde_json::to_value(sample("Night Shift", &["hj-c1r1"])).unwrap();
    let cases = [
        (
            json!([format!("TOKEN={SENTINEL}")]),
            "profile-secret-refused",
        ),
        (json!([" "]), "env-name-invalid"),
    ];
    for (env, code) in cases {
        with_env["panes"][0]["env"] = env;
        let params = json!({"profile": with_env, "expected_generation": 0, "actor": "alice"});
        let reply = rig.line("profile/cas_put", Some(params)).await;
        assert!(
            !reply.contains(SENTINEL),
            "the reply echoes the value: {reply}"
        );
        let err = outcome(&reply, "profile/cas_put").unwrap_err();
        assert_eq!(err.code(), code, "{err:?}");
    }
    assert!(!profiles_file(&rig.state).exists(), "nothing was written");
    let log = rig
        .call("profile/log", Some(json!({"name": "Night Shift"})))
        .await;
    assert!(
        matches!(log, Err(PaneError::ProfileNotFound { .. })),
        "no log: {log:?}"
    );
}

// --- AC 29: profile/watch is a long-poll that answers one batch ----------------------

#[tokio::test]
async fn profile_watch_answers_one_batch_and_its_cursor() {
    let rig = Rig::new();
    create(&rig, "Night Shift").await;
    create(&rig, "Day Shift").await;
    let data = rig
        .call("profile/watch", Some(json!({"since": 0})))
        .await
        .unwrap()
        .unwrap();
    let batch: WatchReply<ProfileEvent> = serde_json::from_value(data).expect("a WatchReply");
    assert_eq!(batch.events.len(), 2);
    assert_eq!(batch.cursor, Cursor(2), "the cursor to resume from");

    // Idle: the reply comes after the window, empty, with the head.
    let idle = rig
        .call("profile/watch", Some(json!({"since": 2})))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(idle, json!({"events": [], "cursor": 2}));

    create(&rig, "Dusk Shift").await;
    let data = rig
        .call("profile/watch", Some(json!({"since": 2})))
        .await
        .unwrap()
        .unwrap();
    let batch: WatchReply<ProfileEvent> = serde_json::from_value(data).unwrap();
    assert_eq!((batch.events.len(), batch.cursor), (1, Cursor(3)));
    assert_eq!(batch.events[0].name.as_str(), "Dusk Shift");

    let ahead = rig
        .call("profile/watch", Some(json!({"since": 9})))
        .await
        .unwrap_err();
    assert_eq!(ahead.code(), "usage", "a cursor ahead of the head is usage");
}

// --- AC 30 (and AC 9, wire half): no API takes a log ---------------------------------

#[tokio::test]
async fn the_log_cannot_be_rewritten_over_the_wire() {
    let rig = Rig::new();
    let created = create(&rig, "Night Shift").await;
    let bytes = std::fs::read(profiles_file(&rig.state)).unwrap();
    let log_before = rig
        .call("profile/log", Some(json!({"name": "Night Shift"})))
        .await
        .unwrap();
    let forged = json!([{"at": 1, "generation": 1, "actor": "mallory", "change": "created"}]);

    let mut in_profile = put_params(&created, 1, "mallory");
    in_profile["profile"]["log"] = forged.clone();
    let mut in_params = put_params(&created, 1, "mallory");
    in_params["log"] = forged;
    for params in [in_profile, in_params] {
        let err = rig.call("profile/cas_put", Some(params)).await.unwrap_err();
        assert!(matches!(err, PaneError::Usage { .. }), "got {err:?}");
    }
    let log_after = rig
        .call("profile/log", Some(json!({"name": "Night Shift"})))
        .await
        .unwrap();
    assert_eq!(log_after, log_before);
    assert_eq!(std::fs::read(profiles_file(&rig.state)).unwrap(), bytes);
}

// --- AC 32: a corrupt registry --------------------------------------------------------

#[tokio::test]
async fn a_corrupt_registry_answers_store_corrupt_on_every_profile_method() {
    let (dir, state) = temp_state();
    std::fs::create_dir_all(&state.hub_dir).unwrap();
    std::fs::write(profiles_file(&state), b"{ not json").unwrap();
    let listing = dir_listing(&state.hub_dir);
    let rig = Rig::on(dir, state);
    let night = sample("Night Shift", &["hj-c1r1"]);
    let requests = [
        ("profile/get", json!({"name": "Night Shift"})),
        ("profile/list", json!({})),
        ("profile/cas_put", put_params(&night, 0, "alice")),
        (
            "profile/delete",
            json!({"name": "Night Shift", "expected_generation": 1, "actor": "a"}),
        ),
        ("profile/watch", json!({"since": 0})),
        ("profile/log", json!({"name": "Night Shift"})),
    ];
    for (method, params) in requests {
        // `outcome` fails the test if the answer is a JSON-RPC error frame.
        let err = rig.call(method, Some(params)).await.unwrap_err();
        assert!(
            matches!(err, PaneError::StoreCorrupt { .. }),
            "{method}: {err:?}"
        );
        assert_eq!(err.code(), "store-corrupt");
    }
    assert_eq!(
        std::fs::read(profiles_file(&rig.state)).unwrap(),
        b"{ not json"
    );
    assert_eq!(dir_listing(&rig.state.hub_dir), listing);

    // profile/rename is #665's: it still answers not-implemented, corrupt registry or not.
    let rename = rig.call("profile/rename", Some(json!({}))).await;
    assert_eq!(rename, Err(PaneError::NotImplemented));
}

// --- AC 33: a detached spec ----------------------------------------------------------

#[tokio::test]
async fn a_detached_spec_is_accepted() {
    let rig = Rig::new();
    create(&rig, "Night Shift").await;
    let pane = sample_pane("hj-c1r1", Some("Night Shift"));
    rig.call("pane/cas_put", Some(pane_put_params(&pane, 0)))
        .await
        .unwrap();

    // Day Shift names the pane of Night Shift, and a pane that does not exist.
    let day = sample("Day Shift", &["hj-c1r1", "hj-c9r9"]);
    let stored = as_profile(
        rig.call("profile/cas_put", Some(put_params(&day, 0, "alice")))
            .await
            .unwrap(),
    );
    assert_eq!(stored.panes.len(), 2);

    let record = rig
        .panes
        .get(&pane_support::name("hj-c1r1"))
        .unwrap()
        .unwrap();
    assert_eq!(
        record.profile.as_ref().map(|p| p.as_str()),
        Some("Night Shift")
    );
    assert_eq!(record.generation, 1, "the pane record was not touched");
}

// --- AC 38: the hook ------------------------------------------------------------------

#[tokio::test]
async fn pane_cas_put_naming_a_missing_profile_is_profile_not_found() {
    let rig = Rig::new();
    let pane = sample_pane("hj-c1r1", Some("Night Shift"));
    let err = rig
        .call("pane/cas_put", Some(pane_put_params(&pane, 0)))
        .await
        .unwrap_err();
    assert!(
        matches!(err, PaneError::ProfileNotFound { .. }),
        "got {err:?}"
    );
    assert_eq!(err.code(), "profile-not-found");
    assert_eq!(
        rig.call("pane/list", None).await,
        Ok(Some(json!([]))),
        "nothing was written"
    );
    assert!(!registry_file(&rig.state).exists(), "no pane file either");

    create(&rig, "Night Shift").await;
    let stored = rig
        .call("pane/cas_put", Some(pane_put_params(&pane, 0)))
        .await;
    assert!(
        stored.is_ok(),
        "the same request succeeds once the profile exists: {stored:?}"
    );
}

// --- AC 39: a pane outside any profile never reads the profile registry ---------------

#[tokio::test]
async fn a_pane_outside_any_profile_never_reads_the_profile_registry() {
    let (dir, state) = temp_state();
    std::fs::create_dir_all(&state.hub_dir).unwrap();
    std::fs::write(profiles_file(&state), b"{ not json").unwrap();
    let rig = Rig::on(dir, state);

    let free = sample_pane("hj-c1r1", None);
    let stored = rig
        .call("pane/cas_put", Some(pane_put_params(&free, 0)))
        .await;
    assert!(
        stored.is_ok(),
        "a pane with no profile is written: {stored:?}"
    );

    let member = sample_pane("hj-c1r2", Some("Night Shift"));
    let err = rig
        .call("pane/cas_put", Some(pane_put_params(&member, 0)))
        .await
        .unwrap_err();
    assert!(
        matches!(err, PaneError::StoreCorrupt { .. }),
        "fail closed, never accepted: {err:?}"
    );
    let listed = rig.call("pane/list", None).await.unwrap().unwrap();
    assert_eq!(
        listed.as_array().unwrap().len(),
        1,
        "the member pane was not stored"
    );
}
