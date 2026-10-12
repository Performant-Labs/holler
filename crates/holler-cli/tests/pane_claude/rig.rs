//! The rig every `pane_claude` case runs over (#831): the test kit's fakes, with the harness
//! port being `FakeHarnesses` (the router over `FakeHarness` and `FakeClaude`, ADR-0021 section 15
//! item 4) behind a linked host.
//!
//! - The **linked host** (`verb_harness::linked_host`, #846) is the one place a host and the
//!   harness fakes meet: `run` of an argv either fake built starts it (`FakeHarnesses::start`)
//!   and `stop_owned` ends both fakes' processes of the pane (`FakeHarnesses::stop`), as tmux
//!   runs and stops a pane's processes (#802). No case writes its own Claude linker (plan
//!   review W5).
//! - `seed_claude` and `seed_opencode` build a live pane through the fakes' own port methods, not
//!   through a verb, so a case of one verb fails only on that verb.
//! - `mark` and `since` scope the call logs to one run.

use std::sync::Arc;

use holler_cli::output::Format;
use holler_pane::pane::{HarnessInfo, HarnessKind, Health, HostInfo, LastObserved, PaneRole};
use holler_pane::ports::SessionChoice;
use holler_pane::profile_snapshot::spec_from_pane;
use holler_pane::{
    Actor, Argv, GridPos, HarnessEndpoint, HarnessPort, HerdrPane, HerdrPort, HerdrSpec, HostPort,
    Pane, PaneName, PaneStore, Ports, Profile, ProfileName, ProfileSpec, ProfileStore, ServeSpec,
};
use holler_pane_testkit::claude::{ClaudeOp, FakeClaude, ProcessView};
use holler_pane_testkit::envelope::check_envelope;
use holler_pane_testkit::fixture::{sample_pane, sample_profile};
use holler_pane_testkit::harness::{FakeHarness, HarnessOp};
use holler_pane_testkit::harnesses::FakeHarnesses;
use holler_pane_testkit::herdr::{FakeHerdr, HerdrOp, PROTOCOL_22_VERSION};
use holler_pane_testkit::host::{FakeHost, HostOp};
use holler_pane_testkit::pane_store::{FakePaneStore, PaneStoreOp};
use holler_pane_testkit::prober::FakeProber;
use holler_pane_testkit::profile_scope::FakeProfileScope;
use holler_pane_testkit::profile_store::FakeProfileStore;
use serde_json::Value;

use crate::verb_harness::linked_host::{FakeHarnessesLink, LinkedHost};
use crate::verb_harness::{run_verb_with, Outcome};

/// The Herdr session and the workspace of the rig.
pub const SESSION: &str = "scratch";
pub const WORKSPACE: &str = "main";
/// The directory every pane works in.
pub const CWD: &str = "/srv/demo";
/// The profile every rig starts with, holding no spec.
pub const PROFILE: &str = "demo";

/// The Claude pane and the OpenCode pane of a mixed grid.
pub const CLAUDE: &str = "demo-c1r2";
pub const OPENCODE: &str = "demo-c1r1";
/// The OpenCode pane's port.
pub const PORT: u16 = 48100;

/// The variables that silently change a Claude Code login (ADR-0021 section 15 item 10).
pub const LOGIN_VARIABLES: [&str; 7] = [
    "ANTHROPIC_API_KEY",
    "CLAUDE_CODE_OAUTH_TOKEN",
    "ANTHROPIC_AUTH_TOKEN",
    "CLAUDE_CONFIG_DIR",
    "CLAUDE_CODE_USE_BEDROCK",
    "CLAUDE_CODE_USE_VERTEX",
    "CLAUDE_CODE_USE_FOUNDRY",
];

pub const fn cell(row: u16, col: u16) -> GridPos {
    GridPos { row, col }
}

pub fn name(text: &str) -> PaneName {
    PaneName::parse(text).unwrap()
}

/// `holler pane launch PANE` of a Claude pane at `grid`, followed by `extra`.
pub fn launch_claude<'a>(pane: &'a str, grid: &'a str, extra: &[&'a str]) -> Vec<&'a str> {
    let mut argv = vec![
        "pane",
        "launch",
        pane,
        "--herdr-session",
        SESSION,
        "--project",
        CWD,
        "--workspace",
        WORKSPACE,
        "--grid",
        grid,
        "--model",
        "anthropic/demo-model",
        "--effort",
        "high",
        "--ctx-soft",
        "100000",
        "--ctx-hard",
        "150000",
        "--harness",
        "claude",
        "--port-policy",
        "none",
    ];
    argv.extend_from_slice(extra);
    argv
}

/// `args` with the value of `flag` replaced by `value`.
pub fn replacing<'a>(mut args: Vec<&'a str>, flag: &str, value: &'a str) -> Vec<&'a str> {
    let at = args
        .iter()
        .position(|a| *a == flag)
        .expect("the flag is there");
    args[at + 1] = value;
    args
}

/// Whether `argv` holds `flag` immediately followed by `value`.
pub fn has_pair(argv: &Argv, flag: &str, value: &str) -> bool {
    argv.as_slice()
        .windows(2)
        .any(|w| w[0] == flag && w[1] == value)
}

/// The calls made through every port, oldest first.
#[derive(Debug, Clone, Default)]
pub struct Calls {
    pub panes: Vec<PaneStoreOp>,
    pub herdr: Vec<HerdrOp>,
    pub host: Vec<HostOp>,
    pub opencode: Vec<HarnessOp>,
    pub claude: Vec<ClaudeOp>,
}

impl Calls {
    /// No pane-store write.
    pub fn wrote_no_record(&self) -> bool {
        !self
            .panes
            .iter()
            .any(|op| matches!(op, PaneStoreOp::CasPut | PaneStoreOp::Delete))
    }

    /// Nothing live acted: no host call, no Herdr call that acts, no harness process started.
    pub fn acted_on_nothing_live(&self) -> bool {
        self.host.is_empty()
            && self.herdr.iter().all(|op| {
                matches!(
                    op,
                    HerdrOp::Snapshot | HerdrOp::ObservePane | HerdrOp::Version
                )
            })
    }
}

/// The fakes, the linked host and the router of the two harness fakes.
pub struct Rig {
    pub panes: Arc<FakePaneStore>,
    pub profiles: Arc<FakeProfileStore>,
    pub scope: FakeProfileScope,
    pub herdr: FakeHerdr,
    pub host: FakeHost,
    pub harnesses: FakeHarnesses,
    pub prober: FakeProber,
}

impl Rig {
    /// An empty registry and the profile `demo` with no spec.
    pub fn new() -> Self {
        let actor = Actor::parse("test seed").unwrap();
        let panes = Arc::new(FakePaneStore::new());
        let profiles = Arc::new(
            FakeProfileStore::seeded([sample_profile(PROFILE, &[]).unwrap()], &actor).unwrap(),
        );
        let scope = FakeProfileScope::new(
            profiles.clone(),
            panes.clone(),
            Actor::parse("holler pane").unwrap(),
        );
        Self {
            panes,
            profiles,
            scope,
            herdr: FakeHerdr::new(SESSION)
                .with_workspace(WORKSPACE, 3, 3)
                .unwrap(),
            host: FakeHost::new(),
            harnesses: FakeHarnesses::new(),
            prober: FakeProber::new(),
        }
    }

    pub fn claude(&self) -> &FakeClaude {
        self.harnesses.claude()
    }

    pub fn opencode(&self) -> &FakeHarness {
        self.harnesses.opencode()
    }

    /// Call `f` with the ports a verb sees.
    pub fn with_ports<R>(&self, f: impl FnOnce(Ports<'_>) -> R) -> R {
        let host = LinkedHost {
            host: &self.host,
            link: FakeHarnessesLink(&self.harnesses),
        };
        f(Ports {
            pane_store: self.panes.as_ref(),
            profile_store: self.profiles.as_ref(),
            herdr: &self.herdr,
            host: &host,
            harness: &self.harnesses,
            scope: &self.scope,
            prober: &self.prober,
        })
    }

    /// Run `holler <argv...>` over the rig.
    pub fn run(&self, argv: &[&str], format: Format) -> Outcome {
        self.with_ports(|ports| run_verb_with(argv, format, ports))
    }

    /// Run `holler <argv...>` over the rig, with the calls the run made.
    pub fn run_logged(&self, argv: &[&str], format: Format) -> (Outcome, Calls) {
        let mark = self.mark();
        let outcome = self.run(argv, format);
        (outcome, self.since(&mark))
    }

    /// Every call made so far.
    pub fn mark(&self) -> Calls {
        Calls {
            panes: self.panes.faults().calls(),
            herdr: self.herdr.faults().calls(),
            host: self.host.faults().calls(),
            opencode: self.opencode().faults().calls(),
            claude: self.claude().faults().calls(),
        }
    }

    /// The calls made after `mark`.
    pub fn since(&self, mark: &Calls) -> Calls {
        let now = self.mark();
        Calls {
            panes: now.panes[mark.panes.len()..].to_vec(),
            herdr: now.herdr[mark.herdr.len()..].to_vec(),
            host: now.host[mark.host.len()..].to_vec(),
            opencode: now.opencode[mark.opencode.len()..].to_vec(),
            claude: now.claude[mark.claude.len()..].to_vec(),
        }
    }

    /// The stored record of `pane`, which must exist.
    pub fn record(&self, pane: &str) -> Pane {
        self.panes
            .get(&name(pane))
            .unwrap()
            .unwrap_or_else(|| panic!("a record for {pane}"))
    }

    /// The stored record of `pane`, if any.
    pub fn maybe_record(&self, pane: &str) -> Option<Pane> {
        self.panes.get(&name(pane)).unwrap()
    }

    /// The live Claude process of `pane`, if any.
    pub fn process_of(&self, pane: &str) -> Option<ProcessView> {
        self.claude()
            .processes()
            .into_iter()
            .find(|p| p.pane == name(pane))
    }

    /// The endpoint the record of `pane` names.
    pub fn endpoint_of(&self, pane: &str) -> HarnessEndpoint {
        self.record(pane).harness.endpoint(&name(pane)).unwrap()
    }

    /// Every argv the host ran for `pane`, oldest first.
    pub fn runs_of(&self, pane: &str) -> Vec<Argv> {
        self.host
            .runs()
            .into_iter()
            .filter(|(n, _, _)| *n == name(pane))
            .map(|(_, _, argv)| argv)
            .collect()
    }

    /// Place a Herdr pane at `grid` in the rig's workspace.
    pub fn place(&self, grid: GridPos) -> HerdrPane {
        self.herdr
            .ensure_pane(&HerdrSpec {
                session: SESSION.to_owned(),
                workspace: WORKSPACE.to_owned(),
                grid,
            })
            .unwrap()
    }

    /// A live Claude pane as a launch leaves it, built through the fakes: a Herdr pane at
    /// `grid`, its tmux session running the process `FakeClaude`'s argv describes on a fresh
    /// session, and its record (`claude`, the pid, no port, healthy, that session as the session
    /// of record and as the session shown). In no profile.
    pub fn seed_claude(&self, pane: &str, grid: GridPos) -> Pane {
        let n = name(pane);
        let herdr = self.place(grid);
        self.host.ensure_session(&n, CWD).unwrap();
        let id = self.harnesses.new_session_id().unwrap();
        let argv = self
            .harnesses
            .process_command(
                &n,
                &ServeSpec::default().with_cwd(CWD),
                &SessionChoice::Fresh(id.clone()),
            )
            .unwrap();
        self.host.run(&n, CWD, &argv).unwrap();
        let pid = self
            .harnesses
            .start(&argv)
            .expect("FakeClaude's own argv")
            .unwrap();
        let harness = HarnessInfo {
            kind: HarnessKind::Claude,
            port: None,
            pid: Some(pid),
            health: Health::Healthy,
        };
        self.store(pane, herdr, harness, id)
    }

    /// A live OpenCode pane as a launch leaves it: a Herdr pane at `grid`, its tmux session
    /// running the server `FakeHarness`'s argv describes on `port`, one session its TUI shows,
    /// and its record. In no profile.
    pub fn seed_opencode(&self, pane: &str, grid: GridPos, port: u16) -> Pane {
        let n = name(pane);
        let herdr = self.place(grid);
        self.host.ensure_session(&n, CWD).unwrap();
        let argv = self
            .harnesses
            .serve_command(&n, port, &ServeSpec::default())
            .unwrap();
        self.host.run(&n, CWD, &argv).unwrap();
        self.harnesses
            .start(&argv)
            .expect("FakeHarness's own argv")
            .unwrap();
        let at = HarnessEndpoint::Port(port);
        let id = self.harnesses.create_session(&at).unwrap();
        self.harnesses.attach_tui(&herdr.pane_id, &at, &id).unwrap();
        let harness = HarnessInfo {
            kind: HarnessKind::Opencode,
            port: Some(port),
            pid: None,
            health: Health::Healthy,
        };
        self.store(pane, herdr, harness, id)
    }

    fn store(&self, pane: &str, herdr: HerdrPane, harness: HarnessInfo, session: String) -> Pane {
        let sample = sample_pane(pane).unwrap();
        let record = Pane {
            herdr,
            host: HostInfo {
                herdr_api_version: Some(PROTOCOL_22_VERSION.to_owned()),
                cwd: CWD.to_owned(),
                ..sample.host.clone()
            },
            harness,
            session_of_record: Some(session.clone()),
            role: PaneRole::Agent,
            last_observed: LastObserved {
                shown: Some(session),
                driven: None,
                at: 0,
            },
            ..sample
        };
        self.panes.concurrent_put(&record).unwrap()
    }

    /// Put `pane`'s record in the profile `demo`, with its spec (as another writer).
    pub fn join_profile(&self, pane: &str) -> Pane {
        let record = Pane {
            profile: Some(ProfileName::parse(PROFILE).unwrap()),
            ..self.record(pane)
        };
        let stored = self.panes.concurrent_put(&record).unwrap();
        let mut specs = self.profile().panes;
        specs.push(spec_from_pane(&stored));
        self.set_specs(specs);
        stored
    }

    /// The profile `demo` as stored.
    pub fn profile(&self) -> Profile {
        self.profiles
            .get(&ProfileName::parse(PROFILE).unwrap())
            .unwrap()
            .expect("the profile exists")
    }

    /// Set the specs of `demo`, as another writer.
    pub fn set_specs(&self, specs: Vec<ProfileSpec>) -> Profile {
        let edited = Profile {
            panes: specs,
            ..self.profile()
        };
        self.profiles
            .concurrent_put(&edited, &Actor::parse("test seed").unwrap())
            .unwrap()
    }

    /// The process of `pane` has had a conversation: its session has a transcript, so a relaunch
    /// resumes it.
    pub fn converse(&self, pane: &str) {
        let session = self
            .record(pane)
            .session_of_record
            .expect("a session of record");
        self.claude().seed_conversation(&session);
    }
}

/// The `(code, message)` of a failed JSON run, read through the envelope checker (which also
/// checks the exit code against the code's class).
pub fn error_of(run: &Outcome) -> (String, String) {
    let envelope = check_envelope(&run.out, run.code).unwrap_or_else(|f| panic!("{f}: {run:?}"));
    let error = envelope
        .error
        .unwrap_or_else(|| panic!("the run failed: {run:?}"));
    (error.code, error.message)
}

/// The `data` of a successful JSON run.
pub fn data_of(run: &Outcome) -> Value {
    assert_eq!(run.code, 0, "the run succeeds: {run:?}");
    check_envelope(&run.out, run.code)
        .unwrap_or_else(|f| panic!("{f}: {run:?}"))
        .data
}

/// The JSON findings of a doctor run's `data`.
pub fn findings(data: &Value) -> Vec<Value> {
    data["findings"]
        .as_array()
        .unwrap_or_else(|| panic!("data.findings is an array: {data}"))
        .clone()
}

/// The kinds of `findings`, in order.
pub fn kinds(findings: &[Value]) -> Vec<String> {
    findings
        .iter()
        .map(|f| f["kind"].as_str().unwrap_or_default().to_owned())
        .collect()
}
