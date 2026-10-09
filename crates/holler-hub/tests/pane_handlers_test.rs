#![allow(clippy::unwrap_used, clippy::expect_used)] // #639
#![allow(clippy::panic, clippy::unreachable)] // #639
//! Issue #639: the `pane/*` wire handlers (epic #633). `panes::dispatch` is called
//! directly with the whole request frame, and the reply line is parsed back the way a
//! client does (a `PaneReply` in a JSON-RPC result). The socket forwarding itself is
//! pinned by `pane_dispatch_test.rs` (#669).
//!
//! The registry is loaded with the short long-poll window, so no test waits the real
//! one. Every wait on a reply is a bounded `tokio::time::timeout`.

mod pane_support;

use std::sync::Arc;
use std::time::Duration;

use holler_hub::panes::{self, PaneState};
use holler_hub::profile::ProfileState;
use holler_hub::state::HubState;
use holler_pane::{Cursor, Pane, PaneError, PaneEvent, WatchReply};
use holler_proto::CorrelationId;
use pane_support::{outcome, registry_file, sample_pane, short_opts, temp_state};
use serde_json::{json, Value};

const REPLY_WITHIN: Duration = Duration::from_secs(10);

/// A pane registry and a profile registry on a throwaway state dir.
struct Rig {
    panes: Arc<PaneState>,
    profiles: Arc<ProfileState>,
    _dir: tempfile::TempDir,
}

impl Rig {
    fn new() -> Self {
        let (dir, state) = temp_state();
        Self::on(dir, &state)
    }

    fn on(dir: tempfile::TempDir, state: &HubState) -> Self {
        Self {
            panes: Arc::new(PaneState::load_with(state, short_opts())),
            profiles: Arc::new(ProfileState::load(state)),
            _dir: dir,
        }
    }

    /// Send one request and parse the reply back. `params: None` leaves the member out.
    async fn call(&self, method: &str, params: Option<Value>) -> Result<Option<Value>, PaneError> {
        call(&self.panes, &self.profiles, method, params).await
    }
}

async fn call(
    panes: &Arc<PaneState>,
    profiles: &Arc<ProfileState>,
    method: &str,
    params: Option<Value>,
) -> Result<Option<Value>, PaneError> {
    let cid = CorrelationId::parse("h-pane-1").unwrap();
    let mut frame = json!({"jsonrpc": "2.0", "id": "h-pane-1", "method": method});
    if let Some(params) = params {
        frame["params"] = params;
    }
    let line = tokio::time::timeout(
        REPLY_WITHIN,
        panes::dispatch(method, &cid, &frame, panes, profiles),
    )
    .await
    .unwrap_or_else(|_| panic!("no reply to {method} within {REPLY_WITHIN:?}"));
    outcome(&line, method)
}

fn cas_put_params(pane: &Pane, expected_generation: u64) -> Value {
    json!({"pane": pane, "expected_generation": expected_generation})
}

fn as_pane(data: Option<Value>) -> Pane {
    serde_json::from_value(data.expect("data")).expect("data is a Pane")
}

// --- AC 15: two verbs racing -------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_cas_put_requests_racing_exactly_one_wins() {
    let rig = Rig::new();
    rig.call(
        "pane/cas_put",
        Some(cas_put_params(&sample_pane("hj-c1r1", None), 0)),
    )
    .await
    .unwrap();
    let gate = Arc::new(tokio::sync::Barrier::new(2));
    let racers: Vec<_> = (0..2_u32)
        .map(|i| {
            let (panes, profiles, gate) = (rig.panes.clone(), rig.profiles.clone(), gate.clone());
            tokio::spawn(async move {
                let mut mine = sample_pane("hj-c1r1", None);
                mine.context.soft = 5_000 + i;
                gate.wait().await;
                call(
                    &panes,
                    &profiles,
                    "pane/cas_put",
                    Some(cas_put_params(&mine, 1)),
                )
                .await
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
    let stored = as_pane(
        rig.call("pane/get", Some(json!({"name": "hj-c1r1"})))
            .await
            .unwrap(),
    );
    assert_eq!(stored.generation, 2);
}

// --- AC 23: the five verbs, and the shape of their data ----------------------------

#[tokio::test]
async fn get_list_cas_put_delete_round_trip() {
    let rig = Rig::new();
    // Created out of order, so the list proves the sort.
    let b = sample_pane("hj-c1r2", None);
    let created_b = as_pane(
        rig.call("pane/cas_put", Some(cas_put_params(&b, 0)))
            .await
            .unwrap(),
    );
    assert_eq!(created_b.generation, 1, "cas_put returns the stored record");
    assert_eq!(created_b, Pane { generation: 1, ..b });
    let a = sample_pane("hj-c1r1", Some("Night Shift"));
    rig.call("pane/cas_put", Some(cas_put_params(&a, 0)))
        .await
        .unwrap();

    let got = rig
        .call("pane/get", Some(json!({"name": "hj-c1r2"})))
        .await
        .unwrap();
    assert_eq!(as_pane(got), created_b);
    let unknown = rig.call("pane/get", Some(json!({"name": "hj-c9r9"}))).await;
    assert_eq!(
        unknown,
        Ok(None),
        "an unknown name is data: null, not an error"
    );

    let list = rig
        .call("pane/list", Some(json!({})))
        .await
        .unwrap()
        .expect("data");
    let names: Vec<&str> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["hj-c1r1", "hj-c1r2"]);
    assert_eq!(list[0]["profile"], json!("Night Shift"));

    let deleted = rig
        .call(
            "pane/delete",
            Some(json!({"name": "hj-c1r2", "expected_generation": 1})),
        )
        .await;
    assert_eq!(deleted, Ok(None), "pane/delete answers data: null");
    assert_eq!(
        rig.call("pane/get", Some(json!({"name": "hj-c1r2"}))).await,
        Ok(None)
    );
    let remaining = rig
        .call("pane/list", Some(json!({})))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(remaining.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn a_request_without_params_works_for_list_and_watch() {
    let rig = Rig::new();
    assert_eq!(rig.call("pane/list", None).await, Ok(Some(json!([]))));
    assert_eq!(
        rig.call("pane/list", Some(Value::Null)).await,
        Ok(Some(json!([])))
    );
    let batch = rig.call("pane/watch", None).await.unwrap().unwrap();
    assert_eq!(batch, json!({"events": [], "cursor": 0}));
}

// --- AC 24: bad params answer their code -------------------------------------------

#[tokio::test]
async fn bad_params_answer_their_code() {
    let rig = Rig::new();
    let missing_name = rig.call("pane/get", Some(json!({}))).await.unwrap_err();
    assert!(
        matches!(missing_name, PaneError::Usage { .. }),
        "got {missing_name:?}"
    );
    assert_eq!(missing_name.code(), "usage");

    // The guard's own code survives `decode_params`: a shell string is not an argv.
    let mut shell = serde_json::to_value(sample_pane("hj-c1r1", None)).unwrap();
    shell["command"] = json!("opencode --port 8095");
    let params = json!({"pane": shell, "expected_generation": 0});
    let not_argv = rig.call("pane/cas_put", Some(params)).await.unwrap_err();
    assert_eq!(not_argv, PaneError::CommandNotArgv);
    assert_eq!(
        rig.call("pane/list", None).await,
        Ok(Some(json!([]))),
        "nothing was stored"
    );

    let gone = rig
        .call(
            "pane/delete",
            Some(json!({"name": "hj-c1r1", "expected_generation": 1})),
        )
        .await;
    assert!(
        matches!(gone, Err(PaneError::PaneNotFound { .. })),
        "got {gone:?}"
    );

    let pane = sample_pane("hj-c1r1", None);
    rig.call("pane/cas_put", Some(cas_put_params(&pane, 0)))
        .await
        .unwrap();
    let stale = rig
        .call("pane/cas_put", Some(cas_put_params(&pane, 0)))
        .await;
    assert_eq!(stale, Err(PaneError::Conflict));
    let stale_delete = rig
        .call(
            "pane/delete",
            Some(json!({"name": "hj-c1r1", "expected_generation": 9})),
        )
        .await;
    assert_eq!(stale_delete, Err(PaneError::Conflict));
}

// --- AC 25: pane/watch is a long-poll that answers one batch -----------------------

#[tokio::test]
async fn pane_watch_answers_one_batch_and_its_cursor() {
    let rig = Rig::new();
    for name in ["hj-c1r1", "hj-c1r2"] {
        rig.call(
            "pane/cas_put",
            Some(cas_put_params(&sample_pane(name, None), 0)),
        )
        .await
        .unwrap();
    }
    let data = rig
        .call("pane/watch", Some(json!({"since": 0})))
        .await
        .unwrap()
        .unwrap();
    let batch: WatchReply<PaneEvent> =
        serde_json::from_value(data).expect("a WatchReply<PaneEvent>");
    assert_eq!(batch.events.len(), 2);
    assert_eq!(batch.cursor, Cursor(2), "the cursor to resume from");

    // Idle: the reply comes after the window, empty, with the head (here equal to `since`).
    let idle = rig
        .call("pane/watch", Some(json!({"since": 2})))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(idle, json!({"events": [], "cursor": 2}));

    rig.call(
        "pane/cas_put",
        Some(cas_put_params(&sample_pane("hj-c1r3", None), 0)),
    )
    .await
    .unwrap();
    let data = rig
        .call("pane/watch", Some(json!({"since": 2})))
        .await
        .unwrap()
        .unwrap();
    let batch: WatchReply<PaneEvent> = serde_json::from_value(data).unwrap();
    assert_eq!((batch.events.len(), batch.cursor), (1, Cursor(3)));
    assert_eq!(batch.events[0].name.as_str(), "hj-c1r3");
}

/// AC 25's one edge (ADR-0021 §6, "Decisions taken" item 7): an idle window answers the
/// head, not `since`. They differ only for a watch from 0 over an all-deleted registry.
/// Resuming from 0 would keep the watcher in snapshot mode, so a pane created and deleted
/// between two polls would never reach it; resuming from the head delivers both events.
#[tokio::test]
async fn an_idle_watch_from_zero_over_an_all_deleted_registry_answers_the_head() {
    let rig = Rig::new();
    let a = sample_pane("hj-c1r1", None);
    rig.call("pane/cas_put", Some(cas_put_params(&a, 0)))
        .await
        .unwrap();
    rig.call(
        "pane/delete",
        Some(json!({"name": "hj-c1r1", "expected_generation": 1})),
    )
    .await
    .unwrap();

    let idle = rig
        .call("pane/watch", Some(json!({"since": 0})))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        idle,
        json!({"events": [], "cursor": 2}),
        "an idle window answers the head, which is not `since` here"
    );

    let b = sample_pane("hj-c1r2", None);
    rig.call("pane/cas_put", Some(cas_put_params(&b, 0)))
        .await
        .unwrap();
    rig.call(
        "pane/delete",
        Some(json!({"name": "hj-c1r2", "expected_generation": 1})),
    )
    .await
    .unwrap();
    let data = rig
        .call("pane/watch", Some(json!({"since": 2})))
        .await
        .unwrap()
        .unwrap();
    let batch: WatchReply<PaneEvent> = serde_json::from_value(data).unwrap();
    let seen: Vec<(Cursor, &str, bool)> = batch
        .events
        .iter()
        .map(|e| (e.cursor, e.name.as_str(), e.pane.is_some()))
        .collect();
    assert_eq!(
        seen,
        [(Cursor(3), "hj-c1r2", true), (Cursor(4), "hj-c1r2", false)],
        "B's create and its delete both reach the watcher"
    );
    assert_eq!(batch.cursor, Cursor(4));
}

// --- AC 26: a corrupt registry --------------------------------------------------------

#[tokio::test]
async fn a_corrupt_registry_answers_store_corrupt_on_every_pane_method() {
    let (dir, state) = temp_state();
    std::fs::create_dir_all(&state.hub_dir).unwrap();
    std::fs::write(registry_file(&state), b"{ not json").unwrap();
    let rig = Rig::on(dir, &state);
    let pane = sample_pane("hj-c1r1", None);
    let requests = [
        ("pane/get", json!({"name": "hj-c1r1"})),
        ("pane/list", json!({})),
        ("pane/cas_put", cas_put_params(&pane, 0)),
        (
            "pane/delete",
            json!({"name": "hj-c1r1", "expected_generation": 1}),
        ),
        ("pane/watch", json!({"since": 0})),
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
    assert_eq!(std::fs::read(registry_file(&state)).unwrap(), b"{ not json");
}
