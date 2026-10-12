//! The session task's rebind and the one source of its attach id (#839, D2 and P2; Acceptance 9).
//! A rebind writes the new id to the task's own configuration once the new driver has attached,
//! so the 30 s attach retry (`attempt_initial_attach`, what the retry timer calls) re-attaches the
//! new id after the driver was dropped, as a turn that ended in error drops it, and the presence
//! mirror advertises it.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #839

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::{broadcast, oneshot};

use super::*;
use crate::config::Interrupt;

#[path = "../../../tests/http_attach_driver_test/fake_server.rs"]
#[allow(dead_code)] // #839: these tests use a few of the fake's controls
mod fake_server;

const OLD: &str = "ses_000000000000000000000000a1";
const NEW: &str = "ses_000000000000000000000000b2";

fn config(endpoint: &str, id: &str) -> SessionConfig {
    SessionConfig {
        name: SessionName::parse("alpha").unwrap(),
        harness: "opencode".to_owned(),
        mode: SessionMode::Attach,
        command: None,
        cwd: None,
        env: None,
        interrupt: Interrupt::Http,
        endpoint: Some(endpoint.to_owned()),
        session_id: Some(id.to_owned()),
        auth_method: None,
    }
}

/// The existence checks the fake saw from request `since` on.
fn existence_checks(server: &fake_server::FakeServer, since: usize) -> Vec<String> {
    server.requests()[since..]
        .iter()
        .filter(|r| r.method == "GET" && r.path.contains("/session/"))
        .map(|r| r.path.clone())
        .collect()
}

#[tokio::test]
async fn the_attach_retry_after_a_rebind_attaches_the_new_id() {
    let server = fake_server::FakeServer::start().await;
    let presence = Arc::new(Mutex::new(SessionPresence::default()));
    let (presence_tx, _rx) = broadcast::channel(8);
    let name = SessionName::parse("alpha").unwrap();
    let mut inner = Inner::new(
        config(&server.endpoint(), OLD),
        Arc::clone(&presence),
        presence_tx,
        name,
    );
    attempt_initial_attach(&mut inner).await;
    assert!(inner.driver.is_some(), "attached the configured id");

    let (reply_tx, reply_rx) = oneshot::channel();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let rebind = SessionCommand::Rebind {
        harness_session_id: NEW.to_owned(),
        deadline,
        reply_tx,
    };
    assert!(
        dispatch_command(&mut inner, Some(rebind)).await,
        "the task goes on"
    );
    assert_eq!(
        reply_rx.await.unwrap().map_err(|e| e.message),
        Ok(NEW.to_owned())
    );
    assert_eq!(
        inner.config.session_id.as_deref(),
        Some(NEW),
        "the one source holds the new id"
    );
    assert_eq!(
        presence.lock().unwrap().harness_session_id.as_deref(),
        Some(NEW),
        "mirrored"
    );

    if let Some(driver) = inner.driver.take() {
        let _ = driver.shutdown().await;
    }
    let since = server.requests().len();
    attempt_initial_attach(&mut inner).await;
    assert!(inner.driver.is_some(), "the retry re-attached");
    let checks = existence_checks(&server, since);
    assert!(
        !checks.is_empty() && checks.iter().all(|p| p.ends_with(NEW)),
        "{checks:?}"
    );
    assert_eq!(
        presence.lock().unwrap().harness_session_id.as_deref(),
        Some(NEW)
    );
}

/// A command dequeued after its deadline does nothing: the binding and the configuration stay.
#[tokio::test]
async fn a_rebind_dequeued_after_its_deadline_changes_nothing() {
    let server = fake_server::FakeServer::start().await;
    let presence = Arc::new(Mutex::new(SessionPresence::default()));
    let (presence_tx, _rx) = broadcast::channel(8);
    let name = SessionName::parse("alpha").unwrap();
    let mut inner = Inner::new(
        config(&server.endpoint(), OLD),
        Arc::clone(&presence),
        presence_tx,
        name,
    );
    attempt_initial_attach(&mut inner).await;
    let since = server.requests().len();
    let (reply_tx, reply_rx) = oneshot::channel();
    let deadline = tokio::time::Instant::now() - Duration::from_millis(1);
    let rebind = SessionCommand::Rebind {
        harness_session_id: NEW.to_owned(),
        deadline,
        reply_tx,
    };
    dispatch_command(&mut inner, Some(rebind)).await;
    let answer = reply_rx.await.unwrap();
    assert!(answer.is_err(), "{answer:?}");
    assert_eq!(inner.config.session_id.as_deref(), Some(OLD));
    assert!(
        existence_checks(&server, since).is_empty(),
        "no attach was tried"
    );
}
