//! `body status` (story #176): report this process's own identity.
//!
//! `body status` is *local*: it reads the body's own persisted identity
//! (`<state>/body/credential.json`) and reports whether **this** body is joined
//! and, if so, who it is (`client_id` / `token_id` / the server it joined). It
//! does **not** contact a live hub — so a downed hub never makes `body status`
//! fail (unlike `hub status`, which is a control-socket round-trip and exits 1
//! when no live hub answers). An unjoined body is a *valid* state (`joined:
//! false`, exit 0), never an error.
//!
//! With `--json` the full status document goes to **stdout** (machine-readable;
//! the ADR 0003 invariant that, with `--json`, only the JSON document appears on
//! stdout). Without it, a short human summary goes to stdout and nothing else
//! does. `connected` is the spec's recency proxy (story #176: "stale > 45 s =
//! false") — `status` reads the file, so `connected` is `true` only while the
//! join is recent, not while a `body run` is (separately) holding a socket.

use std::path::Path;

use serde::Serialize;

use crate::identity::{self, BodyIdentity};

/// The CLI exit code `body status` applies (ADR 0003).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusExit {
    /// The status document was produced; the bin exits 0.
    Ok,
    /// A state-dir I/O failure; the bin exits 1 (the document is `None`).
    Io,
}

/// The `body status` document — what `--json` prints and the summary renders.
#[derive(Debug, Serialize)]
pub struct StatusDoc {
    /// Always `"body"` (the document is this body's, not the hub's).
    role: &'static str,
    /// Whether this body has a persisted identity (has joined).
    joined: bool,
    /// The body's client id (the `cli_` id the hub minted at join), or `None`
    /// when unjoined.
    #[serde(skip_serializing_if = "Option::is_none")]
    client_id: Option<String>,
    /// The token id the body joined with, or `None` when unjoined.
    #[serde(skip_serializing_if = "Option::is_none")]
    token_id: Option<String>,
    /// The server address the body joined, or `None` when unjoined.
    #[serde(skip_serializing_if = "Option::is_none")]
    server: Option<String>,
    /// The claimed hostname, or `None` when unjoined.
    #[serde(skip_serializing_if = "Option::is_none")]
    hostname: Option<String>,
    /// Whether the join is recent (within [`STALE_AFTER_SECS`]); `false` when
    /// unjoined or stale.
    connected: bool,
}

impl StatusDoc {
    /// Build the document from the (absent or present) persisted identity.
    fn build(identity: Option<&BodyIdentity>) -> StatusDoc {
        match identity {
            None => StatusDoc {
                role: "body",
                joined: false,
                client_id: None,
                token_id: None,
                server: None,
                hostname: None,
                connected: false,
            },
            Some(i) => StatusDoc {
                role: "body",
                joined: true,
                client_id: Some(i.client_id.clone()),
                token_id: Some(i.token_id.clone()),
                server: (!i.server_url.is_empty()).then(|| i.server_url.clone()),
                hostname: (!i.hostname.is_empty()).then(|| i.hostname.clone()),
                connected: i.connected(holler_proto::now_secs()),
            },
        }
    }
}

/// `body status [--json]`.
///
/// Reads this body's own identity and returns the [`StatusDoc`] (and the exit
/// code). An absent identity yields the unjoined document (exit 0, never an
/// error); an I/O failure reading the file yields a `None` document (exit 1).
/// The document is **not** printed here — the bin owns stdout — so that `--json`
/// keeps its invariant (only the JSON document on stdout).
pub fn status(state_root: &Path) -> (Option<StatusDoc>, StatusExit) {
    match identity::load(state_root) {
        None => (
            Some(StatusDoc::build(None)),
            StatusExit::Ok,
        ),
        Some(Ok(i)) => (Some(StatusDoc::build(Some(&i))), StatusExit::Ok),
        Some(Err(e)) => {
            eprintln!("error: state dir: {e}");
            (None, StatusExit::Io)
        }
    }
}

/// Render the human-readable `body status` summary (no `--json`). Kept here
/// (not the bin) so it is unit-testable; the bin only prints its result to
/// stdout.
pub fn render_human(doc: &StatusDoc) -> String {
    match doc.joined {
        false => "body: not joined".to_string(),
        true => {
            let mut s = format!("body: joined as {:?}", doc.client_id);
            if let Some(t) = &doc.token_id {
                s.push_str(&format!(" (token {t})"));
            }
            if let Some(sv) = &doc.server {
                s.push_str(&format!(" @ {sv}"));
            }
            s.push_str(if doc.connected {
                " [connected]"
            } else {
                " [stale]"
            });
            s
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #176
mod tests {
    use super::*;

    #[test]
    fn unjoined_document_is_the_false_form() {
        let doc = StatusDoc::build(None);
        assert!(!doc.joined);
        assert!(!doc.connected);
        assert!(doc.client_id.is_none());
        let json = serde_json::to_value(&doc).unwrap();
        assert_eq!(json["role"].as_str(), Some("body"));
        assert_eq!(json["joined"].as_bool(), Some(false));
    }
}
