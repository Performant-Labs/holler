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
//! | no reply within the bound (`Io`, timed out) | `timeout` | `timeout` |
//! | a `data` that does not decode | `unavailable`, naming the method | |

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
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

/// Every write of both clients, each against a fresh fake hub with one connection answered
/// `answer`, with the method each must send.
fn each_write(answer: &Answer) -> Vec<(&'static str, (String, String))> {
    let run = |method: &'static str, call: &dyn Fn() -> (String, String)| {
        let hub = FakeHub::start(vec![answer.clone()]);
        let outcome = hub.with(call);
        assert_eq!(
            hub.requests().first().map(|(m, _)| m.as_str()),
            Some(method),
            "the client sent {method}"
        );
        (method, outcome)
    };
    vec![
        run("pane/cas_put", &|| failure(HubPaneStore::new().cas_put(&pane(), 0))),
        run("pane/delete", &|| failure(HubPaneStore::new().delete(&pname(), 1))),
        run("profile/cas_put", &|| {
            failure(HubProfileStore::new().cas_put(&profile(), 0, &actor()))
        }),
        run("profile/delete", &|| {
            failure(HubProfileStore::new().delete(&prname("hlr649"), 1, &actor()))
        }),
        run("profile/rename", &|| {
            failure(HubProfileStore::new().rename(&prname("hlr649"), &prname("hlr649b"), 1, &actor()))
        }),
    ]
}

/// Every read of both clients, the same way.
fn each_read(answer: &Answer) -> Vec<(&'static str, (String, String))> {
    let run = |method: &'static str, call: &dyn Fn() -> (String, String)| {
        let hub = FakeHub::start(vec![answer.clone()]);
        let outcome = hub.with(call);
        assert_eq!(
            hub.requests().first().map(|(m, _)| m.as_str()),
            Some(method),
            "the client sent {method}"
        );
        (method, outcome)
    };
    vec![
        run("pane/get", &|| failure(HubPaneStore::new().get(&pname()))),
        run("pane/list", &|| failure(HubPaneStore::new().list())),
        run("profile/get", &|| failure(HubProfileStore::new().get(&prname("hlr649")))),
        run("profile/list", &|| failure(HubProfileStore::new().list())),
        run("profile/log", &|| failure(HubProfileStore::new().log(&prname("hlr649")))),
    ]
}

/// AC 3 and D3: a write whose reply is lost after it was sent is `timeout`, naming the
/// write, because it may have landed.
#[test]
fn a_write_whose_reply_is_lost_is_timeout() {
    for (method, (code, text)) in each_write(&Answer::Close) {
        assert_eq!(code, "timeout", "{method}: {text}");
        let op = method.replace('/', "_store.");
        assert!(text.contains(&op), "{method}: the timeout names {op}: {text}");
    }
}

/// W-9: a read whose reply is lost is `unavailable` (nothing was written).
#[test]
fn a_read_whose_reply_is_lost_is_unavailable() {
    for (method, (code, text)) in each_read(&Answer::Close) {
        assert_eq!(code, "unavailable", "{method}: {text}");
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
    assert!(took < Duration::from_secs(13), "bounded by 10 s: took {took:?}");
}

/// W-9: a JSON-RPC error never reached a handler, so nothing was written: `unavailable`,
/// for a read and for a write.
#[test]
fn a_json_rpc_error_is_unavailable_for_reads_and_writes() {
    for (method, (code, text)) in each_read(&Answer::Error)
        .into_iter()
        .chain(each_write(&Answer::Error))
    {
        assert_eq!(code, "unavailable", "{method}: {text}");
    }
}

/// W-9: a reply whose `data` does not decode as the method's is `unavailable` naming the
/// method, never `usage` (which is for the caller's own input).
#[test]
fn data_that_does_not_decode_is_unavailable_naming_the_method() {
    let bad = Answer::Result(json!({ "ok": true, "data": 42 }));
    for (method, (code, text)) in each_read(&bad) {
        assert_eq!(code, "unavailable", "{method}: {text}");
        assert!(text.contains(method), "{method}: the message names it: {text}");
    }
}

/// W-9: with no hub at all, a client call is `unavailable` naming the missing hub.
#[test]
fn a_client_call_with_no_hub_is_unavailable_naming_the_hub() {
    let state = StateDir::new();
    let (code, text) = with_state_dir(state.path(), || failure(HubPaneStore::new().list()));
    assert_eq!(code, "unavailable", "{text}");
    assert!(text.contains("no live holler hub reachable"), "{text}");
}

/// D4: an empty batch is the idle item `Ok(None)`, and the next request resumes from the
/// reply's cursor.
#[test]
fn an_empty_watch_batch_is_idle_and_the_next_poll_resumes_from_its_cursor() {
    let idle = json!({ "ok": true, "data": { "events": [], "cursor": 7 } });
    let hub = FakeHub::start(vec![Answer::Result(idle.clone()), Answer::Result(idle)]);
    let (first, second) = hub.with(|| {
        let mut watch = HubPaneStore::new().watch(Cursor(0)).expect("watch");
        (watch.next(), watch.next())
    });
    assert!(matches!(first, Some(Ok(None))), "idle: {first:?}");
    assert!(matches!(second, Some(Ok(None))), "idle again: {second:?}");
    let requests = hub.requests();
    assert_eq!(requests.len(), 2, "one request per next(): {requests:?}");
    assert_eq!(requests[0].0, "pane/watch");
    assert_eq!(requests[1].1["since"], 7, "resumed from the reply's cursor: {requests:?}");
}

/// D4: a store or transport error yields one `Err` and ends the stream.
#[test]
fn a_watch_error_yields_one_err_and_ends_the_stream() {
    let hub = FakeHub::start(vec![Answer::Error]);
    let (first, second) = hub.with(|| {
        let mut watch = HubProfileStore::new().watch(Cursor(0)).expect("watch");
        (watch.next(), watch.next())
    });
    assert!(matches!(first, Some(Err(_))), "one error: {first:?}");
    assert!(second.is_none(), "then the stream ends: {second:?}");
    assert_eq!(hub.requests().len(), 1, "no request after the error");
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

/// D2: `connect` makes one `control/status` exchange; a hub that refuses it, or closes
/// without a reply, is `unavailable`.
#[test]
fn connect_is_unavailable_when_the_status_exchange_fails() {
    for answer in [Answer::Error, Answer::Close] {
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
