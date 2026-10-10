//! Snapshot of a live pane as a `ProfileSpec`, the half of the spec-versus-live
//! comparison that `profile show`, `profile apply` and the `profile-drift` finding
//! share. Empty stub declared by #637 so that no two stories edit `lib.rs`; story
//! #662 fills it.
//!
//! SIGNATURE STUB (#662 T, RED): the public API of the brief with no logic. Every body
//! returns a wrong-but-typed value so the tests compile and fail on their assertions. F
//! replaces the bodies.

use crate::grid::GridPos;
use crate::pane::{ContextCeilings, HarnessKind, ModelSpec, Pane, PaneRole};
use crate::profile::{Profile, ProfileName, ProfileSpec, SpecHarness, SpecHerdr, SpecHost};

/// The prefix of the port policy that pins a harness to one port.
pub const FIXED_PORT_POLICY_PREFIX: &str = "fixed:";

/// The port policy that pins a pane's harness to `port`: `fixed:<port>`, e.g. `fixed:48100`.
pub fn fixed_port_policy(port: u16) -> String {
    let _ = port;
    String::new()
}

/// The spec that reproduces `pane` as its record holds it. Pure; no port is called.
pub fn spec_from_pane(pane: &Pane) -> ProfileSpec {
    let _ = pane;
    ProfileSpec {
        pane: String::new(),
        herdr: SpecHerdr {
            workspace: String::new(),
            grid: GridPos { row: 1, col: 1 },
        },
        host: SpecHost { cwd: String::new() },
        harness: SpecHarness {
            kind: HarnessKind::Opencode,
            port_policy: String::new(),
        },
        model: ModelSpec {
            provider: String::new(),
            model_id: String::new(),
            effort: String::new(),
        },
        role: PaneRole::Agent,
        env: Vec::new(),
        context: ContextCeilings { soft: 0, hard: 0 },
        command: None,
        check: None,
        expect: Vec::new(),
    }
}

/// A new profile named `name` (slug `name.slug()`, generation 0, `created` and `updated` 0: the store
/// stamps them) with one `spec_from_pane` per entry of `panes`, in the order given.
pub fn profile_from_panes(name: &ProfileName, panes: &[Pane]) -> Profile {
    let _ = panes;
    Profile {
        name: name.clone(),
        slug: String::new(),
        generation: 0,
        panes: Vec::new(),
        created: 0,
        updated: 0,
    }
}
