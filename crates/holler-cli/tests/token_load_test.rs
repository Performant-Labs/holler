#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #301
//! Load test (issue #301): token mint/redeem at realistic onboarding volume.
//!
//! **Bounded/automated**, per the issue's own scope. `token_store_test.rs`'s
//! `concurrent_redeem_has_exactly_one_winner` already covers the adversarial
//! *same*-token race (many threads redeeming one secret, exactly one must
//! win). This file exercises the different, unaddressed shape: many
//! *distinct*, legitimately minted tokens redeemed **concurrently** by that
//! many distinct, real `holler body join` subprocesses — the onboarding-at-
//! scale case a real team rollout produces. Real OS threads each launching a
//! genuine subprocess against a genuine running hub (no in-process
//! simulation of concurrency).

mod support;

use std::collections::HashSet;

use serde_json::Value;

use support::{body_status_json, join, mint_token, Hub, StateDir};

/// Distinct tokens minted and redeemed. Within the issue's 20-50 bound.
const TOKENS: usize = 30;

#[test]
fn concurrent_distinct_token_redemptions_are_isolated_and_uncorrupted() {
    let hub_state = StateDir::new();
    let hub = Hub::start(&hub_state);

    // Mint N distinct tokens up front. Sequential: #301's own bounded scope is
    // concurrent *redeem*, not concurrent mint — concurrent mint/delete
    // against the store is already covered directly by
    // `holler-hub/tests/token_store_test.rs::concurrent_mint_delete_do_not_lose_writes`.
    let minted: Vec<(String, String, String)> = (0..TOKENS)
        .map(|i| {
            let label = format!("body{i}");
            let (token_id, secret) = mint_token(&hub_state, &label);
            (label, token_id, secret)
        })
        .collect();
    assert_eq!(minted.len(), TOKENS, "all {TOKENS} distinct tokens minted");
    // Every minted token_id is distinct up front (the store's own id
    // generation must never collide across a real onboarding batch).
    let minted_ids: HashSet<&str> = minted.iter().map(|(_, id, _)| id.as_str()).collect();
    assert_eq!(minted_ids.len(), TOKENS, "every minted token_id is unique");

    let ws_url = hub.ws_url();

    // Real concurrency: one OS thread per body, each spawning a genuine
    // `holler body join` subprocess (support::join, which shells out to the
    // real binary and blocks on its real exit) against its own isolated
    // state dir, launched at (as close as this harness gets to) the same
    // moment — not a single process simulating N redeemers in-loop.
    //
    // A scoped thread (rather than `thread::spawn`) so each closure can
    // borrow `hub_state` directly for `join`'s hub-pubkey lookup (issue
    // #322: pinning reads the *hub's* state dir, not the body's) instead of
    // needing a `'static` clone of it.
    let results: Vec<(String, String, StateDir)> = std::thread::scope(|scope| {
        let handles: Vec<_> = minted
            .iter()
            .cloned()
            .map(|(label, token_id, secret)| {
                let ws_url = ws_url.clone();
                let hub_state = &hub_state;
                scope.spawn(move || {
                    let body_state = StateDir::new();
                    join(&body_state, hub_state, &ws_url, &token_id, &secret);
                    (label, token_id, body_state)
                })
            })
            .collect();

        handles.into_iter().map(|h| h.join().expect("body join thread")).collect()
    });
    assert_eq!(results.len(), TOKENS, "every one of the {TOKENS} concurrent joins completed");

    let client_ids = assert_every_body_joined_its_own_token(&results);
    assert_store_is_uncorrupted(&hub_state, &results, &client_ids);
}

/// Every body's own persisted identity: joined, naming exactly the token it
/// redeemed, with a well-formed client_id — and no two bodies were handed the
/// same client_id (that would be a hub-side credential mix-up, the sharpest
/// possible form of cross-token corruption). Returns the set of client_ids
/// observed, for [`assert_store_is_uncorrupted`] to cross-check against the
/// store's own view.
fn assert_every_body_joined_its_own_token(results: &[(String, String, StateDir)]) -> HashSet<String> {
    let mut client_ids = HashSet::new();
    for (label, token_id, body_state) in results {
        let doc = body_status_json(body_state);
        assert_eq!(doc["joined"], Value::Bool(true), "{label} must be joined: {doc}");
        assert_eq!(
            doc["token_id"].as_str(),
            Some(token_id.as_str()),
            "{label}'s own body status must name the token it actually redeemed, not another body's: {doc}"
        );
        let client_id = doc["client_id"]
            .as_str()
            .unwrap_or_else(|| panic!("{label}'s body status is missing client_id: {doc}"))
            .to_string();
        assert!(client_id.starts_with("cli_"), "{label}'s client_id looks like a real minted id: {client_id}");
        assert!(
            client_ids.insert(client_id.clone()),
            "{label}'s client_id {client_id} collides with another concurrently-joined body — cross-token corruption"
        );
    }
    assert_eq!(client_ids.len(), TOKENS, "every one of the {TOKENS} bodies got its own distinct client_id");
    client_ids
}

/// The hub-side token store, after all TOKENS concurrent redemptions: no row
/// lost or duplicated, every row bound to exactly the right label and a
/// distinct client_id (proves the on-disk store itself — not just each
/// body's local view — came out uncorrupted under the concurrent
/// flock-guarded writes). Reads the raw `tokens.json` directly (the same file
/// `holler-hub/tests/token_store_test.rs` inspects) rather than `hub token
/// list --json`: that CLI document is a deliberately curated view
/// (`token_cmd.rs::list` — `token_id, label, state, machine, last_seen,
/// expires`) that never carries `client_id`/`body_pubkey`, so it can't
/// answer the cross-token-corruption question this test asks.
fn assert_store_is_uncorrupted(hub_state: &StateDir, results: &[(String, String, StateDir)], client_ids: &HashSet<String>) {
    let raw = std::fs::read_to_string(hub_state.hub().join("tokens.json")).expect("read tokens.json");
    let doc: Value = serde_json::from_str(&raw).expect("tokens.json is valid JSON");
    let rows = doc.as_array().expect("tokens.json is a JSON array of records");
    assert_eq!(rows.len(), TOKENS, "the store has exactly the {TOKENS} minted rows, none lost/duplicated");

    let mut store_client_ids = HashSet::new();
    for (label, token_id, _body_state) in results {
        let row = rows
            .iter()
            .find(|r| r["token_id"].as_str() == Some(token_id.as_str()))
            .unwrap_or_else(|| panic!("token {token_id} ({label}) is missing from the store after redeem: {rows:?}"));
        assert_eq!(row["state"].as_str(), Some("bound"), "{label}'s token row must read bound: {row}");
        assert_eq!(row["label"].as_str(), Some(label.as_str()), "{label}'s row keeps its own label, not another's: {row}");
        let row_client_id = row["client_id"]
            .as_str()
            .unwrap_or_else(|| panic!("{label}'s bound row is missing client_id: {row}"))
            .to_string();
        assert!(
            store_client_ids.insert(row_client_id.clone()),
            "{label}'s stored client_id {row_client_id} is duplicated across rows in the store — cross-token corruption"
        );
    }
    assert_eq!(store_client_ids.len(), TOKENS, "every stored row is bound to a distinct client_id — no cross-token corruption");
    assert_eq!(
        &store_client_ids, client_ids,
        "the store's client_ids exactly match what each body itself observed — hub and body agree, nothing crossed"
    );
}
