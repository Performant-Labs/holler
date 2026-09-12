//! `holler body join|detach|status|run|caps|support|query` (stories #176,
//! #182, #185, #187), split out of `main.rs` to keep that file under the
//! workspace's 900-line build guard (`scripts/lint.sh` check 4) — issue
//! #228, the same split `say_cmd.rs`/`roster_cmd.rs`/`token_cmd.rs`/
//! `hub_cmd.rs` made for their own leaves.
//!
//! The identity leaves (`join`/`detach`/`status`/`run`) each resolve the
//! state dir (the body's credential lives under `<state>/body/`), drive the
//! `holler_body` helpers, and return the ADR 0003 exit code the helper
//! itself computed (0 ok, 1 a runtime failure, 3 a fail-closed policy
//! refusal) for `main.rs` (the one file allowed to end the process) to
//! apply.
//!
//! `caps`/`support`/`query` (issue #185) are **local**: they read this
//! body's own persisted identity, the connection-state file, and the
//! session config — the exact same local probes `crate::query`'s document
//! builders answer a hub-forwarded `query/*` request with (see
//! `holler_body::connection::handle_query`), so a live `body run` and a
//! one-shot `body caps`/`support`/`query` agree by construction. They share
//! their print+exit tail with `hub_cmd.rs`'s own versions via
//! [`crate::query_cmd::print_and_exit_code`] (issue #229).

use crate::query_cmd::{body_local_configs, print_and_exit_code, protocol_version_from_args, FetchOutcome};
use crate::{Cmd, Query, QueryResolution};

/// Resolve the state dir, or fail closed (exit 1) if there is nowhere for
/// the body's credential to live. (The e2e harness always sets
/// `HOLLER_STATE_DIR`.) Returns `Err(1)` instead of exiting so every leaf
/// below stays a plain function the bin applies an exit code to.
fn body_state() -> Result<holler_hub::state::HubState, i32> {
    match holler_hub::state::resolve_state_dir() {
        Some(root) => Ok(holler_hub::state::HubState::from_root(root)),
        // `resolve_state_dir` already printed the refusal to stderr.
        None => Err(1),
    }
}

/// `holler body join --server <url> --token <ID:SECRET> --hub-key <HEX>` —
/// generate this body's Ed25519 signing keypair, redeem a minted one-time
/// join secret over the wire, and persist the body's identity (including the
/// hub's X25519 public key pinned from `--hub-key`, issue #322). The helper
/// prints its own one-line result (a stderr reason on a refusal; a success
/// line naming the `client_id`, never the secret) and returns the exit code
/// (ADR 0003: 0 joined, 1 a runtime failure incl. a refused redeem, 3 a
/// plaintext non-loopback `ws://` or a malformed `--hub-key`).
pub fn join(server: &str, token: &str, hub_key: &str) -> i32 {
    let state = match body_state() {
        Ok(s) => s,
        Err(code) => return code,
    };
    let exit = holler_body::join::join(&state.root, server, token, "default", hub_key);
    match exit {
        holler_body::join::JoinExit::Ok => 0,
        holler_body::join::JoinExit::Refused(_) => 1,
        holler_body::join::JoinExit::Policy => 3,
    }
}

/// `holler body detach` — forget the body's identity: the no-run form
/// deletes `<state>/body/credential.json` outright; when a `body run` is
/// live (issue #182) it is asked to end the circuit first (see
/// `holler_body::detach`'s module doc). Idempotent: always exit 0 except an
/// I/O failure (exit 1).
pub fn detach() -> i32 {
    let state = match body_state() {
        Ok(s) => s,
        Err(code) => return code,
    };
    match holler_body::detach::detach(&state.root) {
        holler_body::detach::DetachExit::Ok => 0,
        holler_body::detach::DetachExit::Io => 1,
    }
}

/// `holler body run` (issue #182 connection loop; issue #187 wires the
/// session config in) — resolves the session config (`--config PATH` >
/// `HOLLER_CONFIG` > `./sessions.toml` > `./session.toml`; none found is a
/// fail-closed exit 3, per "every session is explicit"), builds the
/// in-process [`holler_body::registry::SessionRegistry`], and drives the
/// live connection loop: authenticate, exchange hellos, heartbeat (now
/// carrying every registered session, `idle`), survive drops with backoff,
/// and honour `detach`. Runs until a clean end (detach or a signal) or an
/// unretryable authentication failure.
pub fn run(config_flag: Option<&str>) -> i32 {
    let state = match body_state() {
        Ok(s) => s,
        Err(code) => return code,
    };
    // Not-joined (exit 1, `connection::run`'s own check) outranks a missing
    // config (exit 3): an unjoined body has nothing to run regardless of its
    // session list, and `body_run_unjoined_exits_1` pins that ordering — so
    // this is checked *before* config discovery, even though `connection::
    // run` re-checks it a moment later once it actually needs the identity.
    match holler_body::identity::load(&state.root) {
        None => {
            eprintln!("error: not joined; run `holler body join` first");
            return 1;
        }
        Some(Err(e)) => {
            eprintln!("error: state dir: {e}");
            return 1;
        }
        Some(Ok(_)) => {}
    }

    let cwd = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: cannot resolve the current directory: {e}");
            return 1;
        }
    };
    let parsed = match holler_body::config::discover_and_load(config_flag, &cwd) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {}", e.message());
            return 3;
        }
    };
    for w in &parsed.warnings {
        eprintln!("warn: {w}");
    }
    let registry = holler_body::registry::SessionRegistry::from_sessions(parsed.sessions);

    // Issue #190: `connection::run` now owns the registry outright — it
    // starts a `SessionManager` over it (one task per session, for this
    // process's whole live lifetime), derives issue #185's `query/*`
    // config rows from the same registry internally, and drives every
    // `session/presence` from the manager's *live* state, not the
    // registry's own always-`idle` snapshot.
    match holler_body::connection::run(&state.root, registry) {
        holler_body::connection::RunExit::Ok => 0,
        holler_body::connection::RunExit::NotJoined
        | holler_body::connection::RunExit::AuthFailed(_)
        | holler_body::connection::RunExit::Io(_) => 1,
        holler_body::connection::RunExit::LockHeld(_) => 3,
    }
}

/// `holler body status [--json]` — report this process's own identity
/// (local: it reads the credential file, never a live hub). With `--json`,
/// only the status document goes to stdout; without, a short human summary
/// does. An unjoined body is a valid document (`joined: false`, exit 0),
/// not an error; only a state-dir I/O failure is exit 1.
pub fn status(json: bool) -> i32 {
    let state = match body_state() {
        Ok(s) => s,
        Err(code) => return code,
    };
    let (doc, exit) = holler_body::status::status(&state.root);
    let Some(doc) = doc else {
        // A state-dir I/O failure: the helper already printed the reason.
        return 1;
    };
    if json {
        match serde_json::to_string(&doc) {
            Ok(s) => println!("{s}"),
            Err(e) => {
                eprintln!("error: {e}");
                return 1;
            }
        }
    } else {
        println!("{}", holler_body::status::render_human(&doc));
    }
    match exit {
        holler_body::status::StatusExit::Ok => 0,
        holler_body::status::StatusExit::Io => 1,
    }
}

// --- `holler body caps|support|query` (issue #185) --------------------------

/// `holler body caps [--json]` (issue #185): the local status doc plus a
/// support answer for every known feature/harness/capability id (docs
/// §5.2).
pub fn caps(json: bool) -> i32 {
    let state = match body_state() {
        Ok(s) => s,
        Err(code) => return code,
    };
    let identity = holler_body::identity::load(&state.root).and_then(Result::ok);
    let configs = body_local_configs(&state.root);
    let doc = holler_body::query::local_caps(&state.root, identity.as_ref(), &configs);
    let outcome = doc_outcome(&doc, |doc| format!("body: {} caps known", doc.caps.len()));
    print_and_exit_code(json, outcome)
}

/// `holler body support FEATURE [--json]` (issue #185): a single
/// `query/support` answer. An id outside the v2 vocabulary is exit 1 (the
/// wire's `-32006 unknown_feature`, surfaced as a plain stderr reason here —
/// this leaf never touches the wire).
pub fn support(feature: &str, json: bool) -> i32 {
    let state = match body_state() {
        Ok(s) => s,
        Err(code) => return code,
    };
    let configs = body_local_configs(&state.root);
    let outcome = match holler_body::query::local_support(feature, &configs) {
        Ok(doc) => doc_outcome(&doc, |doc| {
            let reason = doc.reason.as_deref().map(|r| format!(" ({r})")).unwrap_or_default();
            format!("{feature}: {}{reason}", if doc.ok { "ok" } else { "not ok" })
        }),
        Err(e) => FetchOutcome::Err { message: e.message, exit_code: 1 },
    };
    print_and_exit_code(json, outcome)
}

/// `holler body query CMD [ARGS...] [--json]` (issue #185): local only — a
/// body never forwards `query/*` to another peer (that is `hub query
/// TARGET …`'s job). A remote-form tail (`query TARGET CMD…`) is a usage
/// error (exit 2): a body has no routing table to resolve TARGET against.
pub fn query(q: &Query, json: bool) -> i32 {
    let resolved = match q.resolve() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };
    let QueryResolution::Local { cmd, args } = &resolved else {
        eprintln!("error: `body query` has no remote target — did you mean `hub query TARGET …`?");
        return 2;
    };
    let state = match body_state() {
        Ok(s) => s,
        Err(code) => return code,
    };
    let identity = holler_body::identity::load(&state.root).and_then(Result::ok);
    let configs = body_local_configs(&state.root);
    let outcome = match cmd {
        Cmd::Status => {
            let doc = holler_body::query::local_status(&state.root, identity.as_ref(), &configs);
            doc_outcome(&doc, |doc| format!("body: role={:?}", doc.role))
        }
        Cmd::Caps => {
            let doc = holler_body::query::local_caps(&state.root, identity.as_ref(), &configs);
            doc_outcome(&doc, |doc| format!("body: {} caps known", doc.caps.len()))
        }
        Cmd::Support => {
            let Some(feature) = args.first() else {
                eprintln!("error: `query support` needs a FEATURE argument");
                return 2;
            };
            match holler_body::query::local_support(feature, &configs) {
                Ok(doc) => doc_outcome(&doc, |doc| format!("{feature}: {}", doc.ok)),
                Err(e) => FetchOutcome::Err { message: e.message, exit_code: 1 },
            }
        }
        Cmd::Protocol => match protocol_version_from_args(args) {
            Ok(version) => {
                let doc = holler_body::query::local_protocol(version);
                doc_outcome(&doc, |doc| {
                    format!("protocol: session={} min={} max={}", doc.session, doc.min, doc.max)
                })
            }
            Err(e) => FetchOutcome::Err { message: e.message, exit_code: 1 },
        },
    };
    print_and_exit_code(json, outcome)
}

/// Build a [`FetchOutcome::Doc`] from any of the local query documents
/// above: `json_text` is `doc`'s own serialization (kept as a plain string,
/// not round-tripped through `serde_json::Value`, so its field order can
/// never be reshuffled — see [`crate::query_cmd::FetchOutcome`]'s doc), and
/// `human` is the caller's own short summary. A serialization failure is
/// defensive (every document here derives `Serialize` over plain,
/// always-encodable fields) but still reports exit 1 rather than panicking.
fn doc_outcome<T: serde::Serialize>(doc: &T, human: impl FnOnce(&T) -> String) -> FetchOutcome {
    match serde_json::to_string(doc) {
        Ok(json_text) => FetchOutcome::Doc { json_text, human: human(doc) },
        Err(e) => FetchOutcome::Err { message: e.to_string(), exit_code: 1 },
    }
}
