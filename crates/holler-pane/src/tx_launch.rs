//! The launch/relaunch transaction (plan, act, observe, record; I3 and I8), story #644: the
//! engine `holler pane launch` and `holler pane relaunch` run, and `profile apply` (#664) will.
//! It works only through the [`Ports`] (ADR-0021 section 5) and holds no I/O of its own.
//!
//! - **Plan**, writing nothing and changing nothing live: the spec names the pane; a launched
//!   name has no record (`pane-exists`) and its Herdr session is given; the port policy names a
//!   port ([`port_of_policy`]); the harness builds the server's argv (`serve_command`, pure, so
//!   its refusal, `agent-unsupported` say, writes nothing); the probe passes when the spec has a
//!   check (a failing result is not stored); Herdr's version is supported; the target cell holds
//!   no Herdr pane (`grid-occupied`, decided by Herdr's snapshot alone, recorded or not: a pane is
//!   never adopted); and no server answers on the port (`port-in-use`, never adopted).
//! - **Act**, inside `ProfileScope::edit_spec` (I8: with a profile, P is written first and its
//!   specs are put back if the act fails): ensure the Herdr pane and the tmux session, run the
//!   spec's command there, then start the server from the planned argv and check it (both runs
//!   in the spec's directory; the child module `server`, #802), create the session of record
//!   through the harness API (never a "ping" session, never "the most recent"), attach the TUI to
//!   exactly that session, and observe that the TUI shows it and Herdr still lists the pane (a
//!   mismatch is `unavailable`). Only then is the record written, by one compare-and-swap, with no
//!   server pid: the server is a process of the pane's tmux session.
//! - **Rollback.** A failed step stops the tmux session's processes, the harness server among
//!   them, and closes the Herdr pane this run created (one the plan's snapshot did not list), and
//!   nothing is recorded. The tmux session and a session the server created stay: no port call
//!   ends either. A record write that conflicts is not rolled back: another writer owns the
//!   record now. A crash runs no rollback, so a server it started stays in the tmux session.
//! - **Relaunch** is the same act after stopping only what the pane owns, its server among them.
//!   It keeps the pane's directory, moves its cell only when the caller was asked to place it
//!   (`grid_given`), keeps the session of record the restarted server still lists, and after a
//!   move records the new pane before it closes the old one. Its rules are the engine's first
//!   step (E0), so every caller is held to them.
//! - **Bound (I5).** A run's budget ([`DEFAULT_BUDGET`]) starts when the engine is entered and is
//!   checked before every live step (a monotonic clock, no port call). When it has run out the run
//!   rolls back and answers `timeout`. A run takes at most the budget plus one port call's own
//!   bound plus its rollback calls. The health wait ([`wait_healthy`]) asks every
//!   [`TxOptions::boot_poll`] with the budget's end as its deadline, checked before each ask, so
//!   it holds to that bound too; its sleep is the engine's one wait.
//!
//! Nothing here types into a pane (I4), runs a shell, or joins or splits an argv. Untrusted text
//! (session and Herdr ids, workspaces, directories, a spec's pane, an adapter's or the prober's
//! message) reaches a message only through `findings::quoted` or `findings::embedded`.

mod server;

use std::time::{Duration, Instant};

use crate::error::RefusalCode;
use crate::findings::{embedded, quoted};
use crate::pane::{HarnessInfo, Health, Hold, HostInfo, LastObserved, PaneProbe};
use crate::profile_snapshot::FIXED_PORT_POLICY_PREFIX;
use crate::reconcile::shown_differs;
use crate::{
    Argv, GridPos, HerdrPane, HerdrSpec, Pane, PaneError, PaneId, PaneName, Ports, ProbeResult,
    Profile, ProfileName, ProfileSpec, ServeSpec, SpecEdit,
};

pub use server::wait_healthy;

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
/// How often the act asks a server it started whether it is healthy (#802).
pub const BOOT_POLL: Duration = Duration::from_millis(150);
/// `HostInfo.name` of every pane recorded here (epic decision 3: Herdr, tmux and the harness
/// run on the hub's machine).
pub const HOST_NAME: &str = "localhost";
/// The `op` of the `timeout` a launch answers when its budget runs out.
pub const OP_LAUNCH: &str = "pane.launch";
/// The `op` of the `timeout` a relaunch answers when its budget runs out.
pub const OP_RELAUNCH: &str = "pane.relaunch";

/// The port a policy names. The grammar beyond #662a's one form is this story's, and it is that
/// form alone: [`FIXED_PORT_POLICY_PREFIX`] then `<port>`, in 1..=65535 in canonical decimal
/// (digits only, no leading zero). So `port_of_policy(&fixed_port_policy(p)) == Ok(p)` for every
/// such `p`, and a policy it accepts equals `fixed_port_policy` of its port. Anything else (the
/// bare `fixed`, `fixed:048100`, `fixed:0`, `auto`) is `usage`, and the message names the form.
pub fn port_of_policy(policy: &str) -> Result<u16, PaneError> {
    let port = policy
        .strip_prefix(FIXED_PORT_POLICY_PREFIX)
        .filter(|digits| digits.bytes().all(|b| b.is_ascii_digit()) && !digits.starts_with('0'))
        .and_then(|digits| digits.parse::<u16>().ok());
    port.ok_or_else(|| {
        usage(format!(
            "{} is not a port policy: the one form is {FIXED_PORT_POLICY_PREFIX}<port>, with \
             <port> from 1 to 65535 and no leading zero",
            quoted(policy)
        ))
    })
}

/// How one engine run is bounded and clocked.
///
/// `now_ms` is a function, where `ReconcileRequest.now_ms` is a value taken once: a run stamps
/// `last_observed.at` when it writes the record, after the TUI was observed, which can be up to
/// the budget after the verb started, so the engine reads the clock there. A test passes a
/// constant.
#[derive(Debug, Clone, Copy)]
pub struct TxOptions {
    /// The budget of the whole run ([`DEFAULT_BUDGET`]).
    pub budget: Duration,
    /// The timeout of the health probe ([`PROBE_TIMEOUT`]).
    pub probe_timeout: Duration,
    /// How often the health wait asks the started server ([`BOOT_POLL`]); the wait ends with
    /// the budget.
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
/// pane, runs first, also with `spec_only`, which then sets P's spec and calls nothing else.
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
    let plan = Plan::new(name, spec, session, None, profile)?;
    run(ports, &plan, &budget, options)
}

/// Launch a pane again, replacing its processes: E0 (the relaunch rules), then launch's plan
/// and act after stopping only what the pane owns (see the module docs). With `spec_only` only
/// E0's spec check runs, and P's spec is set and nothing else is called.
pub fn relaunch(
    ports: Ports<'_>,
    request: &RelaunchRequest,
    options: &TxOptions,
) -> Result<Launched, TxFailure> {
    let budget = Budget::start(options.budget, OP_RELAUNCH);
    let (old, spec, profile) = (&request.record, &request.spec, request.profile.as_ref());
    check_named(spec, &old.name)?;
    if request.spec_only {
        return spec_only(ports, profile, &old.name, spec);
    }
    check_relaunch_rules(old, spec, request.grid_given)?;
    let session = old.herdr.session.clone();
    let plan = Plan::new(&old.name, spec, session, Some(old), profile)?;
    run(ports, &plan, &budget, options)
}

/// The budget of one run, started when the engine is entered. Its end is fixed once, when it
/// starts, so [`Budget::check`] and the health wait ([`Budget::deadline`]) agree on when it runs
/// out.
struct Budget {
    end: Instant,
    op: &'static str,
}

impl Budget {
    /// A budget of `limit` from now. A limit the clock cannot hold (hundreds of billions of
    /// years) is halved until it can, so such a budget still never runs out in practice, for
    /// `check` and the health wait alike.
    fn start(limit: Duration, op: &'static str) -> Self {
        let started = Instant::now();
        let mut limit = limit;
        let end = loop {
            if let Some(end) = started.checked_add(limit) {
                break end;
            }
            limit /= 2;
        };
        Self { end, op }
    }

    /// `timeout` once the budget has run out: a reading of the monotonic clock against its end,
    /// no port call.
    fn check(&self) -> Result<(), PaneError> {
        if Instant::now() < self.end {
            return Ok(());
        }
        Err(self.expired())
    }

    /// The `timeout` of a run whose budget ran out, its `op` the verb's.
    fn expired(&self) -> PaneError {
        let op = self.op.to_owned();
        PaneError::Timeout { op }
    }

    /// When the budget runs out, the health wait's deadline: the instant from which `check`
    /// fails, so the wait asks only while `check` would pass.
    fn deadline(&self) -> Instant {
        self.end
    }
}

/// What a live run is asked to bring up.
struct Plan<'a> {
    name: &'a PaneName,
    spec: &'a ProfileSpec,
    /// Where the pane goes: the Herdr session, and the spec's workspace and grid.
    target: HerdrSpec,
    port: u16,
    /// What the harness server runs with, from the in-flight spec: the one spec both
    /// `serve_command` (in the plan) and `verify_serving` (in the act) get.
    serve: ServeSpec,
    /// The record a relaunch replaces; `None` for a launch.
    old: Option<&'a Pane>,
    profile: Option<&'a ProfileName>,
}

/// What the plan's reads saw, for the act and the record.
struct Seen {
    /// The argv `serve_command` built, which the act runs exactly as it is.
    server: Argv,
    /// The probe's result when the spec has a check (it passed), written only at R.
    probe: Option<ProbeResult>,
    /// Herdr's version, recorded as `host.herdr_api_version`.
    herdr_api_version: String,
    /// Every Herdr pane the plan's snapshot listed: a pane the act gets that is not one of them
    /// is this run's own, so a rollback closes it.
    listed: Vec<HerdrPane>,
}

impl<'a> Plan<'a> {
    /// The plan of a run that puts `name` in the Herdr session `session`, at the spec's cell, on
    /// the port its policy names (`usage` when it names none).
    fn new(
        name: &'a PaneName,
        spec: &'a ProfileSpec,
        session: String,
        old: Option<&'a Pane>,
        profile: Option<&'a ProfileName>,
    ) -> Result<Self, PaneError> {
        let port = port_of_policy(&spec.harness.port_policy)?;
        let (workspace, grid) = (spec.herdr.workspace.clone(), spec.herdr.grid);
        let target = HerdrSpec {
            session,
            workspace,
            grid,
        };
        // Every field named, never `..Default::default()`: a field `ServeSpec` gains later
        // fails to compile here until the spec's value is passed on.
        let serve = ServeSpec {
            agent: spec.opencode_agent.clone(),
        };
        Ok(Self {
            name,
            spec,
            target,
            port,
            serve,
            old,
            profile,
        })
    }

    /// Steps 3 to 6, none of which writes or changes anything live: the server's argv (pure, so
    /// its refusal stays in the plan), the probe, Herdr's version, the target cell (where a
    /// relaunch's own pane is no occupant) and the port, which for a relaunch is checked only
    /// when it changes (its own server answers there until B1).
    fn observe(&self, ports: Ports<'_>, probe_timeout: Duration) -> Result<Seen, PaneError> {
        let server = ports
            .harness
            .serve_command(self.name, self.port, &self.serve)?;
        let probe = probe(ports, self.spec, probe_timeout)?;
        let herdr_api_version = ports.herdr.version()?;
        let listed = ports.herdr.snapshot()?.panes;
        let own = self.old.map(|old| &old.herdr);
        refuse_occupied(ports, &listed, &self.target, own)?;
        let must_be_free = self.old.is_none_or(|old| old.harness.port != self.port);
        if must_be_free && ports.harness.health(self.port)? {
            return Err(port_in_use(self.port));
        }
        Ok(Seen {
            server,
            probe,
            herdr_api_version,
            listed,
        })
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
        Pane {
            name: self.name.clone(),
            generation: self.old.map_or(0, |old| old.generation),
            herdr: live.herdr,
            host: HostInfo {
                name: HOST_NAME.to_owned(),
                tmux: self.name.as_str().to_owned(),
                cwd: spec.host.cwd.clone(),
                herdr_api_version: Some(seen.herdr_api_version.clone()),
            },
            harness: HarnessInfo {
                kind: spec.harness.kind,
                port: self.port,
                pid: None,
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

/// E0 past the spec's pane, for a live relaunch: it never changes a pane's directory, and moves
/// its cell only when the caller was asked to place it (decision 11).
fn check_relaunch_rules(old: &Pane, spec: &ProfileSpec, grid_given: bool) -> Result<(), PaneError> {
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

/// Step 5: `grid-occupied` when the target cell holds a Herdr pane other than `own`. Whether the
/// run is refused is the snapshot's alone. The records are read only to word the refusal, joined
/// to each occupant by Herdr session and pane id whatever cell a record stores, so a stale record
/// is still named.
fn refuse_occupied(
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
        server::stop_old(ports, plan.name, old, budget)?;
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

/// What the act made, so that a rollback undoes that and nothing else.
#[derive(Default)]
struct Made {
    /// The tmux session's step ran, so its processes, the server among them, are stopped.
    session: bool,
    /// The Herdr pane this run created (the plan's snapshot did not list it), so it is closed.
    pane: Option<PaneId>,
}

/// What the act brought up and observed.
struct Live {
    herdr: HerdrPane,
    session: String,
    shown: Option<String>,
}

/// A1 to O2 (a relaunch's B3 to O2), each after a budget check, and the budget check before R.
/// The spec's command and the server run in the spec's directory, whatever directory the tmux
/// session was made in; the server's step is `server::start`.
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
    server::start(ports, plan, seen, budget, boot_poll)?;
    budget.check()?;
    let session = session_of_record(ports, plan)?;
    budget.check()?;
    ports
        .harness
        .attach_tui(&herdr.pane_id, plan.port, &session)?;
    let shown = observe_live(ports, plan.name, budget, &herdr, &session)?;
    budget.check()?;
    Ok(Live {
        herdr,
        session,
        shown,
    })
}

/// A6: a new session, created through the harness API. B8 for a relaunch: the record's session
/// of record when the restarted server still lists it, else a new one (decision 8).
fn session_of_record(ports: Ports<'_>, plan: &Plan<'_>) -> Result<String, PaneError> {
    if let Some(kept) = plan.old.and_then(|old| old.session_of_record.as_deref()) {
        let listed = ports.harness.list_sessions(plan.port)?;
        if listed.iter().any(|session| session == kept) {
            return Ok(kept.to_owned());
        }
    }
    ports.harness.create_session(plan.port)
}

/// O1 and O2: the TUI shows the session of record (`shown_differs`, the one form of that
/// comparison), and Herdr still lists the pane at its cell. Either mismatch is `unavailable`
/// (decision 14). Returns what the TUI showed.
fn observe_live(
    ports: Ports<'_>,
    name: &PaneName,
    budget: &Budget,
    herdr: &HerdrPane,
    session: &str,
) -> Result<Option<String>, PaneError> {
    budget.check()?;
    let shown = ports.harness.shown_session(&herdr.pane_id)?;
    if shown_differs(Some(session), shown.as_deref()) {
        let seen = shown
            .as_deref()
            .map_or_else(|| "no session".to_owned(), quoted);
        return Err(unavailable(format!(
            "the TUI of {name} shows {seen}, not the session of record {}",
            quoted(session)
        )));
    }
    budget.check()?;
    if !ports.herdr.snapshot()?.panes.contains(herdr) {
        return Err(unavailable(format!(
            "the Herdr pane {} of {name} is gone from {}",
            quoted(herdr.pane_id.as_str()),
            cell(&herdr.workspace, herdr.grid)
        )));
    }
    Ok(shown)
}

/// Undo what `made` says this run made, best effort and in order: stop the tmux session's
/// processes, then close the Herdr pane it created. A rollback call that fails does not hide
/// `error`: its message gains `; rollback failed: <call> (<code>)`.
fn roll_back(ports: Ports<'_>, name: &PaneName, made: &Made, error: PaneError) -> PaneError {
    let mut failed = Vec::new();
    if made.session {
        if let Err(stop) = ports.host.stop_owned(name) {
            failed.push(format!("host.stop_owned ({})", stop.code()));
        }
    }
    if let Some(pane) = &made.pane {
        if let Err(close) = closed(ports.herdr.close(pane)) {
            failed.push(format!("herdr.close ({})", close.code()));
        }
    }
    if failed.is_empty() {
        return error;
    }
    with_note(error, &format!("rollback failed: {}", failed.join(", ")))
}

/// B10, after R and only on a move: close the old Herdr pane, when the plan's snapshot listed it
/// and it is not the pane the record now names. A close that fails never fails the act (the
/// record names the pane that holds the TUI, decision 23); its error is the run's failure.
fn close_old(
    ports: Ports<'_>,
    plan: &Plan<'_>,
    seen: &Seen,
    old: &Pane,
    now: &HerdrPane,
) -> Option<PaneError> {
    let moved = plan.target.workspace != old.herdr.workspace || plan.target.grid != old.herdr.grid;
    let listed = seen.listed.iter().any(|p| same_pane(p, &old.herdr));
    if !moved || !listed || same_pane(&old.herdr, now) {
        return None;
    }
    let error = closed(ports.herdr.close(&old.herdr.pane_id)).err()?;
    Some(unavailable(format!(
        "relaunched {} at {}, but its old Herdr pane {} was not closed ({}: {})",
        plan.name,
        cell(&now.workspace, now.grid),
        quoted(old.herdr.pane_id.as_str()),
        error.code(),
        embedded(&error.to_string())
    )))
}

/// A close's answer, with `pane-not-found` counted as closed: a vanished pane is already gone.
fn closed(answer: Result<(), PaneError>) -> Result<(), PaneError> {
    match answer {
        Err(PaneError::PaneNotFound { .. }) => Ok(()),
        other => other,
    }
}

/// Whether two Herdr panes are one: the same Herdr session and pane id (the join key).
fn same_pane(a: &HerdrPane, b: &HerdrPane) -> bool {
    a.session == b.session && a.pane_id == b.pane_id
}

/// A cell in a message: its grid and its workspace, which is Herdr's text and so quoted.
fn cell(workspace: &str, grid: GridPos) -> String {
    format!("{grid} in workspace {}", quoted(workspace))
}

/// `port-in-use`: a server answers on `port`, and a running server is never adopted. The plan
/// asks before anything moves, and the act again just before the server's run.
fn port_in_use(port: u16) -> PaneError {
    let message = format!(
        "a harness server already answers on port {port}; a running server is never adopted, \
         so stop it or choose another port"
    );
    refused(PORT_IN_USE, message)
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

/// `error` with `; <note>` added to its message, its code kept: through the one payload that
/// travels beside the code (`PaneError::detail`, which the crate's own `from_wire` turns back
/// into the variant), or a `Refused`'s message. A variant with no payload is returned as it is.
fn with_note(error: PaneError, note: &str) -> PaneError {
    let noted = match &error {
        PaneError::Refused { code, message } => {
            Some(refused(code.clone(), format!("{message}; {note}")))
        }
        other => other.detail().map(|detail| {
            let detail = Some(format!("{detail}; {note}"));
            PaneError::from_wire(other.code().to_owned(), String::new(), detail)
        }),
    };
    noted.unwrap_or(error)
}
