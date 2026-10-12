//! The no-hub glue `no_hub`, `wired` and `fake_hub` share (#846; the #649 gate's W-3 and W-6):
//! the OpenCode trap the wiring is pointed at, and the text the CLI prints for a missing hub.

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// The phrase the CLI prints for a missing hub (`say_cmd.rs`, `hub_cmd.rs`; A's W-9).
pub const NO_HUB: &str = "no live holler hub reachable";

/// The mark the OpenCode trap leaves in its directory when it runs.
pub const OPENCODE_MARK: &str = "opencode-ran";

/// Whether `message` names the missing hub and its state dir `state`.
pub fn names_the_missing_hub(message: &str, state: &Path) -> bool {
    message.contains(NO_HUB) && message.contains(&state.display().to_string())
}

/// A scratch directory in `/tmp` holding the OpenCode trap: an executable `opencode` script that
/// leaves [`OPENCODE_MARK`] in the directory when it runs.
pub fn opencode_trap() -> tempfile::TempDir {
    let dir = tempfile::Builder::new()
        .prefix("hlr649-")
        .tempdir_in("/tmp")
        .unwrap();
    let script = dir.path().join("opencode");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\ntouch '{}'\n",
            dir.path().join(OPENCODE_MARK).display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    dir
}
