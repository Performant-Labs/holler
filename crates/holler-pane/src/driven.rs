//! Which live body drives a pane (#839, ADR-0021 section 11 point 3): the join `pane switch`,
//! `pane reset` and `pane doctor` make between a pane's record and the attach-mode bodies the hub
//! advertises (`crate::hub::Advertised`). Pure: the caller reads the hub, the pane's server and
//! the other records, and passes them in.
//!
//! **A body is accepted only on positive evidence.** The exclusion rule comes first: a body that
//! advertises another pane's session of record, or another pane's recorded DRIVEN, is that
//! pane's and is never a candidate for any rule below, the fallback included. The exception is a
//! body that also advertises this pane's own session of record or recorded DRIVEN: this pane's
//! evidence is checked first, so a stale DRIVEN on another pane never takes a pane's own body.
//!
//! Of the bodies that remain, the first of these rules that matches decides:
//!
//! 1. the bodies that advertise the pane's session of record;
//! 2. those that advertise its recorded DRIVEN (the record's own memory: it finds a body whose
//!    session the pane's server no longer lists, and a body that is behind);
//! 3. for a switch, those that advertise the target (that body already drives where the pane is
//!    going, so it needs no rebind).
//!
//! One body is [`Driver::One`], more are [`Driver::Ambiguous`]. With no evidence, a body counts
//! only if the pane's server lists its id and lists no other pane's session of record (servers
//! that share a data directory list each other's sessions, so a listed id proves nothing there):
//! one such body is `One`, more are `Ambiguous`. Where the listing is shared, the listed bodies
//! are [`Driver::Unproven`]. The verbs refuse `Ambiguous` and `Unproven` alike, before anything
//! moves ([`not_driven`]), and `pane doctor` reports them with no remedy.

use std::collections::BTreeSet;

use crate::findings::{parse_session_id, quoted, switch_command};
use crate::hub::{Advertised, PANE_NOT_DRIVEN};
use crate::{Pane, PaneError, PaneName};

/// The body that drives a pane, as [`driver_of`] finds it.
#[derive(Debug, PartialEq, Eq)]
pub enum Driver<'a> {
    /// No body is the pane's.
    None,
    /// This body drives the pane.
    One(&'a Advertised),
    /// More than one body has the same evidence (or, on a listing that is not shared, more than
    /// one is listed): the pane cannot tell which is its own.
    Ambiguous(Vec<&'a Advertised>),
    /// No evidence names a body, and the listed ones are on a listing shared with another pane:
    /// any of them might be the pane's, and nothing says which.
    Unproven(Vec<&'a Advertised>),
}

/// The body of `advertised` that drives `pane` (see the module docs). `server_sessions` is what
/// the pane's own server lists (empty when it was not read), `others` the other panes' records as
/// the caller read them (never values the caller is about to write, so the answer does not depend
/// on pane order), and `target` the session a switch moves to (`None` for a reset and for
/// reconcile).
pub fn driver_of<'a>(
    pane: &Pane,
    advertised: &'a [Advertised],
    server_sessions: &[String],
    others: &[Pane],
    target: Option<&str>,
) -> Driver<'a> {
    let others: Vec<&Pane> = others
        .iter()
        .filter(|other| other.name != pane.name)
        .collect();
    let own: BTreeSet<&str> = evidence(pane).collect();
    let theirs: BTreeSet<&str> = others.iter().flat_map(|&other| evidence(other)).collect();
    let candidates: Vec<&Advertised> = advertised
        .iter()
        .filter(|body| own.contains(id_of(body)) || !theirs.contains(id_of(body)))
        .collect();
    let rules = [
        pane.session_of_record.as_deref(),
        pane.last_observed.driven.as_deref(),
        target,
    ];
    for id in rules.into_iter().flatten() {
        let on: Vec<&Advertised> = candidates
            .iter()
            .copied()
            .filter(|body| id_of(body) == id)
            .collect();
        if !on.is_empty() {
            return one_or_ambiguous(on);
        }
    }
    let listed: Vec<&Advertised> = candidates
        .into_iter()
        .filter(|body| server_sessions.iter().any(|id| id == id_of(body)))
        .collect();
    let shared = others
        .iter()
        .filter_map(|other| other.session_of_record.as_deref())
        .any(|record| server_sessions.iter().any(|id| id == record));
    match (listed.is_empty(), shared) {
        (true, _) => Driver::None,
        (false, true) => Driver::Unproven(listed),
        (false, false) => one_or_ambiguous(listed),
    }
}

/// The bodies of `advertised` other than `driver` that advertise `target`: a switch onto a
/// session another live body drives is refused (P6), because two bodies on one harness session is
/// what ADR 0014 leaves undefined. Every live attach body counts, the ones the exclusion rule
/// removed from [`driver_of`]'s candidates included; the driver itself on the target is the
/// idempotent re-run, never another body.
pub fn elsewhere<'a>(
    advertised: &'a [Advertised],
    driver: Option<&Advertised>,
    target: &str,
) -> Vec<&'a Advertised> {
    advertised
        .iter()
        .filter(|body| body.harness_session_id == target && driver != Some(*body))
        .collect()
}

/// The `pane-not-driven` refusal of a driver that cannot be shown to be `pane`'s own: each
/// candidate named as `label/session` with the id it advertises (quoted: advertised text is
/// untrusted), and the two steps that pair the pane with its body by hand, the switch command
/// built only for an id that is a valid session id. `None` for [`Driver::None`] and
/// [`Driver::One`].
pub fn not_driven(pane: &PaneName, driver: &Driver<'_>) -> Option<PaneError> {
    let (bodies, unproven) = match driver {
        Driver::Ambiguous(bodies) => (bodies, false),
        Driver::Unproven(bodies) => (bodies, true),
        Driver::None | Driver::One(_) => return None,
    };
    let named: Vec<String> = bodies.iter().map(|body| named(body)).collect();
    let commands: Vec<String> = bodies
        .iter()
        .filter_map(|body| switch_command(pane, &body.harness_session_id))
        .collect();
    let pair = if commands.is_empty() {
        format!("switch {pane} to the session its own body drives")
    } else {
        format!(
            "switch {pane} to the session its own body drives ({})",
            commands.join(" or ")
        )
    };
    let steps = format!("{pair}, then holler pane switch {pane} <session of record>");
    let message = if unproven {
        format!(
            "no body is known to drive {pane}; a listed body might be its own: {} ({steps}; if \
             {pane} has no body, start one first)",
            named.join(", ")
        )
    } else {
        format!(
            "{} bodies could drive {pane}: {}; {steps}",
            bodies.len(),
            named.join(", ")
        )
    };
    Some(PaneError::Refused {
        code: PANE_NOT_DRIVEN,
        message,
    })
}

/// A body as a message names it: its name and the id it advertises, both quoted, and a word when
/// the id is not a valid session id (so no command is built from it).
pub(crate) fn named(body: &Advertised) -> String {
    let (name, id) = (quoted(&body.name), quoted(&body.harness_session_id));
    if parse_session_id(&body.harness_session_id).is_ok() {
        format!("{name} on {id}")
    } else {
        format!("{name} on {id} (not a valid session id)")
    }
}

/// The id a body advertises.
fn id_of(body: &Advertised) -> &str {
    &body.harness_session_id
}

/// The evidence a record holds about its body: its session of record and its recorded DRIVEN.
fn evidence(pane: &Pane) -> impl Iterator<Item = &str> {
    [
        pane.session_of_record.as_deref(),
        pane.last_observed.driven.as_deref(),
    ]
    .into_iter()
    .flatten()
}

/// One body is the driver; more are ambiguous.
fn one_or_ambiguous(bodies: Vec<&Advertised>) -> Driver<'_> {
    if let [one] = bodies[..] {
        return Driver::One(one);
    }
    Driver::Ambiguous(bodies)
}
