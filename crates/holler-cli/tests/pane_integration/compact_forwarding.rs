//! #878, wrapper forwarding: `HarnessPort::compact` is default-armed (`compact-unsupported`), so
//! a wrapper that forgets it still compiles and silently answers the default. The wiring's
//! `WiredHarness` says in its doc that every method is forwarded explicitly; this pins it for
//! `compact` (the router and `FakeHarnesses` are pinned in `holler-pane-testkit`'s
//! `fake_harnesses_test.rs`).
//!
//! The inner harness is the test kit's fake with a pin and a context to compact, so a `compact`
//! that reaches it answers an outcome, which the default never does, and the fake's own log
//! shows the call that reached it, with the session and no model.

use std::sync::Arc;

use holler_cli::pane::wiring::bind::WiredHarness;
use holler_pane::ports::{CompactModel, CompactOutcome};
use holler_pane::{HarnessEndpoint, HarnessPort, PaneError, PaneName, ServeSpec};
use holler_pane_testkit::harness::{CompactRun, FakeHarness, HarnessOp};

const PORT: u16 = 48100;

fn pin() -> CompactModel {
    CompactModel {
        provider: "pinned-provider".to_owned(),
        model_id: "pinned-model".to_owned(),
    }
}

/// A fake with a server on `PORT`, one session of 90 000 tokens that compacts to 12 000, and a
/// pin; and that session.
fn compactable() -> (Arc<FakeHarness>, String) {
    let fake = Arc::new(FakeHarness::new());
    let name = PaneName::parse("demo-c1r1").unwrap();
    let argv = fake
        .serve_command(&name, PORT, &ServeSpec::default())
        .unwrap();
    fake.start_server(&argv).unwrap().unwrap();
    let session = fake.create_session(&HarnessEndpoint::Port(PORT)).unwrap();
    fake.seed_context(PORT, &session, 90_000, 12_000);
    fake.set_compaction_pin(PORT, Some(pin()));
    (fake, session)
}

#[test]
fn the_wired_harness_forwards_compact_and_not_the_default() {
    let (fake, session) = compactable();
    let inner: Arc<dyn HarnessPort> = fake.clone();
    let wired = WiredHarness::new(Ok(inner));

    assert_eq!(
        wired.compact(&HarnessEndpoint::Port(PORT), &session, None),
        Ok(CompactOutcome {
            tokens_before: 90_000,
            tokens_after: 12_000
        }),
        "the inner harness's answer, not the default compact-unsupported"
    );
    assert_eq!(
        fake.compact_runs(),
        vec![CompactRun {
            port: PORT,
            session,
            asked: None,
            ran_on: pin(),
        }],
        "it reached the inner harness once, with every argument"
    );
    assert_eq!(fake.faults().calls().last(), Some(&HarnessOp::Compact));
}

/// A `WiredHarness` built from a misconfiguration answers that error from `compact`, as from
/// every other method, and never the default.
#[test]
fn a_misconfigured_wired_harness_answers_its_error_from_compact() {
    let wired = WiredHarness::new(Err(PaneError::Unavailable {
        what: "no opencode binary: set HOLLER_OPENCODE_BIN".to_owned(),
    }));
    let error = wired
        .compact(&HarnessEndpoint::Port(PORT), "ses_1", None)
        .expect_err("a misconfigured harness cannot compact");
    assert_eq!(error.code(), "unavailable", "{error}");
    assert!(error.to_string().contains("HOLLER_OPENCODE_BIN"), "{error}");
}
