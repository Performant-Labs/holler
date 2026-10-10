//! How the adapter reaches Herdr: one request out, one reply line back, by a deadline
//! (the Herdr spike, `docs/research/herdr-api-spike.md`, section 3: one request per
//! connection on a local Unix socket).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use holler_pane::PaneError;

use crate::protocol::Request;

/// The most bytes a reply line may hold; a longer one is `unavailable`.
pub const MAX_REPLY_BYTES: usize = 16 * 1024 * 1024;

/// How the adapter reaches Herdr: one request out, one reply line back, by a deadline.
pub trait Transport: Send + Sync {
    /// Send `request.to_line()` and return Herdr's reply line without its newline, or
    /// `timeout` (`op` = "herdr.<method>") once `deadline` passes, or `unavailable`.
    fn exchange(&self, request: &Request, deadline: Instant) -> Result<String, PaneError>;
}

impl<T: Transport + ?Sized> Transport for Arc<T> {
    fn exchange(&self, request: &Request, deadline: Instant) -> Result<String, PaneError> {
        (**self).exchange(request, deadline)
    }
}

/// Herdr's local Unix socket: a new connection per request (spike section 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnixSocketTransport {
    socket: PathBuf,
}

impl UnixSocketTransport {
    /// The transport to the socket at `socket`.
    pub fn new(socket: impl Into<PathBuf>) -> Self {
        Self {
            socket: socket.into(),
        }
    }

    /// The socket's path.
    pub fn socket(&self) -> &Path {
        &self.socket
    }
}

impl Transport for UnixSocketTransport {
    fn exchange(&self, _request: &Request, _deadline: Instant) -> Result<String, PaneError> {
        // stub (#640 part 2): F fills
        Err(PaneError::NotImplemented)
    }
}
