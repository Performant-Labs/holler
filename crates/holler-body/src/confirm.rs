//! `holler body confirm` (issue #351): the one-time, interactive operator
//! confirmation gate for a pairing's Short Authentication String.
//!
//! #339 (merged via #350) derives the pairing SAS (HKDF-SHA256 over the
//! completed Noise XK handshake hash, `holler_proto::sas::derive_sas`) and
//! logs it on **every** successful `body run` (re)connect — but deliberately
//! never blocks on an operator confirming it, because `body run`'s
//! connection loop (`crate::connection::connect_and_serve`) is also the
//! fully-automated reconnect path every e2e/interop test and unattended
//! daemon depends on. This module is the missing other half: a **separate,
//! one-time, interactive** command that performs its own throwaway
//! connect + Noise handshake (the exact same `circuit/authenticate` →
//! `circuit/prove` exchange `body run` does, reusing
//! `crate::connection::handshake::authenticate` so the SAS is derived
//! identically), shows the operator the resulting SAS, asks them to confirm
//! it matches what the hub's own console displayed for this same pairing,
//! and — only on an explicit "yes" — persists `sas_confirmed: true` on the
//! body's identity (`crate::identity::BodyIdentity`, alongside the pinned
//! hub key from `body join`).
//!
//! This is deliberately **not** part of `body join`: at join time there is no
//! Noise session yet (`body join` is a one-shot `circuit/join` redeem over
//! its own throwaway socket, closed by the hub immediately after — see
//! `crate::join`'s module doc), so no SAS exists to show the operator. A SAS
//! only exists once a full handshake completes, which `body join` does not
//! perform. Splitting confirmation into its own command also keeps it
//! optional and re-runnable independent of whether a `body run` happens to
//! be live.
//!
//! Blocking on stdin here is safe *because* this is a distinct, operator-run
//! command the automated reconnect loop never calls — unlike `body run`,
//! `body confirm` is not on any hot path a test harness or unattended daemon
//! depends on running unattended.

use std::io::Write as _;
use std::path::Path;

use futures_util::StreamExt;
use tokio_tungstenite::tungstenite::Message;

use crate::connection::{handshake, Attempt};
use crate::identity::{self, BodyIdentity};

/// The CLI exit outcome `holler body confirm` applies (ADR 0003: 0 ok, 1 a
/// runtime failure/decline/not-joined — there is no policy (exit 3) case
/// here, since nothing about this command is a fail-closed configuration
/// check).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmExit {
    /// This pairing's SAS was already confirmed on a previous run — a no-op
    /// success (confirmation is idempotent: "asked at most once per hub
    /// pairing").
    AlreadyConfirmed,
    /// The operator confirmed the SAS just now; the flag is now persisted.
    Confirmed,
    /// The operator answered "no" (the SAS did not match): nothing is
    /// persisted, and the body remains unconfirmed.
    Declined,
    /// This body has never joined a hub — there is no pairing to confirm.
    NotJoined,
    /// A runtime failure (connect, handshake, or state-dir I/O). The message
    /// is the one line the CLI prints on stderr.
    Refused(String),
}

/// `holler body confirm` — see the module doc for the full design.
///
/// `read_line`/`print_prompt` are injected (rather than calling
/// `std::io::stdin()`/`println!` directly) so the integration tests can
/// drive this over a real piped child-process stdin/stdout (the same
/// discipline `body join`/`body run`'s own tests use: real subprocesses, no
/// mocks of the circuit) while unit tests can supply a canned answer without
/// a terminal.
pub fn confirm(state_root: &Path, read_answer: impl FnOnce() -> std::io::Result<String>) -> ConfirmExit {
    let identity = match identity::load(state_root) {
        None => return ConfirmExit::NotJoined,
        Some(Err(e)) => return ConfirmExit::Refused(format!("state dir: {e}")),
        Some(Ok(identity)) => identity,
    };
    if identity.sas_confirmed {
        return ConfirmExit::AlreadyConfirmed;
    }

    let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
        Ok(rt) => rt,
        Err(e) => return ConfirmExit::Refused(format!("runtime: {e}")),
    };
    let sas = match rt.block_on(derive_sas(state_root, &identity)) {
        Ok(sas) => sas,
        Err(e) => return ConfirmExit::Refused(e),
    };

    println!("pairing SAS for {}: {sas}", identity.server_url);
    println!("compare this against the SAS the hub logged for this same connection.");
    print!("does it match? [y/N] ");
    if std::io::stdout().flush().is_err() {
        // Non-fatal: a failed flush just risks the prompt appearing after the
        // answer is typed on a slow terminal; the read below still works.
    }
    let answer = match read_answer() {
        Ok(a) => a,
        Err(e) => return ConfirmExit::Refused(format!("could not read the operator's answer: {e}")),
    };
    if !is_yes(&answer) {
        println!("not confirmed — this pairing's SAS remains unconfirmed.");
        return ConfirmExit::Declined;
    }

    match identity::mark_sas_confirmed(state_root) {
        Some(Ok(_)) => {
            println!("confirmed — {} is now a trusted pairing.", identity.server_url);
            ConfirmExit::Confirmed
        }
        Some(Err(e)) => ConfirmExit::Refused(format!("state dir: {e}")),
        None => ConfirmExit::NotJoined, // race: detached between the load above and here.
    }
}

/// Whether `answer` (trimmed, case-insensitive) is an affirmative "y"/"yes".
/// Anything else — including empty input (a bare Enter) — is a "no": this
/// prompt's default is refusal, never silent acceptance.
fn is_yes(answer: &str) -> bool {
    matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

/// Connect to `identity.server_url` and run the same `circuit/authenticate`
/// → `circuit/prove` Noise XK handshake `body run` performs, returning the
/// derived SAS. The socket is closed immediately after (this command never
/// exchanges `circuit/hello` or joins the live session — it only needs the
/// completed handshake hash the SAS derives from).
async fn derive_sas(state_root: &Path, identity: &BodyIdentity) -> Result<String, String> {
    let (ws, _) = tokio_tungstenite::connect_async(&identity.server_url)
        .await
        .map_err(|e| format!("could not reach the hub: {e}"))?;
    let (mut sink, mut stream) = ws.split();
    let sas = handshake::authenticate(&mut sink, &mut stream, identity, state_root)
        .await
        .map_err(attempt_message)?;
    let _ = futures_util::SinkExt::send(&mut sink, Message::Close(None)).await;
    let _ = futures_util::SinkExt::flush(&mut sink).await;
    Ok(sas)
}

/// Render an [`Attempt`] (the reconnect loop's own outcome type) as the
/// one-line reason this command reports on a handshake failure. `Ended` is
/// unreachable here (`handshake::authenticate` never returns it — only
/// `AuthFailed`/`Dropped` — but the match must be total since the type is
/// shared with the reconnect loop).
fn attempt_message(attempt: Attempt) -> String {
    match attempt {
        Attempt::AuthFailed(m) | Attempt::Dropped(m) => m,
        Attempt::Ended(_) => "unexpected: handshake reported a clean end".to_string(),
    }
}
