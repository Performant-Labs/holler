//! #647 part 2, wrapper forwarding (brief W1 and W3): the wiring's two host wrappers forward
//! the required `HostPort::attached_clients` and (#831) `HostPort::foreground_pids` faithfully.
//! Neither has a default, so a wrapper that leaves one out does not compile; these tests catch the other failure, a
//! wrapper that compiles and forwards wrongly (answers a constant, drops the name, calls
//! another inner method). Over a recording inner host that answers `Ok(7)` for one name and an
//! error for another, each wrapper answers what the inner host did, and the inner host saw one
//! `attached_clients` call per ask, with the name given.
//!
//! The harness side (`agent_view` through `WiredHarness`) is `bind.rs`'s recording harness
//! (W4); the host conformance suite over both wrappers is `bind.rs` too (W2).

use std::sync::{Arc, Mutex, PoisonError};

use holler_cli::pane::wiring::bind::{BindingHost, Bindings, WiredHost};
use holler_pane::{Argv, HostPort, PaneError, PaneName};

/// The pane whose session has seven clients.
const SEVEN: &str = "demo-c1r1";
/// The pane whose count the inner host cannot read.
const FAILING: &str = "demo-c2r1";

fn name(text: &str) -> PaneName {
    PaneName::parse(text).unwrap()
}

fn unreadable() -> PaneError {
    PaneError::Unavailable {
        what: "tmux list-panes printed a made-up line".to_owned(),
    }
}

/// A host that answers `attached_clients` distinctively and records every call by method and
/// name. Every other method is a call it records and answers `Ok`, so a wrapper that forwards
/// `attached_clients` to another method shows in the log.
#[derive(Default)]
struct Counting {
    calls: Mutex<Vec<(&'static str, String)>>,
}

impl Counting {
    fn note(&self, method: &'static str, name: &PaneName) {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((method, name.to_string()));
    }

    fn calls(&self) -> Vec<(&'static str, String)> {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl HostPort for Counting {
    fn ensure_session(&self, name: &PaneName, _: &str) -> Result<(), PaneError> {
        self.note("ensure_session", name);
        Ok(())
    }
    fn run(&self, name: &PaneName, _: &str, _: &Argv) -> Result<(), PaneError> {
        self.note("run", name);
        Ok(())
    }
    fn stop_owned(&self, name: &PaneName) -> Result<(), PaneError> {
        self.note("stop_owned", name);
        Ok(())
    }
    fn ps(&self, name: &PaneName) -> Result<Vec<u32>, PaneError> {
        self.note("ps", name);
        Ok(vec![4242])
    }
    fn foreground_commands(&self, name: &PaneName) -> Result<Vec<String>, PaneError> {
        self.note("foreground_commands", name);
        Ok(vec!["sh".to_owned()])
    }
    fn foreground_pids(&self, name: &PaneName) -> Result<Vec<u32>, PaneError> {
        self.note("foreground_pids", name);
        if name.as_str() == SEVEN {
            Ok(vec![4242, 4243])
        } else {
            Err(unreadable())
        }
    }
    fn attached_clients(&self, name: &PaneName) -> Result<u32, PaneError> {
        self.note("attached_clients", name);
        if name.as_str() == SEVEN {
            Ok(7)
        } else {
            Err(unreadable())
        }
    }
}

/// Ask `wrapper` for both panes' counts: it answers the inner host's 7 and its error, and the
/// inner host saw exactly the two `attached_clients` calls, each with the name asked.
fn assert_forwards(wrapper: &dyn HostPort, inner: &Counting, what: &str) {
    assert_eq!(wrapper.attached_clients(&name(SEVEN)), Ok(7), "{what}");
    assert_eq!(
        wrapper.attached_clients(&name(FAILING)),
        Err(unreadable()),
        "{what}: the inner host's error, as it is"
    );
    assert_eq!(
        wrapper.foreground_pids(&name(SEVEN)),
        Ok(vec![4242, 4243]),
        "{what}: foreground_pids, as the inner host answered"
    );
    assert_eq!(
        wrapper.foreground_pids(&name(FAILING)),
        Err(unreadable()),
        "{what}: the inner host's error from foreground_pids, as it is"
    );
    assert_eq!(
        inner.calls(),
        vec![
            ("attached_clients", SEVEN.to_owned()),
            ("attached_clients", FAILING.to_owned()),
            ("foreground_pids", SEVEN.to_owned()),
            ("foreground_pids", FAILING.to_owned()),
        ],
        "{what}: one inner call per ask, with the name given"
    );
}

#[test]
fn the_binding_host_forwards_attached_clients() {
    let inner = Arc::new(Counting::default());
    let wrapper = BindingHost::new(inner.clone(), Bindings::new());
    assert_forwards(&wrapper, &inner, "BindingHost");
}

#[test]
fn the_wired_host_forwards_attached_clients() {
    let inner = Arc::new(Counting::default());
    let port: Arc<dyn HostPort> = inner.clone();
    let wrapper = WiredHost::new(Ok(port));
    assert_forwards(&wrapper, &inner, "WiredHost");
}

/// The production host is `BindingHost` over `WiredHost`: the stack forwards too.
#[test]
fn the_binding_host_over_the_wired_host_forwards_attached_clients() {
    let inner = Arc::new(Counting::default());
    let port: Arc<dyn HostPort> = inner.clone();
    let wired: Arc<dyn HostPort> = Arc::new(WiredHost::new(Ok(port)));
    let wrapper = BindingHost::new(wired, Bindings::new());
    assert_forwards(&wrapper, &inner, "BindingHost over WiredHost");
}

/// W3: a `WiredHost` built from a misconfiguration answers that error from `attached_clients`,
/// as from every other method, and never a count (nor, #831, a list of pids).
#[test]
fn a_misconfigured_wired_host_answers_its_error_from_attached_clients() {
    let wired = WiredHost::new(Err(PaneError::Usage {
        message: "HOLLER_TMUX_SOCKET must be an absolute path".to_owned(),
    }));
    let error = wired
        .attached_clients(&name(SEVEN))
        .expect_err("a misconfigured host has no count");
    let pids = wired
        .foreground_pids(&name(SEVEN))
        .expect_err("a misconfigured host has no pids");
    assert_eq!(pids.code(), "usage", "{pids}");
    assert!(pids.to_string().contains("HOLLER_TMUX_SOCKET"), "{pids}");
    assert_eq!(error.code(), "usage", "{error}");
    assert!(error.to_string().contains("HOLLER_TMUX_SOCKET"), "{error}");
}
