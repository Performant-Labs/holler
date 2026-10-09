//! The fixtures the fakes' tests and the conformance suites build their records from,
//! all valid, deterministic and harmless: `sample_pane` (a `Pane`), `sample_spec` (a
//! `ProfileSpec`) and `sample_profile` (a `Profile` of sample specs).
//!
//! A sample spec places, configures and runs its pane as the sample pane of the same
//! name is placed, configured and run (one workspace, grid cell, directory, harness,
//! model and context ceilings), so a sample pane and its sample spec agree.

use holler_pane::pane::{
    ContextCeilings, HarnessInfo, HarnessKind, Health, Hold, HostInfo, LastObserved, ModelSpec,
    PaneProbe, PaneRole,
};
use holler_pane::profile::{SpecHarness, SpecHerdr, SpecHost};
use holler_pane::{
    GridPos, HerdrPane, Pane, PaneError, PaneId, PaneName, Profile, ProfileName, ProfileSpec,
};

/// The harness port of every sample pane.
const SAMPLE_PORT: u16 = 48100;

/// The Herdr session and workspace of every sample pane and spec: a scratch name,
/// never the name of a live session.
const SCRATCH: &str = "scratch";

/// The grid cell of every sample pane and spec.
const SAMPLE_GRID: GridPos = GridPos { row: 1, col: 1 };

/// The project directory of every sample pane and spec.
const SAMPLE_CWD: &str = "/srv/demo";

/// The port policy of every sample spec.
const SAMPLE_PORT_POLICY: &str = "fixed";

/// A valid, deterministic `Pane` named `name`: generation 0, grid `r1c1`, no profile,
/// no session of record, harness port 48100 with its health unknown, an agent with no
/// hold, no command and no probe. Its Herdr session and workspace are scratch names,
/// never a live session's. Its tmux session is the pane's name, as on a real pane, so
/// pick a neutral one such as `demo-c1r1`. Two calls with one name return equal panes.
///
/// `usage` when `name` is not a valid pane name.
pub fn sample_pane(name: &str) -> Result<Pane, PaneError> {
    let name = PaneName::parse(name)?;
    Ok(Pane {
        generation: 0,
        herdr: HerdrPane {
            session: SCRATCH.to_owned(),
            workspace: SCRATCH.to_owned(),
            pane_id: PaneId::new(format!("{SCRATCH}:{name}")),
            grid: SAMPLE_GRID,
        },
        host: HostInfo {
            name: "localhost".to_owned(),
            tmux: name.to_string(),
            cwd: SAMPLE_CWD.to_owned(),
            herdr_api_version: None,
        },
        harness: HarnessInfo {
            kind: HarnessKind::Opencode,
            port: SAMPLE_PORT,
            pid: None,
            health: Health::Unknown,
        },
        session_of_record: None,
        role: PaneRole::Agent,
        hold: Hold::None,
        last_observed: LastObserved {
            shown: None,
            driven: None,
            at: 0,
        },
        profile: None,
        model: sample_model(),
        env: Vec::new(),
        context: sample_context(),
        command: None,
        probe: PaneProbe {
            check: None,
            expect: Vec::new(),
            last: None,
        },
        name,
    })
}

/// A valid, deterministic `ProfileSpec` for the pane named `pane`: workspace
/// `scratch`, grid `r1c1`, directory `/srv/demo`, the OpenCode harness with the port
/// policy `fixed`, the sample pane's model and context ceilings, an agent, and no env,
/// no command, no check and no expect. Two calls with one name return equal specs.
///
/// `pane` is plain text and is not checked, because a spec may name a pane that has no
/// record (a detached spec).
pub fn sample_spec(pane: &str) -> ProfileSpec {
    ProfileSpec {
        pane: pane.to_owned(),
        herdr: SpecHerdr {
            workspace: SCRATCH.to_owned(),
            grid: SAMPLE_GRID,
        },
        host: SpecHost {
            cwd: SAMPLE_CWD.to_owned(),
        },
        harness: SpecHarness {
            kind: HarnessKind::Opencode,
            port_policy: SAMPLE_PORT_POLICY.to_owned(),
        },
        model: sample_model(),
        role: PaneRole::Agent,
        env: Vec::new(),
        context: sample_context(),
        command: None,
        check: None,
        expect: Vec::new(),
    }
}

/// A valid, deterministic `Profile` named `name`: its slug is the name's slug, its
/// generation 0, its specs one [`sample_spec`] per entry of `panes`, in order, and its
/// `created` and `updated` 0. Two calls with the same arguments return equal profiles.
///
/// `usage` when `name` is not a valid profile name.
pub fn sample_profile(name: &str, panes: &[&str]) -> Result<Profile, PaneError> {
    let name = ProfileName::parse(name)?;
    Ok(Profile {
        slug: name.slug(),
        generation: 0,
        panes: panes.iter().copied().map(sample_spec).collect(),
        created: 0,
        updated: 0,
        name,
    })
}

/// The model of every sample pane and spec: a neutral provider and model.
fn sample_model() -> ModelSpec {
    ModelSpec {
        provider: "demo-provider".to_owned(),
        model_id: "demo-model".to_owned(),
        effort: "medium".to_owned(),
    }
}

/// The context ceilings of every sample pane and spec.
fn sample_context() -> ContextCeilings {
    ContextCeilings {
        soft: 100_000,
        hard: 150_000,
    }
}
