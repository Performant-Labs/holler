//! The version string of the running build (issue #851): the workspace version and the
//! commit the binary was built from, for example `0.4.0 (bda1069)`. `holler --version`
//! prints it after `holler `, and the `version` field of `query/status` (both roles) and of
//! the hub's `control/status` carries it.
//!
//! Only the `holler` binary knows the commit (its build script asks git), and the status
//! documents are built in `holler-hub` and `holler-body`, which cannot depend on the binary.
//! So `main` installs the string here once, before any command runs, the way it installs the
//! logging config ([`crate::log::init`]). Code that runs where nothing installed it (a
//! library's own tests) reads the bare workspace version, the value these fields carried
//! before #851. Not the protocol version: that is [`crate::version`].

use std::sync::OnceLock;

static VERSION: OnceLock<&'static str> = OnceLock::new();

/// Install the running build's version string. The first call wins; a later one changes
/// nothing.
pub fn install(version: &'static str) {
    let _ = VERSION.set(version);
}

/// The running build's version string: the one [`install`]ed, else the bare workspace
/// version (`CARGO_PKG_VERSION`, which every crate of the workspace shares).
pub fn version() -> &'static str {
    VERSION.get().copied().unwrap_or(env!("CARGO_PKG_VERSION"))
}
