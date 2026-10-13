//! A driver-agnostic wrapper (issue #195) so `session_manager::task` can hold
//! the ACP spawn-mode driver ([`AcpDriver`], issue #188), the HTTP
//! attach-mode driver ([`HttpAttachDriver`], issue #194) or the attach driver
//! of a Claude Code process ([`ClaudeAttachDriver`], #850) behind one type.
//! This is the entire reason the two drivers were built to share their public
//! method shapes and return the very same [`DriverEventStream`]/
//! [`DriverError`]/[`Status`]/[`StopReason`] types (see each driver's own
//! module doc, and `HttpAttachDriver`'s own doc comment on its shared shape
//! with `AcpDriver`) — `task::Inner` needs exactly one `Option<Driver>` field
//! regardless of which mode a session's config selects, with no second code
//! path for `Prompt`/`Cancel`/`Answer`/`Shutdown` dispatch.

use std::sync::Arc;

use holler_pane::HarnessPort;
use holler_proto::docs::PendingItem;

use crate::acp_driver::{AcpDriver, DriverError, DriverEventStream, StopReason};
use crate::claude_attach_driver::ClaudeAttachDriver;
use crate::config::{SessionConfig, SessionMode};
use crate::http_attach_driver::HttpAttachDriver;

/// Any driver, behind one type.
pub(super) enum Driver {
    Acp(AcpDriver),
    HttpAttach(HttpAttachDriver),
    ClaudeAttach(ClaudeAttachDriver),
}

impl Driver {
    /// Bring up the right driver for `config`, the one place a driver is
    /// chosen — `SessionMode::Spawn` spawns a fresh harness child
    /// ([`AcpDriver::spawn`]); an attach row that attaches to a Claude Code
    /// process ([`SessionConfig::attaches_to_claude_code`], #850) attaches
    /// through `claude`, the harness the body was handed
    /// ([`ClaudeAttachDriver::attach`]), and is refused when it was handed
    /// none; any other attach row attaches to the already-existing OpenCode
    /// session named by `config.endpoint`/`config.session_id`
    /// ([`HttpAttachDriver::attach`]).
    pub(super) async fn spawn_or_attach(
        config: &SessionConfig,
        claude: Option<&Arc<dyn HarnessPort>>,
    ) -> Result<Self, DriverError> {
        match config.mode {
            SessionMode::Spawn => AcpDriver::spawn(config).await.map(Driver::Acp),
            SessionMode::Attach if config.attaches_to_claude_code() => {
                let port = claude.ok_or_else(|| {
                    DriverError::Startup(
                        "this body was handed no Claude Code harness, so a row that attaches to a \
                         Claude Code process cannot attach; `holler body run` builds one from the \
                         `claude` program (HOLLER_CLAUDE_BIN, else PATH), Claude Code's \
                         configuration directory and the Holler state directory, and warned at \
                         its start which of them did not resolve"
                            .to_owned(),
                    )
                })?;
                ClaudeAttachDriver::attach(config, Arc::clone(port)).await.map(Driver::ClaudeAttach)
            }
            SessionMode::Attach => HttpAttachDriver::attach(config).await.map(Driver::HttpAttach),
        }
    }

    pub(super) async fn prompt(&self, text: &str) -> DriverEventStream {
        match self {
            Driver::Acp(d) => d.prompt(text).await,
            Driver::HttpAttach(d) => d.prompt(text).await,
            Driver::ClaudeAttach(d) => d.prompt(text).await,
        }
    }

    pub(super) async fn cancel(&self) -> Result<StopReason, DriverError> {
        match self {
            Driver::Acp(d) => d.cancel().await,
            Driver::HttpAttach(d) => d.cancel().await,
            Driver::ClaudeAttach(d) => d.cancel().await,
        }
    }

    pub(super) async fn answer(&self, choice: &str) -> Result<(), DriverError> {
        match self {
            Driver::Acp(d) => d.answer(choice).await,
            Driver::HttpAttach(d) => d.answer(choice).await,
            Driver::ClaudeAttach(d) => d.answer(choice).await,
        }
    }

    pub(super) fn pending(&self) -> Vec<PendingItem> {
        match self {
            Driver::Acp(d) => d.pending(),
            Driver::HttpAttach(d) => d.pending(),
            Driver::ClaudeAttach(d) => d.pending(),
        }
    }

    /// End this driver. For [`Driver::Acp`] this closes the ACP session and
    /// kills the spawned child's process tree (bounded, `AcpDriver`'s own
    /// grace period). For [`Driver::HttpAttach`] this **never touches** the
    /// attached OpenCode process — it only ends this driver's own background
    /// loop (see [`HttpAttachDriver::shutdown`]'s own doc comment) — which is
    /// exactly the issue #195 contract: `SessionManager::shutdown`/`body
    /// detach`/a WS drop/SIGINT must never kill a body that this process
    /// never spawned. [`Driver::ClaudeAttach`] likewise ends only its own
    /// turn task and never touches the Claude Code process (#850).
    pub(super) async fn shutdown(&self) -> Result<(), DriverError> {
        match self {
            Driver::Acp(d) => d.shutdown().await,
            Driver::HttpAttach(d) => d.shutdown().await,
            Driver::ClaudeAttach(d) => d.shutdown().await,
        }
    }
}
