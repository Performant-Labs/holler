//! `holler pane launch PANE --herdr-session NAME [SPEC FLAGS] [--profile NAME] [--spec-only]`
//! (story #644): create a pane, its tmux session, its harness server and its session of record,
//! and record them (ADR-0021 section 8, "Launch and relaunch as built").
//!
//! The verb types its arguments, reads the profile it names, builds the effective spec, runs the
//! engine (`holler_pane::tx_launch::launch`) and prints what it answered. In order:
//!
//! 1. `PANE` is a pane name (`usage`), the spec flags pass `SpecFlags::validate` (each guard's
//!    own code) and `--profile` is a profile name (`usage`).
//! 2. With `--profile P`, P is read (`profile-not-found`, and nothing else is called); its spec
//!    for the pane, when it has one, is the base the flags overlay. The pane's record is not read
//!    here: the engine's first live step does that.
//! 3. [`effective_spec`]: every flag given replaces its field of the base. With no base the
//!    position, directory, model, effort, ceilings and port policy must all be given.
//! 4. The engine, then [`emit_outcome`].
//!
//! `relaunch.rs` shares [`effective_spec`], [`emit_outcome`], [`Verb`], [`stored_profile`] and
//! [`spec_of`], because the frozen `pane/mod.rs` admits no new module.

use clap::Args;
use holler_pane::findings::quoted;
use holler_pane::pane::{ContextCeilings, HarnessKind, ModelSpec, PaneRole};
use holler_pane::profile::{SpecHarness, SpecHerdr, SpecHost};
use holler_pane::tx_launch::{
    launch, port_of_policy, LaunchRequest, Launched, TxFailure, TxOptions,
};
use holler_pane::{Pane, PaneError, PaneName, Ports, Profile, ProfileName, ProfileSpec};
use serde::Serialize;

use super::args::{ProfileOpt, SpecFlags, SpecOnly, SpecValues};
use super::list::{optional_text, profile_name, text_value};
use super::profile_scope::reconcile_step;
use crate::output::{emit, emit_error, ErrorBody, VerbCtx};

/// Create a pane: its Herdr pane, its tmux session, its harness server and its session of
/// record, then its record.
///
/// The run checks first and changes nothing when a check fails: a name that already has a
/// record (pane-exists: relaunch it instead), a failing health check, an unsupported Herdr, a
/// Herdr cell that already holds a pane (grid-occupied: a pane is never adopted) and a port a
/// server already answers on (port-in-use). It then makes the Herdr pane and the tmux session,
/// runs the spec's command there as an argv (never through a shell), starts the harness server
/// and checks it, creates the session of record over the harness API, attaches the TUI to that
/// session, checks that the TUI shows it and that Herdr still lists the pane, and only then
/// writes the record. A step that fails undoes what the run made (the tmux session stays, its
/// processes stopped) and exits with that step's error; once the live change has started, the
/// message ends with the command that reconciles.
///
/// With no `--profile` every position, directory, model, effort, ceiling and port-policy flag is
/// needed; with `--profile P` the profile's spec for the pane fills the ones not given, and the
/// run sets P's spec for the pane in the same transaction (a failed run puts P's specs back).
/// `--spec-only` sets P's spec and changes nothing live. The one port policy is fixed:PORT.
///
/// `--format=json` prints one envelope whose data is `{"verb", "pane", "profile", "spec_only"}`:
/// the stored record, the profile's name, slug and generation, or null.
#[derive(Args, Debug)]
pub struct PaneLaunch {
    /// The pane's name (also its tmux session name), e.g. demo-c1r1.
    #[arg(value_name = "PANE")]
    pub pane: String,
    /// The Herdr session the pane lives in. Required unless --spec-only.
    #[arg(long, value_name = "NAME")]
    pub herdr_session: Option<String>,
    /// Boxed so `PaneCmd` (inside the frozen `Command`) stays under clippy's
    /// `large_enum_variant` bound now that the verb has its own positional.
    #[command(flatten)]
    pub spec: Box<SpecFlags>,
    #[command(flatten)]
    pub profile: ProfileOpt,
    #[command(flatten)]
    pub spec_only: SpecOnly,
}

/// Run `holler pane launch`: type the arguments, run the engine and print what it answered.
pub fn run(args: &PaneLaunch, ctx: &mut VerbCtx<'_>) -> i32 {
    match launch_request(args, ctx.ports) {
        Ok(request) => {
            let result = launch(ctx.ports, &request, &TxOptions::default());
            emit_outcome(
                ctx,
                Verb::Launch,
                &request.name,
                request.profile.as_ref(),
                result,
            )
        }
        Err(error) => emit_error(&mut ctx.sink, ctx.format, ErrorBody::from(&error)),
    }
}

/// Steps 1 to 3 of the module docs: the request the engine runs.
fn launch_request(args: &PaneLaunch, ports: Ports<'_>) -> Result<LaunchRequest, PaneError> {
    let name = PaneName::parse(&args.pane)?;
    let values = args.spec.validate()?;
    let profile = stored_profile(ports, profile_name(&args.profile)?.as_ref())?;
    let base = profile.as_ref().and_then(|profile| spec_of(profile, &name));
    let spec = effective_spec(base, &name, &values)?;
    Ok(LaunchRequest {
        name,
        herdr_session: args.herdr_session.clone(),
        spec,
        profile: profile.map(|profile| profile.name),
        spec_only: args.spec_only.spec_only,
    })
}

/// The profile `profile` as stored (`profile-not-found` when there is none), or `None` without
/// one. A run passes on the stored name, not the text typed (a profile is found by slug), so its
/// reconcile step names P exactly as the profile scope's own errors do.
pub(crate) fn stored_profile(
    ports: Ports<'_>,
    profile: Option<&ProfileName>,
) -> Result<Option<Profile>, PaneError> {
    let Some(profile) = profile else {
        return Ok(None);
    };
    match ports.profile_store.get(profile)? {
        Some(stored) => Ok(Some(stored)),
        None => Err(PaneError::ProfileNotFound {
            what: quoted(profile.as_str()),
        }),
    }
}

/// The spec `profile` holds for the pane `name`, if any.
pub(crate) fn spec_of<'a>(profile: &'a Profile, name: &PaneName) -> Option<&'a ProfileSpec> {
    profile.panes.iter().find(|spec| spec.pane == name.as_str())
}

/// Overlay the typed flags on `base` (with `None`, every required field must come from the
/// flags). A flag given replaces its field; a repeatable flag given at least once (`--env`,
/// `--expect`, `--command-arg`, `--check-arg`) replaces the whole list; `--role` defaults to
/// `agent` and the harness to OpenCode for a new spec. `usage`, in one line naming every flag
/// that is missing or bad: with no base, a missing `--project`, `--workspace`, `--grid`,
/// `--model`, `--effort`, `--ctx-soft`, `--ctx-hard` or `--port-policy`; a `--model` that is not
/// `PROVIDER/ID` with both halves non-empty; `--ctx-soft` above `--ctx-hard`; a port policy
/// `port_of_policy` refuses; and an `--expect` with no check.
pub(crate) fn effective_spec(
    base: Option<&ProfileSpec>,
    name: &PaneName,
    values: &SpecValues,
) -> Result<ProfileSpec, PaneError> {
    let mut problems = Problems::default();
    let given = |value: Option<&String>, of: fn(&ProfileSpec) -> &String| {
        value.or_else(|| base.map(of)).cloned()
    };
    let cwd = problems.required("--project", given(values.project.as_ref(), |b| &b.host.cwd));
    let workspace = problems.required(
        "--workspace",
        given(values.workspace.as_ref(), |b| &b.herdr.workspace),
    );
    let grid = problems.required("--grid", values.grid.or(base.map(|b| b.herdr.grid)));
    let model = problems.model(values.model.as_deref(), base.map(|b| &b.model));
    let effort = problems.required(
        "--effort",
        given(values.effort.as_ref(), |b| &b.model.effort),
    );
    let soft = problems.required(
        "--ctx-soft",
        values.ctx_soft.or(base.map(|b| b.context.soft)),
    );
    let hard = problems.required(
        "--ctx-hard",
        values.ctx_hard.or(base.map(|b| b.context.hard)),
    );
    let port_policy = problems.required(
        "--port-policy",
        given(values.port_policy.as_ref(), |b| &b.harness.port_policy),
    );
    let command = values
        .command
        .clone()
        .or(base.and_then(|b| b.command.clone()));
    let check = values.check.clone().or(base.and_then(|b| b.check.clone()));
    let opencode_agent = values
        .agent
        .clone()
        .or(base.and_then(|b| b.opencode_agent.clone()));
    let env = replaced(&values.env, base.map(|b| &b.env));
    let expect = replaced(&values.expect, base.map(|b| &b.expect));
    problems.check_values(
        soft.zip(hard),
        port_policy.as_deref(),
        !expect.is_empty() && check.is_none(),
    );
    let (
        Some(cwd),
        Some(workspace),
        Some(grid),
        Some((provider, model_id)),
        Some(effort),
        Some(soft),
        Some(hard),
        Some(port_policy),
    ) = (cwd, workspace, grid, model, effort, soft, hard, port_policy)
    else {
        return Err(problems.into_error());
    };
    if !problems.is_empty() {
        return Err(problems.into_error());
    }
    Ok(ProfileSpec {
        pane: name.as_str().to_owned(),
        herdr: SpecHerdr { workspace, grid },
        host: SpecHost { cwd },
        harness: SpecHarness {
            kind: base.map_or(HarnessKind::Opencode, |b| b.harness.kind),
            port_policy,
        },
        model: ModelSpec {
            provider,
            model_id,
            effort,
        },
        opencode_agent,
        role: values
            .role
            .or(base.map(|b| b.role))
            .unwrap_or(PaneRole::Agent),
        env,
        context: ContextCeilings { soft, hard },
        command,
        check,
        expect,
    })
}

/// A repeatable flag's list: the values given, or else the base's list (empty with no base).
fn replaced<T: Clone>(given: &[T], base: Option<&Vec<T>>) -> Vec<T> {
    if given.is_empty() {
        base.cloned().unwrap_or_default()
    } else {
        given.to_vec()
    }
}

/// What [`effective_spec`] found wrong: the flags missing, then the values refused.
#[derive(Default)]
struct Problems {
    missing: Vec<&'static str>,
    bad: Vec<String>,
}

impl Problems {
    /// `value`, noting `flag` as missing when there is none.
    fn required<T>(&mut self, flag: &'static str, value: Option<T>) -> Option<T> {
        if value.is_none() {
            self.missing.push(flag);
        }
        value
    }

    /// The provider and model id: `--model` split at its first `/`, both halves non-empty, or
    /// else the base's.
    fn model(&mut self, flag: Option<&str>, base: Option<&ModelSpec>) -> Option<(String, String)> {
        let Some(text) = flag else {
            let base = base.map(|m| (m.provider.clone(), m.model_id.clone()));
            return self.required("--model", base);
        };
        match text.split_once('/') {
            Some((provider, id)) if !provider.is_empty() && !id.is_empty() => {
                Some((provider.to_owned(), id.to_owned()))
            }
            _ => {
                self.bad.push(format!(
                    "--model {} must be PROVIDER/ID, both non-empty",
                    quoted(text)
                ));
                None
            }
        }
    }

    /// The checks of values that are present: the ceilings' order, the port policy and an
    /// `--expect` with no check.
    fn check_values(
        &mut self,
        ceilings: Option<(u32, u32)>,
        port_policy: Option<&str>,
        expect_without_check: bool,
    ) {
        if let Some((soft, hard)) = ceilings.filter(|(soft, hard)| soft > hard) {
            self.bad.push(format!(
                "--ctx-soft ({soft}) must not be above --ctx-hard ({hard})"
            ));
        }
        if let Some(Err(refused)) = port_policy.map(port_of_policy) {
            self.bad.push(format!("--port-policy {refused}"));
        }
        if expect_without_check {
            self.bad
                .push("--expect needs a check (--check-arg or --check-json)".to_owned());
        }
    }

    fn is_empty(&self) -> bool {
        self.missing.is_empty() && self.bad.is_empty()
    }

    /// One `usage` line naming every missing flag, then every refused value.
    fn into_error(self) -> PaneError {
        let mut parts = self.bad;
        if !self.missing.is_empty() {
            let missing = format!(
                "missing {} (a pane with no spec to start from needs each of them)",
                self.missing.join(", ")
            );
            parts.insert(0, missing);
        }
        if parts.is_empty() {
            parts.push("the spec is incomplete".to_owned());
        }
        PaneError::Usage {
            message: parts.join("; "),
        }
    }
}

/// The verb a run is, as its output names it.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Verb {
    Launch,
    Relaunch,
}

impl Verb {
    /// The verb's name, `data.verb` in JSON.
    fn as_str(self) -> &'static str {
        match self {
            Verb::Launch => "launch",
            Verb::Relaunch => "relaunch",
        }
    }

    /// What a successful live run did, the first word of its text line.
    fn done(self) -> &'static str {
        match self {
            Verb::Launch => "launched",
            Verb::Relaunch => "relaunched",
        }
    }
}

/// The data of a successful run. Its JSON keys are its fields, in this order.
#[derive(Debug, Serialize)]
struct Report {
    verb: &'static str,
    /// The stored record; `null` with `--spec-only`.
    pane: Option<Pane>,
    /// The profile as the run stored it; `null` without `--profile`.
    profile: Option<ProfileRef>,
    spec_only: bool,
}

/// A profile in the data: its name, slug and generation after the run.
#[derive(Debug, Serialize)]
struct ProfileRef {
    name: ProfileName,
    slug: String,
    generation: u64,
}

impl Report {
    fn new(verb: Verb, launched: Launched) -> Self {
        Self {
            verb: verb.as_str(),
            spec_only: launched.pane.is_none(),
            pane: launched.pane,
            profile: launched.profile.map(|profile| ProfileRef {
                name: profile.name,
                slug: profile.slug,
                generation: profile.generation,
            }),
        }
    }

    /// The text form, one line. A stored string goes through `list::text_value`, so no control
    /// sequence reaches the terminal; a profile name has no control character and is quoted.
    fn text(&self, verb: Verb, name: &PaneName) -> String {
        match (&self.pane, &self.profile) {
            (Some(pane), profile) => {
                let pid = pane
                    .harness
                    .pid
                    .map_or_else(String::new, |pid| format!(" (pid {pid})"));
                let mut line = format!(
                    "{} {} at {} in {}: session {}, harness port {}{pid}",
                    verb.done(),
                    pane.name,
                    pane.herdr.grid,
                    text_value(&pane.herdr.workspace),
                    optional_text(pane.session_of_record.as_deref()),
                    pane.harness.port
                );
                if let Some(profile) = profile {
                    line.push_str(&format!(
                        "; profile {} is at generation {}",
                        quoted(profile.name.as_str()),
                        profile.generation
                    ));
                }
                line
            }
            (None, Some(profile)) => format!(
                "updated the spec of {name} in profile {} (generation {}); nothing live changed",
                quoted(profile.name.as_str()),
                profile.generation
            ),
            (None, None) => format!("{name}: nothing changed"),
        }
    }
}

/// Print a run's result through `output::emit` and return its exit code. A failure that reached
/// the act gets `"; "` and the run's reconcile step appended to its message, unless the message
/// already holds that exact step (the profile scope's own errors carry it: ADR-0021 section 8,
/// step 6). The step is `profile_scope::reconcile_step` of the run's profile, so the scope's step
/// and the verb's compare equal, and nothing else does.
pub(crate) fn emit_outcome(
    ctx: &mut VerbCtx<'_>,
    verb: Verb,
    name: &PaneName,
    profile: Option<&ProfileName>,
    result: Result<Launched, TxFailure>,
) -> i32 {
    let result = result
        .map(|launched| Report::new(verb, launched))
        .map_err(|failure| failure_body(failure, profile));
    emit(&mut ctx.sink, ctx.format, result, |report| {
        report.text(verb, name)
    })
}

/// A failure's error body: its code and text, and the reconcile step when it reached the act.
fn failure_body(failure: TxFailure, profile: Option<&ProfileName>) -> ErrorBody {
    let mut body = ErrorBody::from(&failure.error);
    let step = reconcile_step(profile);
    if failure.acted && !body.message.contains(step.as_str()) {
        body.message = format!("{}; {step}", body.message);
    }
    body
}
