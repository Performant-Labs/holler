//! Parsing and policy for a body's `--server` address (story #176).
//!
//! `body join --server <url>` names the hub the body will pair with. The
//! protocol (ADR 0002 / docs §3) fixes the transport by scheme: **plain
//! `ws` is loopback only** (`127.0.0.1` / `::1`); off-loopback is **`wss`**
//! (TLS, through a proxy). A `--server` string is parsed into
//! [`ServerAddress`], and `loopback_only_check` is the one fail-closed rule
//! `join` applies before it opens any connection: a plaintext `ws://` to a
//! non-loopback host is refused (CLI exit 3) — plaintext credentials and the
//! join secret must never cross a non-loopback network.
//!
//! This module is I/O-free: it parses a URL string and inspects the resulting
//! host, so it is directly unit-testable (the e2e harness can't reach a
//! non-loopback host, and would not want to dial one even if it could).

use std::net::IpAddr;

/// The protocol's default body-join port (ADR 0002 / docs §3:
/// "default `127.0.0.1:41807`"). A bare `ws://host` / `wss://host` with no
/// explicit port uses this.
pub const DEFAULT_PORT: u16 = 41807;

/// A parsed hub WebSocket address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerAddress {
    /// `ws` (loopback only) or `wss` (TLS, the off-loopback transport).
    pub scheme: &'static str,
    /// The host (a hostname or IP literal), normalised lowercase.
    pub host: String,
    /// The port (explicit, or [`DEFAULT_PORT`]).
    pub port: u16,
}

impl ServerAddress {
    /// True for the loopback transports: an IPv4 `127.0.0.1` or the IPv6
    /// `::1` literal. A non-loopback host (a hostname that is not `localhost`,
    /// a LAN IP, a public IP, …) is `false`.
    pub fn is_loopback(&self) -> bool {
        // An IPv6 literal is stored with its brackets (`[::1]`); strip them
        // before the `IpAddr` parse (brackets are not part of an `IpAddr`).
        let host = self.host.strip_prefix('[').and_then(|h| h.strip_suffix(']')).unwrap_or(&self.host);
        match host.parse::<IpAddr>() {
            Ok(IpAddr::V4(v4)) => v4.is_loopback(),
            Ok(IpAddr::V6(v6)) => v6.is_loopback(),
            // A hostname: the only loopback hostname the protocol sanctions.
            Err(_) => self.host == "localhost",
        }
    }
}

/// A `--server` value that is not a usable WebSocket URL.
#[derive(Debug, Clone, PartialEq)]
pub enum AddressError {
    /// The scheme is neither `ws` nor `wss`.
    BadScheme(String),
    /// A `ws`/`wss` URL carried no host.
    MissingHost,
    /// The explicit port was not a 16-bit number.
    BadPort(String),
}

impl std::fmt::Display for AddressError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AddressError::BadScheme(s) => write!(f, "a server address must be ws:// or wss:// (got scheme {s:?})"),
            AddressError::MissingHost => write!(f, "a server address must name a host (ws://<host>[:port])"),
            AddressError::BadPort(p) => write!(f, "server port {p:?} is not a 16-bit number"),
        }
    }
}

impl std::error::Error for AddressError {}

/// Parse a `--server` value into a [`ServerAddress`].
///
/// Accepted: `ws://host`, `ws://host:port`, `wss://host`, `wss://host:port`.
/// A missing port defaults to [`DEFAULT_PORT`] (41807). Anything else
/// (a bare hostname, an `http`/`https` URL, a port that is not a number, a
/// missing host) is an [`AddressError`] — a fail-closed refusal.
pub fn parse(input: &str) -> Result<ServerAddress, AddressError> {
    let rest = match input.split_once("://") {
        Some((scheme, rest)) => (scheme, rest),
        None => return Err(AddressError::BadScheme(input.to_string())),
    };
    let scheme = match rest.0 {
        "ws" => "ws",
        "wss" => "wss",
        other => return Err(AddressError::BadScheme(other.to_string())),
    };
    // Split the authority (host[:port]) from any path/query: a body's server
    // address is a host and port, and a trailing `/` (or path) is not a port.
    // `authority` is non-empty here (a `ws://` with an empty authority is
    // handled below).
    let authority = rest.1.split('/').next().unwrap_or(rest.1);
    if authority.is_empty() {
        return Err(AddressError::MissingHost);
    }
    // An IPv6 literal is bracketed (`[::1]` / `[::1]:port`). The colons *inside*
    // the brackets are part of the address, so the host/port separator is the
    // colon *after* the closing `]` — not the last colon in the string (which,
    // for a bare `[::1]`, would land inside the brackets and mangle the host).
    // Every other host (a hostname or a dotted-quad IPv4) has a single
    // host/port `:` (or none), which `rfind` finds.
    let (host, port) = if authority.starts_with('[') {
        match authority.find(']') {
            Some(close) => {
                let ip_literal = &authority[..close + 1];
                let remainder = &authority[close + 1..];
                let port = match remainder.strip_prefix(':') {
                    Some(p) => parse_port(p)?,
                    None => DEFAULT_PORT,
                };
                (ip_literal.to_string(), port)
            }
            None => {
                // A `[` with no closing `]` is not a well-formed IPv6 literal:
                // fail closed rather than guess.
                return Err(AddressError::BadPort(authority.to_string()));
            }
        }
    } else {
        match authority.rfind(':') {
            Some(idx) => {
                let host = &authority[..idx];
                let port_str = &authority[idx + 1..];
                if host.is_empty() {
                    return Err(AddressError::MissingHost);
                }
                (host.to_string(), parse_port(port_str)?)
            }
            None => (authority.to_string(), DEFAULT_PORT),
        }
    };
    Ok(ServerAddress {
        scheme,
        host: host.to_ascii_lowercase(),
        port,
    })
}

fn parse_port(s: &str) -> Result<u16, AddressError> {
    if s.is_empty() {
        return Err(AddressError::BadPort(s.to_string()));
    }
    s.parse::<u16>().map_err(|_| AddressError::BadPort(s.to_string()))
}

/// The one fail-closed transport rule: a plaintext `ws://` address that is not
/// loopback is refused. Returns `Some(reason)` (for a CLI exit 3) when the
/// address must not be connected, `None` when connecting is permitted
/// (a `wss` address, or a `ws` address to a loopback host).
pub fn loopback_only_check(addr: &ServerAddress) -> Option<&'static str> {
    if addr.scheme == "ws" && !addr.is_loopback() {
        Some(
            "a plaintext ws:// to a non-loopback host is refused; \
             off-loopback pairing is wss:// (ADR 0002)",
        )
    } else {
        None
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #176
mod tests {
    use super::*;

    #[test]
    fn bare_ws_defaults_port() {
        let a = parse("ws://loopback.example").unwrap();
        assert_eq!(a.scheme, "ws");
        assert_eq!(a.host, "loopback.example");
        assert_eq!(a.port, 41807);
    }

    #[test]
    fn explicit_port_wins() {
        let a = parse("ws://127.0.0.1:4321").unwrap();
        assert_eq!(a.port, 4321);
        assert!(a.is_loopback());
    }

    #[test]
    fn wss_defaults_port() {
        let a = parse("wss://hub.example.ts.net").unwrap();
        assert_eq!(a.scheme, "wss");
        assert_eq!(a.port, 41807);
        assert!(!a.is_loopback());
    }

    #[test]
    fn ipv6_loopback_is_loopback() {
        let a = parse("ws://[::1]:41807").unwrap();
        assert!(a.is_loopback());
    }

    #[test]
    fn non_loopback_ip_is_not_loopback() {
        let a = parse("ws://10.0.0.5").unwrap();
        assert!(!a.is_loopback());
    }

    #[test]
    fn bad_scheme_is_an_error() {
        assert!(matches!(parse("http://hub"), Err(AddressError::BadScheme(_))));
    }

    #[test]
    fn missing_host_is_an_error() {
        assert!(matches!(parse("ws://"), Err(AddressError::MissingHost)));
    }

    #[test]
    fn bad_port_is_an_error() {
        assert!(matches!(parse("ws://127.0.0.1:99999"), Err(AddressError::BadPort(_))));
    }

    #[test]
    fn loopback_only_refuses_plain_non_loopback() {
        let a = parse("ws://10.0.0.5").unwrap();
        assert!(loopback_only_check(&a).is_some());
    }

    #[test]
    fn loopback_only_allows_wss_non_loopback() {
        let a = parse("wss://hub.example.ts.net").unwrap();
        assert!(loopback_only_check(&a).is_none());
    }

    #[test]
    fn loopback_only_allows_plain_loopback() {
        let a = parse("ws://127.0.0.1:41807").unwrap();
        assert!(loopback_only_check(&a).is_none());
    }
}
