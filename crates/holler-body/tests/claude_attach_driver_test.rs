#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #850
//! The Claude Code attach driver (#850, Acceptance 2; ADR 0014 points 8 to 10: the delivery rule,
//! the turn rule and the cancel rule), over the test kit's `FakeClaude` as the `HarnessPort` the
//! driver is handed (D1): it delivers with `send_prompt`, reads the process's state with
//! `session_activity`, finds the process that holds the row's session with `locate_process` on
//! every call, and calls nothing else (I7: never `login_status`).
//!
//! **The contract these tests pin for F** (the names F must give, mirroring `HttpAttachDriver`):
//! `holler_body::claude_attach_driver::ClaudeAttachDriver::attach(&SessionConfig, Arc<dyn
//! HarnessPort>) -> Result<Self, DriverError>` (async; a row nobody holds is
//! `DriverError::NotFound`), and on the value `prompt`, `cancel`, `answer`, `pending`, `status`,
//! `session_id` and `shutdown` with `HttpAttachDriver`'s signatures and the same `DriverEvent`,
//! `DriverEventStream`, `DriverError`, `Status` and `StopReason` types.
//!
//! A turn is observed through the stream a collector task drains, and the fake's process is moved
//! through `busy`, `waiting` and `idle` by the test. Readiness is observed (the fake's call log
//! says how many ticks the driver has polled), never guessed with a fixed sleep. Two cases take
//! real time because the rule is a time rule: the 5 s start wait and the 10 s `OP_BOUND`.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use holler_body::acp_driver::{DriverError, DriverEvent, DriverState, Status, StopReason};
use holler_body::claude_attach_driver::ClaudeAttachDriver;
use holler_body::config::{Interrupt, SessionConfig, SessionMode};
use holler_pane::ports::SessionChoice;
use holler_pane::{HarnessPort, PaneName, ServeSpec};
use holler_pane_testkit::claude::{ClaudeOp, FakeClaude};
use holler_proto::SessionName;

/// The pane's session of record: a placeholder UUID.
const SESSION: &str = "0b6f8f2e-5c1d-4a43-9d51-2f1f6a7f0a11";
const PANE: &str = "c1r1";
const WAIT: Duration = Duration::from_secs(15);

fn config() -> SessionConfig {
    SessionConfig {
        name: SessionName::parse(PANE).unwrap(),
        harness: "claude".to_owned(),
        mode: SessionMode::Attach,
        command: None,
        cwd: None,
        env: None,
        interrupt: Interrupt::Acp,
        endpoint: None,
        session_id: Some(SESSION.to_owned()),
        auth_method: None,
    }
}

/// A process of the pane `PANE` on `SESSION`, started as the fake's own argv describes (so it
/// accepts a cross-session message as Holler's launch does); `resume` starts it on a session that
/// already has a conversation.
fn start_process(fake: &FakeClaude, resume: bool) -> u32 {
    let choice = if resume {
        SessionChoice::Resume(SESSION.to_owned())
    } else {
        SessionChoice::Fresh(SESSION.to_owned())
    };
    let argv = fake
        .process_command(
            &PaneName::parse(PANE).unwrap(),
            &ServeSpec::default().with_cwd("/srv/demo"),
            &choice,
        )
        .unwrap();
    fake.start(&argv).expect("the fake's own argv").unwrap()
}

/// A fake with one live process on `SESSION`, and the driver attached to it.
async fn attached() -> (Arc<FakeClaude>, u32, ClaudeAttachDriver) {
    let fake = Arc::new(FakeClaude::new());
    let pid = start_process(&fake, false);
    let port: Arc<dyn HarnessPort> = fake.clone();
    let driver = ClaudeAttachDriver::attach(&config(), port)
        .await
        .expect("a process holds the session, so the attach succeeds");
    (fake, pid, driver)
}

type Seen = Arc<Mutex<Vec<DriverEvent>>>;

/// Drain a turn's stream into a list a test can poll.
fn collect(mut stream: holler_body::acp_driver::DriverEventStream) -> Seen {
    let seen: Seen = Arc::default();
    let sink = Arc::clone(&seen);
    tokio::spawn(async move {
        while let Some(event) = stream.next().await {
            sink.lock().unwrap_or_else(PoisonError::into_inner).push(event);
        }
    });
    seen
}

fn events(seen: &Seen) -> Vec<DriverEvent> {
    seen.lock().unwrap_or_else(PoisonError::into_inner).clone()
}

/// Wait, bounded, until `check` holds.
async fn until(what: &str, timeout: Duration, mut check: impl FnMut() -> bool) {
    let deadline = Instant::now() + timeout;
    while !check() {
        assert!(Instant::now() < deadline, "never: {what}");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// The reason the turn ended, once it has.
async fn done(seen: &Seen, timeout: Duration) -> StopReason {
    until("the turn ends", timeout, || {
        events(seen).iter().any(|e| matches!(e, DriverEvent::Done(_)))
    })
    .await;
    events(seen)
        .into_iter()
        .find_map(|e| match e {
            DriverEvent::Done(reason) => Some(reason),
            _ => None,
        })
        .unwrap()
}

fn texts(seen: &Seen) -> Vec<String> {
    events(seen)
        .into_iter()
        .filter_map(|e| match e {
            DriverEvent::Chunk(text) => Some(text),
            _ => None,
        })
        .collect()
}

/// How many times the driver has read the process's state.
fn polls(fake: &FakeClaude) -> usize {
    fake.faults()
        .calls()
        .iter()
        .filter(|op| **op == ClaudeOp::SessionActivity)
        .count()
}

/// Wait until the driver has read the process's state twice since this call: the first read may
/// have begun before the test's last change, the second began after the first was handled.
async fn until_polled_twice(fake: &FakeClaude) {
    let before = polls(fake);
    until("the driver polls twice", WAIT, || polls(fake) >= before + 2).await;
}

fn inbox(fake: &FakeClaude, pid: u32) -> Vec<String> {
    fake.process(pid).map(|p| p.inbox).unwrap_or_default()
}

/// Send `text` and wait until the process has it. Returns the turn's events.
async fn send(fake: &FakeClaude, pid: u32, driver: &ClaudeAttachDriver, text: &str) -> Seen {
    let seen = collect(driver.prompt(text).await);
    until("the message reaches the process", WAIT, || {
        inbox(fake, pid).last().map(String::as_str) == Some(text)
    })
    .await;
    seen
}

/// A short turn: the process is seen busy, then idle.
async fn run_turn(fake: &FakeClaude, pid: u32, driver: &ClaudeAttachDriver, text: &str) -> StopReason {
    let seen = send(fake, pid, driver, text).await;
    fake.stall(pid).unwrap();
    until_polled_twice(fake).await;
    fake.set_idle(pid).unwrap();
    done(&seen, WAIT).await
}

// --- the attach (the delivery rule) ------------------------------------------------------------

#[tokio::test]
async fn attach_fails_closed_when_no_process_holds_the_session() {
    let fake = Arc::new(FakeClaude::new());
    let port: Arc<dyn HarnessPort> = fake.clone();
    match ClaudeAttachDriver::attach(&config(), port).await {
        Err(DriverError::NotFound(_)) => {}
        Err(other) => panic!("expected NotFound, got {other}"),
        Ok(_) => panic!("an attach to a session nobody holds must fail closed"),
    }
    assert!(fake.processes().is_empty(), "nothing was created or started");
    let writes = [
        ClaudeOp::SendPrompt,
        ClaudeOp::ProcessCommand,
        ClaudeOp::NewSessionId,
        ClaudeOp::Abort,
        ClaudeOp::LoginStatus,
    ];
    let calls = fake.faults().calls();
    assert!(
        !calls.iter().any(|op| writes.contains(op)),
        "the attach only looks: {calls:?}"
    );
}

#[tokio::test]
async fn a_prompt_writes_one_frame_with_the_text_and_streams_no_text() {
    let (fake, pid, driver) = attached().await;
    assert_eq!(driver.session_id(), SESSION);
    assert_eq!(driver.status(), Status::Idle);
    let seen = send(&fake, pid, &driver, "hello pane").await;
    assert_eq!(inbox(&fake, pid), vec!["hello pane".to_owned()], "one message, the text as given");
    assert_eq!(driver.status(), Status::Working, "working from when the adapter accepted it");
    fake.stall(pid).unwrap();
    until_polled_twice(&fake).await;
    fake.set_idle(pid).unwrap();
    assert_eq!(done(&seen, WAIT).await, StopReason::EndTurn);
    assert!(texts(&seen).is_empty(), "the driver reads nothing the process prints: {:?}", events(&seen));
    assert_eq!(inbox(&fake, pid).len(), 1, "nothing was sent twice");
    assert_eq!(driver.status(), Status::Idle);
}

#[tokio::test]
async fn a_turn_ends_end_turn_when_the_process_is_idle_again_after_busy() {
    let (fake, pid, driver) = attached().await;
    let seen = send(&fake, pid, &driver, "work").await;
    fake.stall(pid).unwrap();
    until_polled_twice(&fake).await;
    assert!(
        !events(&seen).iter().any(|e| matches!(e, DriverEvent::Done(_))),
        "a busy process is not a finished turn"
    );
    let idle_at = Instant::now();
    fake.set_idle(pid).unwrap();
    assert_eq!(done(&seen, WAIT).await, StopReason::EndTurn);
    assert!(
        idle_at.elapsed() < Duration::from_secs(4),
        "idle after busy ends the turn at the next ticks, not at the start wait: {:?}",
        idle_at.elapsed()
    );
}

/// A turn too short to see: the process never reads `busy` between two polls. The turn does not
/// end while the start wait runs (the process reads idle, as it would before it picked the message
/// up) and ends `end_turn` once the process has been idle 5 s after the write.
#[tokio::test]
async fn a_turn_too_short_to_see_ends_after_the_start_wait() {
    let (fake, pid, driver) = attached().await;
    let started = Instant::now();
    let seen = send(&fake, pid, &driver, "quick").await;
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert!(
        !events(&seen).iter().any(|e| matches!(e, DriverEvent::Done(_))),
        "still inside the start wait after 2 s: {:?}",
        events(&seen)
    );
    assert_eq!(done(&seen, WAIT).await, StopReason::EndTurn);
    let took = started.elapsed();
    assert!(
        took >= Duration::from_secs(4) && took < Duration::from_secs(9),
        "ended about 5 s after the write, took {took:?}"
    );
}

#[tokio::test]
async fn a_relaunch_between_prompts_needs_nothing_from_the_driver() {
    let (fake, first, driver) = attached().await;
    assert_eq!(run_turn(&fake, first, &driver, "one").await, StopReason::EndTurn);

    // `holler interrupt --pane` relaunches the pane on its session of record: a new pid.
    fake.sigint(first).unwrap();
    let second = start_process(&fake, true);
    assert_ne!(first, second, "a relaunch changes the pid and keeps the session");

    assert_eq!(run_turn(&fake, second, &driver, "two").await, StopReason::EndTurn);
    assert_eq!(inbox(&fake, second), vec!["two".to_owned()], "the new process got the second message");
    assert_eq!(inbox(&fake, first), Vec::<String>::new(), "the old one is gone");
}

#[tokio::test]
async fn a_relaunch_during_a_turn_ends_it_cancelled() {
    let (fake, first, driver) = attached().await;
    let seen = send(&fake, first, &driver, "long").await;
    fake.stall(first).unwrap();
    until_polled_twice(&fake).await;

    fake.sigint(first).unwrap();
    let second = start_process(&fake, true);
    assert_eq!(done(&seen, WAIT).await, StopReason::Cancelled);
    assert_eq!(
        inbox(&fake, second),
        Vec::<String>::new(),
        "the turn is lost, never re-sent to the new process"
    );
    assert_eq!(driver.status(), Status::Idle);
}

#[tokio::test]
async fn a_vanished_process_ends_the_turn_error() {
    let (fake, pid, driver) = attached().await;
    let seen = send(&fake, pid, &driver, "long").await;
    fake.stall(pid).unwrap();
    until_polled_twice(&fake).await;
    fake.sigint(pid).unwrap();
    assert_eq!(done(&seen, WAIT).await, StopReason::Error);
    assert_eq!(driver.status(), Status::Idle);
}

/// The adapter refuses the message (a process run by hand holds it behind a dialog): the turn ends
/// `error` after one text chunk, which carries the adapter's own message and quotes nothing the
/// process printed.
#[tokio::test]
async fn a_refused_delivery_ends_the_turn_error_after_one_chunk_that_quotes_no_process_output() {
    let fake = Arc::new(FakeClaude::new());
    let pid = fake.run_by_hand(&PaneName::parse(PANE).unwrap(), SESSION);
    let port: Arc<dyn HarnessPort> = fake.clone();
    let driver = ClaudeAttachDriver::attach(&config(), port).await.unwrap();
    let seen = collect(driver.prompt("secret-looking text").await);
    assert_eq!(done(&seen, WAIT).await, StopReason::Error);
    let chunks = texts(&seen);
    assert_eq!(chunks.len(), 1, "one chunk: {:?}", events(&seen));
    assert!(chunks[0].contains("behind a dialog"), "the adapter's own message: {}", chunks[0]);
    assert!(!chunks[0].contains("secret-looking text"), "the message text is not echoed: {}", chunks[0]);
    assert_eq!(inbox(&fake, pid), Vec::<String>::new(), "nothing was delivered");
    let last = events(&seen).last().cloned();
    assert_eq!(last, Some(DriverEvent::Done(StopReason::Error)), "the chunk comes before the end");
}

/// A permission wait is `input-required` with nothing pending, and busy again is `working`.
#[tokio::test]
async fn a_permission_wait_is_input_required_with_nothing_pending() {
    let (fake, pid, driver) = attached().await;
    let seen = send(&fake, pid, &driver, "needs a permission").await;
    fake.stall(pid).unwrap();
    until_polled_twice(&fake).await;

    fake.hold_permission(pid).unwrap();
    until("input-required is reported", WAIT, || {
        events(&seen).contains(&DriverEvent::State(DriverState::InputRequired))
    })
    .await;
    assert_eq!(driver.status(), Status::InputRequired);
    assert!(driver.pending().is_empty(), "nothing is pending: a permission is answered in the pane");

    fake.stall(pid).unwrap();
    until("working again is reported", WAIT, || {
        let seen = events(&seen);
        let at = seen.iter().position(|e| *e == DriverEvent::State(DriverState::InputRequired));
        at.is_some_and(|at| seen[at..].contains(&DriverEvent::State(DriverState::Working)))
    })
    .await;
    assert_eq!(driver.status(), Status::Working);

    until_polled_twice(&fake).await;
    fake.set_idle(pid).unwrap();
    assert_eq!(done(&seen, WAIT).await, StopReason::EndTurn);
    assert!(texts(&seen).is_empty());
}

// --- cancel, replace and answer (the cancel rule) ----------------------------------------------

#[tokio::test]
async fn answer_is_always_refused() {
    let (fake, pid, driver) = attached().await;
    let idle = driver.answer("0").await.expect_err("nothing to answer");
    assert!(!idle.message().is_empty());

    let seen = send(&fake, pid, &driver, "needs a permission").await;
    fake.stall(pid).unwrap();
    until_polled_twice(&fake).await;
    fake.hold_permission(pid).unwrap();
    until("input-required is reported", WAIT, || {
        events(&seen).contains(&DriverEvent::State(DriverState::InputRequired))
    })
    .await;
    let waiting = driver.answer("0").await.expect_err("a permission is answered in the pane");
    assert!(waiting.message().contains("pane"), "{}", waiting.message());
    assert_eq!(fake.process(pid).unwrap().inbox.len(), 1, "an answer writes nothing");
}

#[tokio::test]
async fn cancel_in_flight_is_refused_naming_the_pane_verb_and_cancel_when_idle_applies() {
    let (fake, pid, driver) = attached().await;
    assert!(driver.cancel().await.is_ok(), "nothing in flight: it applies at once");

    let seen = send(&fake, pid, &driver, "long").await;
    fake.stall(pid).unwrap();
    until_polled_twice(&fake).await;
    match driver.cancel().await {
        Err(DriverError::Cancel(message)) => assert!(
            message.contains(&format!("holler interrupt --pane {PANE}")),
            "the refusal names the verb that relaunches the pane: {message}"
        ),
        other => panic!("a cancel in flight must be refused, got {other:?}"),
    }
    assert!(
        !events(&seen).iter().any(|e| matches!(e, DriverEvent::Done(_))),
        "the turn runs on after the refusal"
    );
    assert!(
        !fake.faults().calls().contains(&ClaudeOp::Abort),
        "no abort, no signal, no keypress: {:?}",
        fake.faults().calls()
    );
    fake.set_idle(pid).unwrap();
    assert_eq!(done(&seen, WAIT).await, StopReason::EndTurn);
}

// --- what the driver may call (I5, I7) -----------------------------------------------------------

#[tokio::test]
async fn the_driver_never_calls_login_status() {
    let (fake, pid, driver) = attached().await;
    let setup = fake.faults().calls().len(); // the test's own start of the process is not the driver's
    assert_eq!(run_turn(&fake, pid, &driver, "hi").await, StopReason::EndTurn);
    driver.shutdown().await.unwrap();
    let all = fake.faults().calls();
    let calls = all[setup..].to_vec();
    assert!(!calls.contains(&ClaudeOp::LoginStatus), "I7: the login is never read: {calls:?}");
    let allowed = [ClaudeOp::LocateProcess, ClaudeOp::SessionActivity, ClaudeOp::SendPrompt];
    assert!(
        calls.iter().all(|op| allowed.contains(op)),
        "the driver calls only send_prompt, session_activity and locate_process: {calls:?}"
    );
    assert!(fake.process(pid).is_some(), "shutdown leaves the process running");
}

/// I5: a port call that does not return is bounded by the driver (`OP_BOUND`, 10 s), so a wedged
/// `claude` ends the turn `error` and the session task is never stuck behind it.
#[tokio::test]
async fn a_stalled_port_call_is_bounded() {
    let (fake, _pid, driver) = attached().await;
    fake.faults().set_delay(Some(Duration::from_secs(12)));
    let started = Instant::now();
    let seen = collect(driver.prompt("stuck").await);
    assert_eq!(done(&seen, Duration::from_secs(14)).await, StopReason::Error);
    let took = started.elapsed();
    assert!(took < Duration::from_millis(11_500), "the bound fired before the call returned: {took:?}");
    assert_eq!(driver.status(), Status::Idle);
}
