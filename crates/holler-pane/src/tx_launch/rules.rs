//! The plan's rules of a launch and a relaunch that read no port but the registry: the port
//! policy (#644, and the kind-aware rule of #831), the spec's login variables (#831), the relaunch
//! rules (E0: the directory, the cell and, since #831, the harness), and the target cell
//! (`grid-occupied`, decided by Herdr's snapshot alone). Each refusal is a plan refusal: nothing
//! is written and nothing live changes.
//!
//! The open codes here are the engine's own, declared where they are raised (ADR-0021 section
//! 9): [`PORT_POLICY_UNSUPPORTED`], [`HARNESS_CHANGE_REFUSED`] and [`CLAUDE_LOGIN_ENV_REFUSED`].

use crate::error::RefusalCode;
use crate::findings::quoted;
use crate::pane::{HarnessKind, Shape};
use crate::profile_snapshot::{FIXED_PORT_POLICY_PREFIX, NO_PORT_POLICY};
use crate::{HerdrPane, HerdrSpec, Pane, PaneError, Ports, ProfileSpec};

use super::{cell, refused, same_pane, usage, GRID_OCCUPIED};

/// `port-policy-unsupported`: a port policy of the grammar that the harness's kind does not
/// take: `none` for `opencode`, whose server needs a port, or `fixed:<port>` for `claude`, which
/// is one process and has no port (#831, ADR-0021 section 15 item 12). A refusal, exit 3.
pub const PORT_POLICY_UNSUPPORTED: RefusalCode =
    RefusalCode::from_static("port-policy-unsupported");

/// `harness-change-refused`: a relaunch, or `profile apply`, whose spec names another harness
/// than the live pane runs (#831): a pane's harness is never rewritten live; close it and launch
/// it anew. A refusal, exit 3.
pub const HARNESS_CHANGE_REFUSED: RefusalCode = RefusalCode::from_static("harness-change-refused");

/// `claude-login-env-refused`: the spec's environment names a variable that silently changes a
/// Claude Code login (`HarnessKind::login_variables`; ADR-0021 section 15 item 10). The message
/// names the variable, never a value (I7). A refusal, exit 3.
pub const CLAUDE_LOGIN_ENV_REFUSED: RefusalCode =
    RefusalCode::from_static("claude-login-env-refused");

/// The port the fixed policy `policy` names. The grammar beyond #662a's one form is this story's,
/// and it is that form alone: [`FIXED_PORT_POLICY_PREFIX`] then `<port>`, in 1..=65535 in
/// canonical decimal (digits only, no leading zero). So `port_of_policy(&fixed_port_policy(p)) ==
/// Ok(p)` for every such `p`, and a policy it accepts equals `fixed_port_policy` of its port.
/// Anything else (the bare `fixed`, `fixed:048100`, `fixed:0`, `auto`, and `none`, which names no
/// port) is `usage`, and the message names the forms. [`port_for`] is the rule that knows the
/// harness's kind.
pub fn port_of_policy(policy: &str) -> Result<u16, PaneError> {
    let port = policy
        .strip_prefix(FIXED_PORT_POLICY_PREFIX)
        .filter(|digits| digits.bytes().all(|b| b.is_ascii_digit()) && !digits.starts_with('0'))
        .and_then(|digits| digits.parse::<u16>().ok());
    port.ok_or_else(|| {
        usage(format!(
            "{} is not a port policy: the forms are {FIXED_PORT_POLICY_PREFIX}<port>, with \
             <port> from 1 to 65535 and no leading zero (a harness server), and {NO_PORT_POLICY} \
             (a harness with no port)",
            quoted(policy)
        ))
    })
}

/// The port the policy `policy` names for a harness of `kind`: the one rule of the port policy
/// that knows the kind (#831, plan review W13), which both `holler pane launch`'s check of its
/// flags and the engine's plan call. `Some(port)` for a server's `fixed:<port>`, `None` for a
/// single process's [`NO_PORT_POLICY`]. A policy outside the grammar is `usage`
/// ([`port_of_policy`]'s); one of the grammar that the kind does not take is
/// [`PORT_POLICY_UNSUPPORTED`].
pub fn port_for(kind: HarnessKind, policy: &str) -> Result<Option<u16>, PaneError> {
    let port = if policy == NO_PORT_POLICY {
        None
    } else {
        Some(port_of_policy(policy)?)
    };
    match (kind.shape(), port) {
        (Shape::ServerAndTui, Some(port)) => Ok(Some(port)),
        (Shape::SingleProcess, None) => Ok(None),
        (Shape::ServerAndTui, None) => Err(refused(
            PORT_POLICY_UNSUPPORTED,
            format!(
                "the port policy {NO_PORT_POLICY} is for a harness with no port; an opencode \
                 harness runs a server, so its policy is {FIXED_PORT_POLICY_PREFIX}<port>"
            ),
        )),
        (Shape::SingleProcess, Some(_)) => Err(refused(
            PORT_POLICY_UNSUPPORTED,
            format!(
                "a claude harness is one process and has no port, so its port policy is \
                 {NO_PORT_POLICY}, not {}",
                quoted(policy)
            ),
        )),
    }
}

/// The spec names no environment variable that changes the login of its harness's kind
/// ([`CLAUDE_LOGIN_ENV_REFUSED`]): the first such name is refused, by its name.
pub(super) fn refuse_login_env(spec: &ProfileSpec) -> Result<(), PaneError> {
    let kind = spec.harness.kind;
    let Some(name) = spec
        .env
        .iter()
        .find(|name| kind.login_variables().contains(&name.as_str()))
    else {
        return Ok(());
    };
    Err(refused(
        CLAUDE_LOGIN_ENV_REFUSED,
        format!(
            "the spec of {} names the environment variable {}, which silently changes a \
             Claude Code login (a subscription login is never replaced); remove it from the \
             spec's env",
            quoted(&spec.pane),
            name.as_str()
        ),
    ))
}

/// E0 past the spec's pane, for a live relaunch: it never changes a pane's harness or its
/// directory, and moves its cell only when the caller was asked to place it (decision 11).
pub(super) fn check_relaunch_rules(
    old: &Pane,
    spec: &ProfileSpec,
    grid_given: bool,
) -> Result<(), PaneError> {
    check_harness_unchanged(old, spec)?;
    if spec.host.cwd != old.host.cwd {
        return Err(usage(format!(
            "relaunch cannot change the directory of {} (from {} to {}); close it and launch it \
             again",
            old.name,
            quoted(&old.host.cwd),
            quoted(&spec.host.cwd)
        )));
    }
    let moved = spec.herdr.workspace != old.herdr.workspace || spec.herdr.grid != old.herdr.grid;
    if moved && !grid_given {
        return Err(usage(format!(
            "relaunch moves a pane only with --grid: the record is at {}, the spec gives {}",
            cell(&old.herdr.workspace, old.herdr.grid),
            cell(&spec.herdr.workspace, spec.herdr.grid)
        )));
    }
    Ok(())
}

/// The spec names the harness the live pane `old` runs ([`HARNESS_CHANGE_REFUSED`] otherwise):
/// E0's rule, which `profile apply` also applies to every recorded pane before it acts (#831).
pub(crate) fn check_harness_unchanged(old: &Pane, spec: &ProfileSpec) -> Result<(), PaneError> {
    if spec.harness.kind == old.harness.kind {
        return Ok(());
    }
    Err(refused(
        HARNESS_CHANGE_REFUSED,
        format!(
            "{} runs the {} harness and its spec names {}; a live pane's harness is never \
             changed: close it and launch it again",
            old.name,
            old.harness.kind.as_str(),
            spec.harness.kind.as_str()
        ),
    ))
}

/// Step 5: `grid-occupied` when the target cell holds a Herdr pane other than `own`. Whether the
/// run is refused is the snapshot's alone. The records are read only to word the refusal, joined
/// to each occupant by Herdr session and pane id whatever cell a record stores, so a stale record
/// is still named.
pub(super) fn refuse_occupied(
    ports: Ports<'_>,
    listed: &[HerdrPane],
    target: &HerdrSpec,
    own: Option<&HerdrPane>,
) -> Result<(), PaneError> {
    let at_target = |p: &&HerdrPane| {
        p.session == target.session && p.workspace == target.workspace && p.grid == target.grid
    };
    let occupants: Vec<&HerdrPane> = listed
        .iter()
        .filter(at_target)
        .filter(|p| own.is_none_or(|own| !same_pane(own, p)))
        .collect();
    if occupants.is_empty() {
        return Ok(());
    }
    let records = ports.pane_store.list();
    let held: Vec<String> = occupants
        .iter()
        .map(|pane| occupant(pane, records.as_deref()))
        .collect();
    let message = format!(
        "{} holds {}; a Herdr pane is never adopted, so choose a free cell",
        cell(&target.workspace, target.grid),
        held.join(" and ")
    );
    Err(refused(GRID_OCCUPIED, message))
}

/// One occupant in a `grid-occupied` message: its id, and the records that name it or that no
/// record does.
fn occupant(pane: &HerdrPane, records: Result<&[Pane], &PaneError>) -> String {
    let id = quoted(pane.pane_id.as_str());
    let records = match records {
        Ok(records) => records,
        Err(error) => {
            let code = error.code();
            return format!("Herdr pane {id} (the records could not be read: {code})");
        }
    };
    let names: Vec<&str> = records
        .iter()
        .filter(|record| same_pane(&record.herdr, pane))
        .map(|record| record.name.as_str())
        .collect();
    if names.is_empty() {
        return format!("Herdr pane {id}, which no record names");
    }
    format!("Herdr pane {id}, recorded as {}", names.join(", "))
}
