#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #640
//! What `HerdrAdapter`'s errors say (#640 part 3, AC 16-18), in process through the wire
//! fake's `Tap`:
//!
//! - a `timeout` names the port method that ran out (`herdr.ensure_pane`), never the wire
//!   method under it, and `connect` names itself (`herdr.connect`);
//! - every other error from an exchange passes through unchanged;
//! - text that Herdr sent is quoted cut to 64 characters, on one line.

mod wire_herdr;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use holler_adapter_herdr::adapter::{HerdrAdapter, HerdrConfig};
use holler_adapter_herdr::plan::Extent;
use holler_pane::{GridPos, HerdrPort, HerdrSpec, Key, PaneError, PaneId};
use holler_pane_testkit::fault::PortOp;
use holler_pane_testkit::herdr::HerdrOp;
use serde_json::Value;
use wire_herdr::{Tap, Tapped, WireHerdr};

const SESSION: &str = "scratch";
/// An absolute socket path that the in-process tests never open.
const UNUSED_SOCKET: &str = "/unused/h.sock";
const WORKSPACE: &str = "w";
/// The one `op` that names no port method: `connect` and `connect_with` (Decision 9).
const CONNECT_OP: &str = "herdr.connect";

// --- helpers ---

fn spec(workspace: &str, row: u16, col: u16) -> HerdrSpec {
    HerdrSpec {
        session: SESSION.to_owned(),
        workspace: workspace.to_owned(),
        grid: GridPos { row, col },
    }
}

fn config(label: &str, rows: u16, cols: u16) -> HerdrConfig {
    HerdrConfig::new(SESSION, UNUSED_SOCKET).with_workspace(label, Extent { rows, cols })
}

/// The `method` of a request line.
fn method_of(line: &str) -> String {
    let request: Value = serde_json::from_str(line).unwrap();
    request["method"].as_str().unwrap_or("").to_owned()
}

/// What the trap fails: the `nth` request (counted from 1, from when it is armed) whose
/// method is `method`.
struct Trap {
    armed: bool,
    method: &'static str,
    nth: usize,
    seen: usize,
    fired: bool,
}

/// The error a trap answers its exchange with, given the wire method.
type Inject = fn(&str) -> PaneError;

/// A timeout as the socket transport reports it: its `op` names the wire method.
fn wire_timeout(method: &str) -> PaneError {
    PaneError::Timeout {
        op: format!("herdr.{method}"),
    }
}

fn injected_unavailable(_method: &str) -> PaneError {
    PaneError::Unavailable {
        what: "injected-unavailable".into(),
    }
}

/// A `Tap` in front of `fake` that fails the trap's exchange with `inject` and forwards
/// every other one.
fn trapped(fake: &Arc<WireHerdr>, trap: &Arc<Mutex<Trap>>, inject: Inject) -> Tap {
    let trap = Arc::clone(trap);
    Tap::new(Arc::clone(fake), move |_, line, _| {
        let mut trap = trap.lock().unwrap();
        if trap.armed && method_of(&line) == trap.method {
            trap.seen += 1;
            if trap.seen == trap.nth {
                trap.armed = false;
                trap.fired = true;
                return Tapped::Fail(inject(trap.method));
            }
        }
        Tapped::Forward(line)
    })
}

/// The port calls of AC 16's table, each with the state it starts from.
#[derive(Debug, Clone, Copy)]
enum Call {
    /// `connect_with` itself.
    Connect,
    /// `ensure_pane(r1c1)` with the workspace missing.
    EnsureFirst,
    /// `ensure_pane(r2c1)` with `r1c1` placed.
    EnsureSecond,
    SendText,
    SendKeys,
    Read,
    Close,
    /// `snapshot` of one workspace with a tab.
    Snapshot,
    Version,
}

impl Call {
    /// The `op` a timeout of this call must name: `HerdrOp`'s string, or `herdr.connect`.
    fn op(self) -> &'static str {
        let op = match self {
            Call::Connect => return CONNECT_OP,
            Call::EnsureFirst | Call::EnsureSecond => HerdrOp::EnsurePane,
            Call::SendText => HerdrOp::SendText,
            Call::SendKeys => HerdrOp::SendKeys,
            Call::Read => HerdrOp::Read,
            Call::Close => HerdrOp::Close,
            Call::Snapshot => HerdrOp::Snapshot,
            Call::Version => HerdrOp::Version,
        };
        op.as_str()
    }
}

/// AC 16's table: each call, and each wire exchange of it (method, which occurrence)
/// that is made to fail.
const TABLE: [(Call, &str, usize); 14] = [
    (Call::Connect, "ping", 1),
    (Call::EnsureFirst, "session.snapshot", 1),
    (Call::EnsureFirst, "workspace.create", 1),
    (Call::EnsureFirst, "layout.export", 1),
    (Call::EnsureSecond, "layout.export", 1),
    (Call::EnsureSecond, "pane.split", 1),
    (Call::EnsureSecond, "layout.export", 2),
    (Call::SendText, "pane.send_text", 1),
    (Call::SendKeys, "pane.send_keys", 1),
    (Call::Read, "pane.read", 1),
    (Call::Close, "pane.close", 1),
    (Call::Snapshot, "session.snapshot", 1),
    (Call::Snapshot, "layout.export", 1),
    (Call::Version, "ping", 1),
];

/// Run `call` from its starting state, with the `nth` `method` exchange failed by
/// `inject` once the state is set up: the call's result, and whether the trap fired.
fn run(
    call: Call,
    method: &'static str,
    nth: usize,
    inject: Inject,
) -> (Result<(), PaneError>, bool) {
    let fake = Arc::new(WireHerdr::new());
    let trap = Arc::new(Mutex::new(Trap {
        armed: false,
        method,
        nth,
        seen: 0,
        fired: false,
    }));
    let arm = || trap.lock().unwrap().armed = true;
    let tap = trapped(&fake, &trap, inject);
    let config = config(WORKSPACE, 2, 1);
    if let Call::Connect = call {
        arm();
        let result = HerdrAdapter::connect_with(config, tap).map(drop);
        let fired = trap.lock().unwrap().fired;
        return (result, fired);
    }
    let pane = match call {
        Call::SendText | Call::SendKeys | Call::Read | Call::Close | Call::Snapshot => {
            fake.create_workspace(WORKSPACE)
        }
        _ => PaneId::new("w1:p1"),
    };
    let port = HerdrAdapter::connect_with(config, tap).expect("connect");
    if let Call::EnsureSecond = call {
        port.ensure_pane(&spec(WORKSPACE, 1, 1))
            .expect("ensure r1c1");
    }
    arm();
    let result = match call {
        Call::Connect => panic!("connect is run above, before the port exists"),
        Call::EnsureFirst => port.ensure_pane(&spec(WORKSPACE, 1, 1)).map(drop),
        Call::EnsureSecond => port.ensure_pane(&spec(WORKSPACE, 2, 1)).map(drop),
        Call::SendText => port.send_text(&pane, "x"),
        Call::SendKeys => port.send_keys(&pane, &[Key::new("enter")]),
        Call::Read => port.read(&pane, 1).map(drop),
        Call::Close => port.close(&pane),
        Call::Snapshot => port.snapshot().map(drop),
        Call::Version => port.version().map(drop),
    };
    let fired = trap.lock().unwrap().fired;
    (result, fired)
}

// --- AC 16 ---

#[test]
fn a_timeout_names_the_port_method() {
    let mut wrong = Vec::new();
    for (call, method, nth) in TABLE {
        let (result, fired) = run(call, method, nth, wire_timeout);
        assert!(fired, "{call:?}: the {nth}. {method} was never sent");
        let expected = Err(PaneError::Timeout {
            op: call.op().to_owned(),
        });
        if result != expected {
            wrong.push(format!(
                "{call:?} with {method} #{nth} timed out: got {result:?}, want {expected:?}"
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

// --- AC 17 ---

#[test]
fn every_other_error_passes_through_unchanged() {
    for (call, method, nth) in TABLE {
        let (result, fired) = run(call, method, nth, injected_unavailable);
        assert!(fired, "{call:?}: the {nth}. {method} was never sent");
        assert_eq!(
            result,
            Err(injected_unavailable(method)),
            "{call:?} with {method} #{nth} failed"
        );
    }
}

// --- AC 18 ---

/// An id or label Herdr sends that is longer than a message may quote.
fn long() -> String {
    "x".repeat(64) + "-TAIL-NOT-QUOTED"
}

/// `what` quotes `long()` cut to 64 characters, and stays on one line.
fn assert_cut_to_64(result: Result<impl std::fmt::Debug, PaneError>) {
    let what = match result {
        Err(PaneError::Unavailable { what }) => what,
        other => panic!("expected Unavailable, got {other:?}"),
    };
    let cut = format!("{:?}...", "x".repeat(64));
    assert!(what.contains(&cut), "{what:?} does not quote {cut}");
    assert!(!what.contains("TAIL-NOT-QUOTED"), "{what:?}");
    assert!(!what.contains('\n'), "{what:?}");
}

/// `reply` with every quoted `from` replaced by the quoted `to`.
fn renamed(reply: &str, from: &str, to: &str) -> String {
    reply.replace(&format!("\"{from}\""), &format!("\"{to}\""))
}

#[test]
fn a_misplaced_new_panes_id_is_cut_to_64() {
    let fake = Arc::new(WireHerdr::new());
    let tap = Tap::new(Arc::clone(&fake), |fake, line, _| {
        let line = if method_of(&line) == "pane.split" {
            line.replace(r#""direction":"down""#, r#""direction":"right""#)
        } else {
            line
        };
        // Herdr's id for the new pane is LONG, in the split's reply and in the tree.
        Tapped::Reply(renamed(&fake.answer(&line), "w1:p2", &long()))
    });
    let port = HerdrAdapter::connect_with(config(WORKSPACE, 2, 2), tap).expect("connect");
    port.ensure_pane(&spec(WORKSPACE, 1, 1))
        .expect("ensure r1c1");

    assert_cut_to_64(port.ensure_pane(&spec(WORKSPACE, 2, 1)));
}

#[test]
fn a_new_pane_with_no_cell_has_its_id_cut_to_64() {
    let fake = Arc::new(WireHerdr::new());
    let tap = Tap::new(Arc::clone(&fake), |fake, line, _| {
        if method_of(&line) == "pane.split" {
            return Tapped::Reply(renamed(&fake.answer(&line), "w1:p2", &long()));
        }
        Tapped::Forward(line)
    });
    let port = HerdrAdapter::connect_with(config(WORKSPACE, 2, 1), tap).expect("connect");
    port.ensure_pane(&spec(WORKSPACE, 1, 1))
        .expect("ensure r1c1");

    assert_cut_to_64(port.ensure_pane(&spec(WORKSPACE, 2, 1)));
}

#[test]
fn a_vanished_split_targets_id_is_cut_to_64() {
    let fake = Arc::new(WireHerdr::new());
    fake.create_workspace(WORKSPACE);
    // Herdr's tree names the pane at r1c1 LONG; the fake holds no such pane, so it
    // answers the split of it `pane_not_found`.
    let tap = Tap::new(Arc::clone(&fake), |fake, line, _| {
        if method_of(&line) == "layout.export" {
            return Tapped::Reply(renamed(&fake.answer(&line), "w1:p1", &long()));
        }
        Tapped::Forward(line)
    });
    let port = HerdrAdapter::connect_with(config(WORKSPACE, 2, 1), tap).expect("connect");

    assert_cut_to_64(port.ensure_pane(&spec(WORKSPACE, 2, 1)));
    assert!(
        fake.methods().iter().any(|m| m == "pane.split"),
        "the split was sent"
    );
}

#[test]
fn a_tabless_workspaces_label_is_cut_to_64() {
    let fake = Arc::new(WireHerdr::new());
    fake.create_workspace(&long());
    let stripped = Arc::new(AtomicBool::new(false));
    let seen = Arc::clone(&stripped);
    // Herdr's snapshot lists the workspace with no tab.
    let tap = Tap::new(Arc::clone(&fake), move |fake, line, _| {
        if method_of(&line) == "session.snapshot" {
            let mut reply: Value = serde_json::from_str(&fake.answer(&line)).unwrap();
            reply["result"]["snapshot"]["tabs"] = Value::Array(Vec::new());
            reply["result"]["snapshot"]["panes"] = Value::Array(Vec::new());
            seen.store(true, Ordering::SeqCst);
            return Tapped::Reply(reply.to_string());
        }
        Tapped::Forward(line)
    });
    let port = HerdrAdapter::connect_with(config(&long(), 2, 1), tap).expect("connect");

    assert_cut_to_64(port.ensure_pane(&spec(&long(), 1, 1)));
    assert!(
        stripped.load(Ordering::SeqCst),
        "the snapshot was rewritten"
    );
}
