//! A driver-agnostic wrapper (issue #195) so `session_manager::task` can hold
//! either the ACP spawn-mode driver ([`AcpDriver`], issue #188) or the HTTP
//! attach-mode driver ([`HttpAttachDriver`], issue #194) behind one type.
//! This is the entire reason the two drivers were built to share their public
//! method shapes and return the very same [`DriverEventStream`]/
//! [`DriverError`]/[`Status`]/[`StopReason`] types (see each driver's own
//! module doc, and `HttpAttachDriver`'s own doc comment on its shared shape
//! with `AcpDriver`) — `task::Inner` needs exactly one `Option<Driver>` field
//! regardless of which mode a session's config selects, with no second code
//! path for `Prompt`/`Cancel`/`Answer`/`Shutdown` dispatch.

use holler_proto::docs::PendingItem;

use crate::acp_driver::{AcpDriver, DriverError, DriverEventStream, StopReason};
use crate::config::{SessionConfig, SessionMode};
use crate::http_attach_driver::HttpAttachDriver;

/// Either driver, behind one type.
pub(super) enum Driver {
    Acp(AcpDriver),
    HttpAttach(HttpAttachDriver),
}

impl Driver {
    /// Bring up the right driver for `config`'s `mode` — `SessionMode::Spawn`
    /// spawns a fresh harness child ([`AcpDriver::spawn`]); `SessionMode::
    /// Attach` attaches to the already-existing OpenCode session named by
    /// `config.endpoint`/`config.session_id` ([`HttpAttachDriver::attach`]).
    pub(super) async fn spawn_or_attach(config: &SessionConfig) -> Result<Self, DriverError> {
        match config.mode {
            SessionMode::Spawn => AcpDriver::spawn(config).await.map(Driver::Acp),
            SessionMode::Attach => HttpAttachDriver::attach(config).await.map(Driver::HttpAttach),
        }
    }

    pub(super) async fn prompt(&self, text: &str) -> DriverEventStream {
        match self {
            Driver::Acp(d) => d.prompt(text).await,
            Driver::HttpAttach(d) => d.prompt(text).await,
        }
    }

    pub(super) async fn cancel(&self) -> Result<StopReason, DriverError> {
        match self {
            Driver::Acp(d) => d.cancel().await,
            Driver::HttpAttach(d) => d.cancel().await,
        }
    }

    pub(super) async fn answer(&self, choice: &str) -> Result<(), DriverError> {
        match self {
            Driver::Acp(d) => d.answer(choice).await,
            Driver::HttpAttach(d) => d.answer(choice).await,
        }
    }

    pub(super) fn pending(&self) -> Vec<PendingItem> {
        match self {
            Driver::Acp(d) => d.pending(),
            Driver::HttpAttach(d) => d.pending(),
        }
    }

    /// End this driver. For [`Driver::Acp`] this closes the ACP session and
    /// kills the spawned child's process tree (bounded, `AcpDriver`'s own
    /// grace period). For [`Driver::HttpAttach`] this **never touches** the
    /// attached OpenCode process — it only ends this driver's own background
    /// loop (see [`HttpAttachDriver::shutdown`]'s own doc comment) — which is
    /// exactly the issue #195 contract: `SessionManager::shutdown`/`body
    /// detach`/a WS drop/SIGINT must never kill a body that this process
    /// never spawned.
    pub(super) async fn shutdown(&self) -> Result<(), DriverError> {
        match self {
            Driver::Acp(d) => d.shutdown().await,
            Driver::HttpAttach(d) => d.shutdown().await,
        }
    }
}
