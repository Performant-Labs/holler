//! The linked host (#846): one `HostPort` over the test kit's `FakeHost` that also drives the
//! harness fakes, shared by every rig that needs the two to agree (`profile_verbs`' apply rig,
//! `pane_verbs`' launch rig, `pane_claude`'s rig and `pane_integration`'s `bind`).
//!
//! The host and harness fakes share no state (#802). The test kit prescribes the wrapper this
//! module is (`holler-pane-testkit/src/host.rs:12-17`): its `run` also starts what the argv
//! describes and its `stop_owned` also ends what ran for the pane. [`LinkedHost`] is that wrapper
//! once: `ensure_session`, `ps` and `foreground_commands` are the `FakeHost`'s, `run` is the
//! host's run then [`HarnessLink::started`], and `stop_owned` is the host's stop then
//! [`HarnessLink::stopped`]. It lives here, not in the test kit, because a test-kit type is a
//! public API of a non-test crate (brief D2); folding it into the test kit is a follow-up.
//!
//! The links differ in real behaviour, which the rigs' existing tests pin, so it is kept as two
//! named modes rather than unified:
//!
//! - **On start** ([`OnStart`]): [`OnStart::Propagate`] makes a server the fake refuses to start
//!   `run`'s error (profile apply and `pane_integration`); [`OnStart::Swallow`] answers what
//!   `FakeHost::run` answered, as tmux starts the process anyway and the health wait sees it
//!   (`pane_claude`, through [`FakeHarnessesLink`], and the launch rig's own link).
//! - **On stop** ([`OnStop`]): [`OnStop::KillServed`] kills the server of every given port that
//!   runs for the pane (profile apply, and the launch rig when linked); [`OnStop::KillNothing`]
//!   ends no server (`pane_integration`, and the launch rig's plain-host case).
//!
//! The ready-made links are [`FakeHarnessLink`] (a `FakeHarness`, both modes chosen) and
//! [`FakeHarnessesLink`] (the `FakeHarnesses` router: start swallowed, both fakes' processes
//! ended). The launch rig keeps a private link, as its server-start hooks are rig-local, and
//! ends servers through the same [`OnStop`].

use std::borrow::Borrow;

use holler_pane::{Argv, HostPort, PaneError, PaneName};
use holler_pane_testkit::harness::FakeHarness;
use holler_pane_testkit::harnesses::FakeHarnesses;
use holler_pane_testkit::host::FakeHost;

/// What a linked host does with the harness fakes after the `FakeHost` ran or stopped.
pub trait HarnessLink: Send + Sync {
    /// The host ran `argv`: start what it describes.
    fn started(&self, argv: &Argv) -> Result<(), PaneError>;

    /// The host stopped the pane `name`'s processes: end what ran for it.
    fn stopped(&self, name: &PaneName) -> Result<(), PaneError>;
}

/// A `FakeHost` (owned, borrowed or shared) linked to the harness fakes through `link`.
pub struct LinkedHost<H, L> {
    pub host: H,
    pub link: L,
}

impl<H, L> HostPort for LinkedHost<H, L>
where
    H: Borrow<FakeHost> + Send + Sync,
    L: HarnessLink,
{
    fn ensure_session(&self, name: &PaneName, cwd: &str) -> Result<(), PaneError> {
        self.host.borrow().ensure_session(name, cwd)
    }

    fn run(&self, name: &PaneName, cwd: &str, argv: &Argv) -> Result<(), PaneError> {
        self.host.borrow().run(name, cwd, argv)?;
        self.link.started(argv)
    }

    fn stop_owned(&self, name: &PaneName) -> Result<(), PaneError> {
        self.host.borrow().stop_owned(name)?;
        self.link.stopped(name)
    }

    fn ps(&self, name: &PaneName) -> Result<Vec<u32>, PaneError> {
        self.host.borrow().ps(name)
    }

    fn foreground_commands(&self, name: &PaneName) -> Result<Vec<String>, PaneError> {
        self.host.borrow().foreground_commands(name)
    }
}

/// What `run` answers when the fake refuses to start the server its argv describes (a port
/// another pane's server holds, a frozen one).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnStart {
    /// The refusal is `run`'s error.
    Propagate,
    /// `run` answers what `FakeHost::run` answered.
    Swallow,
}

/// What `stop_owned` does to the harness servers after the host stopped the pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnStop {
    /// Kill the server of each of these ports that runs for the pane.
    KillServed(&'static [u16]),
    /// End no server.
    KillNothing,
}

impl OnStop {
    /// End what this mode ends of the pane `name`'s servers in `harness`.
    pub fn end(self, harness: &FakeHarness, name: &PaneName) -> Result<(), PaneError> {
        let OnStop::KillServed(ports) = self else {
            return Ok(());
        };
        for &port in ports {
            if harness.server(port).is_some_and(|s| s.name == *name) {
                harness.kill(port)?;
            }
        }
        Ok(())
    }
}

/// The link to one `FakeHarness` (borrowed, or shared where the host must be `'static`): `run`
/// of an argv `serve_command` built starts its server (any other argv starts none).
pub struct FakeHarnessLink<F> {
    pub harness: F,
    pub on_start: OnStart,
    pub on_stop: OnStop,
}

impl<F: Borrow<FakeHarness> + Send + Sync> HarnessLink for FakeHarnessLink<F> {
    fn started(&self, argv: &Argv) -> Result<(), PaneError> {
        match (self.harness.borrow().start_server(argv), self.on_start) {
            (Some(Err(refused)), OnStart::Propagate) => Err(refused),
            _ => Ok(()),
        }
    }

    fn stopped(&self, name: &PaneName) -> Result<(), PaneError> {
        self.on_stop.end(self.harness.borrow(), name)
    }
}

/// The link to the `FakeHarnesses` router (#831): `run` of an argv either fake built starts it,
/// a refusal swallowed ([`OnStart::Swallow`]); `stop_owned` ends both fakes' processes of the
/// pane (`FakeHarnesses::stop`).
pub struct FakeHarnessesLink<'a>(pub &'a FakeHarnesses);

impl HarnessLink for FakeHarnessesLink<'_> {
    fn started(&self, argv: &Argv) -> Result<(), PaneError> {
        let _ = self.0.start(argv);
        Ok(())
    }

    fn stopped(&self, name: &PaneName) -> Result<(), PaneError> {
        self.0.stop(name);
        Ok(())
    }
}
