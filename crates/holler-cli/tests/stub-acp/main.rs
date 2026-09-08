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
    crash_after_prompt: bool,
}

/// The in-flight turn's position. `emitted` counts `agent_message_chunk`
/// notifications already sent (0-based index of the next one);
/// `awaiting_permission` is true once the permission request has been raised
/// and before the client's answer (id 10) has resumed it. (The prompt's id
/// lives in the worker's separate `prompt` slot, not here.)
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
    /// True once the permission request has been raised and before the client's
    /// answer (id 10) has resumed the turn. (Cleared by the id-10 answer; the
    /// turn then continues with the next chunk.)
    awaiting_permission: bool,
    /// True once the permission request has been raised, for the life of the
    /// turn. Unlike `awaiting_permission` it is never cleared: it is what stops
    /// `advance` from re-firing the permission arm after the id-10 answer
    /// clears the gate (otherwise the `(emitted == 1, !awaiting)` arm would
    /// raise the request a second time and re-park the turn forever).
    asked_permission: bool,
    /// True from the moment the permission gate is raised (parked) until the
    /// turn's *resume* step has re-emitted `state_update:running`. A real ACP
    /// agent, when woken from a permission block, re-announces `running` before
    /// continuing (that is exactly what the `--ask-permission` contract test
    /// pins: resume → `running` → remaining chunks). `advance` sets this when it
    /// raises the gate and clears it once the resume `running` has been sent, so
    /// the remaining chunks then stream out without a second `running`.
    resumed_running: bool,
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
    let mut prompt: Option<Value> = None; // the id of the in-flight prompt, if any
    let mut pending: VecDeque<Value> = VecDeque::new(); // inbound messages not yet routed
    let mut turn: Option<Turn> = None; // in-flight turn state, if any

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
                // `drain` → `advance` clears BOTH `*turn` and the worker's
                // `prompt` mirror the moment the turn resolves (the terminal
                // arm does `prompt.take()`). Clearing `prompt` is what stops the
                // next gap from re-entering `drain` and re-running the
                // (now-reset) turn — if only `*turn` were cleared, `prompt`
                // would still be `Some` and the turn would re-emit `running`
                // forever. A resolved turn therefore has `prompt == None` and
                // the `is_some()` guard below makes subsequent gaps no-ops.
                if prompt.is_some() {
                    drain(cfg, &mut lock, &mut pending, &mut prompt, &mut turn);
                }
                continue;
            }
            // The input stream ended (in-band marker, or a hard channel close).
            // A bare EOF is NOT a cancel: we do not synthesize a
            // `session/cancel` or force an in-flight turn to `cancelled`. (A
            // real client that sent `session/cancel` already resolved the turn
            // via the routing below.) If a turn is still in flight it has not
            // produced its terminal response yet, so emit the next step (which,
            // at the chunk ceiling, is the `end_turn`/`cancelled` the turn
            // genuinely resolves to). A resolved turn has `prompt == None` and
            // `drain` is a no-op. Then stop: the worker has emitted everything
            // it will. (We do NOT consume a further stash here: a stash only
            // holds unrelated traffic, and the marker path means `main` is done
            // sending, so draining further could only re-process noise.)
            Incoming::Eof => {
                if prompt.is_some() {
                    drain(cfg, &mut lock, &mut pending, &mut prompt, &mut turn);
                }
                break;
            }
            Incoming::Message(msg) => {
                // The in-band EOF marker: stdin is closed. Treat it exactly
                // like the hard-close EOF path — resolve any in-flight turn to
                // its terminal step and stop. (It is never routed as a normal
                // message: it has no `method`/`id` a client would send.)
                if is_eof_marker(&msg) {
                    if prompt.is_some() {
                        drain(cfg, &mut lock, &mut pending, &mut prompt, &mut turn);
                    }
                    break;
                }
                // Route the message by kind (may start a turn, park it, resume
                // it, or resolve it to `cancelled`).
                route(&mut lock, &mut pending, &mut prompt, &mut turn, &msg);
                // If routing resumed a parked turn (the permission answer), emit
                // its next step now (the remaining chunks). A cancel or a
                // completed turn has already resolved it, so `drain` is a no-op.
                if prompt.is_some() {
                    drain(cfg, &mut lock, &mut pending, &mut prompt, &mut turn);
                }
                // `drain` → `advance` clears `*turn` the moment the turn
                // resolves, so clear the worker's `prompt` mirror and stop
                // pacing it.
                if prompt.is_some() && turn.is_none() {
                    prompt = None;
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
    prompt: &mut Option<Value>,
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
                    json!({ "protocolVersion": PROTOCOL_VERSION, "agentCapabilities": {} }),
                );
            }
        }
        Some("session/new") => {
            if let Some(id) = msg.get("id") {
                send_response(lock, id.clone(), json!({ "sessionId": SESSION_ID }));
            }
        }
        Some("session/prompt") => {
            // A prompt (re)starts the turn: a new prompt supersedes any
            // still-pending one, so any in-flight turn state is dropped and a
            // fresh turn begins.
            if let Some(id) = msg.get("id") {
                *prompt = Some(id.clone());
                *turn = Some(Turn {
                    started: false,
                    emitted: 0,
                    awaiting_permission: false,
                    asked_permission: false,
                    resumed_running: false,
                });
            }
        }
        Some("session/cancel") => {
            // A cancel outranks anything else in flight: the in-flight turn
            // resolves to `cancelled` FIRST, then the cancel notification itself
            // is acknowledged with an empty response. Emitting the prompt's
            // terminal response before the cancel's ack matches the order a real
            // ACP client observes (its outstanding `session/prompt` settles, then
            // the `session/cancel` it sent is acked). (Issue #147: this ordering
            // is what `cancel_yields_cancelled_stop_reason_and_stub_survives`
            // pins — prompt id resolves to `cancelled`, then the cancel id to
            // `{}`.)
            //
            // Resolving the turn here (clearing `prompt`/`turn`) rather than
            // deferring to the caller's `drain` guarantees the cancel wins even
            // if a permission answer is also stashed: the turn is already gone
            // by the time `drain` runs, so a stale resume can't re-emit it.
            if prompt.is_some() {
                // The in-flight prompt's id (captured before `prompt` is
                // cleared — a `Value::Null` here would orphan the prompt
                // response, and a client matching on the prompt's id would
                // hang forever).
                let prompt_id = prompt.take().unwrap_or(Value::Null);
                *turn = None;
                send_response(lock, prompt_id, json!({ "stopReason": "cancelled" }));
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
        // A client *response* (no method). The only one the turn cares about is
        // the permission answer (id 10); every other response is stashed for
        // the worker's next `recv`.
        None => {
            if msg.get("id").and_then(Value::as_i64) == Some(PERMISSION_REQUEST_ID) {
                // The answer to the permission request: resume a parked turn by
                // clearing its gate. The caller's `drain` then emits the
                // remaining chunks. (A stray id-10 response with no in-flight
                // turn is dropped.)
                if let Some(t) = turn.as_mut() {
                    if t.awaiting_permission {
                        t.awaiting_permission = false;
                    }
                }
            } else {
                pending.push_back(msg.clone());
            }
        }
    }
}

/// Consume turn-acting messages from the stash (in arrival order), then emit
/// the in-flight turn's next step. Returns when the turn resolves (`prompt` /
/// `turn` cleared) or parks on the permission gate.
///
/// A cancel is consumed before a resume, so a cancel that arrived while the
/// turn was parked on the permission gate still wins: the turn reports
/// `cancelled`, never a stale `end_turn`. Unrelated stashed messages are left
/// in the queue (re-examined by the worker's next `recv`).
fn drain(
    cfg: &Config,
    lock: &mut impl Write,
    pending: &mut VecDeque<Value>,
    prompt: &mut Option<Value>,
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
            // The permission answer (id 10) resumes a parked turn: clear the
            // gate so the next `advance` continues with the remaining chunks.
            // (A stray id-10 response with no in-flight turn is dropped.)
            None if front.get("id").and_then(Value::as_i64) == Some(PERMISSION_REQUEST_ID) => {
                if let Some(t) = turn.as_mut() {
                    if t.awaiting_permission {
                        t.awaiting_permission = false;
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
        send_response(
            lock,
            prompt.take().unwrap_or(Value::Null),
            json!({ "stopReason": "cancelled" }),
        );
        *turn = None;
        return;
    }

    // Phase 2: emit the turn's next step (and return, so the worker can pace
    // the following gap with `recv_timeout`). `advance` clears both `*turn` and
    // the worker's `prompt` mirror the moment the turn resolves, so the worker
    // stops pacing it.
    if turn.is_some() {
        advance(cfg, lock, prompt, turn.as_mut().unwrap());
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
fn advance(cfg: &Config, lock: &mut impl Write, prompt: &mut Option<Value>, t: &mut Turn) {
    // The turn is a small, fully-deterministic state machine keyed on
    // `t.emitted` (how many chunks have streamed) and `t.awaiting_permission`
    // (parked at the permission gate). The worker calls `advance` exactly once
    // per pacing gap, so each call emits exactly one step and returns. The
    // machine is written as a single `match` so the next state is explicit and
    // unreachable states simply cannot arise:
    //
    //   * all `chunks` streamed → terminal `end_turn` response, turn cleared.
    //   * not yet started (`emitted == 0`, fresh) → emit `running`, then park
    //     one gap so a cancel that lands right after the prompt is honoured
    //     before any chunk. The step counter advances, so `running` is never
    //     re-sent (a resume straightens into the remaining chunks instead).
    //   * `ask_permission` and `emitted == 1`, gate not yet raised → raise the
    //     permission request + `requires_action`, then park until the client
    //     answers (id 10) or a cancel lands.
    //   * parked (`awaiting_permission`) → emit nothing; the worker keeps
    //     pacing until the id-10 answer (or a cancel) clears the gate.
    //   * otherwise → emit the next `agent_message_chunk` and advance. (This is
    //     also where a resumed turn lands: the id-10 answer cleared
    //     `awaiting_permission`, so the remaining chunks stream out.)
    // The state is matched on `(started, emitted, awaiting, asked)`: `started`
    // is set by the `running` step (so the next step is chunk 0, not another
    // `running`), `emitted` is the 0-based index of the next chunk, `awaiting`
    // marks the open permission gate, and `asked` (never cleared) marks that the
    // gate has already been raised this turn — what stops the id-10 resume from
    // re-raising it. Guards keep the arms mutually exclusive so no step can ever
    // re-fire.
    match (
        t.started,
        t.emitted,
        t.awaiting_permission,
        t.asked_permission,
        t.resumed_running,
    ) {
        // Terminal: everything has streamed. Resolve the turn and clear its
        // state (the worker's `prompt` mirror is cleared too) so the worker
        // stops pacing it and a later `drain` is a no-op.
        (_, emitted, _, _, _) if emitted >= cfg.chunks => {
            // Emit the terminal response carrying the *original* prompt's id,
            // then clear BOTH the turn and the worker's `prompt` mirror. Clearing
            // `prompt` (not just `*t`) is what stops the worker from re-pacing
            // a resolved turn on the next `recv_timeout` gap — if `prompt`
            // stayed `Some`, the worker's `if prompt.is_some()` guard would
            // re-enter `drain` and re-run the (now-reset) turn forever.
            send_response(
                lock,
                prompt.take().unwrap_or(Value::Null),
                json!({ "stopReason": "end_turn" }),
            );
            *t = Turn {
                started: false,
                emitted: 0,
                awaiting_permission: false,
                asked_permission: false,
                resumed_running: false,
            };
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
        }
        // Permission gate (first pass): raise the ask + `requires_action`, park.
        // Fires once, after chunk 0 (`emitted == 1`), and *only if the gate has
        // never been raised this turn* (`!asked_permission`). Setting
        // `resumed_running` here means that, when the id-10 answer clears
        // `awaiting_permission`, the very next `advance` re-announces `running`
        // (matching a real agent waking from a permission block) before the
        // remaining chunks stream out. `asked_permission` stays set so the gate
        // is never re-raised.
        (_, 1, false, false, _) if cfg.ask_permission => {
            t.asked_permission = true;
            t.awaiting_permission = true;
            t.resumed_running = true;
            send_request(
                lock,
                PERMISSION_REQUEST_ID,
                "session/request_permission",
                json!({
                    "sessionId": SESSION_ID,
                    "toolCall": { "title": "stub tool" },
                    "options": [
                        { "optionId": "allow", "kind": "allow_once" },
                        { "optionId": "deny",  "kind": "reject_once" }
                    ]
                }),
            );
            send_notification(
                lock,
                json!({
                    "sessionId": SESSION_ID,
                    "update": { "sessionUpdate": "state_update", "state": "requires_action" }
                }),
            );
        }
        // Parked on the permission gate: emit nothing; wait for the id-10
        // answer (or a cancel) to clear `awaiting_permission`.
        (_, _, true, _, _) => {}
        // Resume from the permission gate: the id-10 answer just cleared
        // `awaiting_permission`, so re-announce `running` (a real agent wakes
        // back to running) and clear the flag; the remaining chunks then stream
        // via the arm below on the following gaps. This is the step the
        // `--ask-permission` contract test pins (`resume → running`).
        (true, _, false, true, true) => {
            t.resumed_running = false;
            send_notification(
                lock,
                json!({
                    "sessionId": SESSION_ID,
                    "update": { "sessionUpdate": "state_update", "state": "running" }
                }),
            );
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

/// Write an outbound JSON-RPC request (the permission ask) as one line.
fn send_request(lock: &mut impl Write, id: i64, method: &str, params: Value) {
    let msg = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
    let _ = writeln!(lock, "{msg}");
    let _ = lock.flush();
}
