//! The hub's WS listen-address defaulting and the round-robin accept future
//! over its bound listeners (issue #469), split out of `serve.rs` to keep that
//! file under the 900-line build guard (precedent: `ws_handshake.rs`, #184).

use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::net::{TcpListener, TcpStream};

/// The address `hub serve` listens on when no `--listen` is given: loopback
/// plain `ws` (ADR 0006), the default `docs/protocol/v2.md` documents. This is
/// the one copy of the default in the hub and the CLI; `hub token mint`'s join
/// line dials it when the hub has no `--advertise`.
pub const DEFAULT_LISTEN: &str = "127.0.0.1:41807";

/// The addresses the hub listens on: the `--listen` values when any are given
/// (they replace the default, never add to it), else [`DEFAULT_LISTEN`] alone.
/// Never empty, so the hub always has an address to bind.
pub(crate) fn effective_listen(listen: &[String]) -> Vec<String> {
    if listen.is_empty() {
        vec![DEFAULT_LISTEN.to_string()]
    } else {
        listen.to_vec()
    }
}

/// A future that resolves when **any** of the WS listeners has a pending
/// accept. The listeners are polled round-robin, starting after the one that
/// resolved last, so one busy listener cannot starve the others;
/// `tokio::select!` in the caller drives it.
///
/// `L` holds the listeners: the hub moves its `Vec` in (the accept loop runs
/// in a spawned task), and a caller that keeps them lends a reference.
/// [`AcceptAny::new`] refuses an empty list (issue #469), so there is always a
/// listener to poll.
pub(crate) struct AcceptAny<L> {
    listeners: L,
    /// The listener polled first: the one after the last to resolve.
    idx: usize,
}

impl<L: AsRef<[TcpListener]>> AcceptAny<L> {
    /// `None` for an empty listener list. A future over no listeners could
    /// never resolve, and the old index-based poll panicked on one (issue
    /// #469); the caller turns this refusal into an exit code.
    pub(crate) fn new(listeners: L) -> Option<Self> {
        if listeners.as_ref().is_empty() {
            None
        } else {
            Some(AcceptAny { listeners, idx: 0 })
        }
    }
}

impl<L: AsRef<[TcpListener]> + Unpin> Future for AcceptAny<L> {
    type Output = std::io::Result<(TcpStream, SocketAddr)>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let listeners = this.listeners.as_ref();
        let n = listeners.len();
        // Each listener once, from `idx` to the end and then from the start:
        // iterating rather than indexing, so no poll can go out of bounds.
        let order = listeners.iter().enumerate().skip(this.idx);
        for (i, listener) in order.chain(listeners.iter().enumerate().take(this.idx)) {
            match listener.poll_accept(cx) {
                // A connection or an accept error resolves the future; the
                // next poll starts after this listener (`n` > 0: it exists).
                Poll::Ready(res) => {
                    this.idx = (i + 1) % n;
                    return Poll::Ready(res);
                }
                // No connection pending on this listener; `poll_accept`
                // registered `cx` with the reactor, so we will be woken when
                // one arrives.
                Poll::Pending => {}
            }
        }
        Poll::Pending
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #469
mod tests {
    use super::{effective_listen, AcceptAny, DEFAULT_LISTEN};
    use tokio::net::TcpListener;

    /// Criterion 1: with no `--listen`, the hub listens on exactly the one
    /// documented loopback default (ADR 0006), and the constant is that literal
    /// (the drift check `token_cmd`'s default join URL also relies on).
    #[test]
    fn effective_listen_with_nothing_given_is_the_one_default() {
        assert_eq!(DEFAULT_LISTEN, "127.0.0.1:41807", "the documented default listen address");
        assert_eq!(
            effective_listen(&[]),
            vec!["127.0.0.1:41807".to_string()],
            "no --listen must resolve to exactly the default, nothing else"
        );
    }

    /// Criterion 1: any `--listen` replaces the default (never appends to it),
    /// and several given addresses are all kept, in the order given.
    #[test]
    fn effective_listen_given_addresses_replace_the_default_in_order() {
        assert_eq!(
            effective_listen(&["127.0.0.1:0".to_string()]),
            vec!["127.0.0.1:0".to_string()],
            "an explicit --listen replaces the default"
        );
        let two = vec!["127.0.0.1:5001".to_string(), "[::1]:5002".to_string()];
        assert_eq!(effective_listen(&two), two, "two given addresses are both kept, in order");
    }

    /// Criterion 2: an `AcceptAny` over zero listeners is refused at
    /// construction (decision 7), so the accept loop can never index into an
    /// empty list (the `len().max(1)` / `listeners[0]` panic) or hot-spin.
    #[test]
    fn accept_any_refuses_an_empty_listener_list() {
        let none: Vec<TcpListener> = Vec::new();
        assert!(
            AcceptAny::new(&none).is_none(),
            "AcceptAny over no listeners must be refused, not built"
        );
    }

    /// The relocation keeps the round-robin contract: a connection pending on
    /// the *second* of two listeners resolves the future with that peer.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn accept_any_resolves_a_connection_on_any_listener() {
        let first = TcpListener::bind("127.0.0.1:0").await.expect("bind the first listener");
        let second = TcpListener::bind("127.0.0.1:0").await.expect("bind the second listener");
        let target = second.local_addr().expect("second listener addr");
        let listeners = vec![first, second];

        let client = tokio::net::TcpStream::connect(target).await.expect("dial the second listener");
        let client_addr = client.local_addr().expect("client local addr");

        let accept = AcceptAny::new(&listeners).expect("two listeners build an AcceptAny");
        let (_stream, peer) = tokio::time::timeout(std::time::Duration::from_secs(10), accept)
            .await
            .expect("AcceptAny resolves once a connection is pending")
            .expect("the accept succeeds");
        assert_eq!(peer, client_addr, "the accepted peer is the client that dialed the second listener");
    }
}
