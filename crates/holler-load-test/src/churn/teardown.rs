//! The four ways a churn cycle can tear its bodies down (Run #2 of #373).

use std::collections::HashMap;
use std::time::Instant;

use clap::ValueEnum;

use super::probes::{roster_rows, rows_named, str_field};
use super::{expect_baseline, teardown_all, wait_for_counts, with_resident_traffic, Accum, Cycle, REGISTER_BUDGET};
use crate::fleet::FleetMember;
use crate::Res;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum TeardownMode {
    /// `body detach`, then SIGKILL of the process tree (Run #1's only mode).
    Graceful,
    /// SIGKILL with no detach: the body never says goodbye.
    Crash,
    /// SIGSTOP: sockets stay open, the body goes silent. Then SIGKILL.
    Hang,
    /// Crash, then `body run` again on the same saved credential.
    Restart,
}

impl TeardownMode {
    pub fn name(self) -> &'static str {
        match self {
            Self::Graceful => "graceful",
            Self::Crash => "crash",
            Self::Hang => "hang",
            Self::Restart => "restart",
        }
    }
}

/// Tear the wave down in `mode`, recording into `acc`. Errors (aborting the
/// run) when the hub misses a hard invariant.
pub fn run(mode: TeardownMode, c: &Cycle, members: Vec<FleetMember>, acc: &mut Accum) -> Res<()> {
    match mode {
        TeardownMode::Graceful => kill_and_expect_baseline(c, members, acc, mode, FleetMember::stop),
        TeardownMode::Crash => kill_and_expect_baseline(c, members, acc, mode, FleetMember::crash),
        TeardownMode::Hang => hang(c, members, acc),
        TeardownMode::Restart => restart(c, members, acc),
    }
}

fn kill_and_expect_baseline(
    c: &Cycle,
    members: Vec<FleetMember>,
    acc: &mut Accum,
    mode: TeardownMode,
    kill: fn(FleetMember),
) -> Res<()> {
    let started = Instant::now();
    with_resident_traffic(c, acc, || teardown_all(members, c.cfg.parallel_teardown, kill));
    acc.detach.push(started.elapsed());
    let killed_at = Instant::now();
    expect_baseline(c, mode.name())?;
    acc.record_cleanup(mode, killed_at.elapsed());
    Ok(())
}

/// SIGSTOP every body and see whether the hub cleans up on its own within
/// the hang budget (recorded, not failed), then SIGKILL them and require the
/// hub back at baseline (failed if not).
fn hang(c: &Cycle, members: Vec<FleetMember>, acc: &mut Accum) -> Res<()> {
    let frozen = with_resident_traffic(c, acc, || members.iter().map(FleetMember::hang).collect::<Res<Vec<()>>>());
    let hung_at = Instant::now();
    if let Err(e) = frozen {
        teardown_all(members, c.cfg.parallel_teardown, FleetMember::crash);
        return Err(format!("churn cycle {}: could not SIGSTOP the wave: {e}", c.cycle).into());
    }

    let seen = wait_for_counts(c.hub, c.baseline, c.cfg.hang_budget);
    acc.hang_cycles += 1;
    if seen == Some(c.baseline) {
        acc.hang_cleaned += 1;
        acc.hang_latency.push(hung_at.elapsed());
    } else if acc.hang_roster.is_none() {
        let rows = roster_rows(c.hub);
        let first = members.first().and_then(|m| m.session_names.first());
        let row = first.and_then(|n| rows_named(&rows, n).next());
        acc.hang_roster = Some(format!("{}; hub status {}", super::probes::describe(row), super::fmt(seen)));
    }

    let killed_at = Instant::now();
    teardown_all(members, c.cfg.parallel_teardown, FleetMember::crash);
    expect_baseline(c, "hang, after SIGKILL")?;
    acc.record_cleanup(TeardownMode::Hang, killed_at.elapsed());
    Ok(())
}

/// Crash every body without detaching, wait for the hub to notice, then
/// respawn each on its saved credential and check it came back as the same
/// identity, with exactly one connected roster row per session.
fn restart(c: &Cycle, mut members: Vec<FleetMember>, acc: &mut Accum) -> Res<()> {
    let names: Vec<String> = members.iter().flat_map(|m| m.session_names.clone()).collect();
    let before: HashMap<String, String> = {
        let rows = roster_rows(c.hub);
        names
            .iter()
            .filter_map(|n| rows_named(&rows, n).next().and_then(|r| str_field(r, "client_id")).map(|id| (n.clone(), id.to_string())))
            .collect()
    };

    with_resident_traffic(c, acc, || {
        for m in &mut members {
            m.kill_keep_identity();
        }
    });
    let killed_at = Instant::now();
    if let Err(e) = expect_baseline(c, "restart, after crash") {
        teardown_all(members, false, FleetMember::crash);
        return Err(e);
    }
    acc.record_cleanup(TeardownMode::Restart, killed_at.elapsed());

    let respawned_at = Instant::now();
    if let Some(e) = members.iter_mut().find_map(|m| m.respawn().err()) {
        teardown_all(members, false, FleetMember::crash);
        return Err(format!("churn cycle {}: respawn on the saved credential failed: {e}", c.cycle).into());
    }
    let seen = wait_for_counts(c.hub, c.want, REGISTER_BUDGET);
    if seen != Some(c.want) {
        teardown_all(members, false, FleetMember::crash);
        return Err(format!(
            "churn cycle {}: restarted bodies never rejoined; wanted {}, saw {}",
            c.cycle,
            super::fmt(Some(c.want)),
            super::fmt(seen)
        )
        .into());
    }
    acc.rejoin.push(respawned_at.elapsed());
    acc.restart_cycles += 1;
    check_rejoined_rows(c, &names, &before, acc);

    teardown_all(members, c.cfg.parallel_teardown, FleetMember::stop);
    expect_baseline(c, "restart, final detach")
}

fn check_rejoined_rows(c: &Cycle, names: &[String], before: &HashMap<String, String>, acc: &mut Accum) {
    let rows = roster_rows(c.hub);
    for name in names {
        let named: Vec<&serde_json::Value> = rows_named(&rows, name).collect();
        if named.len() != 1 {
            acc.ghost_or_missing += 1;
        }
        let row = named.iter().find(|r| str_field(r, "conn_state") == Some("connected")).or(named.first());
        if row.and_then(|r| str_field(r, "conn_state")) != Some("connected") {
            acc.not_connected += 1;
        }
        if row.and_then(|r| str_field(r, "client_id")) != before.get(name).map(String::as_str) {
            acc.identity_changed += 1;
        }
    }
}
