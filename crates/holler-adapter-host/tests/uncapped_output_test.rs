#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #696
//! The host adapter keeps the whole of a `tmux` call's output, whatever its size (#696): it
//! has no cap, unlike the probe and the other adapters, and the shared bounded runner must
//! not give it one. The reason an `unavailable` quotes is the first line of `tmux`'s stderr,
//! so a `tmux` that fails with a 3 MiB line shows whether anything was cut.

mod common;

use std::time::{Duration, Instant};

use common::{host, name, Fake};
use holler_pane::{HostPort, PaneError};

/// Far past 1 MiB, the largest cap any other consumer of the runner has.
const SIZE: usize = 3 << 20;

#[test]
fn a_tmux_line_of_three_mib_is_quoted_whole_and_never_blocks_the_call() {
    let fake = Fake::new(&format!(
        "head -c {SIZE} /dev/zero | tr '\\0' Z >&2\necho >&2\nexit 1"
    ));
    let host = host(&fake);
    let started = Instant::now();
    let result = host.ps(&name("demo-c1r1"));
    let what = match result {
        Err(PaneError::Unavailable { what }) => what,
        other => panic!("expected Unavailable, got {other:?}"),
    };
    assert!(
        started.elapsed() < Duration::from_secs(4),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(what.matches('Z').count(), SIZE, "kept {} bytes", what.len());
}
