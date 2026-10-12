//! The launch/relaunch transaction (plan, act, observe, record; I3 and I8), story #644: the
//! engine `holler pane launch` and `holler pane relaunch` run, and `profile apply` (#664) does.
//! It works only through the [`Ports`] (ADR-0021 section 5) and holds no I/O of its own.
//!
//! - **Plan**, writing nothing and changing nothing live: the spec names the pane; a launched
//!   name has no record (`pane-exists`) and its Herdr session is given; the port policy fits the
//!   harness's kind ([`port_for`]: a port for OpenCode, `none` for Claude Code); a Claude spec
//!   names no variable that changes its login (`claude-login-env-refused`); the harness builds
//!   its argv (`serve_command` for a server, `process_command` for a single process, both pure,
//!   so a refusal, `agent-unsupported` say, writes nothing); the probe passes when the spec has
//!   a check (a failing result is not stored); Herdr's version is supported; the target cell
//!   holds no Herdr pane (`grid-occupied`, decided by Herdr's snapshot alone, recorded or not: a
//!   pane is never adopted); and nothing already runs the harness: no server answers on the port
//!   (`port-in-use`), no Claude Code runs in the tmux session (`process-in-use`, #831). Neither
//!   is ever adopted.
//! - **Act**, inside `ProfileScope::edit_spec` (I8: with a profile, P is written first and its
//!   specs are put back if the act fails): ensure the Herdr pane and the tmux session, run the
//!   spec's command there, then bring the harness up from the planned argv, in the spec's
//!   directory. A server (the child module `server`, #802) is started and checked, the session
//!   of record is created through the harness API (never a "ping" session, never "the most
//!   recent"), the TUI is attached to exactly that session and seen showing it. A single process
//!   (the child module `process`, #831) is started holding its session, waited for until the
//!   harness locates it idle, and seen holding that session. Then Herdr must still list the pane
//!   (a mismatch is `unavailable`). Only then is the record written, by one compare-and-swap: a
//!   server's port and no pid (it is a process of the pane's tmux session), or a process's pid
//!   and no port.
//! - **Rollback.** A failed step stops the tmux session's processes, the harness among them,
//!   and closes the Herdr pane this run created (one the plan's snapshot did not list), and
//!   nothing is recorded. The tmux session and a session the server created stay: no port call
//!   ends either. A record write that conflicts is not rolled back: another writer owns the
//!   record now. A crash runs no rollback, so a harness it started stays in the tmux session.
//! - **Relaunch** is the same act after stopping only what the pane owns, its harness among
//!   them. It keeps the pane's directory and harness (`harness-change-refused`), moves its cell
//!   only when the caller was asked to place it (`grid_given`), and after a move records the new
//!   pane before it closes the old one. A server keeps the session of record the restarted
//!   server still lists. A single process starts on the session [`relaunch_on`] is given
//!   ([`ProcessSession`]): the session of record, kept (`relaunch`, `interrupt`, doctor's
//!   repair), another conversation (`switch`) or a fresh one (`reset`); [`relaunch_recorded`] is
//!   that one sequence for a pane relaunched as recorded. Its rules are the engine's first step
//!   (E0), so every caller is held to them.
//! - **Bound (I5).** A run's budget ([`DEFAULT_BUDGET`]) starts when the engine is entered and is
//!   checked before every live step (a monotonic clock, no port call). When it has run out the run
//!   rolls back and answers `timeout`. A run takes at most the budget plus one port call's own
//!   bound plus its rollback calls. The waits ([`wait_healthy`] for a server, [`wait_located`]
//!   and the act's idle wait for a process) are the one loop [`wait_until`]: it asks every
//!   [`TxOptions::boot_poll`] with the budget's end as its deadline, checked before each ask, so
//!   it holds to that bound too; its sleep is the engine's one wait.
//!
//! Nothing here types into a pane (I4), runs a shell, or joins or splits an argv. Untrusted text
//! (session and Herdr ids, workspaces, directories, a spec's pane, an adapter's or the prober's
//! message) reaches a message only through `findings::quoted` or `findings::embedded`.

mod budget;
mod process;
mod rules;
mod server;
mod undo;

use std::thread;
use std::time::{Duration, Instant};

use crate::error::RefusalCode;
use crate::findings::{embedded, quoted};
use crate::pane::{HarnessInfo, Health, Hold, HostInfo, LastObserved, PaneProbe, Shape};
use crate::ports::SessionChoice;
use crate::profile_snapshot::spec_from_pane;
use crate::{
    Argv, GridPos, HerdrPane, HerdrSpec, Pane, PaneError, PaneName, Ports, ProbeResult, Profile,
    ProfileName, ProfileSpec, ServeSpec, SpecEdit,
};

pub use process::{wait_located, PROCESS_IN_USE};
pub use rules::{
    port_for, port_of_policy, CLAUDE_LOGIN_ENV_REFUSED, HARNESS_CHANGE_REFUSED,
    PORT_POLICY_UNSUPPORTED,
};
pub use server::wait_healthy;

pub(crate) use rules::check_harness_unchanged;

use budget::Budget;
use undo::{close_old, roll_back, stop_old, Made};

/// `pane-exists`: `launch` of a name that already has a record (relaunch it instead). A
/// refusal, exit 3.
pub const PANE_EXISTS: RefusalCode = RefusalCode::from_static("pane-exists");
/// `grid-occupied`: the target cell already holds a Herdr pane (never adopted, recorded or
/// not). A refusal, exit 3.
pub const GRID_OCCUPIED: RefusalCode = RefusalCode::from_static("grid-occupied");
/// `port-in-use`: a harness server already answers on the port before this verb started one
/// (never adopted). A refusal, exit 3.
pub const PORT_IN_USE: RefusalCode = RefusalCode::from_static("port-in-use");
/// The budget of one whole verb run (I5's default bound), checked before every step.
pub const DEFAULT_BUDGET: Duration = Duration::from_secs(10);
/// The timeout handed to `Prober::run_probe`.
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(5);
/// How often the act asks a harness it started whether it is up (#802, #831).
pub const BOOT_POLL: Duration = Duration::from_millis(150);
/// `HostInfo.name` of every pane recorded here (epic decision 3: Herdr, tmux and the harness
/// run on the hub's machine).
pub const HOST_NAME: &str = "localhost";
/// The `op` of the `timeout` a launch answers when its budget runs out.
pub const OP_LAUNCH: &str = "pane.launch";
/// The `op` of the `timeout` a relaunch answers when its budget runs out.
pub const OP_RELAUNCH: &str = "pane.relaunch";

/// How one engine run is bounded and clocked.
///
/// `now_ms` is a function, where `ReconcileRequest.now_ms` is a value taken once: a run stamps
/// `last_observed.at` when it writes the record, after the harness was observed, which can be up
/// to the budget after the verb started, so the engine reads the clock there. A test passes a
/// constant.
#[derive(Debug, Clone, Copy)]
pub struct TxOptions {
    /// The budget of the whole run ([`DEFAULT_BUDGET`]).
    pub budget: Duration,
    /// The timeout of the health probe ([`PROBE_TIMEOUT`]).
    pub probe_timeout: Duration,
    /// How often the waits ask the started harness ([`BOOT_POLL`]); a wait ends with the
    /// budget.
    pub boot_poll: Duration,
    /// The wall clock stamped into `last_observed.at` (`holler_proto::clock::now_millis`).
    pub now_ms: fn() -> i64,
}

impl Default for TxOptions {
    fn default() -> Self {
        Self {
            budget: DEFAULT_BUDGET,
            probe_timeout: PROBE_TIMEOUT,
            boot_poll: BOOT_POLL,
            now_ms: holler_proto::clock::now_millis,
        }
    }
}

/// What `launch` is asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchRequest {
    pub name: PaneName,
    /// Required for a live launch (`usage` otherwise); unused with `spec_only`.
    pub herdr_session: Option<String>,
    /// The effective spec, complete; the engine checks `spec.pane == name` (`usage`).
    pub spec: ProfileSpec,
    /// The profile whose spec for the pane the run sets (I8), by its stored name.
    pub profile: Option<ProfileName>,
    /// Only with `Some(profile)`; the engine re-checks it (`usage`).
    pub spec_only: bool,
}

/// What `relaunch` is asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelaunchRequest {
    /// As the caller read it; its generation is the expected generation of the record write.
    pub record: Pane,
    /// The effective spec, complete.
    pub spec: ProfileSpec,
    /// The caller was asked to place the pane (`--grid`); the engine derives the move.
    pub grid_given: bool,
    /// The profile whose spec for the pane the run sets (I8), by its stored name.
    pub profile: Option<ProfileName>,
    /// Only with `Some(profile)`; the engine re-checks it (`usage`).
    pub spec_only: bool,
}

/// Which session a relaunched single process starts on (#831, plan review W1 and W4). A
/// server keeps its session of record (B8), so a server pane takes [`ProcessSession::Keep`]
/// alone; anything else is `usage` (a server switches sessions in place: `pane switch`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessSession {
    /// The session of record: resumed when it has a conversation, started on its id when it
    /// has none (relaunch, interrupt, doctor's repair). A record with no session of record
    /// starts on a fresh one.
    Keep,
    /// Another conversation, which must exist: `session-not-found` otherwise, before
    /// anything is stopped (switch).
    Resume(String),
    /// A fresh session, its id the harness's (reset).
    Fresh,
}

/// What a run that succeeded stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launched {
    /// The stored record (`None` with `spec_only`).
    pub pane: Option<Pane>,
    /// The profile as `edit_spec` stored it (`None` without a profile).
    pub profile: Option<Profile>,
}

/// Why a run failed, and whether it got as far as the act.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxFailure {
    pub error: PaneError,
    /// `edit_spec` entered the act, or a step after it failed: the CLI prints the reconcile
    /// step.
    pub acted: bool,
}

impl From<PaneError> for TxFailure {
    fn from(error: PaneError) -> Self {
        Self {
            error,
            acted: false,
        }
    }
}

/// Create a pane: plan, act, observe, record (see the module docs). Step 0, the spec naming the
/// pane, runs first, also with `spec_only`, which then sets P's spec and calls nothing else. A
/// single process starts on a fresh session.
pub fn launch(
    ports: Ports<'_>,
    request: &LaunchRequest,
    options: &TxOptions,
) -> Result<Launched, TxFailure> {
    let budget = Budget::start(options.budget, OP_LAUNCH);
    let (name, spec, profile) = (&request.name, &request.spec, request.profile.as_ref());
    check_named(spec, name)?;
    if request.spec_only {
        return spec_only(ports, profile, name, spec);
    }
    if ports.pane_store.get(name)?.is_some() {
        let message = format!("{name} already has a record; relaunch it instead");
        return Err(refused(PANE_EXISTS, message).into());
    }
    let Some(session) = request.herdr_session.clone() else {
        return Err(usage("--herdr-session is required unless --spec-only".to_owned()).into());
    };
    let fresh = ProcessSession::Fresh;
    let plan = Plan::new(name, spec, session, None, profile, &fresh)?;
    run(ports, &plan, &budget, options)
}

/// Launch a pane again, replacing its processes: E0 (the relaunch rules), then launch's plan
/// and act after stopping only what the pane owns (see the module docs). A single process keeps
/// the session of record. With `spec_only` only E0's spec check runs, and P's spec is set and
/// nothing else is called.
pub fn relaunch(
    ports: Ports<'_>,
    request: &RelaunchRequest,
    options: &TxOptions,
) -> Result<Launched, TxFailure> {
    relaunch_on(ports, request, &ProcessSession::Keep, options)
}

/// [`relaunch`], with the session a single process starts on (#831): the one stop, run, wait,
/// locate and record sequence that `relaunch`, `switch`, `reset`, `interrupt` and doctor's repair
/// share. A choice the harness cannot honour (a [`ProcessSession::Resume`] of a session with no
/// conversation) is refused in the plan, before anything is stopped.
pub fn relaunch_on(
    ports: Ports<'_>,
    request: &RelaunchRequest,
    session: &ProcessSession,
    options: &TxOptions,
) -> Result<Launched, TxFailure> {
    let budget = Budget::start(options.budget, OP_RELAUNCH);
    let (old, spec, profile) = (&request.record, &request.spec, request.profile.as_ref());
    check_named(spec, &old.name)?;
    if request.spec_only {
        return spec_only(ports, profile, &old.name, spec);
    }
    rules::check_relaunch_rules(old, spec, request.grid_given)?;
    if spec.harness.kind.shape() == Shape::ServerAndTui && *session != ProcessSession::Keep {
        let message = format!(
            "{} runs a harness server, which keeps its session of record on a relaunch; switch \
             it with holler pane switch, or reset it with holler pane reset",
            old.name
        );
        return Err(usage(message).into());
    }
    let herdr_session = old.herdr.session.clone();
    let plan = Plan::new(&old.name, spec, herdr_session, Some(old), profile, session)?;
    run(ports, &plan, &budget, options)
}

/// Relaunch `record` as it is recorded, on `session` (#831): its own spec (`spec_from_pane`, the
/// one mapping), its cell, its directory and its profile, through [`relaunch_on`]. The sequence
/// `switch`, `reset`, `interrupt` and doctor's repair of a single process run.
pub fn relaunch_recorded(
    ports: Ports<'_>,
    record: &Pane,
    session: &ProcessSession,
    options: &TxOptions,
) -> Result<Launched, TxFailure> {
    let request = RelaunchRequest {
        record: record.clone(),
        spec: spec_from_pane(record),
        grid_given: false,
        profile: None,
        spec_only: false,
    };
    relaunch_on(ports, &request, session, options)
}

/// The engine's one wait (#802, #831): ask `probe` every `poll` until it answers `Some` or
/// `deadline` passes (`Ok(None)`). An error of `probe` is returned as it is. The deadline is
/// checked before every ask, the first one included, and no ask starts once it has passed: a
/// deadline already past asks nothing, and one that comes during a sleep ends the sleep and
/// the wait. So a call takes at most until `deadline` plus one ask's own bound.
pub fn wait_until<T>(
    poll: Duration,
    deadline: Instant,
    mut probe: impl FnMut() -> Result<Option<T>, PaneError>,
) -> Result<Option<T>, PaneError> {
    loop {
        if Instant::now() >= deadline {
            return Ok(None);
        }
        if let Some(found) = probe()? {
            return Ok(Some(found));
        }
        thread::sleep(deadline.saturating_duration_since(Instant::now()).min(poll));
    }
}

/// What a run starts, by the harness's shape and its port policy ([`port_for`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Start {
    /// A harness server on this port, and its TUI (OpenCode).
    Server(u16),
    /// One process holding one session (Claude Code, #831).
    Process,
}

/// What a live run is asked to bring up.
struct Plan<'a> {
    name: &'a PaneName,
    spec: &'a ProfileSpec,
    /// Where the pane goes: the Herdr session, and the spec's workspace and grid.
    target: HerdrSpec,
    start: Start,
    /// What the harness runs with, from the in-flight spec: the one spec both the argv's
    /// builder (in the plan) and `verify_serving` (in a server's act) get.
    serve: ServeSpec,
    /// The record a relaunch replaces; `None` for a launch.
    old: Option<&'a Pane>,
    profile: Option<&'a ProfileName>,
    /// The session a single process starts on.
    session: &'a ProcessSession,
}

/// What the plan's reads saw, for the act and the record.
struct Seen {
    /// The argv `serve_command` or `process_command` built, which the act runs exactly as it
    /// is.
    command: Argv,
    /// The session a single process's argv holds; `None` for a server.
    held: Option<String>,
    /// The probe's result when the spec has a check (it passed), written only at R.
    probe: Option<ProbeResult>,
    /// Herdr's version, recorded as `host.herdr_api_version`.
    herdr_api_version: String,
    /// Every Herdr pane the plan's snapshot listed: a pane the act gets that is not one of them
    /// is this run's own, so a rollback closes it.
    listed: Vec<HerdrPane>,
}

impl<'a> Plan<'a> {
    /// The plan of a run that puts `name` in the Herdr session `herdr_session`, at the spec's
    /// cell, with the harness its spec names: on the port its policy names, or as one process
    /// (`usage` for a policy outside the grammar, `port-policy-unsupported` for one the kind
    /// does not take), its spec naming no login variable of the kind.
    fn new(
        name: &'a PaneName,
        spec: &'a ProfileSpec,
        herdr_session: String,
        old: Option<&'a Pane>,
        profile: Option<&'a ProfileName>,
        session: &'a ProcessSession,
    ) -> Result<Self, PaneError> {
        let start = match port_for(spec.harness.kind, &spec.harness.port_policy)? {
            Some(port) => Start::Server(port),
            None => Start::Process,
        };
        rules::refuse_login_env(spec)?;
        let (workspace, grid) = (spec.herdr.workspace.clone(), spec.herdr.grid);
        let target = HerdrSpec {
            session: herdr_session,
            workspace,
            grid,
        };
        let process = start == Start::Process;
        // Every field named, never `..Default::default()`: a field `ServeSpec` gains later
        // fails to compile here until the spec's value is passed on. The model and the
        // directory go to a single process only (#829 applies the model to OpenCode).
        let serve = ServeSpec {
            agent: spec.opencode_agent.clone(),
            model: process.then(|| spec.model.clone()),
            cwd: process.then(|| spec.host.cwd.clone()),
        };
        Ok(Self {
            name,
            spec,
            target,
            start,
            serve,
            old,
            profile,
            session,
        })
    }

    /// The plan's reads, none of which writes or changes anything live: first the harness's
    /// argv (pure, so its refusal stays in the plan; #802, #831), then steps 3 to 6: the probe,
    /// Herdr's version, the target cell (where a relaunch's own pane is no occupant) and what
    /// already runs the harness ([`Plan::refuse_running`]).
    fn observe(&self, ports: Ports<'_>, probe_timeout: Duration) -> Result<Seen, PaneError> {
        let (command, held) = self.command(ports)?;
        let probe = probe(ports, self.spec, probe_timeout)?;
        let herdr_api_version = ports.herdr.version()?;
        let listed = ports.herdr.snapshot()?.panes;
        let own = self.old.map(|old| &old.herdr);
        rules::refuse_occupied(ports, &listed, &self.target, own)?;
        self.refuse_running(ports)?;
        Ok(Seen {
            command,
            held,
            probe,
            herdr_api_version,
            listed,
        })
    }

    /// The harness's argv, and for a single process the session it holds.
    fn command(&self, ports: Ports<'_>) -> Result<(Argv, Option<String>), PaneError> {
        match self.start {
            Start::Server(port) => {
                let argv = ports.harness.serve_command(self.name, port, &self.serve)?;
                Ok((argv, None))
            }
            Start::Process => {
                let choice = self.choice(ports)?;
                let argv = ports
                    .harness
                    .process_command(self.name, &self.serve, &choice)?;
                Ok((argv, Some(choice.session().to_owned())))
            }
        }
    }

    /// The session choice a single process's argv is built for, from the run's
    /// [`ProcessSession`]: a fresh id is the harness's (`new_session_id`).
    fn choice(&self, ports: Ports<'_>) -> Result<SessionChoice, PaneError> {
        let kept = self.old.and_then(|old| old.session_of_record.clone());
        Ok(match (self.session, kept) {
            (ProcessSession::Keep, Some(session)) => SessionChoice::Keep(session),
            (ProcessSession::Resume(session), _) => SessionChoice::Resume(session.clone()),
            (ProcessSession::Keep, None) | (ProcessSession::Fresh, _) => {
                SessionChoice::Fresh(ports.harness.new_session_id()?)
            }
        })
    }

    /// Nothing already runs the harness, which is never adopted: no server answers on the port
    /// (`port-in-use`; for a relaunch only when the port changes, since its own server answers
    /// there until B1), and no Claude Code runs in the tmux session of a launch
    /// (`process-in-use`; a relaunch stops what the pane owns first).
    fn refuse_running(&self, ports: Ports<'_>) -> Result<(), PaneError> {
        match self.start {
            Start::Server(port) => {
                let must_be_free = self.old.is_none_or(|old| old.harness.port != Some(port));
                if must_be_free && ports.harness.health(&server::endpoint(port))? {
                    return Err(server::port_in_use(port));
                }
                Ok(())
            }
            Start::Process if self.old.is_none() => process::refuse_in_use(ports, self.name),
            Start::Process => Ok(()),
        }
    }

    /// The record R writes: what the act brought up and observed, and the spec's values. A
    /// relaunch keeps the record's hold, its stored DRIVEN (never inferred) and, without a
    /// profile, its profile.
    fn record(&self, seen: &Seen, live: Live, at: i64) -> Pane {
        let (hold, driven, profile) = match self.old {
            Some(old) => (
                old.hold.clone(),
                old.last_observed.driven.clone(),
                self.profile.or(old.profile.as_ref()).cloned(),
            ),
            None => (Hold::None, None, self.profile.cloned()),
        };
        let spec = self.spec;
        let port = match self.start {
            Start::Server(port) => Some(port),
            Start::Process => None,
        };
        Pane {
            name: self.name.clone(),
            generation: self.old.map_or(0, |old| old.generation),
            // The label Herdr reports is observed, never recorded (#808, constraint (c)).
            herdr: HerdrPane {
                label: None,
                ..live.herdr
            },
            host: HostInfo {
                name: HOST_NAME.to_owned(),
                tmux: self.name.as_str().to_owned(),
                cwd: spec.host.cwd.clone(),
                herdr_api_version: Some(seen.herdr_api_version.clone()),
            },
            harness: HarnessInfo {
                kind: spec.harness.kind,
                port,
                pid: live.pid,
                health: Health::Healthy,
            },
            session_of_record: Some(live.session),
            role: spec.role,
            hold,
            last_observed: LastObserved {
                shown: live.shown,
                driven,
                at,
            },
            profile,
            model: spec.model.clone(),
            opencode_agent: spec.opencode_agent.clone(),
            env: spec.env.clone(),
            context: spec.context,
            command: spec.command.clone(),
            probe: PaneProbe {
                check: spec.check.clone(),
                expect: spec.expect.clone(),
                last: seen.probe.clone(),
            },
        }
    }
}

/// Step 0, and E0's first check: the spec is the named pane's.
fn check_named(spec: &ProfileSpec, name: &PaneName) -> Result<(), PaneError> {
    if spec.pane == name.as_str() {
        return Ok(());
    }
    let named = quoted(&spec.pane);
    Err(usage(format!(
        "the spec names the pane {named}, not {name}"
    )))
}

/// `spec_only`: P's spec for the pane is set, and nothing else is called (no probe, no live
/// call, no record). Without a profile it is `usage`, the engine's re-check of the CLI's rule.
fn spec_only(
    ports: Ports<'_>,
    profile: Option<&ProfileName>,
    name: &PaneName,
    spec: &ProfileSpec,
) -> Result<Launched, TxFailure> {
    let Some(profile) = profile else {
        let message = "--spec-only needs --profile: it edits a profile's spec and changes nothing \
                       live";
        return Err(usage(message.to_owned()).into());
    };
    let edit = SpecEdit::Set(Box::new(spec.clone()));
    let profile = ports
        .scope
        .edit_spec(Some(profile), name, &edit, &mut || Ok(()))?;
    Ok(Launched {
        pane: None,
        profile,
    })
}

/// Step 3: the spec's health check, when it has one. A pass is kept for the record; a failure
/// refuses the run, quoting only the expected strings or the prober's reason, never the output.
fn probe(
    ports: Ports<'_>,
    spec: &ProfileSpec,
    timeout: Duration,
) -> Result<Option<ProbeResult>, PaneError> {
    let Some(check) = &spec.check else {
        return Ok(None);
    };
    let message = match ports.prober.run_probe(check, &spec.expect, timeout) {
        ProbeResult::Ok => return Ok(Some(ProbeResult::Ok)),
        ProbeResult::Failed { missing } => {
            let missing: Vec<String> = missing.iter().map(String::as_str).map(quoted).collect();
            format!("missing {}", missing.join(", "))
        }
        ProbeResult::Error(reason) => embedded(&reason),
    };
    Err(PaneError::ProbeFailed { message })
}

/// What a live act stored, and the error of a relaunch's close of its old pane (B10), which
/// comes back as the run's failure with the record and P already stored.
struct Acted {
    stored: Pane,
    unclosed: Option<PaneError>,
}

/// Steps 3 to 7 of both verbs: the plan's reads, then P's edit, when the run names P, and the
/// act, as one transaction (I8).
fn run(
    ports: Ports<'_>,
    plan: &Plan<'_>,
    budget: &Budget,
    options: &TxOptions,
) -> Result<Launched, TxFailure> {
    let seen = plan.observe(ports, options.probe_timeout)?;
    let edit = SpecEdit::Set(Box::new(plan.spec.clone()));
    let (mut entered, mut acted) = (false, None);
    let mut act = || {
        entered = true;
        acted = Some(act_live(ports, plan, &seen, budget, options)?);
        Ok(())
    };
    let edited = ports
        .scope
        .edit_spec(plan.profile, plan.name, &edit, &mut act);
    let profile = edited.map_err(|error| TxFailure {
        error,
        acted: entered,
    })?;
    let Some(acted) = acted else {
        let what = format!(
            "the profile scope did not run the live change of {}",
            plan.name
        );
        return Err(unavailable(what).into());
    };
    match acted.unclosed {
        None => Ok(Launched {
            pane: Some(acted.stored),
            profile,
        }),
        Some(error) => Err(TxFailure { error, acted: true }),
    }
}

/// The act of both verbs: B1 and B2 for a relaunch, then A1 to O2 (B3 to O2), the record (R)
/// and, after a relaunch's move, B10. A step before R that fails, or a budget that runs out
/// before one, rolls back what this run made; R's own failure is returned as it is.
fn act_live(
    ports: Ports<'_>,
    plan: &Plan<'_>,
    seen: &Seen,
    budget: &Budget,
    options: &TxOptions,
) -> Result<Acted, PaneError> {
    if let Some(old) = plan.old {
        stop_old(ports, plan.name, old, budget)?;
    }
    let mut made = Made::default();
    let live = bring_up(ports, plan, seen, budget, options.boot_poll, &mut made)
        .map_err(|error| roll_back(ports, plan.name, &made, error))?;
    let expected = plan.old.map_or(0, |old| old.generation);
    let record = plan.record(seen, live, (options.now_ms)());
    let stored = ports.pane_store.cas_put(&record, expected)?;
    let unclosed = plan
        .old
        .and_then(|old| close_old(ports, plan, seen, old, &stored.herdr));
    Ok(Acted { stored, unclosed })
}

/// What the act brought up and observed.
struct Live {
    herdr: HerdrPane,
    session: String,
    shown: Option<String>,
    /// The single process's pid; `None` for a server, which records none.
    pid: Option<u32>,
}

/// A1 to O2 (a relaunch's B3 to O2), each after a budget check, and the budget check before R.
/// The spec's command and the harness run in the spec's directory, whatever directory the tmux
/// session was made in; the harness's steps are `server::bring_up` or `process::bring_up`.
fn bring_up(
    ports: Ports<'_>,
    plan: &Plan<'_>,
    seen: &Seen,
    budget: &Budget,
    boot_poll: Duration,
    made: &mut Made,
) -> Result<Live, PaneError> {
    budget.check()?;
    let herdr = ports.herdr.ensure_pane(&plan.target)?;
    if !seen.listed.iter().any(|p| same_pane(p, &herdr)) {
        made.pane = Some(herdr.pane_id.clone());
    }
    budget.check()?;
    let cwd = &plan.spec.host.cwd;
    ports.host.ensure_session(plan.name, cwd)?;
    made.session = true;
    if let Some(command) = &plan.spec.command {
        budget.check()?;
        ports.host.run(plan.name, cwd, command)?;
    }
    let live = match plan.start {
        Start::Server(port) => server::bring_up(ports, plan, seen, budget, boot_poll, herdr, port)?,
        Start::Process => process::bring_up(ports, plan, seen, budget, boot_poll, herdr)?,
    };
    observe_listed(ports, plan.name, budget, &live.herdr)?;
    budget.check()?;
    Ok(live)
}

/// O2: Herdr still lists the pane at its cell; a pane gone is `unavailable` (decision 14).
fn observe_listed(
    ports: Ports<'_>,
    name: &PaneName,
    budget: &Budget,
    herdr: &HerdrPane,
) -> Result<(), PaneError> {
    budget.check()?;
    let listed = ports.herdr.snapshot()?.panes;
    if !listed.iter().any(|pane| same_place(pane, herdr)) {
        return Err(unavailable(format!(
            "the Herdr pane {} of {name} is gone from {}",
            quoted(herdr.pane_id.as_str()),
            cell(&herdr.workspace, herdr.grid)
        )));
    }
    Ok(())
}

/// Whether two Herdr panes are one: the same Herdr session and pane id (the join key).
fn same_pane(a: &HerdrPane, b: &HerdrPane) -> bool {
    a.session == b.session && a.pane_id == b.pane_id
}

/// Whether two Herdr panes are one pane at one place: [`same_pane`], in the same workspace
/// and cell. The label is not compared: it is observed, not a place (#808).
fn same_place(a: &HerdrPane, b: &HerdrPane) -> bool {
    same_pane(a, b) && a.workspace == b.workspace && a.grid == b.grid
}

/// A cell in a message: its grid and its workspace, which is Herdr's text and so quoted.
fn cell(workspace: &str, grid: GridPos) -> String {
    format!("{grid} in workspace {}", quoted(workspace))
}

fn usage(message: String) -> PaneError {
    PaneError::Usage { message }
}

fn unavailable(what: String) -> PaneError {
    PaneError::Unavailable { what }
}

/// A refusal with one of this story's open codes.
fn refused(code: RefusalCode, message: String) -> PaneError {
    PaneError::Refused { code, message }
}
