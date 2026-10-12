//! The store clients and `connect` against a hub that misbehaves on purpose: D2, D3, D4 and
//! A's W-9 mapping, one row per test.
//!
//! [`FakeHub`] listens on the control socket of a fresh state dir and answers each
//! connection, in order, with the next scripted [`Answer`]: it closes without a reply, holds
//! the connection open without one, answers a JSON-RPC error, or answers a result. It records
//! the method and params of every request it read. A connection after the script ends is
//! refused (the listener is gone), which the client sees as no hub.
//!
//! The rows (A's W-9), each pinned below:
//!
//! | what the exchange did | a read | a write |
//! |---|---|---|
//! | no socket (`NoLiveHub`) | `unavailable`, naming the hub | `unavailable`, naming the hub |
//! | a JSON-RPC error (`Refused`) | `unavailable` | `unavailable` |
//! | closed before the reply (`BadReply`) | `unavailable` | `timeout` (D3: it may have landed) |
//! | no reply within the bound (`Io`, timed out) | `timeout`, naming the store method | `timeout` |
//! | a `data` that does not decode | `unavailable`, naming the method | |

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use holler_cli::pane::wiring::store::{HubPaneStore, HubProfileStore};
use holler_cli::pane::wiring::Wiring;
use holler_hub::state::{control_sock_path, HubState};
use holler_pane::{
    Actor, Cursor, PaneError, PaneName, PaneStore, ProfileName, ProfileStore,
};
use holler_pane_testkit::fixture::{sample_pane, sample_profile};
use holler_proto::{Code, CorrelationId, Envelope, WireError};
use serde_json::{json, Value};

use crate::support::StateDir;
use crate::traps::NO_HUB;
use crate::with_state_dir;

/// What the fake hub does with one connection, after reading its request line.
#[derive(Debug, Clone)]
pub enum Answer {
    /// Close the connection without a reply.
    Close,
    /// Keep the connection open, without a reply, for this long.
    Hang(Duration),
    /// A JSON-RPC error frame.
    Error,
    /// A result frame carrying this value (a `PaneReply`, or `control/status`'s result).
    Result(Value),
    /// The same, after this long.
    After(Duration, Value),
}

/// A control socket that answers from a script.
pub struct FakeHub {
    pub state: StateDir,
    requests: Arc<Mutex<Vec<(String, Value)>>>,
}

impl FakeHub {
    pub fn start(script: Vec<Answer>) -> Self {
        let state = StateDir::new();
        let hub = HubState::from_root(state.path().to_path_buf());
        std::fs::create_dir_all(&hub.hub_dir).unwrap();
        let listener = UnixListener::bind(control_sock_path(&hub)).unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&requests);
        thread::spawn(move || {
            for answer in script {
                let Ok((stream, _)) = listener.accept() else {
                    return;
                };
                serve(stream, &answer, &seen);
            }
            // The listener drops here: a later connect is refused.
        });
        Self { state, requests }
    }

    /// The method and params of every request read so far, oldest first.
    pub fn requests(&self) -> Vec<(String, Value)> {
        self.requests
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Run `f` with this process pointed at the fake hub.
    pub fn with<R>(&self, f: impl FnOnce() -> R) -> R {
        with_state_dir(self.state.path(), f)
    }
}

fn serve(stream: UnixStream, answer: &Answer, seen: &Mutex<Vec<(String, Value)>>) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut line = String::new();
    if reader.read_line(&mut line).unwrap_or(0) == 0 {
        return;
    }
    let request: Value = serde_json::from_str(&line).unwrap_or(Value::Null);
    let method = request["method"].as_str().unwrap_or_default().to_owned();
    let id = CorrelationId::parse(request["id"].as_str().unwrap_or("b-fake")).unwrap();
    seen.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push((method, request["params"].clone()));
    let mut stream = stream;
    let frame = match answer {
        Answer::Close => return,
        Answer::Hang(how_long) => {
            thread::sleep(*how_long);
            return;
        }
        Answer::Error => Envelope::error_frame(
            &id,
            &WireError::new(Code::InvalidRequest, "the fake hub refuses", None),
        ),
        Answer::Result(value) => Envelope::response(&id, Some(value.clone())),
        Answer::After(delay, value) => {
            thread::sleep(*delay);
            Envelope::response(&id, Some(value.clone()))
        }
    };
    let text = holler_proto::encode(&frame).unwrap();
    let _ = stream.write_all(format!("{text}\n").as_bytes());
    let _ = stream.flush();
}

fn pane() -> holler_pane::Pane {
    sample_pane("hlr649-c1r1").unwrap()
}

fn pname() -> PaneName {
    PaneName::parse("hlr649-c1r1").unwrap()
}

fn profile() -> holler_pane::Profile {
    sample_profile("hlr649", &[]).unwrap()
}

fn prname(text: &str) -> ProfileName {
    ProfileName::parse(text).unwrap()
}

fn actor() -> Actor {
    Actor::parse("holler pane").unwrap()
}

/// The code of `result`'s error, and its text.
fn failure<T: std::fmt::Debug>(result: Result<T, PaneError>) -> (String, String) {
    let error = result.expect_err("the call fails");
    (error.code().to_owned(), error.to_string())
}

/// One client call: the method it must send, and the call, yielding its error's code and text.
type Call = (&'static str, fn() -> (String, String));

/// Every write of both clients.
fn writes() -> [Call; 5] {
    [
        ("pane/cas_put", || failure(HubPaneStore::new().cas_put(&pane(), 0))),
        ("pane/delete", || failure(HubPaneStore::new().delete(&pname(), 1))),
        ("profile/cas_put", || {
            failure(HubProfileStore::new().cas_put(&profile(), 0, &actor()))
        }),
        ("profile/delete", || {
            failure(HubProfileStore::new().delete(&prname("hlr649"), 1, &actor()))
        }),
        ("profile/rename", || {
            failure(HubProfileStore::new().rename(&prname("hlr649"), &prname("hlr649b"), 1, &actor()))
        }),
    ]
}

/// Every read of both clients.
fn reads() -> [Call; 5] {
    [
        ("pane/get", || failure(HubPaneStore::new().get(&pname()))),
        ("pane/list", || failure(HubPaneStore::new().list())),
        ("profile/get", || failure(HubProfileStore::new().get(&prname("hlr649")))),
        ("profile/list", || failure(HubProfileStore::new().list())),
        ("profile/log", || failure(HubProfileStore::new().log(&prname("hlr649")))),
    ]
}

/// One call's failure against a fake hub, and that hub's state dir.
struct Outcome {
    method: &'static str,
    code: String,
    text: String,
    state: PathBuf,
}

impl Outcome {
    /// AC 3's "naming the hub": the message says which holler hub, by its state dir. The dir
    /// is a fresh temporary path, so neither a transport error's text nor the fake hub's own
    /// refusal text can supply it.
    fn assert_names_the_hub(&self) {
        let dir = self.state.display().to_string();
        assert!(
            self.text.contains("holler hub") && self.text.contains(&dir),
            "{}: the message names the hub at {dir}: {}",
            self.method,
            self.text
        );
    }
}

/// Each of `calls` against a fresh fake hub with one connection answered `answer`. Each call
/// must send its own method.
fn against(answer: &Answer, calls: [Call; 5]) -> Vec<Outcome> {
    calls
        .into_iter()
        .map(|(method, call)| {
            let hub = FakeHub::start(vec![answer.clone()]);
            let (code, text) = hub.with(call);
            assert_eq!(
                hub.requests().first().map(|(m, _)| m.as_str()),
                Some(method),
                "the client sent {method}"
            );
            Outcome { method, code, text, state: hub.state.path().to_path_buf() }
        })
        .collect()
}

/// AC 3 and D3: a write whose reply is lost after it was sent is `timeout`, naming the
/// write, because it may have landed.
#[test]
fn a_write_whose_reply_is_lost_is_timeout() {
    for Outcome { method, code, text, .. } in against(&Answer::Close, writes()) {
        assert_eq!(code, "timeout", "{method}: {text}");
        let op = method.replace('/', "_store.");
        assert!(text.contains(&op), "{method}: the timeout names {op}: {text}");
    }
}

/// W-9 and AC 3: a read whose reply is lost is `unavailable` (nothing was written), naming
/// the hub.
#[test]
fn a_read_whose_reply_is_lost_is_unavailable_naming_the_hub() {
    for outcome in against(&Answer::Close, reads()) {
        assert_eq!(outcome.code, "unavailable", "{}: {}", outcome.method, outcome.text);
        outcome.assert_names_the_hub();
    }
}

/// D3 and I5: a write the hub never answers is `timeout`, within the 10 s bound.
#[test]
fn a_write_the_hub_never_answers_is_timeout_within_the_bound() {
    let hub = FakeHub::start(vec![Answer::Hang(Duration::from_secs(14))]);
    let (code, text, took) = hub.with(|| {
        let started = Instant::now();
        let (code, text) = failure(HubPaneStore::new().cas_put(&pane(), 0));
        (code, text, started.elapsed())
    });
    assert_eq!(code, "timeout", "{text}");
    assert!(text.contains("pane_store.cas_put"), "the timeout names the write: {text}");
    assert!(took < Duration::from_secs(13), "bounded by 10 s: took {took:?}");
}

/// W-9: a read the hub never answers is `timeout`, within the 10 s bound.
#[test]
fn a_read_the_hub_never_answers_is_timeout_within_the_bound() {
    let hub = FakeHub::start(vec![Answer::Hang(Duration::from_secs(14))]);
    let (code, text, took) = hub.with(|| {
        let started = Instant::now();
        let (code, text) = failure(HubPaneStore::new().get(&pname()));
        (code, text, started.elapsed())
    });
    assert_eq!(code, "timeout", "{text}");
    assert!(text.contains("pane_store.get"), "the timeout names the store method: {text}");
    assert!(took < Duration::from_secs(13), "bounded by 10 s: took {took:?}");
}

/// D3, D9 and I5, from below: the store's bound is its own 10 s, not the control client's
/// default 5 s, so a read and a write the hub answers after 6 s each succeed. With the two
/// hang rows above, this pins the bound between 6 s and 13 s.
#[test]
fn a_read_and_a_write_answered_after_6_s_succeed() {
    let after = Duration::from_secs(6);
    let written = pane();
    let hub = FakeHub::start(vec![
        Answer::After(after, json!({ "ok": true, "data": null })),
        Answer::After(after, json!({ "ok": true, "data": written })),
    ]);
    let (read, write) = hub.with(|| {
        let store = HubPaneStore::new();
        (store.get(&pname()), store.cas_put(&written, 0))
    });
    assert!(matches!(read, Ok(None)), "the read waits past 5 s: {read:?}");
    assert_eq!(write.ok(), Some(written), "the write waits past 5 s");
    let methods: Vec<String> = hub.requests().into_iter().map(|(method, _)| method).collect();
    assert_eq!(methods, ["pane/get", "pane/cas_put"]);
}

/// D3: a watch request the hub took and never answered is `timeout` naming
/// `pane_store.watch_next`, within the watch's own bound (`WATCH_WAIT` + 5 s). Whether the
/// request is `watch`'s own first one or a later `next()`'s, the answer is the same.
#[test]
fn a_watch_request_the_hub_never_answers_is_timeout_naming_watch_next() {
    let hub = FakeHub::start(vec![Answer::Hang(Duration::from_secs(16))]);
    let (error, took) = hub.with(|| {
        let started = Instant::now();
        let error = match HubPaneStore::new().watch(Cursor(0)) {
            Err(error) => error,
            Ok(mut watch) => match watch.next() {
                Some(Err(error)) => error,
                other => panic!("want one error; got {other:?}"),
            },
        };
        (error, started.elapsed())
    });
    assert_eq!(error.code(), "timeout", "{error}");
    assert!(error.to_string().contains("pane_store.watch_next"), "{error}");
    assert!(took < Duration::from_secs(13), "bounded by WATCH_WAIT + 5 s: took {took:?}");
}

/// W-9 and AC 3: a JSON-RPC error never reached a handler, so nothing was written:
/// `unavailable`, naming the hub, for a read and for a write.
#[test]
fn a_json_rpc_error_is_unavailable_naming_the_hub_for_reads_and_writes() {
    for outcome in against(&Answer::Error, reads()).into_iter().chain(against(&Answer::Error, writes())) {
        assert_eq!(outcome.code, "unavailable", "{}: {}", outcome.method, outcome.text);
        outcome.assert_names_the_hub();
    }
}

/// W-9: a reply whose `data` does not decode as the method's is `unavailable` naming the
/// method, never `usage` (which is for the caller's own input).
#[test]
fn data_that_does_not_decode_is_unavailable_naming_the_method() {
    let bad = Answer::Result(json!({ "ok": true, "data": 42 }));
    for Outcome { method, code, text, .. } in against(&bad, reads()) {
        assert_eq!(code, "unavailable", "{method}: {text}");
        assert!(text.contains(method), "{method}: the message names it: {text}");
    }
}

/// W-9 and D3: with no hub at all, every read and every write is `unavailable` naming the
/// missing hub and its state dir. A write is never `timeout` here: the request never left,
/// so it cannot have landed.
#[test]
fn every_read_and_write_with_no_hub_is_unavailable_naming_the_hub() {
    for (method, call) in reads().into_iter().chain(writes()) {
        let state = StateDir::new();
        let (code, text) = with_state_dir(state.path(), call);
        assert_eq!(code, "unavailable", "{method}: {text}");
        let want = format!("{NO_HUB} at {}", state.path().display());
        assert!(text.contains(&want), "{method}: the message says {want:?}: {text}");
    }
}

/// D4: a store or transport error yields one `Err` and ends the stream.
#[test]
fn a_watch_error_yields_one_err_and_ends_the_stream() {
    let idle = json!({ "ok": true, "data": { "events": [], "cursor": 3 } });
    let hub = FakeHub::start(vec![Answer::Result(idle), Answer::Error]);
    let (first, second, third) = hub.with(|| {
        let mut watch = HubProfileStore::new().watch(Cursor(0)).expect("watch");
        (watch.next(), watch.next(), watch.next())
    });
    assert!(matches!(first, Some(Ok(None))), "idle: {first:?}");
    assert!(matches!(second, Some(Err(_))), "one error: {second:?}");
    assert!(third.is_none(), "then the stream ends: {third:?}");
    assert_eq!(hub.requests().len(), 2, "no request after the error");
}

/// D4 (audit): `watch(since)` makes the first request itself, eagerly, and keeps its batch
/// for the first `next()`; each later `next()` sends one request, resuming from the cursor of
/// the reply before it.
#[test]
fn watch_makes_its_first_request_when_called_and_keeps_the_batch_for_the_first_next() {
    let idle = json!({ "ok": true, "data": { "events": [], "cursor": 7 } });
    let hub = FakeHub::start(vec![Answer::Result(idle.clone()), Answer::Result(idle)]);
    let (at_watch, first, at_first, second) = hub.with(|| {
        let mut watch = HubPaneStore::new().watch(Cursor(0)).expect("watch");
        let at_watch = hub.requests().len();
        let first = watch.next();
        let at_first = hub.requests().len();
        (at_watch, first, at_first, watch.next())
    });
    assert_eq!(at_watch, 1, "watch() itself sent the first request");
    assert!(matches!(first, Some(Ok(None))), "the kept batch, idle: {first:?}");
    assert_eq!(at_first, 1, "the first next() sent nothing: it yields the kept batch");
    assert!(matches!(second, Some(Ok(None))), "idle again: {second:?}");
    let requests = hub.requests();
    assert_eq!(requests.len(), 2, "the second next() sent one request: {requests:?}");
    assert_eq!(requests[0].1["since"], 0, "{requests:?}");
    assert_eq!(requests[1].1["since"], 7, "resumed from the reply's cursor: {requests:?}");
}

/// D4 (audit): a cursor ahead of the head is the hub's `usage`, passed through, and it is
/// refused by `watch()` itself, before any item: `pane watch`'s `open()` promises that every
/// refusal comes before the first line. The same for both clients.
#[test]
fn a_cursor_ahead_of_the_head_is_refused_by_watch_itself() {
    let refusal = json!({
        "ok": false,
        "error": { "code": "usage", "message": "the cursor 99 is ahead of the head 0" }
    });
    let pane = FakeHub::start(vec![Answer::Result(refusal.clone())]);
    let pane_answer = pane.with(|| HubPaneStore::new().watch(Cursor(99)).map(drop));
    let profile = FakeHub::start(vec![Answer::Result(refusal)]);
    let profile_answer = profile.with(|| HubProfileStore::new().watch(Cursor(99)).map(drop));
    for (store, answer, hub) in [("pane", pane_answer, &pane), ("profile", profile_answer, &profile)] {
        let error = answer.expect_err("watch() refuses the cursor itself");
        assert_eq!(error.code(), "usage", "{store}: {error}");
        let requests = hub.requests();
        assert_eq!(requests.len(), 1, "{store}: {requests:?}");
        assert_eq!(requests[0].1["since"], 99, "{store}: {requests:?}");
    }
}

/// D4 (audit) with D3: with no hub, `watch()` itself is `unavailable` naming the missing hub,
/// not a stream whose first item is the error.
#[test]
fn watch_with_no_hub_is_unavailable_from_watch_itself() {
    let state = StateDir::new();
    let answer = with_state_dir(state.path(), || HubPaneStore::new().watch(Cursor(0)).map(drop));
    let error = answer.expect_err("watch() fails at once");
    assert_eq!(error.code(), "unavailable", "{error}");
    assert!(error.to_string().contains(NO_HUB), "{error}");
}

/// D4: a watch request's own timeout is the hub's window plus a margin (`WATCH_WAIT` + 5 s),
/// so a long-poll answered after more than the control client's default 5 s is still an
/// idle batch, not a `timeout`.
#[test]
fn a_watch_poll_waits_past_the_default_control_timeout() {
    let idle = json!({ "ok": true, "data": { "events": [], "cursor": 0 } });
    let hub = FakeHub::start(vec![Answer::After(Duration::from_secs(6), idle)]);
    let first = hub.with(|| {
        let mut watch = HubPaneStore::new().watch(Cursor(0)).expect("watch");
        watch.next()
    });
    assert!(matches!(first, Some(Ok(None))), "idle, not a timeout: {first:?}");
}

/// D2: `connect` makes one `control/status` exchange; a hub that refuses it, closes without
/// a reply, or does not answer within the exchange's own 5 s, is `unavailable`.
#[test]
fn connect_is_unavailable_when_the_status_exchange_fails() {
    for answer in [Answer::Error, Answer::Close, Answer::Hang(Duration::from_secs(8))] {
        let hub = FakeHub::start(vec![answer.clone()]);
        let outcome = hub.with(|| Wiring::connect().err().map(|e| e.code().to_owned()));
        assert_eq!(outcome.as_deref(), Some("unavailable"), "{answer:?}");
        assert_eq!(
            hub.requests().first().map(|(m, _)| m.as_str()),
            Some("control/status"),
            "{answer:?}: connect asks the hub's status"
        );
    }
}
