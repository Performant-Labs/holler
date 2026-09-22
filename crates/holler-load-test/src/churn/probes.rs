//! Read-only probes the churn scenario runs against the hub: roster rows, the
//! dead-backend detection check, and the label-reuse check.

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use crate::fleet::FleetMember;
use crate::hub::{own_process_group, Hub};
use crate::metrics::DeadBackendProbe;
use crate::{kill_tree, Config, Res};

use super::{POLL, REGISTER_BUDGET};

/// Every roster row, including `gone` ones; empty if the hub can't be read.
pub fn roster_rows(hub: &Hub) -> Vec<serde_json::Value> {
    holler_hub::control::roster_at(hub.state().path(), true, None)
        .ok()
        .and_then(|doc| doc.get("rows").and_then(|r| r.as_array()).cloned())
        .unwrap_or_default()
}

/// Every roster row whose `name` is `name`.
pub fn rows_named<'a>(rows: &'a [serde_json::Value], name: &'a str) -> impl Iterator<Item = &'a serde_json::Value> {
    rows.iter().filter(move |r| r.get("name").and_then(|n| n.as_str()) == Some(name))
}

pub fn str_field<'a>(row: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    row.get(key).and_then(|v| v.as_str())
}

/// Healthy = connected, and idle or working: exactly what the 2026-09-21
/// roster kept (wrongly) showing for a dead backend.
pub fn is_healthy(row: Option<&serde_json::Value>) -> bool {
    row.is_some_and(|r| {
        str_field(r, "conn_state") == Some("connected") && matches!(str_field(r, "state"), Some("idle" | "working"))
    })
}

pub fn describe(row: Option<&serde_json::Value>) -> String {
    row.map_or_else(
        || "row absent".to_string(),
        |r| format!("state={} conn_state={}", str_field(r, "state").unwrap_or("?"), str_field(r, "conn_state").unwrap_or("?")),
    )
}

fn row_for(hub: &Hub, name: &str) -> Option<serde_json::Value> {
    let rows = roster_rows(hub);
    let row = rows_named(&rows, name).next().cloned();
    row
}

/// Token records in the hub's store, or `None` if it can't be read.
pub fn token_count(hub: &Hub) -> Option<usize> {
    let hub_state = holler_hub::state::HubState::from_root(hub.state().path().to_path_buf());
    holler_hub::token::list(&hub_state).ok().map(|records| records.len())
}

/// Try to mint a new token under a label whose body was gracefully detached.
pub fn label_reuse(hub: &Hub, label: &str) -> String {
    let hub_state = holler_hub::state::HubState::from_root(hub.state().path().to_path_buf());
    match holler_hub::token::mint(label, 3600, &hub_state) {
        Ok(_) => format!("label {label:?} was freed: re-minting it after `body detach` succeeded"),
        Err(e) => format!("label {label:?} stays claimed after `body detach`: re-mint refused ({e})"),
    }
}

/// Start a body attached to a real fake-OpenCode process, kill that process,
/// and time how long the roster takes to stop showing the session healthy.
pub fn dead_backend(cfg: &Config, hub: &Hub) -> Res<DeadBackendProbe> {
    let (mut backend, endpoint) = spawn_fake_opencode()?;
    let member = match FleetMember::start_attach(hub, &cfg.holler_bin, "churn-dead", "attached", &endpoint, "ses_churn") {
        Ok(m) => m,
        Err(e) => {
            kill_tree(&mut backend);
            return Err(e);
        }
    };
    let name = member.session_names[0].clone();

    let healthy_by = Instant::now() + REGISTER_BUDGET;
    while !is_healthy(row_for(hub, &name).as_ref()) {
        if Instant::now() >= healthy_by {
            kill_tree(&mut backend);
            member.stop();
            return Err(format!(
                "dead-backend probe: attach session never showed healthy; roster: {}",
                describe(row_for(hub, &name).as_ref())
            )
            .into());
        }
        std::thread::sleep(POLL);
    }

    kill_tree(&mut backend);
    let killed_at = Instant::now();
    let mut detection_ms = None;
    let mut row = row_for(hub, &name);
    while killed_at.elapsed() < cfg.dead_backend_window {
        row = row_for(hub, &name);
        if !is_healthy(row.as_ref()) {
            detection_ms = Some(killed_at.elapsed().as_secs_f64() * 1000.0);
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    member.stop();
    Ok(DeadBackendProbe {
        window_secs: cfg.dead_backend_window.as_secs(),
        detected: detection_ms.is_some(),
        detection_ms,
        roster_after: describe(row.as_ref()),
    })
}

/// Re-execute this binary in `--fake-opencode` mode and read the endpoint
/// it prints.
fn spawn_fake_opencode() -> Res<(Child, String)> {
    let mut cmd = Command::new(std::env::current_exe()?);
    cmd.arg("--fake-opencode").stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    own_process_group(&mut cmd);
    let mut child = cmd.spawn()?;
    let mut line = String::new();
    let read = child.stdout.take().map(|s| BufReader::new(s).read_line(&mut line));
    if !matches!(read, Some(Ok(n)) if n > 0) {
        kill_tree(&mut child);
        return Err("fake OpenCode process exited before printing its endpoint".into());
    }
    Ok((child, line.trim().to_string()))
}
