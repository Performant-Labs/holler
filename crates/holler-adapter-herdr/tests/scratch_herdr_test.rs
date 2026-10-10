#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #640
//! `HerdrAdapter` against a real, throwaway Herdr server (#640 part 3, the issue's last
//! acceptance line), and the harness's guards.
//!
//! The guards (AC 1-7) are pure and run in the default run: they touch no process,
//! socket, file or environment. The two tests against a real server are ignored by
//! default and also need `HOLLER_HERDR_SCRATCH=1`; CI never runs them:
//!
//! ```text
//! HOLLER_HERDR_SCRATCH=1 cargo test -p holler-adapter-herdr --test scratch_herdr_test -- --ignored --test-threads=1
//! ```

mod scratch_herdr;

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use holler_adapter_herdr::adapter::{HerdrAdapter, HerdrConfig};
use holler_adapter_herdr::plan::Extent;
use holler_pane::{GridPos, HerdrPane, HerdrPort, HerdrSpec, Key, PaneError, PaneId};
use holler_pane_testkit::conformance::herdr::{run_herdr_conformance, HerdrFixture};
use scratch_herdr::{
    check_name, check_socket, gate, new_name, opt_in, poll, prove, scratch_env, Gate, ScratchHerdr,
    SOCKET_PATH_LIMIT,
};

/// The workspace both ignored tests place their panes in, 2 rows by 1 column.
const GRID: &str = "holler640-grid";
const GRID_EXTENT: Extent = Extent { rows: 2, cols: 1 };
/// A literal scratch root; no test of the guards touches it.
const ROOT: &str = "/r/h640.ab";
const NAME: &str = "holler640-0123abcd";
const SOCKET: &str = "/r/h640.ab/home/.config/herdr/sessions/holler640-0123abcd/herdr.sock";

// --- AC 1-4: the gate and the name ---

#[test]
fn the_gate_runs_only_on_exactly_1() {
    assert_eq!(gate(Some("1")), Gate::Run);
    for value in [
        None,
        Some(""),
        Some("0"),
        Some("true"),
        Some("1 "),
        Some("yes"),
    ] {
        assert_eq!(gate(value), Gate::Skip, "{value:?}");
    }
}

#[test]
fn the_name_guard_refuses_the_default_session_by_name() {
    for name in ["default", "Default"] {
        let refusal = check_name(name).expect_err(name);
        assert!(refusal.contains("default"), "{name}: {refusal:?}");
    }
}

#[test]
fn the_name_guard_accepts_only_holler640_and_8_hex() {
    assert_eq!(check_name("holler640-0123abcd"), Ok(()));
    for name in [
        "",
        "holler640-",
        "holler640-0123abc",
        "holler640-0123abcde",
        "holler640-0123ABCD",
        "holler640-0123abcg",
        " holler640-0123abcd",
        "holler640-0123abcd/x",
        "spike636-0123abcd",
        "scratch",
    ] {
        assert!(check_name(name).is_err(), "{name:?} was accepted");
    }
}

#[test]
fn generated_names_pass_the_guard_and_differ() {
    let names: Vec<String> = (0..64).map(|_| new_name()).collect();
    for name in &names {
        assert_eq!(check_name(name), Ok(()), "{name:?}");
    }
    let distinct: BTreeSet<&String> = names.iter().collect();
    assert_eq!(distinct.len(), 64, "{names:?}");
}

// --- AC 5-6: the socket and the proof ---

#[test]
fn the_socket_guard_keeps_the_socket_inside_the_root() {
    let root = Path::new(ROOT);
    assert_eq!(check_socket(root, Path::new(SOCKET)), Ok(()));

    let at_limit = format!("{ROOT}/{}", "s".repeat(SOCKET_PATH_LIMIT - ROOT.len() - 1));
    assert_eq!(at_limit.len(), SOCKET_PATH_LIMIT);
    for socket in [
        "/r/h640.abc/herdr.sock",
        "/r/other/herdr.sock",
        "h640.ab/herdr.sock",
        "/r/h640.ab/../x/herdr.sock",
        "/r/h640.ab/./herdr.sock",
        ROOT,
        &at_limit,
    ] {
        assert!(
            check_socket(root, Path::new(socket)).is_err(),
            "{socket:?} was accepted"
        );
    }
    let under_limit = &at_limit[..SOCKET_PATH_LIMIT - 1];
    assert_eq!(check_socket(root, Path::new(under_limit)), Ok(()));
}

#[test]
fn the_server_is_proven_by_its_own_status() {
    let root = Path::new(ROOT);
    let status = |session: &str, socket: &str| {
        serde_json::json!({"session": session, "socket": socket, "running": true}).to_string()
    };

    assert_eq!(
        prove(&status(NAME, SOCKET), NAME, root),
        Ok(PathBuf::from(SOCKET))
    );

    let refusal = prove(&status("default", SOCKET), NAME, root).expect_err("default");
    assert!(refusal.contains("default"), "{refusal:?}");
    let refused = [
        status("holler640-ffffffff", SOCKET),
        status(NAME, "/r/other/herdr.sock"),
        serde_json::json!({"socket": SOCKET}).to_string(),
        serde_json::json!({"session": NAME}).to_string(),
        serde_json::json!({"session": NAME, "socket": 7}).to_string(),
        "not json".to_owned(),
    ];
    for json in refused {
        assert!(prove(&json, NAME, root).is_err(), "{json} was accepted");
    }
}

// --- AC 7: the environment ---

#[test]
fn the_scratch_env_carries_nothing_of_the_live_herdr() {
    let inherited = [
        ("HERDR_SOCKET_PATH", "/live/herdr.sock"),
        ("HERDR_SESSION", "default"),
        ("HERDR_SOMETHING_NEW", "x"),
        ("TMUX", "/tmp/tmux-1/default,1,0"),
        ("TMUX_PANE", "%1"),
        ("HOME", "/home/someone"),
        ("XDG_CONFIG_HOME", "/home/someone/.config"),
        ("XDG_RUNTIME_DIR", "/run/user/1"),
        ("PATH", "/usr/bin:/bin"),
        ("LANG", "C.UTF-8"),
    ];
    let pairs = inherited
        .iter()
        .map(|(k, v)| (OsString::from(k), OsString::from(v)));
    let env: Vec<(String, String)> = scratch_env(Path::new(ROOT), pairs)
        .into_iter()
        .map(|(k, v)| (k.into_string().unwrap(), v.into_string().unwrap()))
        .collect();
    let values_of = |name: &str| -> Vec<&str> {
        env.iter()
            .filter(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
            .collect()
    };

    for (name, value) in &env {
        assert!(!name.starts_with("HERDR_"), "{name}={value}");
        assert!(name != "TMUX" && name != "TMUX_PANE", "{name}={value}");
        for live in ["/live", "/home/someone", "/run/user"] {
            assert!(!value.contains(live), "{name}={value}");
        }
    }
    for name in [
        "HOME",
        "XDG_CONFIG_HOME",
        "XDG_STATE_HOME",
        "XDG_DATA_HOME",
        "XDG_CACHE_HOME",
        "XDG_RUNTIME_DIR",
    ] {
        let values = values_of(name);
        assert_eq!(values.len(), 1, "{name}: {values:?}");
        let under = Path::new(values[0]);
        assert!(
            under.starts_with(ROOT) && under != Path::new(ROOT),
            "{name}={}",
            values[0]
        );
    }
    assert_eq!(values_of("SHELL"), ["/bin/sh"]);
    assert_eq!(values_of("PATH"), ["/usr/bin:/bin"]);
    assert_eq!(values_of("LANG"), ["C.UTF-8"]);
}

// --- AC 12-13: against a real scratch Herdr (opt-in) ---

/// An adapter connected to `server`, which knows `GRID` to be 2 rows by 1 column.
fn connect(server: &ScratchHerdr) -> HerdrAdapter {
    let config =
        HerdrConfig::new(server.session(), server.socket()).with_workspace(GRID, GRID_EXTENT);
    HerdrAdapter::connect(config).expect("connect")
}

#[test]
#[ignore = "opt-in: a scratch Herdr server; set HOLLER_HERDR_SCRATCH=1 (#640)"]
fn scratch_herdr_passes_the_conformance_suite() {
    let Some(herdr) = opt_in() else {
        return;
    };
    let begun = Instant::now();

    let result = run_herdr_conformance(|| {
        let server = ScratchHerdr::start(&herdr);
        let fixture = HerdrFixture {
            port: connect(&server),
            session: server.session().to_owned(),
            workspace: GRID.to_owned(),
        };
        (fixture, server)
    });

    eprintln!("the conformance suite took {:?}", begun.elapsed());
    assert_eq!(result, Ok(()));
}

/// The panes of `GRID` in `port`'s snapshot. Every pane listed carries `session`.
fn grid_panes(port: &dyn HerdrPort, session: &str) -> Vec<HerdrPane> {
    let panes = port.snapshot().expect("snapshot").panes;
    for pane in &panes {
        assert_eq!(pane.session, session, "{pane:?}");
    }
    panes.into_iter().filter(|p| p.workspace == GRID).collect()
}

/// Read `pane` every 100 ms for at most 10 s until a line of it satisfies `wanted`.
/// The failure names how many lines the last read returned, never the screen.
fn wait_for_line(port: &dyn HerdrPort, pane: &PaneId, what: &str, wanted: impl Fn(&str) -> bool) {
    let mut last_lines = 0;
    let found = poll(Duration::from_secs(10), Duration::from_millis(100), || {
        let text = port.read(pane, 50).expect("read");
        last_lines = text.lines().count();
        text.lines().any(&wanted).then_some(())
    });
    assert!(
        found.is_some(),
        "{what}: not read back in 10 s; the last read returned {last_lines} lines"
    );
}

fn spec(session: &str, row: u16) -> HerdrSpec {
    HerdrSpec {
        session: session.to_owned(),
        workspace: GRID.to_owned(),
        grid: GridPos { row, col: 1 },
    }
}

#[test]
#[ignore = "opt-in: a scratch Herdr server; set HOLLER_HERDR_SCRATCH=1 (#640)"]
fn scratch_herdr_creates_runs_sends_reads_closes_and_snapshots_a_pane() {
    let Some(herdr) = opt_in() else {
        return;
    };
    let begun = Instant::now();
    let server = ScratchHerdr::start(&herdr);
    let session = server.session().to_owned();

    // 1. Connect, and the version on one line.
    let port = connect(&server);
    let version = port.version().expect("version");
    assert!(
        !version.is_empty() && !version.contains('\n'),
        "{version:?}"
    );
    eprintln!("Herdr reports version {version}");

    // 2. Create.
    let a = port.ensure_pane(&spec(&session, 1)).expect("ensure r1c1");
    assert_eq!(
        (a.session.as_str(), a.workspace.as_str()),
        (session.as_str(), GRID)
    );
    assert_eq!(a.grid, GridPos { row: 1, col: 1 });
    assert_eq!(grid_panes(&port, &session), std::slice::from_ref(&a));

    // 3. Run: the shell prints the line; the typed command line cannot match it.
    port.send_text(&a.pane_id, r"printf 'holler640-%s\n' ran")
        .expect("send_text");
    port.send_keys(&a.pane_id, &[Key::new("enter")])
        .expect("send_keys");
    wait_for_line(&port, &a.pane_id, "the command's output", |line| {
        line.trim() == "holler640-ran"
    });

    // 4. Send, with no Enter.
    port.send_text(&a.pane_id, "holler640-typed")
        .expect("send_text");
    wait_for_line(&port, &a.pane_id, "the typed text", |line| {
        line.contains("holler640-typed")
    });

    // 5. A second pane below the first.
    let b = port.ensure_pane(&spec(&session, 2)).expect("ensure r2c1");
    let mut listed = grid_panes(&port, &session);
    listed.sort_by_key(|p| p.grid.row);
    assert_eq!(listed, [a.clone(), b.clone()]);
    assert_eq!(b.grid, GridPos { row: 2, col: 1 });

    // 6. Close.
    assert_eq!(port.close(&b.pane_id), Ok(()));
    let ids: Vec<PaneId> = grid_panes(&port, &session)
        .into_iter()
        .map(|p| p.pane_id)
        .collect();
    assert_eq!(ids, std::slice::from_ref(&a.pane_id));
    let again = port.close(&b.pane_id);
    assert!(
        matches!(again, Err(PaneError::PaneNotFound { .. })),
        "close again: {again:?}"
    );
    let read = port.read(&b.pane_id, 1);
    assert!(
        matches!(read, Err(PaneError::PaneNotFound { .. })),
        "read closed: {read:?}"
    );

    eprintln!("the pane pass took {:?}", begun.elapsed());
}
