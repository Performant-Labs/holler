#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #483
//! Issue #483: the body's X25519 identity key is created once, atomically.
//! Concurrent first callers of `x25519_identity::ensure` must all end up with
//! the one key that was persisted: never a second key, never a read of a
//! half-written file.

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::sync::{Arc, Barrier};

use holler_body::x25519_identity;

#[test]
fn concurrent_first_ensures_all_return_the_one_persisted_key() {
    const CALLERS: usize = 16;
    for round in 0..20 {
        let root = std::env::temp_dir().join(format!("holler-body-x25519race-{}-{round}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("scratch dir");
        let barrier = Arc::new(Barrier::new(CALLERS));
        let handles: Vec<_> = (0..CALLERS)
            .map(|_| {
                let (root, barrier) = (root.clone(), barrier.clone());
                std::thread::spawn(move || {
                    barrier.wait();
                    x25519_identity::ensure(&root).map(|id| id.public_hex()).map_err(|e| e.to_string())
                })
            })
            .collect();
        let keys: Vec<String> = handles
            .into_iter()
            .map(|h| h.join().expect("caller thread").unwrap_or_else(|e| panic!("round {round}: ensure failed: {e}")))
            .collect();

        let persisted = x25519_identity::ensure(&root).expect("reload").public_hex();
        for (i, key) in keys.iter().enumerate() {
            assert_eq!(key, &persisted, "round {round}: caller {i} got a key that is not the persisted one");
        }
        let path = x25519_identity::identity_path(&root);
        assert_eq!(std::fs::metadata(&path).expect("stat").len(), 32, "round {round}: the key file is raw 32 bytes");
        let mode = std::fs::metadata(&path).expect("stat").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "round {round}: x25519_identity.key must be 0600");
        let _ = std::fs::remove_dir_all(&root);
    }
}
