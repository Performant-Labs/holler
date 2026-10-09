//! `sample_pane`, the `Pane` fixture the fakes' tests and the conformance suites build
//! their records from: valid, deterministic and harmless.

use holler_pane::pane::{
    ContextCeilings, HarnessInfo, HarnessKind, Health, Hold, HostInfo, LastObserved, ModelSpec,
    PaneProbe, PaneRole,
};
use holler_pane::{GridPos, HerdrPane, Pane, PaneError, PaneId, PaneName};

/// The harness port of every sample pane.
const SAMPLE_PORT: u16 = 48100;

/// The Herdr session and workspace of every sample pane: a scratch name, never the
/// name of a live session.
const SCRATCH: &str = "scratch";

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
            grid: GridPos { row: 1, col: 1 },
        },
        host: HostInfo {
            name: "localhost".to_owned(),
            tmux: name.to_string(),
            cwd: "/srv/demo".to_owned(),
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
        model: ModelSpec {
            provider: "demo-provider".to_owned(),
            model_id: "demo-model".to_owned(),
            effort: "medium".to_owned(),
        },
        env: Vec::new(),
        context: ContextCeilings {
            soft: 100_000,
            hard: 150_000,
        },
        command: None,
        probe: PaneProbe {
            check: None,
            expect: Vec::new(),
            last: None,
        },
        name,
    })
}
