//! The registry's unit tests (#185), moved out of `live.rs` whole (#839), so that file has
//! room for the rebind's forward under the 900-line guard. A pure move: the tests are as they
//! were.

use super::*;

#[tokio::test]
async fn find_target_matches_by_token_id_client_id_or_label() {
    let registry = Registry::new();
    let (mut rx, mut cancel_rx, _seq) = registry.insert("cli_1", "kiwi", "tok_1", "127.0.0.1:1").await;

    assert!(matches!(registry.find_target("tok_1").await, TargetLookup::Found(_)));
    assert!(matches!(registry.find_target("cli_1").await, TargetLookup::Found(_)));
    assert!(matches!(registry.find_target("kiwi").await, TargetLookup::Found(_)));
    // A `label/session` form resolves by its label segment.
    assert!(matches!(registry.find_target("kiwi/alpha").await, TargetLookup::Found(_)));
    assert!(matches!(registry.find_target("nobody").await, TargetLookup::NotConnected));

    rx.close();
    cancel_rx.close();
}

/// Two live bodies sharing the same hostname (a collision issue #184's
/// uniqueness enforcement is what actually prevents in practice — this
/// registry alone does not) resolve as [`TargetLookup::Ambiguous`], not a
/// silently-picked one.
#[tokio::test]
async fn ambiguous_target_is_reported_as_ambiguous() {
    let registry = Registry::new();
    let (mut rx1, mut cancel_rx1, _seq) = registry.insert("cli_1", "kiwi", "tok_1", "127.0.0.1:1").await;
    let (mut rx2, mut cancel_rx2, _seq) = registry.insert("cli_2", "kiwi", "tok_2", "127.0.0.1:1").await;

    assert!(matches!(registry.find_target("kiwi").await, TargetLookup::Ambiguous));
    // Each body's own token/client id still resolves unambiguously.
    assert!(matches!(registry.find_target("tok_1").await, TargetLookup::Found(_)));

    rx1.close();
    rx2.close();
    cancel_rx1.close();
    cancel_rx2.close();
}

#[tokio::test]
async fn confirm_harness_and_advertised_are_independent() {
    let registry = Registry::new();
    let (mut rx, mut cancel_rx, _seq) = registry.insert("cli_1", "kiwi", "tok_1", "127.0.0.1:1").await;
    registry.set_harnesses_advertised("cli_1", vec!["opencode".to_string(), "claude".to_string()]).await;
    assert_eq!(registry.harnesses_known().await, vec!["claude".to_string(), "opencode".to_string()]);
    assert!(registry.harnesses_confirmed().await.is_empty(), "advertising alone confirms nothing");

    registry.confirm_harness("cli_1", "opencode").await;
    let confirmed = registry.harnesses_confirmed().await;
    assert_eq!(confirmed.len(), 1);
    assert_eq!(confirmed[0].id, "opencode");
    assert_eq!(confirmed[0].bodies, vec!["kiwi".to_string()]);

    rx.close();
    cancel_rx.close();
}

#[tokio::test]
async fn session_count_sums_across_bodies() {
    let registry = Registry::new();
    let (mut rx1, mut cancel_rx1, _seq) = registry.insert("cli_1", "kiwi", "tok_1", "127.0.0.1:1").await;
    let (mut rx2, mut cancel_rx2, _seq) = registry.insert("cli_2", "mango", "tok_2", "127.0.0.1:1").await;
    registry.set_session_count("cli_1", 2).await;
    registry.set_session_count("cli_2", 3).await;
    assert_eq!(registry.total_sessions().await, 5);
    rx1.close();
    rx2.close();
    cancel_rx1.close();
    cancel_rx2.close();
}
