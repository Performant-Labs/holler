#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #640
//! `HerdrAdapter` passes #638's `HerdrPort` conformance suite (#640 part 2, AC 31-34)
//! against the wire fake, from the two starting states the suite allows (an empty
//! Herdr, and a workspace holding only its root pane), in process and over a real Unix
//! socket. The suite's cases are run, never copied.

mod wire_herdr;

use std::path::Path;
use std::sync::Arc;

use holler_adapter_herdr::adapter::{HerdrAdapter, HerdrConfig};
use holler_adapter_herdr::plan::Extent;
use holler_adapter_herdr::protocol::ALLOWED_METHODS;
use holler_pane_testkit::conformance::herdr::{run_herdr_conformance, HerdrFixture};
use holler_pane_testkit::conformance::Conformance;
use wire_herdr::serve::serve;
use wire_herdr::WireHerdr;

const SCRATCH: &str = "scratch";
/// An absolute socket path that the in-process runs never open.
const UNUSED_SOCKET: &str = "/unused/h.sock";

/// The suite's workspace, 2 rows by 1 column, in the scratch session at `socket`.
fn config(socket: &Path) -> HerdrConfig {
    HerdrConfig::new(SCRATCH, socket).with_workspace(SCRATCH, Extent { rows: 2, cols: 1 })
}

fn fixture<H>(port: H) -> HerdrFixture<H> {
    HerdrFixture {
        port,
        session: SCRATCH.into(),
        workspace: SCRATCH.into(),
    }
}

/// The suite in process, a fresh fake per case; with `root_pane`, each fake's workspace
/// is created first and holds its root pane. Each fake is pushed onto `fakes`.
fn run_in_process(root_pane: bool, fakes: &mut Vec<Arc<WireHerdr>>) -> Conformance {
    run_herdr_conformance(|| {
        let fake = Arc::new(WireHerdr::new());
        if root_pane {
            fake.create_workspace(SCRATCH);
        }
        fakes.push(Arc::clone(&fake));
        let port =
            HerdrAdapter::connect_with(config(Path::new(UNUSED_SOCKET)), fake).expect("connect");
        (fixture(port), ())
    })
}

/// The suite over a real socket: each case serves a fresh, empty fake, and the server
/// is the case's guard, dropped after the port.
fn run_over_a_socket(fakes: &mut Vec<Arc<WireHerdr>>) -> Conformance {
    run_herdr_conformance(|| {
        let fake = Arc::new(WireHerdr::new());
        fakes.push(Arc::clone(&fake));
        let served = serve(Arc::clone(&fake));
        let config = config(served.path());
        let port = HerdrAdapter::connect(config).expect("connect");
        (fixture(port), served)
    })
}

#[test]
fn the_adapter_passes_the_suite_from_an_empty_herdr() {
    assert_eq!(run_in_process(false, &mut Vec::new()), Ok(()));
}

#[test]
fn the_adapter_passes_the_suite_with_a_root_pane() {
    assert_eq!(run_in_process(true, &mut Vec::new()), Ok(()));
}

#[test]
fn the_adapter_passes_the_suite_over_a_real_socket() {
    assert_eq!(run_over_a_socket(&mut Vec::new()), Ok(()));
}

#[test]
fn no_case_calls_a_method_off_the_allow_list() {
    let mut fakes = Vec::new();
    let _ = run_in_process(false, &mut fakes);
    let _ = run_in_process(true, &mut fakes);
    let _ = run_over_a_socket(&mut fakes);

    assert!(!fakes.is_empty());
    for fake in &fakes {
        let methods = fake.methods();
        assert!(!methods.is_empty(), "a case sent nothing");
        for method in methods {
            assert!(
                ALLOWED_METHODS.contains(&method.as_str()),
                "{method} is not on the allow-list"
            );
        }
    }
}
