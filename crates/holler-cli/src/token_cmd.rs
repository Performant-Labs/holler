//! `holler hub token …` (story #163), split out of `main.rs` to keep that
//! file under the workspace's 900-line build guard (`scripts/lint.sh` check
//! 4) — the same split `say_cmd.rs` (#190) and `roster_cmd.rs` (#186) made
//! for `say`/`roster` (issue #228).
//!
//! Each verb runs the token store's blocking `flock` operation. The CLI is
//! not a tokio runtime, so these call the store's sync fns directly (the
//! `spawn_blocking` wrappers exist for the serve path, not here). Every verb
//! returns the ADR 0003 exit code its caller (`main.rs`, the one file
//! allowed to end the process) should apply: `0` on success
//! (stdout already printed), `1` on an I/O failure, `3` on a fail-closed
//! policy refusal.

use crate::Mint;
use crate::time_fmt::format_epoch;
use holler_proto::TokenError;

/// Resolve this invocation's state dir, or fail closed (exit 1) if there is
/// nowhere to put the token store. Returns `Err(1)` instead of exiting so
/// every verb below stays a plain function the bin applies an exit code to.
fn token_state() -> Result<holler_hub::state::HubState, i32> {
    match holler_hub::state::resolve_state_dir() {
        Some(root) => Ok(holler_hub::state::HubState::from_root(root)),
        // `resolve_state_dir` already printed the refusal to stderr.
        None => Err(1),
    }
}

/// The one shared error path for the token verbs: print the store's message
/// to stderr and report exit 1 (a store refusal such as an I/O failure). The
/// fail-closed *policy* refusals (duplicate label, unknown id) are mapped to
/// exit 3 at the call sites — see `token_delete`/`token_mint` — because
/// those are the spec's "exit 3" cases.
fn token_err(e: TokenError) -> i32 {
    eprintln!("error: {e}");
    1
}

/// Format a unix epoch second as a local-time `YYYY-MM-DD HH:MM:SS` string
/// (the `list`/`ping` EXPIRES column). No chrono dependency: a manual civil
/// calendar conversion (Howard Hinnant's algorithm) is enough for epoch
/// seconds — see `crate::time_fmt::format_epoch`.
/// `holler hub token mint --label L [--ttl 24h] [--json]` — mint a join
/// token and hand the operator the one-time secret. Human output prints
/// `token_id`, `secret`, `expires`, `hub_pubkey` (issue #322), and a
/// ready-to-paste `holler body join … --hub-key HEX` line; `--json` prints
/// `{token_id, secret, expires, hub_pubkey, join_command}`. A duplicate label
/// or an invalid label is a fail-closed refusal (exit 3); an I/O failure is
/// exit 1.
pub fn mint(mint: &Mint, json: bool) -> i32 {
    let state = match token_state() {
        Ok(s) => s,
        Err(code) => return code,
    };
    let ttl_secs = match parse_ttl(&mint.ttl) {
        Some(t) => t,
        None => {
            eprintln!("error: invalid --ttl {:?} (use e.g. 15m, 2h, 24h)", mint.ttl);
            return 3;
        }
    };
    // Issue #322: resolve (generating on first use, same as `hub serve`) the
    // hub's identity keypair *before* minting — the join line this command
    // prints must carry a real key even on a hub that has never `serve`d yet.
    let hub_pubkey = match holler_hub::identity::ensure(&state) {
        Ok(identity) => identity.public_hex(),
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };
    let minted = match holler_hub::token::mint(&mint.label, ttl_secs, &state) {
        Ok(m) => m,
        Err(e) => {
            // A duplicate label or a malformed label is a policy refusal
            // (exit 3); any other store error (I/O) is a runtime failure (1).
            if e.message.contains("label") {
                eprintln!("error: {e}");
                return 3;
            }
            return token_err(e);
        }
    };
    let join_command = join_command(&state, &minted.record.token_id, &minted.secret, &hub_pubkey);
    if json {
        let doc = serde_json::json!({
            "token_id": minted.record.token_id,
            "secret": minted.secret,
            "expires": minted.record.expires,
            "hub_pubkey": hub_pubkey,
            "join_command": join_command,
        });
        println!("{}", doc);
    } else {
        println!("token_id:    {}", minted.record.token_id);
        println!("secret:      {}   (shown once; not stored)", minted.secret);
        println!("expires:     {}", format_epoch(minted.record.expires));
        println!("hub_pubkey:  {hub_pubkey}");
        println!();
        println!("  {join_command}");
    }
    0
}

/// `holler hub token list [--json]` — list every token. Human output prints
/// a `TOKEN_ID LABEL STATE MACHINE LAST_SEEN EXPIRES` table; secrets are
/// never printed. `--json` prints a `{"tokens":[{token_id,label,state,
/// machine,last_seen,expires}...]}` document (machine = hostname, last_seen/
/// expires as epoch seconds).
pub fn list(json: bool) -> i32 {
    let state = match token_state() {
        Ok(s) => s,
        Err(code) => return code,
    };
    let rows = match holler_hub::token::list(&state) {
        Ok(r) => r,
        Err(e) => return token_err(e),
    };
    if json {
        let toks: Vec<serde_json::Value> = rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "token_id": r.token_id,
                    "label": r.label,
                    "state": holler_hub::token::state_str(r.state),
                    "machine": r.hostname,
                    "last_seen": r.last_seen,
                    "expires": r.expires,
                })
            })
            .collect();
        println!("{}", serde_json::json!({ "tokens": toks }));
    } else {
        println!(
            "{:<14} {:<10} {:<8} {:<10} {:<10} EXPIRES",
            "TOKEN_ID", "LABEL", "STATE", "MACHINE", "LAST_SEEN"
        );
        for r in &rows {
            let machine = r.hostname.as_deref().unwrap_or("-");
            let last_seen = r.last_seen.map(format_epoch).unwrap_or_else(|| "-".into());
            println!(
                "{:<14} {:<10} {:<8} {:<10} {:<10} {}",
                r.token_id,
                r.label,
                holler_hub::token::state_str(r.state),
                machine,
                last_seen,
                format_epoch(r.expires)
            );
        }
    }
    0
}

/// `holler hub token delete ID` (aliases `rm`/`remove`) and
/// `holler hub token revoke ID` share one operation (the store's
/// `delete`): an `unused` token is invalidated (secret void), a `bound`
/// token is revoked (credential void, **row kept**). Both print the spec's
/// shape: `revoked <id> (<label>, <hostname>)` or `invalidated <id> (<label>,
/// unused)`. An unknown id is a fail-closed refusal (exit 3); an I/O failure
/// is exit 1.
pub fn delete(id: &str, json: bool) -> i32 {
    run_inactivate(id, "delete", json)
}

pub fn revoke(id: &str, json: bool) -> i32 {
    run_inactivate(id, "revoke", json)
}

/// The shared delete/revoke body (the store treats them identically). `verb`
/// is only used in the `--json` document (so a machine can tell which
/// spelling was used); the human line's verb word comes from the record's
/// pre-state via `word` (unused→"invalidated", bound→"revoked").
fn run_inactivate(id: &str, verb: &str, json: bool) -> i32 {
    let state = match token_state() {
        Ok(s) => s,
        Err(code) => return code,
    };
    // A bound token is `revoked`; an unused one is `invalidated`. Peek the
    // pre-state to pick the word (the store flips both to `revoked`).
    let was_bound = matches!(
        holler_hub::token::list(&state),
        Ok(rows) if rows.iter().any(|r| r.token_id == id && r.state == holler_hub::token::TokenState::Bound)
    );
    let record = match holler_hub::token::delete(id, &state) {
        Ok(r) => r,
        Err(e) => {
            // An unknown id is a policy refusal (exit 3); an I/O failure is 1.
            if e.message.contains("no such token") {
                eprintln!("error: {e}");
                return 3;
            }
            return token_err(e);
        }
    };
    // Issue #184: force-close the live socket, if `id` currently has one —
    // best-effort (a `NoLiveHub`/any other `ControlError` is silently
    // ignored: the store-side revoke just above already took effect either
    // way, and the spec's "closes the socket immediately" only applies when
    // there is a live hub process to ask).
    let _ = holler_hub::control::revoke_live(id);
    let word = if was_bound { "revoked" } else { "invalidated" };
    let suffix = match &record.hostname {
        Some(host) => format!("{}, {host}", record.label),
        None => format!("{}, unused", record.label),
    };
    if json {
        let doc = serde_json::json!({
            "token_id": record.token_id,
            "label": record.label,
            "state": "revoked",
            "hostname": record.hostname,
            "verb": verb,
        });
        println!("{}", doc);
    } else {
        println!("{word} {} ({suffix})", record.token_id);
    }
    0
}

/// `holler hub token ping ID` (issue #182): first the store's own record
/// check — a `revoked`/`expired`/`not_found` token can never be live, so
/// those stay a fail-closed policy refusal (exit 3) without ever touching
/// the live hub. An **unused** (not-yet-redeemed) token has never had a
/// socket to begin with, so it keeps the pre-#182 behaviour: `valid`, exit
/// 0 (there is nothing live to probe). Only a **bound** token is actually
/// probed: the control socket asks the live hub to send a `circuit/ping`
/// over that token's socket and report `{hostname, rtt_ms}`. No live socket
/// for it — including no live hub at all — is the spec's `-32004
/// not_connected` (exit 1); a genuine answer is exit 0.
pub fn ping(id: &str, json: bool) -> i32 {
    let state = match token_state() {
        Ok(s) => s,
        Err(code) => return code,
    };
    let rows = match holler_hub::token::list(&state) {
        Ok(r) => r,
        Err(e) => return token_err(e),
    };
    let Some(record) = rows.iter().find(|r| r.token_id == id) else {
        if json {
            println!("{}", serde_json::json!({ "token_id": id, "state": "not_found" }));
        }
        eprintln!("not_found {id} (no such token)");
        return 3;
    };
    let now = holler_hub::token::now_secs();
    let refusal = match record.state {
        holler_hub::token::TokenState::Revoked => Some("revoked"),
        holler_hub::token::TokenState::Unused if record.expires < now => Some("expired"),
        holler_hub::token::TokenState::Bound if record.expires < now => Some("expired"),
        holler_hub::token::TokenState::Unused => None, // never redeemed: nothing to probe, `valid`.
        holler_hub::token::TokenState::Bound => None,  // worth probing live.
    };
    if let Some(status) = refusal {
        if json {
            println!("{}", serde_json::json!({ "token_id": id, "label": record.label, "state": status }));
        } else {
            println!("{status} ({})", record.label);
        }
        return 3;
    }
    if record.state == holler_hub::token::TokenState::Bound {
        return ping_live(id, &record.label, json);
    }
    if json {
        println!("{}", serde_json::json!({ "token_id": id, "label": record.label, "state": "valid" }));
    } else {
        println!("valid ({})", record.label);
    }
    0
}

/// The live half of `hub token ping`: probe the hub's control socket. Split
/// out of [`ping`] to keep that dispatch's cognitive complexity under the
/// workspace threshold.
fn ping_live(id: &str, label: &str, json: bool) -> i32 {
    match holler_hub::control::token_ping(id) {
        Ok(doc) => {
            let hostname = doc.get("hostname").and_then(|v| v.as_str()).unwrap_or("?");
            let rtt_ms = doc.get("rtt_ms").and_then(|v| v.as_u64()).unwrap_or(0);
            if json {
                println!("{}", serde_json::json!({ "token_id": id, "label": label, "state": "valid", "hostname": hostname, "rtt_ms": rtt_ms }));
            } else {
                println!("valid ({label}, {hostname}) rtt={rtt_ms}ms");
            }
            0
        }
        Err(e) => {
            if json {
                println!("{}", serde_json::json!({ "token_id": id, "label": label, "state": "not_connected" }));
            }
            eprintln!("not_connected {id} ({e})");
            1
        }
    }
}

/// Parse a `--ttl` like `15m` / `2h` / `24h` into seconds, or `None` if the
/// spelling is not a `<number><m|h|d>` duration (the spec's TTL grammar).
/// The number is the leading run of digits; the unit is the single trailing
/// character. A leading `+`/`-`, an empty number, or a missing/unknown unit
/// all refuse (fail closed at the call site with exit 3).
fn parse_ttl(ttl: &str) -> Option<u64> {
    let digits: &str = ttl.split(|c: char| !c.is_ascii_digit()).next()?;
    if digits.is_empty() {
        return None;
    }
    let rest = &ttl[digits.len()..];
    if rest.len() != 1 {
        return None;
    }
    let num: u64 = digits.parse().ok()?;
    match rest {
        "m" => Some(num * 60),
        "h" => Some(num * 3600),
        "d" => Some(num * 86400),
        _ => None,
    }
}

/// The ready-to-paste `holler body join --server <adv> --token <id>:<secret>
/// --hub-key <hex>` line `mint` prints (and `--json`'s `join_command`). The
/// server is the hub's persisted advertise address if `hub serve --advertise`
/// set one, else the spec's loopback default `ws://127.0.0.1:41807`.
/// `--hub-key` (issue #322) is the hub's X25519 public key: the operator
/// carries it over the same out-of-band channel as the secret, and `body
/// join` pins it — this is the physical channel that makes the pin
/// meaningful, so the join line is where it belongs, not a separate step.
fn join_command(state: &holler_hub::state::HubState, token_id: &str, secret: &str, hub_pubkey: &str) -> String {
    let server = std::fs::read_to_string(holler_hub::state::advertise_path(state))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("advertise").and_then(|a| a.as_str()).map(String::from))
        .unwrap_or_else(|| "ws://127.0.0.1:41807".into());
    format!("holler body join --server {server} --token {token_id}:{secret} --hub-key {hub_pubkey}")
}
