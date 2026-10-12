//! The fixtures the fakes' tests and the conformance suites build their records from,
//! all valid, deterministic and harmless: `sample_pane` (a `Pane`), `sample_spec` (a
//! `ProfileSpec`) and `sample_profile` (a `Profile` of sample specs). `seed_unmanaged_pane`
//! (#832) puts a Herdr pane that no record names into a `FakeHerdr`. `session_id` (#839) is a
//! `ses_`-shaped harness session id, the shape every hub-routed fixture uses.
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
    GridPos, HerdrPane, HerdrPort, HerdrSpec, Pane, PaneError, PaneId, PaneName, PaneObservation,
    Profile, ProfileName, ProfileSpec,
};

use crate::herdr::FakeHerdr;

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
/// hold, the server's default OpenCode agent (no `opencode_agent`), no command and no
/// probe. Its Herdr session and workspace are scratch names,
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
            label: None,
        },
        host: HostInfo {
            name: "localhost".to_owned(),
            tmux: name.to_string(),
            cwd: SAMPLE_CWD.to_owned(),
            herdr_api_version: None,
        },
        harness: HarnessInfo {
            kind: HarnessKind::Opencode,
            port: Some(SAMPLE_PORT),
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
        opencode_agent: None,
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
/// policy `fixed`, the sample pane's model and context ceilings, an agent, the server's
/// default OpenCode agent (no `opencode_agent`), and no env, no command, no check and
/// no expect. Two calls with one name return equal specs.
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
        opencode_agent: None,
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

/// A harness session id shaped as OpenCode's are, and as `FakeHarness` mints them (#839, D8):
/// `ses_` and `n` as 26 hex digits, 30 characters. The name grammar refuses `_`, so such an id
/// is never a valid session name, and a fixture cannot pass by being one. Distinct for
/// distinct `n`.
pub fn session_id(n: u32) -> String {
    format!("ses_{n:026x}")
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

/// The fixture of #832: one **unmanaged** pane in `herdr`'s scratch workspace, at `grid`.
/// No record names it, it runs something no adapter knows (a hand-set-up session whose
/// screen shows one status line and never changes), and it is labelled `label` and works in
/// `cwd`. `last_screen_change` is what `observe_pane` reports for it: `None` is a Herdr
/// that reports no time, as the real adapter does for Herdr 0.8.2.
///
/// The caller declares the workspace first (`FakeHerdr::with_workspace("scratch", ..)`,
/// the name every sample pane uses). Returns the pane as `snapshot` lists it, label
/// included.
pub fn seed_unmanaged_pane(
    herdr: &FakeHerdr,
    grid: GridPos,
    label: &str,
    cwd: &str,
    last_screen_change: Option<i64>,
) -> Result<HerdrPane, PaneError> {
    let placed = herdr.ensure_pane(&HerdrSpec {
        session: SCRATCH.to_owned(),
        workspace: SCRATCH.to_owned(),
        grid,
    })?;
    herdr.rename(&placed.pane_id, label)?;
    herdr.print(&placed.pane_id, "waiting for the seeding to finish...\n")?;
    let mut observation = PaneObservation::default().with_cwd(cwd);
    if let Some(at_ms) = last_screen_change {
        observation = observation.with_last_screen_change(at_ms);
    }
    herdr.set_observation(&placed.pane_id, observation)?;
    herdr
        .snapshot()?
        .panes
        .into_iter()
        .find(|pane| pane.pane_id == placed.pane_id)
        .ok_or_else(|| PaneError::PaneNotFound {
            what: placed.pane_id.as_str().to_owned(),
        })
}
