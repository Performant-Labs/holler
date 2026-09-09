#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #149
//! `stub-acp` — a deterministic ACP **v2** agent stub (story #130).
//!
//! The test suite spawns this binary instead of a real agent. It is a real
//! process speaking JSON-RPC 2.0 over stdio (newline-delimited JSON, one
//! message per line). It is fully deterministic, cross-OS, and — unlike a
//! model — can simulate a permission request so the `blocked` state is
//! testable without a model.
//!
//! Constraints (see the issue): **no tokio** — a blocking `std::io` line
//! loop plus one worker thread; **no shell**; stdout carries *only* JSON
//! lines (diagnostics go to stderr).
//!
//! Concurrency model (one shared channel, no atomics, no polling loop): the
//! main thread reads stdin and forwards every inbound message to a single
//! worker over an `mpsc` channel. The worker is the **sole** reader of that
//! channel and the **sole writer** to stdout. It keeps a small `pending`
//! stash for inbound messages that are *not* the next thing the in-flight
//! turn needs (an unrelated request/response, or a `session/cancel` that
//! arrives before the permission answer the turn is currently parked on).
//! Two properties make this both simple and race-free:
//!
//!   * the worker paces each inter-chunk gap with `rx.recv_timeout(delay)`,
//!     preferring the `pending` stash. So a `session/cancel` (or the
//!     permission answer) delivered *during* a gap is picked up by that `recv`
//!     immediately — no polling loop, no shared flag, no atomic visibility
//!     window — and the turn observes it *before* the next chunk is emitted;
//!   * `drain` processes the stash in arrival order, so a cancel that landed
//!     while the turn was parked on the permission gate is consumed before the
//!     answer can resume it — the turn reports `cancelled`, never a stale
//!     `end_turn`.
//!
//! The turn itself is driven by `advance`, a pure emitter: it sends the next
//! single step of the turn (a `state_update`, a chunk, the permission request,
//! or the terminal response) and returns, so the worker can slot its
//! `recv_timeout` pacing (and any cancel/answer arriving in the gap) between
//! steps. Resuming from the permission gate continues with the remaining
//! chunks — it does not re-emit the `running` state.
//!
//! `--slow` stretches the inter-chunk gap so a `session/cancel` can land
//! mid-turn (and is therefore guaranteed to be observed between chunks).
//!
//! Method names below are ACP v2 as shipped in `agent-client-protocol` 2.1.0
//! (`session/request_permission`, `session/update`, …). If a name here stops
//! matching the schema, fix it here and note it in the story.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::thread;
use std::time::Duration;

use serde_json::{json, Value};

/// ACP v2 protocol version (the `initialize` negotiation constant).
const PROTOCOL_VERSION: u8 = 2;
/// Every session the stub creates is addressed as this.
const SESSION_ID: &str = "stub";
/// Synthetic id for the `session/request_permission` request raised under
/// `--ask-permission`; the client's matching response (id 10) resumes the turn.
const PERMISSION_REQUEST_ID: i64 = 10;
/// Synthetic id for the `elicitation/create` request raised under
/// `--ask-elicitation`/`--ask-elicitation-url`; the client's matching response
/// (id 11) resumes the turn. Distinct from [`PERMISSION_REQUEST_ID`] so a
/// driver test can tell the two gate kinds apart on the wire.
const ELICITATION_REQUEST_ID: i64 = 11;
/// JSON-RPC "Method not found" code (for unknown inbound methods).
const ERROR_METHOD_NOT_FOUND: i64 = -32601;
/// In-band EOF marker `main` sends (instead of relying on a channel drop): a
/// `RecvTimeoutError::Disconnected` is indistinguishable from a real close and
/// std reports it the instant a sender is dropped, which would cut an
/// in-flight turn short. A real in-band marker is unambiguous.
const STDIN_EOF: &str = "__eof__";

/// True if `msg` is the in-band EOF marker `main` emits at stdin EOF.
fn is_eof_marker(msg: &Value) -> bool {
    msg.get(STDIN_EOF).is_some()
}

impl Config {
    /// Which gate this invocation raises mid-turn (at most one of the
    /// `--ask-*` flags is meaningful per invocation; the first one set wins).
    fn gate(&self) -> Option<GateKind> {
        if self.ask_permission {
            Some(GateKind::Permission)
        } else if self.ask_elicitation {
            Some(GateKind::Elicitation)
        } else if self.ask_elicitation_url {
            Some(GateKind::ElicitationUrl)
        } else {
            None
        }
    }
}

// Four independent CLI switches (`--ask-permission`/`--ask-elicitation`/
// `--ask-elicitation-url`/`--crash-after-prompt`), each a simple on/off flag a
// test passes in isolation — not overlapping machine states (`Config::gate`
// already picks at most one of the first three), so an enum would just move
// the same four booleans one level down without adding meaning. Same
// reasoning as `Turn`'s allow just below.
#[allow(clippy::struct_excessive_bools)] // #188
#[derive(Clone)]
struct Config {
    /// Advertised session names (comma-separated). Parsed but not otherwise
    /// used by the wire behaviour: `session/new` always returns `SESSION_ID`.
    // Forward-contract stub path: exercised by a later story (the file-level
    // `dead_code` allow covers this — see the `// #149` blanket at the top).
    sessions: String,
    /// Number of `agent_message_chunk` notifications emitted per prompt.
    chunks: usize,
    /// Inter-chunk delay in ms: ~50 normally, 200 under `--slow` so a
    /// `session/cancel` can land mid-turn and be observed between chunks.
    chunk_delay_ms: u64,
    ask_permission: bool,
    /// Raise a real multi-field `elicitation/create` (form mode, two enum
    /// properties: `color` single-select, `toppings` multi-select) instead of
    /// a permission request (story #188's answerable-blocking coverage).
    ask_elicitation: bool,
    /// Raise an `elicitation/create` in `url` mode — the one shape story
    /// #188's driver deliberately does not resolve (reported unsupported,
    /// never silently dropped).
    ask_elicitation_url: bool,
    crash_after_prompt: bool,
}

/// The in-flight turn's position. `emitted` counts `agent_message_chunk`
/// notifications already sent (0-based index of the next one);
/// `awaiting_gate` is true once a permission/elicitation request has been
/// raised and before the client's answer has resumed it. (The `session/prompt`
/// request is acked immediately when it arrives — see `route`'s
/// `"session/prompt"` arm — so there is no pending response id to track here
/// or anywhere else in the worker.)
/// The four booleans are deliberately kept as separate fields rather than folded
/// into a single state enum: the `advance` state machine below is a single
/// `match` over exactly these facets, and naming each one in the guard (rather
/// than decoding an enum) keeps every reachable/unreachable transition explicit
/// and self-documenting. A 4-boolean struct would otherwise trip
/// `clippy::struct-excessive-bools`, so the lint is suppressed *on this
/// fixture-only struct* (the workspace denies it for production code, where the
/// heuristic is sound); here the booleans are the machine's state, not a
/// signal that the struct is doing too much.
#[allow(clippy::struct_excessive_bools)] // #147
#[derive(Clone)]
struct Turn {
    /// True once the turn's `running` state_update has been sent (the first
    /// step). Kept separate from `emitted` (the count of chunks sent) so the
    /// chunk indices stay 0-based: `running` does not itself consume a chunk.
    started: bool,
    /// Count of `agent_message_chunk` notifications already sent.
    emitted: usize,
    /// True once a gate request (permission or elicitation — whichever
    /// `cfg.ask_*` selects) has been raised and before the client's answer
    /// (the fixed [`PERMISSION_REQUEST_ID`]/[`ELICITATION_REQUEST_ID`]) has
    /// resumed the turn. (Cleared by that answer; the turn then continues
    /// with the next chunk.) Named generically (not `awaiting_permission`)
    /// because story #188 added a second gate kind — `elicitation/create` —
    /// that parks the turn exactly the same way.
    awaiting_gate: bool,
    /// True once the gate request has been raised, for the life of the turn.
    /// Unlike `awaiting_gate` it is never cleared: it is what stops `advance`
    /// from re-firing the gate arm after the answer clears it (otherwise the
    /// `(emitted == 1, !awaiting)` arm would raise the request a second time
    /// and re-park the turn forever).
    gate_raised: bool,
    /// True from the moment a gate is raised (parked) until the turn's
    /// *resume* step has re-emitted `state_update:running`. A real ACP agent,
    /// when woken from a permission/elicitation block, re-announces `running`
    /// before continuing (that is exactly what the `--ask-permission` contract
    /// test pins: resume → `running` → remaining chunks). `advance` sets this
    /// when it raises the gate and clears it once the resume `running` has
    /// been sent, so the remaining chunks then stream out without a second
    /// `running`.
    resumed_running: bool,
}

/// Which gate (if any) this stub invocation raises mid-turn, and the fixed
/// request id the client answers it with.
#[derive(Clone, Copy, PartialEq, Eq)]
enum GateKind {
    Permission,
    Elicitation,
    ElicitationUrl,
}

fn main() {
    let cfg = parse_args();
    let (tx, rx) = std::sync::mpsc::channel::<Value>();

    // The worker is the sole reader of the channel and the sole stdout writer.
    // It reads stdin (via the main thread's forwarded messages) and, on EOF,
    // terminates *itself* (closing the channel and returning) so the process
    // exits cleanly without a thread still parked in `recv_timeout`. `rx` is
    // moved into the worker; the stdin forwarding below uses `tx`, which stays
    // on the main thread.
    let worker = thread::Builder::new()
        .name("stub-acp-worker".into())
        .spawn(move || worker_loop(&cfg, rx))
        .expect("spawn the stub-acp worker thread");

    // The main thread only reads stdin and forwards; it never touches stdout or
    // the worker's stash, so it cannot deadlock against the worker. The loop
    // exits on stdin EOF (or if the worker already went, so `send` errors).
    let stdin = std::io::stdin();
    for line in BufReader::new(stdin).lines() {
        let line = match line {
            Ok(line) => line,
            Err(_) => break, // EOF / closed stdin
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(trimmed) {
            Ok(m) => {
                if tx.send(m).is_err() {
                    // The worker already terminated (e.g. on an earlier EOF):
                    // stop feeding.
                    break;
                }
            }
            Err(e) => {
                eprintln!("stub-acp: ignoring malformed line: {e}");
            }
        }
    }
    // stdin reached EOF (or the worker already went). Signal EOF to the worker
    // with a real in-band marker (NOT just by dropping the channel): a
    // `RecvTimeoutError::Disconnected` is indistinguishable from a real close,
    // and std reports it the instant a sender is dropped, which would cut an
    // in-flight turn short (the worker would see `Disconnected` mid-turn and
    // emit only the *next* step, not the turn's full remainder). The marker, by
    // contrast, is an ordinary `Ok(msg)` the worker's loop reads in order: it
    // resolves any in-flight turn to its terminal response and then stops. The
    // worker keeps its own `tx` clone, so `drop(tx)` here does not close the
    // channel from under it — the marker is the authoritative EOF signal and the
    // drop merely lets `worker.join()` observe the worker's exit.
    tx.send(json!({ STDIN_EOF: true })).ok();
    drop(tx);
    let _ = worker.join();
}

/// Parse argv with a tiny `--flag` / `--flag value` scanner (no CLI lib, so
/// the stub stays a single file).
fn parse_args() -> Config {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut cfg = Config {
        sessions: "alpha,beta".into(),
        chunks: 3,
        chunk_delay_ms: 50,
        ask_permission: false,
        ask_elicitation: false,
        ask_elicitation_url: false,
        crash_after_prompt: false,
    };
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--sessions" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    cfg.sessions = v.clone();
                }
            }
            "--chunks" => {
                i += 1;
                cfg.chunks = args.get(i).and_then(|v| v.parse().ok()).unwrap_or(3);
            }
            "--slow" => cfg.chunk_delay_ms = 200,
            "--ask-permission" => cfg.ask_permission = true,
            "--ask-elicitation" => cfg.ask_elicitation = true,
            "--ask-elicitation-url" => cfg.ask_elicitation_url = true,
            "--crash-after-prompt" => cfg.crash_after_prompt = true,
            // Unknown / positional args are ignored.
            _ => {}
        }
        i += 1;
    }
    cfg
}

/// The sole worker: routes every inbound message, and — for an in-flight turn —
/// paces the gaps between its steps with `recv_timeout` so a cancel or the
/// permission answer arriving mid-gap is observed before the next emission. All
/// stdout writes go through the one `lock`, so the JSON-line stream is serial
/// and ordered.
fn worker_loop(cfg: &Config, rx: Receiver<Value>) {
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    let mut pending: VecDeque<Value> = VecDeque::new(); // inbound messages not yet routed
    let mut turn: Option<Turn> = None; // in-flight turn state, if any (the sole "is a turn active" flag: a `session/prompt` is acked immediately, so there is no separate pending-response id to track)

    loop {
        // Acquire the next inbound message: a stashed one first (so a
        // cancel/answer that arrived while we were parked is acted on
        // immediately), else ask the channel. We must distinguish three
        // outcomes — they are handled very differently:
        //   * a *stashed* message → act on it;
        //   * a *fresh* message from `recv` → route it;
        //   * a *timeout* (no traffic in the pacing gap) → NOT a message: it
        //     just means "the gap elapsed", so keep pacing any in-flight turn
        //     (this is what streams the chunks to their terminal response);
        //   * the channel is *closed* (stdin hit EOF) → the input stream ended.
        //
        // `recv_timeout` (the channel's timed receive) returns `Ok(msg)` when a
        // message is delivered, `Err(RecvTimeoutError::Timeout)` when the pacing
        // gap elapses with no traffic, and `Err(RecvTimeoutError::Disconnected)`
        // once `main` drops the channel (stdin EOF). A `VecDeque::pop_front` is
        // always `Option` (never errors). So: a stash hit is a message; a
        // `recv` `Ok` is a message; a `Timeout` is "the gap elapsed, no message"
        // (NOT an EOF — the channel is still open); a `Disconnected` (and the
        // stash empty) is the EOF path.
        enum Incoming {
            Message(Value),
            Timeout,
            Eof,
        }
        let incoming = if let Some(m) = pending.pop_front() {
            // A stashed inbound message is the highest-priority source: a
            // cancel or the permission answer that arrived while we were pacing
            // must be acted on immediately, before any new gap. (The in-band
            // EOF marker is never stashed — `main` sends it only after stdin
            // has fully drained, so it always reaches the worker via `recv`.)
            Incoming::Message(m)
        } else {
            match rx.recv_timeout(Duration::from_millis(cfg.chunk_delay_ms)) {
                // A real inbound message (or the in-band EOF marker `main`
                // sent) was delivered.
                Ok(m) => Incoming::Message(m),
                // The pacing gap elapsed with no traffic: NOT a message and NOT
                // an EOF — the channel is still open. Just "the gap elapsed",
                // so keep pacing any in-flight turn.
                Err(RecvTimeoutError::Timeout) => Incoming::Timeout,
                // The channel fully closed (every sender dropped, incl. the
                // worker's own). The normal EOF path is the in-band marker above
                // (`Ok(marker)`); this arm only fires if `main` died hard
                // (killed / panicked) before sending it. The worker must exit
                // rather than loop on a dead channel. The stash is empty here,
                // so a final `drain` emits any in-flight turn's terminal step.
                Err(RecvTimeoutError::Disconnected) => Incoming::Eof,
            }
        };

        match incoming {
            // A pacing gap elapsed with no inbound traffic. If a turn is in
            // flight, advance it one step (so its chunks keep streaming to the
            // terminal response); otherwise just keep looping, waiting for the
            // next inbound message (or EOF). Crucially this is NOT an EOF: the
            // channel is still open, so we must not break.
            Incoming::Timeout => {
                // A pacing gap elapsed with no inbound traffic. If a turn is in
                // flight, advance it one step (so its chunks keep streaming to
                // the terminal response); otherwise just keep looping. This is
                // NOT an EOF — the channel is still open, so we must not break.
                //
                // `drain` clears `*turn` the moment the turn resolves, so the
                // `is_some()` guard below makes subsequent gaps no-ops.
                if turn.is_some() {
                    drain(cfg, &mut lock, &mut pending, &mut turn);
                }
                continue;
            }
            // The input stream ended (in-band marker, or a hard channel close).
            // A bare EOF is NOT a cancel: we do not synthesize a
            // `session/cancel` or force an in-flight turn to `cancelled`. (A
            // real client that sent `session/cancel` already resolved the turn
            // via the routing below.) If a turn is still in flight it has not
            // resolved yet, so emit the next step (which, at the chunk ceiling,
            // is the `end_turn`/`cancelled` idle state_update the turn
            // genuinely resolves to). Then stop: the worker has emitted
            // everything it will. (We do NOT consume a further stash here: a
            // stash only holds unrelated traffic, and the marker path means
            // `main` is done sending, so draining further could only
            // re-process noise.)
            Incoming::Eof => {
                if turn.is_some() {
                    drain(cfg, &mut lock, &mut pending, &mut turn);
                }
                break;
            }
            Incoming::Message(msg) => {
                // The in-band EOF marker: stdin is closed. Treat it exactly
                // like the hard-close EOF path — resolve any in-flight turn to
                // its terminal step and stop. (It is never routed as a normal
                // message: it has no `method`/`id` a client would send.)
                if is_eof_marker(&msg) {
                    if turn.is_some() {
                        drain(cfg, &mut lock, &mut pending, &mut turn);
                    }
                    break;
                }
                // Route the message by kind (may start a turn, park it, resume
                // it, or resolve it to `cancelled`).
                route(&mut lock, &mut pending, &mut turn, &msg);
                // If routing resumed a parked turn (the gate answer), emit its
                // next step now (the remaining chunks). A cancel or a
                // completed turn has already resolved it, so `drain` is a no-op.
                if turn.is_some() {
                    drain(cfg, &mut lock, &mut pending, &mut turn);
                }
            }
        }
    }
}

/// Route one inbound message by its JSON-RPC `method` (or, for a client
/// *response*, by its `id`). Turns the message into the appropriate stub
/// reaction (a response, an error, a permission-resume, or a stash).
fn route(
    lock: &mut impl Write,
    pending: &mut VecDeque<Value>,
    turn: &mut Option<Turn>,
    msg: &Value,
) {
    // Routing is pure: turn the message into the stub's immediate reaction
    // (a response, an error, a permission-resume, or a stash) and adjust the
    // in-flight turn's state. The caller (`worker_loop`) then drives the turn's
    // next step, so this function never `drain`s on its own.
    match msg.get("method").and_then(Value::as_str) {
        Some("initialize") => {
            if let Some(id) = msg.get("id") {
                send_response(
                    lock,
                    id.clone(),
                    // Real `v2::AgentCapabilities` deserializes the session
                    // surface under `capabilities.session` (`agentCapabilities`
                    // is not a field the real schema recognises at all — a
                    // typed client reading it would see `capabilities.session`
                    // default to `None` and refuse the connection). `{}` for
                    // `session` means "the baseline session/* methods are
                    // supported" per the schema's own doc comment.
                    json!({
                        "protocolVersion": PROTOCOL_VERSION,
                        "info": { "name": "stub-acp", "version": "0" },
                        "capabilities": { "session": {} }
                    }),
                );
            }
        }
        Some("session/new") => {
            if let Some(id) = msg.get("id") {
                send_response(lock, id.clone(), json!({ "sessionId": SESSION_ID }));
            }
        }
        Some("session/prompt") => {
            // Real ACP v2 decouples a `session/prompt` response from turn
            // completion: the response only means "accepted" (`v2::PromptResponse`
            // carries no `stopReason` at all — see the driver story's "Decisions I
            // made"). So the stub acks it **immediately**, here, rather than
            // deferring to the turn's terminal step the way issue #130's original
            // (v1-shaped) design did. Completion is reported entirely through the
            // `idle` state_update (`send_idle_state`).
            //
            // A prompt (re)starts the turn: a new prompt supersedes any
            // still-pending one, so any in-flight turn state is dropped and a
            // fresh turn begins.
            if let Some(id) = msg.get("id") {
                send_response(lock, id.clone(), json!({}));
                *turn = Some(Turn {
                    started: false,
                    emitted: 0,
                    awaiting_gate: false,
                    gate_raised: false,
                    resumed_running: false,
                });
            }
        }
        Some("session/cancel") => {
            // A cancel outranks anything else in flight: the in-flight turn
            // resolves to `cancelled` FIRST (an `idle` state_update — the
            // `session/prompt` response was already sent when the prompt
            // arrived, so there is no second response to send here), then the
            // cancel notification itself is acknowledged with an empty
            // response.
            //
            // Resolving the turn here (clearing it) rather than deferring to
            // the caller's `drain` guarantees the cancel wins even if a
            // permission answer is also stashed: the turn is already gone by
            // the time `drain` runs, so a stale resume can't re-emit it.
            if turn.is_some() {
                *turn = None;
                send_idle_state(lock, "cancelled");
            }
            // Now acknowledge the cancel notification itself (its own id).
            if let Some(id) = msg.get("id") {
                send_response(lock, id.clone(), json!({}));
            }
        }
        Some(_) => {
            // An unknown method: JSON-RPC -32601, keep running.
            if let Some(id) = msg.get("id") {
                send_error(lock, id.clone(), ERROR_METHOD_NOT_FOUND, "method not found");
            }
        }
        // A client *response* (no method). The only ones the turn cares about
        // are the gate answers (id 10 = permission, id 11 = elicitation);
        // every other response is stashed for the worker's next `recv`.
        None => {
            let id = msg.get("id").and_then(Value::as_i64);
            if id == Some(PERMISSION_REQUEST_ID) || id == Some(ELICITATION_REQUEST_ID) {
                // The answer to the gate request: resume a parked turn by
                // clearing its gate. The caller's `drain` then emits the
                // remaining chunks. (A stray answer with no in-flight turn is
                // dropped.)
                if let Some(t) = turn.as_mut() {
                    if t.awaiting_gate {
                        t.awaiting_gate = false;
                    }
                }
            } else {
                pending.push_back(msg.clone());
            }
        }
    }
}

/// Consume turn-acting messages from the stash (in arrival order), then emit
/// the in-flight turn's next step. Returns when the turn resolves (`*turn`
/// cleared) or parks on the permission/elicitation gate.
///
/// A cancel is consumed before a resume, so a cancel that arrived while the
/// turn was parked on the permission gate still wins: the turn reports
/// `cancelled`, never a stale `end_turn`. Unrelated stashed messages are left
/// in the queue (re-examined by the worker's next `recv`).
fn drain(
    cfg: &Config,
    lock: &mut impl Write,
    pending: &mut VecDeque<Value>,
    turn: &mut Option<Turn>,
) {
    // Phase 1: consume turn-acting messages from the stash (in arrival order).
    let mut cancelled = false;
    while let Some(front) = pending.pop_front() {
        match front.get("method").and_then(Value::as_str) {
            // A cancel was stashed (it arrived before the turn parked, or
            // between the permission ask and its answer). `worker_loop` already
            // sent the cancel's ack; here we only record that it must win over
            // any resume.
            Some("session/cancel") => {
                cancelled = true;
                continue;
            }
            // The gate answer (id 10 permission, id 11 elicitation) resumes a
            // parked turn: clear the gate so the next `advance` continues with
            // the remaining chunks. (A stray answer with no in-flight turn is
            // dropped.)
            None if matches!(
                front.get("id").and_then(Value::as_i64),
                Some(PERMISSION_REQUEST_ID) | Some(ELICITATION_REQUEST_ID)
            ) =>
            {
                if let Some(t) = turn.as_mut() {
                    if t.awaiting_gate {
                        t.awaiting_gate = false;
                    }
                }
                continue;
            }
            // An unrelated inbound message (a request the stub does not
            // implement, a second session/prompt, …): re-queue it and stop the
            // phase-1 scan (the worker re-routes it on its next `recv`).
            _ => {
                pending.push_front(front);
                break;
            }
        }
    }

    // A cancel outranks a resume: if one was consumed above, resolve the turn
    // to `cancelled` and stop — even if a resume is also in the stash, the
    // cancel is what the turn observed first. Clear the turn state here (the
    // worker then sees `turn.is_none()` and stops pacing it).
    if cancelled {
        send_idle_state(lock, "cancelled");
        *turn = None;
        return;
    }

    // Phase 2: emit the turn's next step (and return, so the worker can pace
    // the following gap with `recv_timeout`). `advance` reports whether the
    // turn just resolved so `*turn` is cleared here — the worker's own
    // `if turn.is_some()` gate is what stops it from re-pacing a resolved
    // turn on the next `recv_timeout` gap.
    if let Some(t) = turn.as_mut() {
        if advance(cfg, lock, t) {
            *turn = None;
        }
    }
}

/// Send the one outbound request that raises `kind`'s gate (the permission or
/// elicitation ask). Split out of `advance` purely to keep that function under
/// the workspace's 100-line-per-function guard (`clippy::too_many_lines`) —
/// this is a single `match` with no state of its own.
fn send_gate_request(lock: &mut impl Write, kind: GateKind) {
    match kind {
        GateKind::Permission => send_request(
            lock,
            PERMISSION_REQUEST_ID,
            "session/request_permission",
            json!({
                "sessionId": SESSION_ID,
                "title": "stub tool wants to run",
                "toolCall": { "title": "stub tool" },
                "options": [
                    { "optionId": "allow", "name": "Allow", "kind": "allow_once" },
                    { "optionId": "deny",  "name": "Deny",  "kind": "reject_once" }
                ]
            }),
        ),
        GateKind::Elicitation => send_request(
            lock,
            ELICITATION_REQUEST_ID,
            "elicitation/create",
            json!({
                "mode": "form",
                "sessionId": SESSION_ID,
                "message": "pick your options",
                "requestedSchema": {
                    "type": "object",
                    "properties": {
                        "color": { "type": "string", "enum": ["red", "blue"] },
                        "size": {
                            "type": "string",
                            "oneOf": [
                                { "const": "s", "title": "Small" },
                                { "const": "m", "title": "Medium" }
                            ]
                        }
                    },
                    "required": ["color", "size"]
                }
            }),
        ),
        GateKind::ElicitationUrl => send_request(
            lock,
            ELICITATION_REQUEST_ID,
            "elicitation/create",
            json!({
                "mode": "url",
                "sessionId": SESSION_ID,
                "elicitationId": "elic-1",
                "url": "https://example.invalid/consent",
                "message": "open this url to continue"
            }),
        ),
    }
}

/// Emit exactly the next step of the in-flight turn, then return. The worker
/// calls this once per loop iteration and paces the gaps between calls with
/// `recv_timeout`, so a cancel/answer arriving in a gap is stashed and consumed
/// by the next `drain` *before* the following step is emitted.
///
/// The step is, in order: `state_update` running (first step only) →
/// `agent_message_chunk` i → (permission request + `requires_action`, then
/// park) → more chunks → terminal `end_turn` response (clears the turn).
/// Returns `true` when this step resolved the turn (the caller must then
/// clear its `Option<Turn>` — `advance` itself only has `&mut Turn`, not the
/// `Option`, so it cannot clear the slot directly).
fn advance(cfg: &Config, lock: &mut impl Write, t: &mut Turn) -> bool {
    // The turn is a small, fully-deterministic state machine keyed on
    // `t.emitted` (how many chunks have streamed) and `t.awaiting_gate`
    // (parked at a permission/elicitation gate). The worker calls `advance`
    // exactly once per pacing gap, so each call emits exactly one step and
    // returns. The machine is written as a single `match` so the next state
    // is explicit and unreachable states simply cannot arise:
    //
    //   * all `chunks` streamed → `idle` state_update + terminal response,
    //     turn cleared.
    //   * not yet started (`emitted == 0`, fresh) → emit `running`, then park
    //     one gap so a cancel that lands right after the prompt is honoured
    //     before any chunk. The step counter advances, so `running` is never
    //     re-sent (a resume straightens into the remaining chunks instead).
    //   * `cfg.gate()` is `Some` and `emitted == 1`, gate not yet raised →
    //     raise the permission/elicitation request + `requires_action`, then
    //     park until the client answers (id 10/11) or a cancel lands.
    //   * parked (`awaiting_gate`) → emit nothing; the worker keeps pacing
    //     until the answer (or a cancel) clears the gate.
    //   * otherwise → emit the next `agent_message_chunk` and advance. (This is
    //     also where a resumed turn lands: the answer cleared `awaiting_gate`,
    //     so the remaining chunks stream out.)
    // The state is matched on `(started, emitted, awaiting, raised)`: `started`
    // is set by the `running` step (so the next step is chunk 0, not another
    // `running`), `emitted` is the 0-based index of the next chunk, `awaiting`
    // marks the open gate, and `raised` (never cleared) marks that the gate has
    // already been raised this turn — what stops the answer's resume from
    // re-raising it. Guards keep the arms mutually exclusive so no step can
    // ever re-fire.
    match (
        t.started,
        t.emitted,
        t.awaiting_gate,
        t.gate_raised,
        t.resumed_running,
    ) {
        // Terminal: everything has streamed. Emit the `idle` state_update —
        // what a real ACP v2 client actually watches for completion
        // (`v2::PromptResponse` carries no `stopReason`; the `session/prompt`
        // response was already sent, immediately, when the prompt arrived) —
        // then reset the turn to its fresh state. `worker_loop` sees
        // `turn.is_none()`... no: it sees the turn reset to a *fresh*, inert
        // `Turn`, and its own `if turn.is_some()` gate (unchanged) keeps
        // calling `drain` every pacing gap; `drain`/`advance` on a fresh,
        // never-`started` turn is idempotent-inert only while a new
        // `session/prompt` hasn't arrived — so the worker instead clears
        // `*turn` to `None` outright here, matching the pre-#188 contract
        // that a resolved turn stops being paced at all.
        (_, emitted, _, _, _) if emitted >= cfg.chunks => {
            send_idle_state(lock, "end_turn");
            *t = Turn {
                started: false,
                emitted: 0,
                awaiting_gate: false,
                gate_raised: false,
                resumed_running: false,
            };
            true
        }
        // Fresh turn: announce `running`, then stop for one gap. `started` (not
        // `emitted`) records this step, so the chunk indices stay 0-based and
        // `running` is never re-sent (a resume continues into the chunks).
        (false, _, _, _, _) => {
            t.started = true;
            send_notification(
                lock,
                json!({
                    "sessionId": SESSION_ID,
                    "update": { "sessionUpdate": "state_update", "state": "running" }
                }),
            );
            false
        }
        // Gate (first pass): raise the permission/elicitation ask +
        // `requires_action`, park. Fires once, after chunk 0 (`emitted == 1`),
        // and *only if the gate has never been raised this turn*
        // (`!gate_raised`). Setting `resumed_running` here means that, when
        // the answer clears `awaiting_gate`, the very next `advance`
        // re-announces `running` (matching a real agent waking from a
        // permission/elicitation block) before the remaining chunks stream
        // out. `gate_raised` stays set so the gate is never re-raised.
        (_, 1, false, false, _) if cfg.gate().is_some() => {
            t.gate_raised = true;
            t.awaiting_gate = true;
            t.resumed_running = true;
            // `cfg.gate()` is `Some` — this arm's own guard already checked
            // that — so `if let` (not a fallible `match`) reads the intent
            // directly without a dead `else`.
            if let Some(kind) = cfg.gate() {
                send_gate_request(lock, kind);
            }
            send_notification(
                lock,
                json!({
                    "sessionId": SESSION_ID,
                    "update": { "sessionUpdate": "state_update", "state": "requires_action" }
                }),
            );
            false
        }
        // Parked on the gate: emit nothing; wait for the answer (or a cancel)
        // to clear `awaiting_gate`.
        (_, _, true, _, _) => false,
        // Resume from the gate: the answer just cleared `awaiting_gate`, so
        // re-announce `running` (a real agent wakes back to running) and clear
        // the flag; the remaining chunks then stream via the arm below on the
        // following gaps. This is the step the `--ask-permission`/
        // `--ask-elicitation` contract tests pin (`resume → running`).
        (true, _, false, true, true) => {
            t.resumed_running = false;
            send_notification(
                lock,
                json!({
                    "sessionId": SESSION_ID,
                    "update": { "sessionUpdate": "state_update", "state": "running" }
                }),
            );
            false
        }
        // Streaming (started, not parked, not yet done): emit the next
        // `agent_message_chunk` (0-based index = already-emitted) and advance.
        // The resume arm above is listed first and already matched the one-shot
        // `resumed_running` case, so by the time we reach here the remaining
        // chunks simply stream out (whether or not the gate was ever raised).
        (true, emitted, false, _, _) => {
            let index = emitted;
            t.emitted = index + 1;
            send_notification(
                lock,
                json!({
                    "sessionId": SESSION_ID,
                    "update": {
                        "sessionUpdate": "agent_message_chunk",
                        // `messageId` is a required field on the real
                        // `v2::ContentChunk` schema (no `#[serde(default)]`);
                        // omitting it makes a typed client's deserialization
                        // of the whole notification fail closed — silently,
                        // from this stub's point of view, since a malformed
                        // notification is simply never routed to any handler.
                        // One fixed id per turn is enough: every chunk in a
                        // turn belongs to the same streamed message.
                        "messageId": "stub-message",
                        "content": { "type": "text", "text": format!("stub chunk {index}") }
                    }
                }),
            );
            // Crash simulation for driver-error tests: die mid-turn (right
            // after the first chunk has streamed) so a client observing the
            // stream sees a turn that never resolves.
            if cfg.crash_after_prompt {
                eprintln!("stub-acp: --crash-after-prompt: exiting mid-turn");
                std::process::exit(1);
            }
            false
        }
    }
}

/// Write a JSON-RPC response (id + result) to stdout as one line.
fn send_response(lock: &mut impl Write, id: Value, result: Value) {
    let msg = json!({ "jsonrpc": "2.0", "id": id, "result": result });
    let _ = writeln!(lock, "{msg}");
    let _ = lock.flush();
}

/// Write a JSON-RPC error (id + error) to stdout as one line.
fn send_error(lock: &mut impl Write, id: Value, code: i64, message: &str) {
    let msg = json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } });
    let _ = writeln!(lock, "{msg}");
    let _ = lock.flush();
}

/// Write a `session/update` notification (no id) to stdout as one line.
fn send_notification(lock: &mut impl Write, params: Value) {
    let msg = json!({ "jsonrpc": "2.0", "method": "session/update", "params": params });
    let _ = writeln!(lock, "{msg}");
    let _ = lock.flush();
}

/// Write an outbound JSON-RPC request (the permission/elicitation ask) as one
/// line.
fn send_request(lock: &mut impl Write, id: i64, method: &str, params: Value) {
    let msg = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
    let _ = writeln!(lock, "{msg}");
    let _ = lock.flush();
}

/// Write a `session/update` `state_update:idle{stopReason}` notification —
/// the real ACP v2 signal a typed client (`agent_client_protocol` 2.1.0)
/// watches for turn completion. `v2::PromptResponse` (the `session/prompt`
/// response's own type) carries no `stopReason` field at all; only the `idle`
/// state_update does. Callers still also set `stopReason` on the prompt
/// response itself (kept for this file's own raw-JSON contract tests, which
/// predate this notification and assert on it directly).
fn send_idle_state(lock: &mut impl Write, stop_reason: &str) {
    send_notification(
        lock,
        json!({
            "sessionId": SESSION_ID,
            "update": {
                "sessionUpdate": "state_update",
                "state": "idle",
                "stopReason": stop_reason
            }
        }),
    );
}
