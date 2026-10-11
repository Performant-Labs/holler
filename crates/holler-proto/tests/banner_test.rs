//! Banner contract (story #144) — isolated in its **own binary** because
//! `emit_banner` guards on a process-global `OnceLock`/`AtomicU8`. The
//! "exactly once" semantics can only be observed from a fresh process, so this
//! test does not share a process with any other test that installs/prints the
//! banner.
//!
//! It pins: the first `emit_banner` prints the `logging_started` line naming
//! the resolved level and format, and a second call within the same process is
//! a no-op (empty string) — the banner must not print twice.

use holler_proto::log::{emit_banner, init, Config, DebugLevel, LogFormat};

#[test]
fn banner_is_emitted_exactly_once_and_names_settings() {
    // Install the settings the banner will report.
    init(Config {
        debug: DebugLevel::Noisy,
        format: LogFormat::Json,
    });

    // The first emit prints the banner (also written to stderr).
    let first = emit_banner();
    assert!(!first.is_empty(), "the first emit_banner must print the banner");
    assert!(first.contains("logging_started"), "banner names the event: {first:?}");
    assert!(first.contains("level=noisy"), "banner reports the level: {first:?}");
    assert!(first.contains("format=json"), "banner reports the format: {first:?}");

    // A second emit within the same process is a no-op (the once-guard held).
    let second = emit_banner();
    assert!(second.is_empty(), "the banner must not print twice: {second:?}");
}
