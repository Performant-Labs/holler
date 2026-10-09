#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #683
//! `FakeProber` answers a scripted `ProbeResult` per argv, never `Ok` for an argv
//! nobody scripted, and records every run (#683, slice d).

use std::time::Duration;

use holler_pane::{Argv, ProbeResult, Prober};
use holler_pane_testkit::prober::{FakeProber, ProbeCall};

fn argv(parts: &[&str]) -> Argv {
    Argv::new(parts.iter().map(|p| (*p).to_owned()).collect())
}

const TIMEOUT: Duration = Duration::from_secs(5);

#[test]
fn a_scripted_argv_answers_each_probe_result() {
    let fake = FakeProber::new();
    let up = argv(&["curl", "-sf", "http://127.0.0.1:1/health"]);
    let model = argv(&["ollama", "list"]);
    let down = argv(&["nc", "-z", "127.0.0.1", "1"]);
    fake.script(up.clone(), ProbeResult::Ok);
    fake.script(
        model.clone(),
        ProbeResult::Failed {
            missing: vec!["qwen38".into()],
        },
    );
    fake.script(down.clone(), ProbeResult::Error("down".into()));

    let prober: &dyn Prober = &fake;
    assert_eq!(prober.run_probe(&up, &[], TIMEOUT), ProbeResult::Ok);
    assert_eq!(
        prober.run_probe(&model, &["qwen38".into()], TIMEOUT),
        ProbeResult::Failed {
            missing: vec!["qwen38".into()]
        }
    );
    assert_eq!(
        prober.run_probe(&down, &[], TIMEOUT),
        ProbeResult::Error("down".into())
    );
}

#[test]
fn an_unscripted_argv_answers_error_never_ok() {
    let fake = FakeProber::new();
    let scripted = argv(&["ollama", "list"]);
    fake.script(scripted, ProbeResult::Ok);

    let nothing = argv(&["true"]);
    let one_element_off = argv(&["ollama", "ps"]);
    let prefix_only = argv(&["ollama"]);
    for unscripted in [&nothing, &one_element_off, &prefix_only] {
        match fake.run_probe(unscripted, &[], TIMEOUT) {
            ProbeResult::Error(reason) => assert!(
                !reason.is_empty(),
                "the reason is plain text, got an empty one for {unscripted:?}"
            ),
            other => panic!("an unscripted argv {unscripted:?} answered {other:?}"),
        }
    }

    let empty = FakeProber::default();
    assert!(matches!(
        empty.run_probe(&nothing, &[], TIMEOUT),
        ProbeResult::Error(_)
    ));
}

#[test]
fn an_unscripted_error_names_the_argv() {
    let fake = FakeProber::new();
    let unscripted = argv(&["ollama", "list"]);
    match fake.run_probe(&unscripted, &[], TIMEOUT) {
        ProbeResult::Error(reason) => {
            assert!(
                reason.contains("ollama") && reason.contains("list"),
                "the error names the argv it did not know, got: {reason}"
            );
        }
        other => panic!("expected an error, got {other:?}"),
    }
}

#[test]
fn scripting_an_argv_again_replaces_its_result() {
    let fake = FakeProber::new();
    let probe = argv(&["ollama", "list"]);
    fake.script(probe.clone(), ProbeResult::Ok);
    assert_eq!(fake.run_probe(&probe, &[], TIMEOUT), ProbeResult::Ok);

    fake.script(probe.clone(), ProbeResult::Error("gone".into()));
    assert_eq!(
        fake.run_probe(&probe, &[], TIMEOUT),
        ProbeResult::Error("gone".into())
    );
}

#[test]
fn every_run_is_recorded_with_its_expect_and_timeout() {
    let fake = FakeProber::new();
    let scripted = argv(&["ollama", "list"]);
    let unscripted = argv(&["nope"]);
    fake.script(scripted.clone(), ProbeResult::Ok);
    assert!(fake.calls().is_empty());

    let expect = vec!["qwen38".to_owned(), "llama".to_owned()];
    fake.run_probe(&scripted, &expect, Duration::from_secs(3));
    fake.run_probe(&unscripted, &[], Duration::from_millis(250));
    fake.run_probe(&scripted, &[], Duration::from_secs(1));

    assert_eq!(
        fake.calls(),
        vec![
            ProbeCall {
                argv: scripted.clone(),
                expect,
                timeout: Duration::from_secs(3),
            },
            ProbeCall {
                argv: unscripted,
                expect: Vec::new(),
                timeout: Duration::from_millis(250),
            },
            ProbeCall {
                argv: scripted,
                expect: Vec::new(),
                timeout: Duration::from_secs(1),
            },
        ]
    );
}

#[test]
fn the_fake_prober_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<FakeProber>();
}
