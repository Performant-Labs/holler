//! The server side of the WebSocket opening handshake over an already-
//! accepted loopback `TcpStream` (split out of `serve.rs` so that file stays
//! under the 900-line lint gate).
//!
//! tokio-tungstenite ships only a *client* async handshake (`connect_async`);
//! its `handshake::server` machine is blocking (`std::io::Read`/`Write`), which
//! a tokio `TcpStream` does not implement. So we do the handshake by hand on
//! the async socket: read the upgrade `GET` (until the blank line), parse it,
//! compute the `Sec-WebSocket-Accept` via tungstenite's own (tested)
//! `create_response`, write the `101` back with `AsyncWrite`, and hand the
//! socket — plus any bytes we over-read — to `from_partially_read` so no frame
//! data is lost. A client that sends a non-WebSocket `GET` (or nothing) is
//! dropped (returns `None`).

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    tungstenite::{
        handshake::server::{create_response, Request as WsRequest, Response as WsResponse},
        protocol::Role,
    },
    WebSocketStream,
};
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;

/// Perform the server handshake on an already-accepted `TcpStream` and return
/// the upgraded `WebSocketStream` (or `None` if the peer is not a WebSocket
/// client, or leaves mid-handshake).
pub async fn server_handshake(
    mut stream: TcpStream,
    config: WebSocketConfig,
) -> Option<WebSocketStream<TcpStream>> {
    // 1. Drain the HTTP upgrade request: read until the end of the header
    //    section (`\r\n\r\n`). A WebSocket `GET` has no body, so the bytes
    //    after the blank line (if any) are already the first WS frame.
    let mut buf: Vec<u8> = Vec::new();
    let mut tmp = [0u8; 1024];
    let header_end = loop {
        let n = stream.read(&mut tmp).await.ok()?;
        if n == 0 {
            return None; // peer closed before sending a request.
        }
        buf.extend_from_slice(&tmp[..n]);
        // The header section ends at the first `\r\n\r\n`.
        if let Some(pos) = find_blank_line(&buf) {
            break pos;
        }
        if buf.len() > 64 * 1024 {
            return None; // header too large: not a sane upgrade request.
        }
    };

    // `head` is the request (up to and including the blank line); `rest` is
    // any bytes already received past the blank line (the start of a frame).
    let (head, rest) = buf.split_at(header_end);

    // 2. Parse the request and build the 101 response via tungstenite's own
    //    (correct-GUID) accept-key computation.
    let request: WsRequest = match parse_upgrade_request(head) {
        Some(r) => r,
        None => return None, // not a parseable HTTP request (or bad method/headers).
    };
    let response: WsResponse = match create_response(&request) {
        Ok(r) => r,
        Err(_) => return None, // rejected the upgrade (bad key/headers/method).
    };

    // 3. Serialise the 101 response to bytes and write it to the peer.
    let bytes = serialize_response(&response);
    if stream.write_all(&bytes).await.is_err() {
        return None;
    }
    if stream.flush().await.is_err() {
        return None;
    }

    // 4. Wrap the (already-upgraded) socket, folding in any over-read bytes so
    //    the first frame is not lost. No handshake is repeated. `config`
    //    carries the per-connection hygiene (the 2 MiB frame cap) so an
    //    oversized inbound message is rejected with a 1009 close.
    Some(
        WebSocketStream::from_partially_read(
            stream,
            rest.to_vec(),
            Role::Server,
            Some(config),
        )
        .await,
    )
}

/// The offset (exclusive) just past the first `\r\n\r\n` in `buf`, or `None`.
fn find_blank_line(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n").map(|p| p + 4)
}

/// Parse a raw HTTP upgrade request (the bytes up to the blank line) into a
/// tungstenite server `Request`. Returns `None` if it does not parse as an
/// HTTP/1.1 `GET` with headers (a non-WebSocket peer).
fn parse_upgrade_request(head: &[u8]) -> Option<WsRequest> {
    use httparse::Request as RawRequest;
    // httparse parses in place; the header fields borrow from `head`, so the
    // scratch buffer must outlive the `Request` we build. 64 headers is far
    // more than a WebSocket upgrade carries (Host/Connection/Upgrade/Version/Key).
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut raw = RawRequest::new(&mut headers);
    match raw.parse(head).ok()? {
        httparse::Status::Complete(_) => {}
        // An incomplete parse (a header line we read too early) is not a
        // usable request.
        httparse::Status::Partial => return None,
    }
    let method = raw.method?;
    let path = raw.path.unwrap_or("/");
    // A WebSocket upgrade is always HTTP/1.1 (httparse reports the numeric
    // version, `1`); the response below is emitted as HTTP/1.1.
    let mut builder = http::Request::builder()
        .method(method)
        .uri(path)
        .version(http::Version::HTTP_11);
    for h in raw.headers.iter() {
        // httparse's header name/value borrow from `head`; the `http` crate
        // wants owned (or `'static`) header parts, so copy via `from_bytes`
        // (the same route tungstenite's own handshake takes).
        let name = match http::header::HeaderName::from_bytes(h.name.as_bytes()) {
            Ok(n) => n,
            Err(_) => continue,
        };
        let value = match http::header::HeaderValue::from_bytes(h.value) {
            Ok(v) => v,
            Err(_) => continue,
        };
        builder = builder.header(name, value);
    }
    builder.body(()).ok()
}

/// Serialise a tungstenite `101` `Response<()>` to the exact bytes an HTTP
/// writer would emit (status line, each header, then a blank line). We avoid
/// tungstenite's `write_response` (it needs a *sync* `Write`, which a tokio
/// `TcpStream` is not) by emitting the bytes ourselves and writing with
/// `AsyncWrite`.
fn serialize_response(response: &WsResponse) -> Vec<u8> {
    use http::header::HeaderName;
    let mut out = Vec::new();
    out.extend_from_slice(b"HTTP/1.1 101 Switching Protocols\r\n");
    for (name, value) in response.headers() {
        let name: &HeaderName = name;
        let value = value.as_bytes();
        out.extend_from_slice(name.as_str().as_bytes());
        out.extend_from_slice(b": ");
        out.extend_from_slice(value);
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(b"\r\n");
    out
}
