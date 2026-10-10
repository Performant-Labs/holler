#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #640
//! `HerdrAdapter` over the wire fake, in process (#640 part 2, AC 13 and 15-30): plan,
//! act and observe in `ensure_pane`, the snapshot of every workspace, the single-request
//! methods, the version gate, config validation and the one deadline per call. Every
//! request goes through `Request::to_line` and every reply through the adapter's own
//! decoder; the fake keeps the trees.

mod wire_herdr;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use holler_adapter_herdr::adapter::{HerdrAdapter, HerdrConfig, DEFAULT_TIMEOUT};
use holler_adapter_herdr::layout::Direction;
use holler_adapter_herdr::plan::{Extent, GRID_UNREACHABLE};
use holler_adapter_herdr::protocol::{Request, SUPPORTED_VERSIONS};
use holler_adapter_herdr::transport::Transport;
use holler_pane::{GridPos, HerdrPane, HerdrPort, HerdrSpec, Key, PaneError, PaneId};
use holler_pane_testkit::herdr::{PROTOCOL_22_VERSION, UNSUPPORTED_VERSION};
use serde_json::{json, Value};
use wire_herdr::{Tap, Tapped, WireHerdr};

const SESSION: &str = "scratch";
/// An absolute socket path that the in-process tests never open.
const UNUSED_SOCKET: &str = "/unused/h.sock";

// --- helpers ---

fn cell(row: u16, col: u16) -> GridPos {
    GridPos { row, col }
}

fn extent(rows: u16, cols: u16) -> Extent {
    Extent { rows, cols }
}

fn config(workspaces: &[(&str, u16, u16)]) -> HerdrConfig {
    workspaces.iter().fold(
        HerdrConfig::new(SESSION, UNUSED_SOCKET),
        |config, &(label, rows, cols)| config.with_workspace(label, extent(rows, cols)),
    )
}

/// An adapter connected to `fake` in process.
fn connect(fake: &Arc<WireHerdr>, workspaces: &[(&str, u16, u16)]) -> HerdrAdapter<Arc<WireHerdr>> {
    connect_over(config(workspaces), Arc::clone(fake))
}

fn connect_over<T: Transport>(config: HerdrConfig, transport: T) -> HerdrAdapter<T> {
    HerdrAdapter::connect_with(config, transport).expect("connect")
}

/// What `connect_with` refused with; `None` when it connected.
fn connect_error<T: Transport>(config: HerdrConfig, transport: T) -> Option<PaneError> {
    HerdrAdapter::connect_with(config, transport).err()
}

fn spec(workspace: &str, row: u16, col: u16) -> HerdrSpec {
    HerdrSpec {
        session: SESSION.to_owned(),
        workspace: workspace.to_owned(),
        grid: cell(row, col),
    }
}

/// The pane the adapter reports for `pane_id` at `r<row>c<col>` of `workspace`.
fn pane(workspace: &str, pane_id: &str, row: u16, col: u16) -> HerdrPane {
    HerdrPane {
        session: SESSION.to_owned(),
        workspace: workspace.to_owned(),
        pane_id: PaneId::new(pane_id),
        grid: cell(row, col),
    }
}

/// The methods the fake received from the `from`-th request on.
fn sent_since(fake: &WireHerdr, from: usize) -> Vec<String> {
    fake.methods().split_off(from)
}

/// The `params` of every request of `method` the fake received, oldest first.
fn params_of(fake: &WireHerdr, method: &str) -> Vec<Value> {
    fake.requests()
        .into_iter()
        .filter(|request| request["method"] == method)
        .map(|request| request["params"].clone())
        .collect()
}

fn unavailable_what<T: std::fmt::Debug>(result: Result<T, PaneError>) -> String {
    match result {
        Err(PaneError::Unavailable { what }) => what,
        other => panic!("expected Unavailable, got {other:?}"),
    }
}

fn assert_grid_unreachable<T: std::fmt::Debug>(result: Result<T, PaneError>) {
    match result {
        Err(PaneError::Refused { code, .. }) => assert_eq!(code, GRID_UNREACHABLE),
        other => panic!("expected grid-unreachable, got {other:?}"),
    }
}

/// No `pane.split` and no `workspace.create` from the `from`-th request on.
fn assert_nothing_placed(fake: &WireHerdr, from: usize) {
    let sent = sent_since(fake, from);
    assert!(
        !sent
            .iter()
            .any(|m| m == "pane.split" || m == "workspace.create"),
        "a placing request was sent: {sent:?}"
    );
}

fn enter() -> Key {
    Key::new("enter")
}

// --- AC 13, 15-22: placing panes, reading them back ---

#[test]
fn r2c1_and_r1c2_land_in_their_cells_and_read_back() {
    let fake = Arc::new(WireHerdr::new());
    let port = connect(&fake, &[("w", 2, 2)]);

    assert_eq!(
        port.ensure_pane(&spec("w", 1, 1)),
        Ok(pane("w", "w1:p1", 1, 1))
    );
    assert_eq!(
        port.ensure_pane(&spec("w", 2, 1)),
        Ok(pane("w", "w1:p2", 2, 1))
    );
    assert_eq!(
        fake.tree("w"),
        Some(json!({"type":"split","direction":"down","ratio":0.5,
            "first":{"type":"pane","pane_id":"w1:p1"},"second":{"type":"pane","pane_id":"w1:p2"}}))
    );
    assert_eq!(
        port.ensure_pane(&spec("w", 1, 2)),
        Ok(pane("w", "w1:p3", 1, 2))
    );
    assert_eq!(
        fake.tree("w"),
        Some(json!({"type":"split","direction":"down","ratio":0.5,
            "first":{"type":"split","direction":"right","ratio":0.5,
                "first":{"type":"pane","pane_id":"w1:p1"},"second":{"type":"pane","pane_id":"w1:p3"}},
            "second":{"type":"pane","pane_id":"w1:p2"}}))
    );

    let listed = port.snapshot().expect("snapshot").panes;
    assert_eq!(
        listed,
        vec![
            pane("w", "w1:p1", 1, 1),
            pane("w", "w1:p3", 1, 2),
            pane("w", "w1:p2", 2, 1)
        ]
    );
    let written: Vec<String> = listed.iter().map(|p| p.grid.to_string()).collect();
    assert_eq!(written, ["r1c1", "r1c2", "r2c1"]);
    assert_eq!(
        params_of(&fake, "pane.split"),
        vec![
            json!({"target_pane_id":"w1:p1","direction":"down","ratio":0.5,"focus":false}),
            json!({"target_pane_id":"w1:p1","direction":"right","ratio":0.5,"focus":false}),
        ]
    );
}

#[test]
fn a_closed_panes_space_goes_to_its_sibling() {
    let fake = Arc::new(WireHerdr::new());
    let port = connect(&fake, &[("w", 2, 1)]);
    let a = port
        .ensure_pane(&spec("w", 1, 1))
        .expect("ensure r1c1")
        .pane_id;
    let b = port
        .ensure_pane(&spec("w", 2, 1))
        .expect("ensure r2c1")
        .pane_id;

    assert_eq!(port.close(&a), Ok(()));

    assert_eq!(
        fake.tree("w"),
        Some(json!({"type":"pane","pane_id":b.as_str()}))
    );
    assert_eq!(
        port.snapshot().expect("snapshot").panes,
        vec![pane("w", b.as_str(), 1, 1)]
    );
}

#[test]
fn a_pane_that_lands_elsewhere_is_unavailable_and_left_in_place() {
    let fake = Arc::new(WireHerdr::new());
    let tap = Tap::new(Arc::clone(&fake), |_, line, _| {
        if line.contains(r#""method":"pane.split""#) {
            return Tapped::Forward(
                line.replace(r#""direction":"down""#, r#""direction":"right""#),
            );
        }
        Tapped::Forward(line)
    });
    let port = connect_over(config(&[("w", 2, 2)]), tap);
    port.ensure_pane(&spec("w", 1, 1)).expect("ensure r1c1");

    let what = unavailable_what(port.ensure_pane(&spec("w", 2, 1)));

    assert_eq!(
        params_of(&fake, "pane.split")[0]["direction"],
        "right",
        "the rewrite ran"
    );
    assert!(what.contains("w1:p2") && what.contains("r1c2"), "{what:?}");
    let listed = port.snapshot().expect("snapshot").panes;
    assert!(
        listed.contains(&pane("w", "w1:p2", 1, 2)),
        "left where it landed: {listed:?}"
    );
    assert!(!fake.methods().iter().any(|m| m == "pane.close"));
}

#[test]
fn a_split_target_closed_under_the_adapter_is_unavailable_not_pane_not_found() {
    let fake = Arc::new(WireHerdr::new());
    let tap = Tap::new(Arc::clone(&fake), |fake, line, _| {
        let request: Value = serde_json::from_str(&line).unwrap();
        if request["method"] == "pane.split" {
            let target = PaneId::new(request["params"]["target_pane_id"].as_str().unwrap());
            fake.answer(&Request::Close { pane: target }.to_line());
        }
        Tapped::Forward(line)
    });
    let port = connect_over(config(&[("w", 2, 1)]), tap);
    port.ensure_pane(&spec("w", 1, 1)).expect("ensure r1c1");

    unavailable_what(port.ensure_pane(&spec("w", 2, 1)));
}

#[test]
fn an_occupied_cell_sends_no_mutating_request() {
    let fake = Arc::new(WireHerdr::new());
    let port = connect(&fake, &[("w", 2, 1)]);
    let first = port.ensure_pane(&spec("w", 1, 1)).expect("ensure r1c1");
    let from = fake.methods().len();

    assert_eq!(port.ensure_pane(&spec("w", 1, 1)), Ok(first));

    assert_eq!(
        sent_since(&fake, from),
        ["session.snapshot", "layout.export"]
    );
}

#[test]
fn nesting_is_refused_before_any_split() {
    let fake = Arc::new(WireHerdr::new());
    let p1 = fake.create_workspace("w");
    let p2 = fake.split(&p1, Direction::Right);
    let port = connect(&fake, &[("w", 2, 2)]);

    // (a) A row of two panes: a `down` split of r1c1 would nest inside its cell.
    let from = fake.methods().len();
    assert_grid_unreachable(port.ensure_pane(&spec("w", 2, 1)));
    assert_nothing_placed(&fake, from);

    // (b) A nested slot at r1c2: its panes have no position.
    fake.split(&p2, Direction::Down);
    let from = fake.methods().len();
    assert_eq!(
        port.snapshot().expect("snapshot").panes,
        vec![pane("w", "w1:p1", 1, 1)]
    );
    assert_eq!(
        port.ensure_pane(&spec("w", 1, 1)),
        Ok(pane("w", "w1:p1", 1, 1))
    );
    assert_grid_unreachable(port.ensure_pane(&spec("w", 2, 1)));
    assert_nothing_placed(&fake, from);
}

#[test]
fn a_missing_workspace_is_created_only_for_r1c1() {
    let fake = Arc::new(WireHerdr::new());
    let port = connect(&fake, &[("w", 2, 1)]);
    let from = fake.methods().len();

    assert_grid_unreachable(port.ensure_pane(&spec("w", 2, 1)));
    assert_eq!(sent_since(&fake, from), ["session.snapshot"]);

    assert_eq!(
        port.ensure_pane(&spec("w", 1, 1)),
        Ok(pane("w", "w1:p1", 1, 1))
    );
    assert_eq!(
        params_of(&fake, "workspace.create"),
        vec![json!({"label":"w","focus":false})]
    );
}

#[test]
fn a_pane_in_another_tab_neither_lists_nor_places() {
    let fake = Arc::new(WireHerdr::new());
    fake.create_workspace("w");
    let other_tab = fake.add_tab("w");
    let port = connect(&fake, &[("w", 2, 1)]);

    let listed = port.snapshot().expect("snapshot").panes;
    assert!(!listed.iter().any(|p| p.pane_id == other_tab), "{listed:?}");
    assert_eq!(listed, vec![pane("w", "w1:p1", 1, 1)]);

    let made = port.ensure_pane(&spec("w", 2, 1)).expect("ensure r2c1");
    assert_eq!(made.grid, cell(2, 1));
    let splits = params_of(&fake, "pane.split");
    assert_eq!(splits.len(), 1);
    assert_eq!(splits[0]["target_pane_id"], "w1:p1");
    for export in params_of(&fake, "layout.export") {
        assert_eq!(export["tab_id"], "w1:t1", "only the grid tab is read");
    }
}

#[test]
fn snapshot_lists_every_workspace_by_label() {
    let fake = Arc::new(WireHerdr::new());
    fake.create_workspace("w");
    let other = fake.create_workspace("other");
    fake.split(&other, Direction::Down);
    let port = connect(&fake, &[("w", 2, 1)]);

    assert_eq!(
        port.snapshot().expect("snapshot").panes,
        vec![
            pane("w", "w1:p1", 1, 1),
            pane("other", "w2:p1", 1, 1),
            pane("other", "w2:p2", 2, 1),
        ]
    );

    // Two workspaces of one label: both listed, told apart by pane id only.
    let twins = Arc::new(WireHerdr::new());
    twins.create_workspace("w");
    twins.create_workspace("w");
    let port = connect(&twins, &[("w", 2, 1)]);
    assert_eq!(
        port.snapshot().expect("snapshot").panes,
        vec![pane("w", "w1:p1", 1, 1), pane("w", "w2:p1", 1, 1)]
    );
}

#[test]
fn session_and_workspace_errors_send_nothing() {
    let fake = Arc::new(WireHerdr::new());
    fake.create_workspace("w");
    let port = connect(&fake, &[("w", 2, 1)]);
    let from = fake.methods().len();

    let other_session = HerdrSpec {
        session: "other-session".to_owned(),
        ..spec("w", 1, 1)
    };
    unavailable_what(port.ensure_pane(&other_session));
    assert_eq!(sent_since(&fake, from), Vec::<String>::new());

    let what = unavailable_what(port.ensure_pane(&spec("unconfigured", 1, 1)));
    assert!(what.contains("unconfigured"), "{what:?}");
    assert_eq!(sent_since(&fake, from), Vec::<String>::new());

    fake.create_workspace("w");
    unavailable_what(port.ensure_pane(&spec("w", 1, 1)));
}

// --- AC 24-30: the single-request methods, the gate, the config, the deadline ---

#[test]
fn keys_go_out_verbatim() {
    let fake = Arc::new(WireHerdr::new());
    let p = fake.create_workspace("w");
    let port = connect(&fake, &[]);

    assert_eq!(port.send_keys(&p, &[enter(), Key::new("ctrl+c")]), Ok(()));

    assert_eq!(
        params_of(&fake, "pane.send_keys"),
        vec![json!({"pane_id":"w1:p1","keys":["enter","ctrl+c"]})]
    );
}

#[test]
fn read_asks_for_recent_text_and_trims() {
    let fake = Arc::new(WireHerdr::new());
    let p = fake.create_workspace("w");
    let port = connect(&fake, &[]);
    for line in 1..=5 {
        port.send_text(&p, &format!("line{line}"))
            .expect("send_text");
        port.send_keys(&p, &[enter()]).expect("send_keys");
    }

    assert_eq!(port.read(&p, 2), Ok("line4\nline5".to_owned()));
    assert_eq!(port.read(&p, 0), Ok(String::new()));

    let reads = params_of(&fake, "pane.read");
    assert_eq!(
        reads[0],
        json!({"pane_id":"w1:p1","source":"recent","lines":2,"format":"text"})
    );
    assert_eq!(reads[1]["lines"], 1, "read(p, 0) still asks for one line");
}

#[test]
fn version_and_the_gate() {
    let fake = Arc::new(WireHerdr::new());
    let port = connect(&fake, &[]);
    assert_eq!(port.version(), Ok(PROTOCOL_22_VERSION.to_owned()));

    for protocol in [Some(99), None] {
        let fake = Arc::new(WireHerdr::new());
        fake.set_protocol(protocol);
        let message = match connect_error(config(&[]), Arc::clone(&fake)) {
            Some(PaneError::HerdrVersionUnsupported { message }) => message,
            other => {
                panic!("protocol {protocol:?}: expected herdr-version-unsupported, got {other:?}")
            }
        };
        assert!(!message.contains(['\n', '\r']), "{message:?}");
        assert!(message.contains(UNSUPPORTED_VERSION), "{message:?}");
        assert!(message.contains(SUPPORTED_VERSIONS), "{message:?}");
        if protocol.is_some() {
            assert!(message.contains("99"), "{message:?}");
        }
    }

    // The gate is at connect and at version(); the other methods trust it (Decision 8).
    let fake = Arc::new(WireHerdr::new());
    let p = fake.create_workspace("w");
    let port = connect(&fake, &[]);
    fake.set_protocol(Some(99));
    let version = port.version();
    assert!(
        matches!(version, Err(PaneError::HerdrVersionUnsupported { .. })),
        "{version:?}"
    );
    assert_eq!(port.send_text(&p, "x"), Ok(()));
}

#[test]
fn config_is_validated_before_any_request() {
    let relative = "relative-dir/h.sock";
    let zero_timeout = HerdrConfig {
        timeout: Duration::ZERO,
        ..config(&[])
    };
    // (config, what the message names, what it must not name)
    let cases: Vec<(HerdrConfig, Vec<&str>, Vec<&str>)> = vec![
        (HerdrConfig::new("", UNUSED_SOCKET), vec!["session"], vec![]),
        (
            HerdrConfig::new(SESSION, relative),
            vec!["socket", relative],
            vec![],
        ),
        (zero_timeout, vec!["timeout"], vec![]),
        (
            config(&[("ws-rows", 0, 1)]),
            vec!["ws-rows", "rows"],
            vec![],
        ),
        (
            config(&[("ws-cols", 1, 0)]),
            vec!["ws-cols", "cols"],
            vec![],
        ),
        (
            config(&[("ws-both", 0, 0)]),
            vec!["ws-both", "rows"],
            vec!["cols"],
        ),
        // The first invalid entry in label order, and only it.
        (
            config(&[("ws-b", 1, 0), ("ws-a", 0, 1)]),
            vec!["ws-a", "rows"],
            vec!["ws-b"],
        ),
        // `session` is checked before `socket`.
        (
            HerdrConfig::new("", relative),
            vec!["session"],
            vec![relative],
        ),
    ];

    for (config, names, never) in cases {
        let shown = format!("{config:?}");
        let fake = Arc::new(WireHerdr::new());
        let message = match connect_error(config, Arc::clone(&fake)) {
            Some(PaneError::Usage { message }) => message,
            other => panic!("{shown}: expected usage, got {other:?}"),
        };
        assert!(!message.contains(['\n', '\r']), "{message:?}");
        for name in names {
            assert!(message.contains(name), "{message:?} should name {name:?}");
        }
        for name in never {
            assert!(
                !message.contains(name),
                "{message:?} should not name {name:?}"
            );
        }
        assert_eq!(
            fake.requests(),
            Vec::<Value>::new(),
            "{shown}: a request was sent"
        );
    }
}

/// Run `call` and check that every exchange it made carried one deadline `d`, taken
/// once on entry: `before + timeout <= d <= after + timeout`.
fn assert_one_deadline(deadlines: &Mutex<Vec<Instant>>, what: &str, call: impl FnOnce()) {
    deadlines.lock().unwrap().clear();
    let before = Instant::now();
    call();
    let after = Instant::now();
    let seen = deadlines.lock().unwrap().clone();
    assert!(seen.len() >= 2, "{what} made {} exchanges", seen.len());
    let d = seen[0];
    assert!(
        seen.iter().all(|x| *x == d),
        "{what}: more than one deadline: {seen:?}"
    );
    assert!(
        before + DEFAULT_TIMEOUT <= d && d <= after + DEFAULT_TIMEOUT,
        "{what}"
    );
}

#[test]
fn one_deadline_covers_every_exchange_of_a_call() {
    let fake = Arc::new(WireHerdr::new());
    let deadlines = Arc::new(Mutex::new(Vec::new()));
    let record = Arc::clone(&deadlines);
    let tap = Tap::new(Arc::clone(&fake), move |_, line, deadline| {
        record.lock().unwrap().push(deadline);
        Tapped::Forward(line)
    });
    let port = connect_over(config(&[("w", 2, 1)]), tap);
    assert_eq!(port.config().timeout, DEFAULT_TIMEOUT);
    port.ensure_pane(&spec("w", 1, 1)).expect("ensure r1c1");

    assert_one_deadline(&deadlines, "ensure_pane(r2c1)", || {
        port.ensure_pane(&spec("w", 2, 1)).expect("ensure r2c1");
    });
    assert_one_deadline(&deadlines, "snapshot", || {
        port.snapshot().expect("snapshot");
    });
}

#[test]
fn a_garbled_reply_is_unavailable_everywhere() {
    let fake = Arc::new(WireHerdr::new());
    let p = fake.create_workspace("w");
    let garble = Arc::new(AtomicBool::new(true));
    let tap = || {
        let garble = Arc::clone(&garble);
        Tap::new(Arc::clone(&fake), move |_, line, _| {
            if garble.load(Ordering::SeqCst) {
                return Tapped::Reply("not json".to_owned());
            }
            Tapped::Forward(line)
        })
    };
    let refused = connect_error(config(&[("w", 2, 1)]), tap());
    assert!(
        matches!(refused, Some(PaneError::Unavailable { .. })),
        "{refused:?}"
    );

    garble.store(false, Ordering::SeqCst);
    let port = connect_over(config(&[("w", 2, 1)]), tap());
    garble.store(true, Ordering::SeqCst);
    let results = [
        ("ensure_pane", port.ensure_pane(&spec("w", 1, 1)).map(drop)),
        ("send_text", port.send_text(&p, "x")),
        ("send_keys", port.send_keys(&p, &[enter()])),
        ("read", port.read(&p, 1).map(drop)),
        ("close", port.close(&p)),
        ("snapshot", port.snapshot().map(drop)),
        ("version", port.version().map(drop)),
    ];
    for (method, result) in results {
        assert!(
            matches!(result, Err(PaneError::Unavailable { .. })),
            "{method}: {result:?}"
        );
    }
}

#[test]
fn the_fake_numbers_panes_in_base_36() {
    let fake = Arc::new(WireHerdr::new());
    let port = connect(&fake, &[("w", 1, 10)]);
    for col in 1..=9 {
        let made = port.ensure_pane(&spec("w", 1, col)).expect("ensure");
        assert_eq!(made.pane_id.as_str(), format!("w1:p{col}"));
    }

    assert_eq!(
        port.ensure_pane(&spec("w", 1, 10)),
        Ok(pane("w", "w1:pA", 1, 10))
    );
}
