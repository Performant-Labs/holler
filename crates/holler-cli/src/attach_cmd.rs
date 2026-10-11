//! `holler body attach sessions` / `holler body attach init` (issue #196):
//! remove the "curl the endpoint and hand-edit a TOML" dance — list an
//! already-running OpenCode endpoint's sessions and write a ready `mode =
//! "attach"` config row for it.
//!
//! This is a **pure HTTP + file write** convenience: it never touches
//! `holler_body::session_manager` (issue #195's own file), never starts a
//! `body run` process, never prompts a model, and never touches Herdr. It
//! shares no code with `holler_body::http_attach_driver` (issue #194) —
//! sibling story #195 owns wiring that driver into `SessionManager`, and
//! this story is a genuine float over it (its own issue's words) — but it
//! deliberately matches that driver's *documented* endpoint-fallback
//! convention: `GET {endpoint}/session` (bare) first, `GET
//! {endpoint}/api/session` as the fallback (see
//! `crate::attach_cmd`'s `fetch_sessions`, mirroring
//! `http_attach_driver::check_exists`'s own "bare first, `/api` fallback,
//! fail closed on both" shape).
//!
//! Every OpenCode session field beyond `id`/`title`/`time.updated` is
//! ignored (serde's default "unknown fields are fine" behaviour, matching
//! this workspace's own `http_attach_driver::wire::OcEvent` convention) —
//! this module reads only what `sessions`/`init` need.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::time_fmt::format_epoch;
use crate::{Init, Sessions};

/// The `[[session]]` row `init` writes, in exactly the TOML grammar
/// `holler_body::config::parse` validates (its own module doc's grammar
/// comment): `name`/`harness`/`mode`/`endpoint`/`session_id`, `command`
/// entirely omitted (never even a `None` — attach mode never uses it, and
/// the config loader treats a `command` on an attach row as a warning, not
/// an error, but the whole point of this generator is to never trigger that
/// warning in the first place).
#[derive(Debug, Serialize)]
struct AttachConfigFile {
    session: Vec<AttachRow>,
}

#[derive(Debug, Serialize)]
struct AttachRow {
    name: String,
    harness: String,
    mode: String,
    endpoint: String,
    session_id: String,
}

impl AttachConfigFile {
    fn new(name: &str, endpoint: &str, session_id: &str) -> Self {
        AttachConfigFile {
            session: vec![AttachRow {
                name: name.to_string(),
                // "opencode" is the only harness id this story's spec (and
                // ADR 0003's own examples) ever names for an attach row —
                // OpenCode is the one harness with an HTTP attach surface.
                harness: "opencode".to_string(),
                mode: "attach".to_string(),
                endpoint: endpoint.to_string(),
                session_id: session_id.to_string(),
            }],
        }
    }
}

/// ADR 0003's own default OpenCode endpoint for both verbs.
pub const DEFAULT_ENDPOINT: &str = "http://127.0.0.1:4096";

/// One real OpenCode session, as returned by `GET {endpoint}/session` (or
/// `/api/session`) — only the fields this story needs; every other real
/// field (`directory`, `parentID`, `version`, …) is ignored by serde's
/// default "unknown fields are fine" behaviour (no `deny_unknown_fields`
/// here, matching `http_attach_driver::wire`'s own forward-compatible
/// stance).
#[derive(Debug, Clone, Deserialize)]
struct OcSession {
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    time: OcSessionTime,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct OcSessionTime {
    #[serde(default)]
    created: i64,
    #[serde(default)]
    updated: i64,
}

impl OcSession {
    /// The timestamp `sessions`/`init` sort on — `time.updated` when set,
    /// falling back to `time.created` (a session that has never been
    /// touched since creation still sorts sensibly rather than dropping to
    /// the very bottom on a missing `updated`).
    fn sort_key(&self) -> i64 {
        if self.time.updated != 0 {
            self.time.updated
        } else {
            self.time.created
        }
    }
}

/// Fetches `endpoint`'s session list, newest (`sort_key`) first: `GET
/// {endpoint}/session` (bare) first, falling back to `GET
/// {endpoint}/api/session` on a non-2xx/unreachable/undecodable response
/// from the first — the same "bare first, `/api` fallback, fail closed on
/// both" shape `http_attach_driver::check_exists` uses for its own existence
/// check (issue #194), reused here for a listing instead of a single-session
/// probe. `Err` carries a one-line message naming both URLs tried, for the
/// CLI's exit-1 stderr line.
fn fetch_sessions(endpoint: &str) -> Result<Vec<OcSession>, String> {
    let base = endpoint.trim_end_matches('/');
    let bare_url = format!("{base}/session");
    let api_url = format!("{base}/api/session");

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("runtime: {e}"))?;
    rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| format!("http client: {e}"))?;

        if let Some(sessions) = try_get_sessions(&client, &bare_url).await {
            return Ok(sort_newest_first(sessions));
        }
        if let Some(sessions) = try_get_sessions(&client, &api_url).await {
            return Ok(sort_newest_first(sessions));
        }
        Err(format!("could not reach {bare_url} (or its /api/session fallback {api_url})"))
    })
}

/// One GET attempt: `None` on any failure (network error, non-2xx status, or
/// a response body that does not decode as a session array) — the caller
/// tries the next candidate URL, or reports both tried if this was the last.
async fn try_get_sessions(client: &reqwest::Client, url: &str) -> Option<Vec<OcSession>> {
    let response = client.get(url).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    response.json::<Vec<OcSession>>().await.ok()
}

fn sort_newest_first(mut sessions: Vec<OcSession>) -> Vec<OcSession> {
    sessions.sort_by_key(|s| std::cmp::Reverse(s.sort_key()));
    sessions
}

/// `holler body attach sessions [--endpoint URL] [--json]` (ADR 0003):
/// prints `SESSION_ID TITLE UPDATED`, newest first (or, with `--json`, a
/// `{"sessions": [{"session_id","title","updated"}...]}` document in the
/// same order). Exit 0 on success; exit 1 (naming the URL(s) tried) on a
/// non-2xx/unreachable endpoint.
pub fn sessions(args: &Sessions, json: bool) -> i32 {
    let endpoint = args.endpoint.as_deref().unwrap_or(DEFAULT_ENDPOINT);
    let sessions = match fetch_sessions(endpoint) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    if json {
        let docs: Vec<serde_json::Value> = sessions
            .iter()
            .map(|s| {
                serde_json::json!({
                    "session_id": s.id,
                    "title": s.title,
                    "updated": s.sort_key(),
                })
            })
            .collect();
        println!("{}", serde_json::json!({ "sessions": docs }));
    } else {
        println!("{:<24} {:<40} UPDATED", "SESSION_ID", "TITLE");
        for s in &sessions {
            let updated = if s.sort_key() > 0 {
                format_epoch((s.sort_key() / 1000).max(0) as u64)
            } else {
                "-".to_string()
            };
            let title = if s.title.is_empty() { "-" } else { &s.title };
            println!("{:<24} {:<40} {}", s.id, title, updated);
        }
    }
    0
}

/// `holler body attach init [--endpoint URL] [--session ID] [--name NAME]
/// [--out PATH] [--force]` (ADR 0003): writes a valid `mode = "attach"`
/// `[[session]]` row to `--out` (default `./attach.toml`), defaulting
/// `--session` to the endpoint's newest session and `--name` to `alpha`.
///
/// Order of operations: the overwrite check runs *before* any network I/O
/// (`--out` exists without `--force` is exit 3 with nothing touched, and no
/// wasted HTTP round-trip), then the session id is resolved (`--session` if
/// given — no HTTP call needed at all in that case — otherwise the same
/// `fetch_sessions` call `sessions` uses, taking the newest), then the name
/// is validated against the real `SessionName` grammar `holler_body::config`
/// itself enforces (issue #196 fails closed on the *same* rule the config
/// loader would refuse the file on later, rather than writing a file the
/// loader would then reject) before the file is written.
pub fn init(args: &Init, _json: bool) -> i32 {
    let endpoint = args.endpoint.as_deref().unwrap_or(DEFAULT_ENDPOINT);
    let name = args.name.as_deref().unwrap_or("alpha");
    let out = args.out.as_deref().unwrap_or("./attach.toml");
    let out_path = std::path::Path::new(out);

    if out_path.exists() && !args.force {
        eprintln!("error: {out} already exists (pass --force to overwrite)");
        return 3;
    }

    if holler_proto::SessionName::parse(name).is_err() {
        eprintln!("error: --name {name:?} is not a valid session name (lowercase letters/digits/hyphens, 1-32 chars, no leading/trailing/double hyphen)");
        return 3;
    }

    let session_id = match &args.session {
        Some(id) => id.clone(),
        None => match fetch_sessions(endpoint) {
            Ok(sessions) => match sessions.into_iter().next() {
                Some(newest) => newest.id,
                None => {
                    eprintln!("error: {endpoint} has no sessions to attach to");
                    return 1;
                }
            },
            Err(e) => {
                eprintln!("error: {e}");
                return 1;
            }
        },
    };

    let doc = AttachConfigFile::new(name, endpoint, &session_id);
    let rendered = match toml::to_string_pretty(&doc) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: rendering attach config: {e}");
            return 1;
        }
    };
    if let Err(e) = std::fs::write(out_path, rendered) {
        eprintln!("error: writing {out}: {e}");
        return 1;
    }

    println!("wrote {out}");
    println!("next: holler body run --config {out}");
    0
}
