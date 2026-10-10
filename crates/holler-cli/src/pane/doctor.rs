//! `holler pane doctor [PANE] [--fix] [--profile NAME]` (story #647): run one reconcile pass
//! (`holler_pane::reconcile`) over the panes in scope and print its report.
//!
//! - **Exit codes.** A pass that completes exits 0, with or without findings: findings are
//!   kinds in the report, not errors (ADR-0021 section 9), and an envelope cannot carry data
//!   with a failure, so a script reads `data.findings`. The verb fails only when it cannot
//!   run the pass: a bad `PANE` or `--profile` value is `usage` (2); a pane or profile that
//!   does not exist, or a pane outside the profile, is a refusal (3); a pane store or profile
//!   scope that cannot be read is a failure (1).
//! - **The periodic pass.** There is no hub timer (ADR-0021 section 12): a scheduler runs
//!   `holler pane doctor --format=json`, without `--fix`, as often as wanted, and each run
//!   records what changed.
//! - **Text mode** prints one line each: Herdr's version, each host's recorded Herdr API
//!   version, each finding (`<pane> <pos> <kind>: <message>`, then what `--fix` did and the
//!   command to run), and a summary. Untrusted values are quoted (`findings::quoted`), so no
//!   control sequence reaches the terminal; JSON carries the raw values.

use clap::Args;
use holler_pane::findings::{quoted, Finding, FixState};
use holler_pane::reconcile::{reconcile, ReconcileRequest, Report};
use holler_pane::{PaneError, PaneName, Ports, ProfileName};
use holler_proto::clock::now_millis;

use super::args::ProfileOpt;
use crate::output::{emit, ErrorBody, VerbCtx};

/// Check panes against Herdr, tmux and the harness and report what differs.
#[derive(Args, Debug)]
pub struct PaneDoctor {
    /// Check only this pane (default: every pane in scope).
    #[arg(value_name = "PANE")]
    pub pane: Option<String>,
    /// Repair what the record decides (selects the session of record; never changes it).
    #[arg(long)]
    pub fix: bool,
    #[command(flatten)]
    pub profile: ProfileOpt,
}

/// Run `holler pane doctor`: one reconcile pass, printed in the run's format.
pub fn run(args: &PaneDoctor, ctx: &mut VerbCtx<'_>) -> i32 {
    let report = pass(args, ctx.ports).map_err(|error| ErrorBody::from(&error));
    emit(&mut ctx.sink, ctx.format, report, render)
}

/// Type the arguments (a bad pane or profile name is `usage`) and run the pass now.
fn pass(args: &PaneDoctor, ports: Ports<'_>) -> Result<Report, PaneError> {
    let pane = args.pane.as_deref().map(PaneName::parse).transpose()?;
    let profile = args
        .profile
        .profile
        .as_deref()
        .map(ProfileName::parse)
        .transpose()?;
    let request = ReconcileRequest {
        profile: profile.as_ref(),
        pane: pane.as_ref(),
        fix: args.fix,
        now_ms: now_millis(),
    };
    reconcile(ports, &request)
}

/// The text form of `report`, one line each.
fn render(report: &Report) -> String {
    let herdr = match &report.herdr.version {
        Some(version) => format!("herdr: version {}", quoted(version)),
        None => "herdr: version unknown".to_owned(),
    };
    let mut lines = vec![herdr];
    lines.extend(report.hosts.iter().map(|host| {
        let version = host
            .herdr_api_version
            .as_deref()
            .map_or_else(|| "(not recorded)".to_owned(), quoted);
        format!("host {}: herdr_api_version {version}", quoted(&host.name))
    }));
    lines.extend(report.findings.iter().map(finding_line));
    let fixed = report
        .findings
        .iter()
        .filter(|finding| finding.fix == FixState::Fixed)
        .count();
    lines.push(format!(
        "panes checked: {}; findings: {}; fixed: {fixed}",
        report.panes.len(),
        report.findings.len()
    ));
    lines.join("\n")
}

/// `<subject> <kind>: <message>[ [<fix>]][ (run: <remedy>)]`. The subject is `<pane> <pos>`
/// for a pane, `- <pos>` for an unregistered Herdr pane and `-` for anything else; a message
/// is one line with its untrusted values already quoted, and a remedy is built from
/// constant words and a pane name.
fn finding_line(finding: &Finding) -> String {
    let subject = match (&finding.pane, finding.grid) {
        (Some(pane), Some(grid)) => format!("{pane} {grid}"),
        (Some(pane), None) => pane.to_string(),
        (None, Some(grid)) => format!("- {grid}"),
        (None, None) => "-".to_owned(),
    };
    let mut line = format!("{subject} {}: {}", finding.kind.code(), finding.message);
    match (finding.fix, &finding.fix_error) {
        (FixState::NotFixable, _) => {}
        (FixState::Failed, Some(error)) => line.push_str(&format!(" [fix failed: {}]", error.code)),
        (state, _) => line.push_str(&format!(" [{}]", state.as_str())),
    }
    if let Some(remedy) = &finding.remedy {
        line.push_str(&format!(" (run: {remedy})"));
    }
    line
}
