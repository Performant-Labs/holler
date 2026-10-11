#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #483
//! Issue #483: the hub identity key is created once, atomically. Concurrent
//! first callers of `identity::ensure` (the per-handshake paths can race a
//! failed startup `ensure`, and `hub serve` races `hub token mint`) must all
//! end up with the one key that was persisted: never a second key, never a
//! read of a half-written file.

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::sync::{Arc, Barrier};

use holler_hub::identity;
use holler_hub::state::HubState;

#[test]
fn concurrent_first_ensures_all_return_the_one_persisted_key() {
    const CALLERS: usize = 16;
    for round in 0..20 {
        let root = std::env::temp_dir().join(format!("holler-hub-idrace-{}-{round}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("scratch dir");
        let barrier = Arc::new(Barrier::new(CALLERS));
        let handles: Vec<_> = (0..CALLERS)
            .map(|_| {
                let (root, barrier) = (root.clone(), barrier.clone());
                std::thread::spawn(move || {
                    let state = HubState::from_root(root);
                    barrier.wait();
                    identity::ensure(&state).map(|id| id.public_hex()).map_err(|e| e.message)
                })
            })
            .collect();
        let keys: Vec<String> = handles
            .into_iter()
            .map(|h| h.join().expect("caller thread").unwrap_or_else(|e| panic!("round {round}: ensure failed: {e}")))
            .collect();

        let state = HubState::from_root(root.clone());
        let persisted = identity::ensure(&state).expect("reload").public_hex();
        for (i, key) in keys.iter().enumerate() {
            assert_eq!(key, &persisted, "round {round}: caller {i} got a key that is not the persisted one");
        }
        let path = identity::identity_path(&state);
        assert_eq!(std::fs::metadata(&path).expect("stat").len(), 32, "round {round}: the key file is raw 32 bytes");
        let mode = std::fs::metadata(&path).expect("stat").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "round {round}: identity.key must be 0600");
        let _ = std::fs::remove_dir_all(&root);
    }
}
