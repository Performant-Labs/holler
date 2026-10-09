//! stub (#642a): T's pinned signatures; F fills the blocking loopback HTTP/1.1 client.

use std::time::Duration;

/// A reply: the status and the whole body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    pub status: u16,
    pub body: Vec<u8>,
}

/// Why a request got no reply the adapter can read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpError {
    /// Nothing listens (connect refused).
    Refused,
    /// Connected, no full answer within the timeout (a frozen server).
    TimedOut,
    /// Not an HTTP/1.1 response we can read.
    Garbled(String),
}

/// One request to `http://127.0.0.1:<port><path>`, `Connection: close`, JSON body if given.
pub fn request(
    _port: u16,
    _method: &str,
    _path: &str,
    _body: Option<&serde_json::Value>,
    _timeout: Duration,
) -> Result<Reply, HttpError> {
    // stub (#642a): F fills
    Ok(Reply {
        status: 0,
        body: Vec::new(),
    })
}
