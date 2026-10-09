//! `holler pane get` (story #643, epic #633): one pane in full. The record as the registry
//! holds it, its profile's spec for it, and SYNC, the SHOWN/DRIVEN rule of
//! [`SessionSync`].
//!
//! It reads only `PaneStore::get`, `ProfileStore::get` and `ProfileScope::resolve` (at most
//! two calls), observes nothing and writes nothing. The text view is built from the shared
//! helpers of `list.rs`, so every stored string is escaped the same way in all three read
//! verbs.

use clap::Args;
use holler_pane::pane::{HarnessInfo, HarnessKind, Health, HerdrPane, Hold, PaneRole};
use holler_pane::{
    EnvVarName, Pane, PaneError, PaneName, Ports, ProbeResult, Profile, ProfileName, ProfileSpec,
};
use serde::Serialize;

use super::args::ProfileOpt;
use super::list::{
    health_word, hold_word, json_text, observed_at, optional_text, profile_name, text_value,
    SessionSync, NO_VALUE,
};
use crate::output::{emit, ErrorBody, VerbCtx};

/// Show one pane in full: its record, its profile's spec for it, and SYNC.
///
/// Every value is the pane registry's: SHOWN, DRIVEN and health are what reconcile last
/// recorded, and the verb observes nothing itself. With `--profile`, the pane must belong
/// to that profile.
///
/// Text prints one `key: value` line per field, in this order: pane, generation, pos (the
/// grid cell, row first: `r2c1`), profile, project, herdr, host, tmux, herdr-api-version,
/// harness, health (with the reason when it is unhealthy), session-of-record, shown,
/// driven, observed-at, sync (`ok`, MISMATCH or `-`, as in `pane list`), role, hold, model,
/// effort, env (the environment variable names), context (the soft and hard context
/// ceilings), command, probe-check, probe-expect, probe-last and spec.
///
/// A `-` is an empty value. A stored value that is empty or `-`, or that holds a space, a
/// quote, a backslash, an `=` or a character a terminal could act on, is printed quoted and
/// escaped. Times are UTC. The command, the probe's argv and strings, and the spec are
/// printed as JSON.
///
/// `--format=json` prints one envelope whose data is `{"pane": RECORD, "profile": NAME or
/// null, "spec": SPEC or null, "sync": "ok", "mismatch" or "unobserved"}`, the four keys
/// always present. RECORD is the pane record as the registry holds it, with its model, env
/// names, context ceilings (`"context": {"soft": N, "hard": N}`), command and probe; an
/// unset optional field is left out. SPEC is the profile's spec for the pane, with its own
/// context ceilings.
#[derive(Args, Debug)]
pub struct PaneGet {
    /// The pane to show.
    #[arg(value_name = "PANE")]
    pub pane: String,
    #[command(flatten)]
    pub profile: ProfileOpt,
}

/// Run `holler pane get`: read one pane, and its profile's spec for it, and print them.
pub fn run(args: &PaneGet, ctx: &mut VerbCtx<'_>) -> i32 {
    let result = detail(args, ctx.ports).map_err(|e| ErrorBody::from(&e));
    emit(&mut ctx.sink, ctx.format, result, render)
}

/// The data of `pane get --format=json`. The four keys are always present.
#[derive(Debug, Serialize)]
struct PaneDetail {
    /// The record verbatim, in its own serde form.
    pane: Pane,
    /// The profile the record names, if any.
    profile: Option<ProfileName>,
    /// That profile's spec for the pane, if it holds one.
    spec: Option<ProfileSpec>,
    sync: SessionSync,
}

/// The pane `args` names and its spec. The names are typed here, so a bad one is `usage`
/// (exit 2) in both formats.
fn detail(args: &PaneGet, ports: Ports<'_>) -> Result<PaneDetail, PaneError> {
    let name = PaneName::parse(&args.pane)?;
    let (pane, spec) = match profile_name(&args.profile)? {
        Some(profile) => scoped(ports, &profile, &name)?,
        None => unscoped(ports, &name)?,
    };
    Ok(PaneDetail {
        profile: pane.profile.clone(),
        spec,
        sync: SessionSync::of(&pane.last_observed),
        pane,
    })
}

/// With `--profile P`: the pane as `resolve` returns it (no second read), which must belong
/// to P (`pane-not-in-profile`, also for a scope that returns no such pane), and P's spec
/// for it.
fn scoped(
    ports: Ports<'_>,
    profile: &ProfileName,
    name: &PaneName,
) -> Result<(Pane, Option<ProfileSpec>), PaneError> {
    let scope = ports.scope.resolve(profile, Some(name))?;
    let pane = scope
        .panes
        .into_iter()
        .find(|pane| pane.name == *name)
        .ok_or_else(|| PaneError::PaneNotInProfile {
            what: format!("{name} is not in profile {:?}", profile.as_str()),
        })?;
    Ok((pane, spec_for(&scope.profile, name)))
}

/// Without `--profile`: the record (`pane-not-found` when there is none) and, when it names
/// a profile, that profile's spec for it. A profile that is gone, or that holds no spec for
/// the pane, gives none; a failure to read it fails the verb.
fn unscoped(ports: Ports<'_>, name: &PaneName) -> Result<(Pane, Option<ProfileSpec>), PaneError> {
    let pane = ports
        .pane_store
        .get(name)?
        .ok_or_else(|| PaneError::PaneNotFound {
            what: name.to_string(),
        })?;
    let spec = match &pane.profile {
        Some(profile) => ports
            .profile_store
            .get(profile)?
            .and_then(|stored| spec_for(&stored, name)),
        None => None,
    };
    Ok((pane, spec))
}

/// `profile`'s spec for the pane `name`, if it holds one.
fn spec_for(profile: &Profile, name: &PaneName) -> Option<ProfileSpec> {
    profile
        .panes
        .iter()
        .find(|spec| spec.pane == name.as_str())
        .cloned()
}

/// The text view: one `key: value` line per field, in the order `--help` lists.
fn render(detail: &PaneDetail) -> String {
    fields(detail)
        .into_iter()
        .map(|(key, value)| format!("{key}: {value}\n"))
        .collect()
}

/// The fields of the text view, in order. Every stored string goes through `text_value`,
/// and an argv, a list of strings or a spec is JSON (`json_text`).
fn fields(detail: &PaneDetail) -> Vec<(&'static str, String)> {
    let pane = &detail.pane;
    let observed = &pane.last_observed;
    let model = format!("{}/{}", pane.model.provider, pane.model.model_id);
    vec![
        ("pane", text_value(pane.name.as_str())),
        ("generation", pane.generation.to_string()),
        ("pos", pane.herdr.grid.to_string()),
        (
            "profile",
            optional_text(detail.profile.as_ref().map(ProfileName::as_str)),
        ),
        ("project", text_value(&pane.host.cwd)),
        ("herdr", herdr_text(&pane.herdr)),
        ("host", text_value(&pane.host.name)),
        ("tmux", text_value(&pane.host.tmux)),
        (
            "herdr-api-version",
            optional_text(pane.host.herdr_api_version.as_deref()),
        ),
        ("harness", harness_text(&pane.harness)),
        ("health", health_text(&pane.harness.health)),
        (
            "session-of-record",
            optional_text(pane.session_of_record.as_deref()),
        ),
        ("shown", optional_text(observed.shown.as_deref())),
        ("driven", optional_text(observed.driven.as_deref())),
        ("observed-at", observed_at(observed.at)),
        ("sync", detail.sync.text().to_owned()),
        ("role", role_word(pane.role).to_owned()),
        ("hold", hold_text(&pane.hold)),
        ("model", text_value(&model)),
        ("effort", text_value(&pane.model.effort)),
        ("env", env_text(&pane.env)),
        (
            "context",
            format!("soft={} hard={}", pane.context.soft, pane.context.hard),
        ),
        ("command", optional_json(pane.command.as_ref())),
        ("probe-check", optional_json(pane.probe.check.as_ref())),
        ("probe-expect", json_text(&pane.probe.expect)),
        ("probe-last", probe_last_text(pane.probe.last.as_ref())),
        ("spec", optional_json(detail.spec.as_ref())),
    ]
}

/// `session=<s> workspace=<w> pane-id=<id>`.
fn herdr_text(herdr: &HerdrPane) -> String {
    format!(
        "session={} workspace={} pane-id={}",
        text_value(&herdr.session),
        text_value(&herdr.workspace),
        text_value(herdr.pane_id.as_str())
    )
}

/// `<kind> port=<port> pid=<pid or ->`.
fn harness_text(harness: &HarnessInfo) -> String {
    let kind = match harness.kind {
        HarnessKind::Opencode => "opencode",
    };
    let pid = harness
        .pid
        .map_or_else(|| NO_VALUE.to_owned(), |pid| pid.to_string());
    format!("{kind} port={} pid={pid}", harness.port)
}

/// `healthy`, `unknown`, or `unhealthy <reason>`.
fn health_text(health: &Health) -> String {
    match health {
        Health::Unhealthy(reason) => format!("{} {}", health_word(health), text_value(reason)),
        Health::Healthy | Health::Unknown => health_word(health).to_owned(),
    }
}

fn role_word(role: PaneRole) -> &'static str {
    match role {
        PaneRole::Agent => "agent",
        PaneRole::Orchestrator => "orchestrator",
    }
}

/// `none`, `drained`, or `parked reason=<v> until=<v> since=<v>`. `since` is the time the
/// pane was parked, through `text_value` like the others, so a date prints quoted.
fn hold_text(hold: &Hold) -> String {
    match hold {
        Hold::Parked {
            reason,
            release_when,
            since,
        } => format!(
            "{} reason={} until={} since={}",
            hold_word(hold),
            text_value(reason),
            text_value(release_when),
            text_value(&observed_at(*since))
        ),
        Hold::None | Hold::Drained => hold_word(hold).to_owned(),
    }
}

/// The names separated by one space, or `-` for none.
fn env_text(env: &[EnvVarName]) -> String {
    if env.is_empty() {
        return NO_VALUE.to_owned();
    }
    env.iter()
        .map(|name| text_value(name.as_str()))
        .collect::<Vec<_>>()
        .join(" ")
}

/// `ok`, `failed missing=<JSON array>`, `error <reason>`, or `-` before the first run.
fn probe_last_text(last: Option<&ProbeResult>) -> String {
    match last {
        None => NO_VALUE.to_owned(),
        Some(ProbeResult::Ok) => "ok".to_owned(),
        Some(ProbeResult::Failed { missing }) => format!("failed missing={}", json_text(missing)),
        Some(ProbeResult::Error(reason)) => format!("error {}", text_value(reason)),
    }
}

/// [`json_text`] of `value`, or `-` when there is none.
fn optional_json<T: Serialize>(value: Option<&T>) -> String {
    value.map_or_else(|| NO_VALUE.to_owned(), json_text)
}
