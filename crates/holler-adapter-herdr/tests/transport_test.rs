#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #640
//! The socket transport (#640 part 2, AC 1-12a): one request per connection, one
//! deadline that bounds the whole exchange, and every fault as `timeout` or
//! `unavailable`. Each test binds its own `UnixListener` at `<tempdir>/h.sock` and
//! plays the server itself; no test reaches a real Herdr.

use std::fs;
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::panic;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex, Once};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use holler_adapter_herdr::protocol::Request;
use holler_adapter_herdr::transport::{Transport, UnixSocketTransport, MAX_REPLY_BYTES};
use holler_pane::{PaneError, PaneId};
use tempfile::TempDir;

/// The slack every timing bound allows past its deadline, for slow CI runners.
const SLACK: Duration = Duration::from_secs(2);
/// A deadline far enough off that no test meets it unless the transport hangs.
const GENEROUS: Duration = Duration::from_secs(5);
/// The text typed in AC 12, which no error may echo.
const SECRET: &str = "typed-secret-text";
/// The name of every server thread these tests spawn.
const SERVER_THREAD: &str = "test-server";
/// This file's tests: a panic on a thread of another name is a transport worker's.
const TESTS: [&str; 13] = [
    "exchange_writes_the_request_line_and_returns_the_reply_line",
    "each_request_opens_its_own_connection",
    "a_missing_or_non_socket_path_is_unavailable_naming_it",
    "a_socket_path_too_long_for_the_os_is_unavailable",
    "a_silent_server_is_timeout_by_the_deadline",
    "a_dripping_server_is_timeout_by_the_deadline",
    "a_server_that_closes_without_replying_is_unavailable",
    "a_reply_over_the_limit_is_unavailable",
    "a_reply_ended_by_eof_without_a_newline_is_returned",
    "a_non_utf8_reply_is_unavailable",
    "a_passed_deadline_is_timeout_without_connecting",
    "no_transport_error_echoes_typed_text",
    "one_wire_condition_gives_one_answer_at_the_deadline",
];

/// The names of the threads, other than a test's or a server's, that panicked.
static WORKER_PANICS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static PANIC_HOOK: Once = Once::new();

// --- helpers ---

/// A fresh listener at the short path `<tempdir>/h.sock`.
fn socket() -> (TempDir, PathBuf, UnixListener) {
    record_worker_panics();
    let dir = tempfile::tempdir().expect("a scratch directory");
    let path = dir.path().join("h.sock");
    let listener = UnixListener::bind(&path).expect("bind the test socket");
    (dir, path, listener)
}

/// Record every panic on a thread that is neither a test's nor a server's, then let
/// the default hook print it as usual.
fn record_worker_panics() {
    PANIC_HOOK.call_once(|| {
        let previous = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            let name = thread::current().name().map(str::to_owned);
            let known = name
                .as_deref()
                .is_some_and(|n| n == "main" || n == SERVER_THREAD || TESTS.contains(&n));
            if !known {
                let name = name.unwrap_or_else(|| "<unnamed>".to_owned());
                WORKER_PANICS.lock().unwrap().push(name);
            }
            previous(info);
        }));
    });
}

/// Accept one connection on `listener` and hand it to `serve`, on a named thread.
fn serve_one<T: Send + 'static>(
    listener: UnixListener,
    serve: impl FnOnce(UnixStream) -> T + Send + 'static,
) -> JoinHandle<T> {
    thread::Builder::new()
        .name(SERVER_THREAD.to_owned())
        .spawn(move || {
            let (stream, _) = listener.accept().expect("accept");
            serve(stream)
        })
        .expect("spawn the server")
}

/// Accept every connection and keep it open, writing nothing, for as long as the
/// returned list lives in the test.
fn hold_every_connection(listener: UnixListener) -> Arc<Mutex<Vec<UnixStream>>> {
    let held = Arc::new(Mutex::new(Vec::new()));
    let keep = Arc::clone(&held);
    thread::Builder::new()
        .name(SERVER_THREAD.to_owned())
        .spawn(move || {
            for stream in listener.incoming().flatten() {
                keep.lock().unwrap().push(stream);
            }
        })
        .expect("spawn the server");
    held
}

/// The request line the client sent, newline included.
fn request_line(reader: &mut BufReader<&UnixStream>) -> String {
    reader
        .get_ref()
        .set_read_timeout(Some(GENEROUS))
        .expect("a read timeout");
    let mut line = String::new();
    reader.read_line(&mut line).expect("the request line");
    line
}

fn exchange(path: &Path, request: &Request, deadline: Instant) -> Result<String, PaneError> {
    UnixSocketTransport::new(path).exchange(request, deadline)
}

fn timeout(op: &str) -> PaneError {
    PaneError::Timeout { op: op.to_owned() }
}

/// The `what` of an `Unavailable`; anything else fails the test.
fn unavailable_what(result: Result<String, PaneError>) -> String {
    match result {
        Err(PaneError::Unavailable { what }) => what,
        other => panic!("expected Unavailable, got {other:?}"),
    }
}

fn send_secret() -> Request {
    Request::SendText {
        pane: PaneId::new("w1:p1"),
        text: SECRET.to_owned(),
    }
}

// --- AC 1-12a ---

#[test]
fn exchange_writes_the_request_line_and_returns_the_reply_line() {
    let (_dir, path, listener) = socket();
    let reply = r#"{"id":"holler:ping","result":{"type":"pong","version":"v","protocol":22}}"#;
    let server = serve_one(listener, move |stream| {
        let line = request_line(&mut BufReader::new(&stream));
        (&stream)
            .write_all(format!("{reply}\n").as_bytes())
            .unwrap();
        line
    });

    let got = exchange(&path, &Request::Ping, Instant::now() + GENEROUS).expect("exchange");

    assert_eq!(got, reply, "the reply line as text, without its newline");
    assert_eq!(server.join().unwrap(), Request::Ping.to_line());
}

#[test]
fn each_request_opens_its_own_connection() {
    let (_dir, path, listener) = socket();
    let server = thread::Builder::new()
        .name(SERVER_THREAD.to_owned())
        .spawn(move || {
            (0..2)
                .map(|_| {
                    let (stream, _) = listener.accept().expect("accept");
                    let mut reader = BufReader::new(&stream);
                    let line = request_line(&mut reader);
                    (&stream)
                        .write_all(b"{\"id\":\"holler:ping\",\"result\":{}}\n")
                        .unwrap();
                    let mut rest = Vec::new();
                    let closed = reader.read_to_end(&mut rest).map(|_| rest);
                    (line, closed.map_err(|e| e.kind()))
                })
                .collect::<Vec<_>>()
        })
        .unwrap();

    for _ in 0..2 {
        exchange(&path, &Request::Ping, Instant::now() + GENEROUS).expect("exchange");
    }

    let connections = server.join().unwrap();
    assert_eq!(connections.len(), 2);
    for (line, rest) in connections {
        assert_eq!(line, Request::Ping.to_line());
        assert_eq!(rest, Ok(Vec::new()), "one line, then the client closes");
    }
}

#[test]
fn a_missing_or_non_socket_path_is_unavailable_naming_it() {
    let (dir, _path, _listener) = socket();
    let missing = dir.path().join("missing.sock");
    let regular = dir.path().join("regular-file");
    fs::write(&regular, "not a socket").unwrap();

    for path in [missing, regular] {
        let what = unavailable_what(exchange(&path, &Request::Ping, Instant::now() + GENEROUS));
        let shown = path.display().to_string();
        assert!(what.contains(&shown), "{what:?} should name {shown:?}");
    }
}

#[test]
fn a_socket_path_too_long_for_the_os_is_unavailable() {
    let (dir, _path, _listener) = socket();
    let base = dir.path().as_os_str().len();
    let path = dir.path().join("s".repeat(200 - base - 1));
    assert_eq!(path.as_os_str().len(), 200);

    unavailable_what(exchange(&path, &Request::Ping, Instant::now() + GENEROUS));
}

#[test]
fn a_silent_server_is_timeout_by_the_deadline() {
    let (_dir, path, listener) = socket();
    let _held = hold_every_connection(listener);
    let budget = Duration::from_millis(300);

    let start = Instant::now();
    let result = exchange(&path, &Request::Ping, start + budget);

    assert_eq!(result, Err(timeout("herdr.ping")));
    assert!(
        start.elapsed() < budget + SLACK,
        "took {:?}",
        start.elapsed()
    );
}

#[test]
fn a_dripping_server_is_timeout_by_the_deadline() {
    let (_dir, path, listener) = socket();
    serve_one(listener, |mut stream| {
        // The pause simulates a slow peer; it waits for nothing.
        for _ in 0..200 {
            if stream.write_all(b"x").is_err() {
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
    });
    let budget = Duration::from_millis(300);

    let start = Instant::now();
    let result = exchange(&path, &Request::Ping, start + budget);

    assert_eq!(result, Err(timeout("herdr.ping")));
    assert!(
        start.elapsed() < budget + SLACK,
        "took {:?}",
        start.elapsed()
    );
}

#[test]
fn a_server_that_closes_without_replying_is_unavailable() {
    let (_dir, path, listener) = socket();
    serve_one(listener, |stream| {
        request_line(&mut BufReader::new(&stream));
    });

    unavailable_what(exchange(&path, &Request::Ping, Instant::now() + GENEROUS));
}

#[test]
fn a_reply_over_the_limit_is_unavailable() {
    let (_dir, path, listener) = socket();
    let (done, wait) = mpsc::channel::<()>();
    serve_one(listener, move |stream| {
        request_line(&mut BufReader::new(&stream));
        let _ = (&stream).write_all(&vec![b'x'; MAX_REPLY_BYTES + 1]);
        // Hold the connection open, so only the limit can end the read.
        let _ = wait.recv();
    });

    let what = unavailable_what(exchange(
        &path,
        &Request::Ping,
        Instant::now() + Duration::from_secs(10),
    ));
    drop(done);

    let limit = MAX_REPLY_BYTES.to_string();
    assert!(
        what.contains(&limit),
        "{what:?} should name the limit {limit}"
    );
}

#[test]
fn a_reply_ended_by_eof_without_a_newline_is_returned() {
    let (_dir, path, listener) = socket();
    let reply = r#"{"id":"holler:ping","result":{}}"#;
    serve_one(listener, move |stream| {
        request_line(&mut BufReader::new(&stream));
        (&stream).write_all(reply.as_bytes()).unwrap();
    });

    let got = exchange(&path, &Request::Ping, Instant::now() + GENEROUS);

    assert_eq!(got, Ok(reply.to_owned()));
}

#[test]
fn a_non_utf8_reply_is_unavailable() {
    let (_dir, path, listener) = socket();
    serve_one(listener, |stream| {
        request_line(&mut BufReader::new(&stream));
        (&stream).write_all(&[0xff, 0xfe, b'\n']).unwrap();
    });

    unavailable_what(exchange(&path, &Request::Ping, Instant::now() + GENEROUS));
}

#[test]
fn a_passed_deadline_is_timeout_without_connecting() {
    let (_dir, path, listener) = socket();
    listener.set_nonblocking(true).unwrap();

    let result = exchange(&path, &Request::Ping, Instant::now());

    assert_eq!(result, Err(timeout("herdr.ping")));
    let accepted = listener.accept().map(|_| ()).map_err(|e| e.kind());
    assert_eq!(
        accepted,
        Err(ErrorKind::WouldBlock),
        "no connection was made"
    );
}

#[test]
fn no_transport_error_echoes_typed_text() {
    let (dir, silent_path, silent) = socket();
    let _held = hold_every_connection(silent);
    let closing_path = dir.path().join("closing.sock");
    let closing = UnixListener::bind(&closing_path).unwrap();
    serve_one(closing, |stream| {
        request_line(&mut BufReader::new(&stream));
    });
    let missing_path = dir.path().join("missing.sock");

    let silent_error = exchange(
        &silent_path,
        &send_secret(),
        Instant::now() + Duration::from_millis(300),
    );
    let closing_error = exchange(&closing_path, &send_secret(), Instant::now() + GENEROUS);
    let missing_error = exchange(&missing_path, &send_secret(), Instant::now() + GENEROUS);

    assert_eq!(silent_error, Err(timeout("herdr.pane.send_text")));
    for result in [silent_error, closing_error.clone(), missing_error.clone()] {
        let error = result.expect_err("an error");
        for shown in [error.to_string(), format!("{error:?}")] {
            assert!(!shown.contains(SECRET), "{shown:?} echoes the typed text");
        }
    }
    unavailable_what(closing_error);
    unavailable_what(missing_error);
}

#[test]
fn one_wire_condition_gives_one_answer_at_the_deadline() {
    // The listener accepts nothing while the exchanges run: each connection waits in its
    // backlog, which the client cannot tell from a server that accepted and is silent.
    let (_dir, path, listener) = socket();
    let budget = Duration::from_millis(50);

    for round in 0..20 {
        let start = Instant::now();
        let result = exchange(&path, &Request::Ping, start + budget);
        assert_eq!(result, Err(timeout("herdr.ping")), "round {round}");
        assert!(
            start.elapsed() < budget + SLACK,
            "round {round} took {:?}",
            start.elapsed()
        );
    }

    // Every worker drops its socket within its socket timeouts: each kept connection
    // reaches EOF, after at most the one ping line, within the slack.
    listener.set_nonblocking(true).unwrap();
    let mut kept = Vec::new();
    loop {
        match listener.accept() {
            Ok((stream, _)) => kept.push(stream),
            Err(e) if e.kind() == ErrorKind::WouldBlock => break,
            Err(e) => panic!("accept: {e}"),
        }
    }
    assert!(
        kept.len() <= 20,
        "{} connections for 20 exchanges",
        kept.len()
    );
    for mut stream in kept {
        // macOS refuses both calls with EINVAL (`InvalidInput`) once the peer has
        // closed the socket (seen on CI's macOS runner): that is the worker having
        // dropped its socket, which is what this test checks, and the read below then
        // ends at once.
        for set in [
            stream.set_nonblocking(false),
            stream.set_read_timeout(Some(SLACK)),
        ] {
            if let Err(e) = set {
                assert_eq!(e.kind(), ErrorKind::InvalidInput, "{e}");
            }
        }
        let mut received = Vec::new();
        let read = stream.read_to_end(&mut received).map_err(|e| e.kind());
        assert!(
            read.is_ok(),
            "a worker kept its socket open past {SLACK:?}: {read:?}"
        );
        let ping = Request::Ping.to_line();
        assert!(
            ping.as_bytes().starts_with(&received),
            "received {received:?}"
        );
    }
    let panics = WORKER_PANICS.lock().unwrap().clone();
    assert!(panics.is_empty(), "a transport worker panicked: {panics:?}");
}
