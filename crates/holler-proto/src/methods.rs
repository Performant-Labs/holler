//! The v2 method catalog (docs §4).
//!
//! Twenty-two methods, four kinds, three directions. `catalog()` is the
//! single source of truth — the body/hub dispatch and the codec's
//! `method_not_found` path both read from it.
//!
//! Kinds:
//! - **Request** — expects exactly one response carrying the request's `id`.
//! - **Notification** — no `id`, no response.
//!
//! Directions:
//! - **Both** — legal on either side (the two endpoints differ only by
//!   `hello.role`).
//! - **body→hub** — join / authenticate, and (issue #508) the seven
//!   `admin/*` rows, legal only on an admin-role connection.
//! - **hub→body** — prompt / cancel (and the hub's superseded).

/// One row of the catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Method {
    /// The wire method name, verbatim (e.g. `"session/prompt"`).
    pub name: &'static str,
    pub kind: MethodKind,
    pub dir: Direction,
}

/// Whether a method expects a response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MethodKind {
    /// Expects exactly one response with the request's id.
    Request,
    /// Carries no id and expects no response.
    Notification,
}

/// Which side of the circuit may legally send the method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Legal from either endpoint (distinguished only by `hello.role`).
    Both,
    /// Only the body may send (join / authenticate), or — issue #508 — an
    /// admin client sending one of the seven `admin/*` rows below.
    BodyToHub,
    /// Only the hub may send (prompt / cancel).
    HubToBody,
}

/// The complete, closed v2 method catalog (22 rows).
#[rustfmt::skip]
pub const CATALOG: &[Method] = &[
    // name                 kind          direction   (docs §4)
    Method { name: "circuit/join",        kind: MethodKind::Request,      dir: Direction::BodyToHub },
    Method { name: "circuit/authenticate",kind: MethodKind::Request,      dir: Direction::BodyToHub },
    // `circuit/prove` (issue #323): the second frame of the authenticate
    // challenge-response — the body signs the hub's nonce and answers here.
    Method { name: "circuit/prove",       kind: MethodKind::Request,      dir: Direction::BodyToHub },
    Method { name: "circuit/hello",       kind: MethodKind::Request,      dir: Direction::Both      },
    Method { name: "circuit/ping",        kind: MethodKind::Request,      dir: Direction::Both      },
    Method { name: "circuit/superseded",  kind: MethodKind::Notification, dir: Direction::HubToBody },
    Method { name: "query/status",        kind: MethodKind::Request,      dir: Direction::Both      },
    Method { name: "query/caps",          kind: MethodKind::Request,      dir: Direction::Both      },
    Method { name: "query/support",       kind: MethodKind::Request,      dir: Direction::Both      },
    Method { name: "query/protocol",      kind: MethodKind::Request,      dir: Direction::Both      },
    Method { name: "session/presence",    kind: MethodKind::Notification, dir: Direction::BodyToHub },
    Method { name: "session/prompt",      kind: MethodKind::Request,      dir: Direction::HubToBody },
    Method { name: "session/update",      kind: MethodKind::Notification, dir: Direction::BodyToHub },
    Method { name: "session/cancel",      kind: MethodKind::Request,      dir: Direction::HubToBody },
    // `session/answer` (issue #151): resolve a held `input-required`
    // permission/elicitation — same direction/kind as `session/cancel`, the
    // request it's modeled on.
    Method { name: "session/answer",      kind: MethodKind::Request,      dir: Direction::HubToBody },
    // `admin/*` (issue #508/#509, epic #506): the remote hub-admin control
    // channel. Params/results are byte-for-byte the same as the matching
    // `control/*` local form (docs v2.md §3-4); legal only on an admin-role
    // connection, dispatched through the hub's admin allowlist
    // (`circuit/admin.rs`), never a generic passthrough.
    Method { name: "admin/status",        kind: MethodKind::Request,      dir: Direction::BodyToHub },
    Method { name: "admin/roster",        kind: MethodKind::Request,      dir: Direction::BodyToHub },
    Method { name: "admin/say",           kind: MethodKind::Request,      dir: Direction::BodyToHub },
    Method { name: "admin/interrupt",     kind: MethodKind::Request,      dir: Direction::BodyToHub },
    Method { name: "admin/answer",        kind: MethodKind::Request,      dir: Direction::BodyToHub },
    Method { name: "admin/wait",          kind: MethodKind::Request,      dir: Direction::BodyToHub },
    Method { name: "admin/query",         kind: MethodKind::Request,      dir: Direction::BodyToHub },
];

/// Look up a method by its exact wire name.
///
/// `None` for a name that is not in the catalog — the caller maps that to
/// JSON-RPC `-32601 method_not_found` (`error::Code::METHOD_NOT_FOUND`).
pub fn find(name: &str) -> Option<&'static Method> {
    CATALOG.iter().find(|m| m.name == name)
}

/// Whether `method` is a request (vs a notification).
pub fn is_request(method: &str) -> bool {
    matches!(find(method).map(|m| m.kind), Some(MethodKind::Request))
}

/// Whether `method` is a notification.
pub fn is_notification(method: &str) -> bool {
    matches!(find(method).map(|m| m.kind), Some(MethodKind::Notification))
}
