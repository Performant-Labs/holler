//! The snapshot of a live pane as a `ProfileSpec` (#662): the half of the
//! spec-versus-live comparison that `profile show`, `profile apply` (#664) and the
//! `profile-drift` finding (#665) share, and the mapping `profile create
//! --from-current` and the migration (#650) build a profile from.
//!
//! Pure: a snapshot reads the `Pane` record it is given and calls no port, so it needs
//! no Herdr call (ADR-0021 section 3). The position is the record's `herdr.grid`, which
//! the Herdr adapter observed, never one parsed from the pane's name.
//!
//! The record holds the port its harness uses, not the policy that chose the port, so a
//! snapshot writes the one policy that keeps that fact: `fixed:<port>`
//! ([`fixed_port_policy`]). The rest of the policy grammar is #644's.

use crate::pane::Pane;
use crate::profile::{Profile, ProfileName, ProfileSpec, SpecHarness, SpecHerdr, SpecHost};

/// The prefix of the port policy that pins a harness to one port.
pub const FIXED_PORT_POLICY_PREFIX: &str = "fixed:";

/// The port policy that pins a pane's harness to `port`: `fixed:<port>`, e.g. `fixed:48100`.
pub fn fixed_port_policy(port: u16) -> String {
    format!("{FIXED_PORT_POLICY_PREFIX}{port}")
}

/// The spec that reproduces `pane` as its record holds it. Pure; no port is called.
///
/// | Spec field | From the record |
/// |---|---|
/// | `pane` | `name`, as text |
/// | `herdr.workspace`, `herdr.grid` | `herdr.workspace`, `herdr.grid` (never parsed from the name) |
/// | `host.cwd` | `host.cwd` |
/// | `harness.kind` | `harness.kind` |
/// | `harness.port_policy` | [`fixed_port_policy`] of `harness.port` |
/// | `model`, `role`, `context` | `model`, `role`, `context` |
/// | `env` | `env`: the names, in order |
/// | `command` | `command` |
/// | `check`, `expect` | `probe.check`, `probe.expect` (in order) |
///
/// Not copied, because a spec does not hold them: `generation`, `herdr.session`,
/// `herdr.pane_id`, `host.name`, `host.tmux`, `host.herdr_api_version`, `harness.pid`,
/// `harness.health`, `session_of_record`, `hold`, `last_observed`, `profile` and
/// `probe.last`.
///
/// The port is copied as recorded: a record at port 0 gives `fixed:0`, which #644's
/// `port_of_policy` refuses (`usage`).
pub fn spec_from_pane(pane: &Pane) -> ProfileSpec {
    ProfileSpec {
        pane: pane.name.as_str().to_owned(),
        herdr: SpecHerdr {
            workspace: pane.herdr.workspace.clone(),
            grid: pane.herdr.grid,
        },
        host: SpecHost {
            cwd: pane.host.cwd.clone(),
        },
        harness: SpecHarness {
            kind: pane.harness.kind,
            port_policy: fixed_port_policy(pane.harness.port),
        },
        model: pane.model.clone(),
        role: pane.role,
        env: pane.env.clone(),
        context: pane.context,
        command: pane.command.clone(),
        check: pane.probe.check.clone(),
        expect: pane.probe.expect.clone(),
    }
}

/// A new profile named `name` (slug `name.slug()`, generation 0, `created` and `updated` 0: the store
/// stamps them) with one `spec_from_pane` per entry of `panes`, in the order given.
pub fn profile_from_panes(name: &ProfileName, panes: &[Pane]) -> Profile {
    Profile {
        name: name.clone(),
        slug: name.slug(),
        generation: 0,
        panes: panes.iter().map(spec_from_pane).collect(),
        created: 0,
        updated: 0,
    }
}
