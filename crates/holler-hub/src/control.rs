//! The control-socket **client** side: the one-shot CLI commands
//! (`holler hub status` here; `roster`/`say`/… in later stories) reach the
//! live hub process over the control Unix socket rather than the WebSocket
//! circuit (ADR 0006). This module is the client; the server side lives in
//! [`crate::serve`].

use std::io::{BufRead, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use holler_proto::CorrelationId;

use crate::state::{control_sock_path, resolve_state_dir, HubState};

/// The timeout for a one-shot control exchange (the spec: 5 s).
const CLIENT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Why a control exchange could not reach the live hub.
#[derive(Debug)]
pub enum ControlError {
    /// No socket at the expected path (the hub is not running, or the state
    /// dir is wrong) — the spec's `no live holler hub reachable at <dir>`.
    NoLiveHub,
    /// The socket was present but the exchange failed (I/O or a bad reply).
    Io(std::io::Error),
    /// The reply did not parse as a v2 envelope.
    BadReply(String),
}

impl std::fmt::Display for ControlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ControlError::NoLiveHub => write!(f, "no live holler hub reachable"),
            ControlError::Io(e) => write!(f, "control socket I/O error: {e}"),
            ControlError::BadReply(s) => write!(f, "bad reply from the hub: {s}"),
        }
    }
}

/// Resolve the control socket path for the current state dir.
pub fn sock_path() -> PathBuf {
    control_sock_path(&HubState::from_root(resolve_state_dir()))
}

/// Ask the live hub for its status document over the control socket.
///
/// Returns the reply envelope's `result` (the `StatusDoc` JSON value). Errors
/// map to [`ControlError::NoLiveHub`] when the socket is absent (hub not
/// running) — the CLI prints the spec's exact message and exits 1.
pub fn status() -> Result<serde_json::Value, ControlError> {
    let path = control_sock_path(&HubState::from_root(resolve_state_dir()));
    let stream = UnixStream::connect(&path).map_err(|_| ControlError::NoLiveHub)?;
    stream
        .set_read_timeout(Some(CLIENT_TIMEOUT))
        .map_err(ControlError::Io)?;

    let cid = CorrelationId::parse("b-status").expect("a valid body-minted id");
    let req = holler_proto::Envelope::request(&cid, "control/status", None);
    let bytes = format!(
        "{}\n",
        holler_proto::encode(&req).expect("encode the status request")
    );

    let mut stream = stream;
    stream
        .write_all(bytes.as_bytes())
        .map_err(ControlError::Io)?;
    stream.flush().map_err(ControlError::Io)?;

    let mut line = String::new();
    let n = std::io::BufReader::new(stream)
        .read_line(&mut line)
        .map_err(ControlError::Io)?;
    if n == 0 {
        return Err(ControlError::BadReply(
            "the hub closed the socket before replying".into(),
        ));
    }

    let env = holler_proto::decode(&line).map_err(|e| ControlError::BadReply(e.to_string()))?;
    // The reply is a response (result) — but be lenient: accept whatever the
    // hub sent and hand the envelope back; the caller reads `.result`.
    let result = env
        .result()
        .cloned()
        .ok_or_else(|| ControlError::BadReply("the hub reply carried no result".into()))?;
    Ok(result)
}
