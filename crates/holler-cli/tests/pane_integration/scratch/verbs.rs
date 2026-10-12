//! What a scenario reads and does on a live scratch set through the binary and the private
//! servers, shared by #649's scenario and #667's apply scenario. Moved here from #649's
//! `scenario.rs` (#667, decision 3), each a cut and paste with no change of assertion: the
//! constants `C1`, `C2` and the workspace `"hlr649"` of `assert_end_state` became its
//! parameters, and `kill_server` is the server kill of #649's `break_both`. A second
//! `impl Scratch`, so `mod.rs` stays under the line budget (A's W-6 of #667).

use std::collections::BTreeSet;
use std::time::Duration;

use holler_pane::{HostPort, PaneName};
use serde_json::{json, Value};

use super::rig::{refused, signal_group};
use super::Scratch;
use crate::support::wait_for;

impl Scratch {
    /// `pane get <pane>`'s record.
    pub fn record(&self, pane: &str) -> Value {
        self.json(&["pane", "get", pane], 0).data()["pane"].clone()
    }

    /// The session of record of `pane`'s record.
    pub fn session_of_record(&self, pane: &str) -> String {
        self.record(pane)["session_of_record"]
            .as_str()
            .unwrap_or_else(|| panic!("{pane} has a session of record"))
            .to_owned()
    }

    /// `pane doctor`'s finding kinds per pane (a finding about no pane is under `-`). Doctor
    /// writes records (reconcile's compare-and-swap), so it is never called between two
    /// readings that must be equal (#667, A's W-4).
    pub fn findings(&self) -> Vec<(String, String)> {
        let data = self.json(&["pane", "doctor"], 0).data();
        data["findings"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|f| {
                let pane = f["pane"].as_str().unwrap_or("-").to_owned();
                (pane, f["kind"].as_str().unwrap_or_default().to_owned())
            })
            .collect()
    }

    /// The private tmux server's environment is the rig's isolated pairs and nothing else (bar
    /// the `PWD` tmux adds, the scratch directory), so nothing of the operator's reached any
    /// OpenCode the scenario started (W-1).
    pub fn assert_tmux_env_is_scratch(&self) {
        let allowed: BTreeSet<String> =
            super::tmux_server_env(&self.settings.opencode, &self.settings.oc_root)
                .into_iter()
                .map(|(name, _)| name)
                .collect();
        let base = self.settings.base.display().to_string();
        for line in self.tmux_global_env() {
            let (name, value) = line
                .trim_start_matches('-')
                .split_once('=')
                .unwrap_or((&line, ""));
            if name == "PWD" {
                assert_eq!(value, base, "tmux's own PWD is the scratch directory");
                continue;
            }
            assert!(
                allowed.contains(name),
                "the tmux server carries {name}, which the test never set"
            );
        }
    }

    /// Kill `pane`'s harness server, which listens on `port`: SIGKILL to the process group of
    /// each pid tmux lists as one `run` tagged in `pane`'s session that `HostPort::ps` reports
    /// live there, then wait until the port refuses.
    pub fn kill_server(&self, pane: &str, port: u16) {
        let tagged = self.owned_pids(pane);
        let live = self.host.ps(&PaneName::parse(pane).unwrap()).expect("ps");
        let server: Vec<u32> = tagged
            .into_iter()
            .filter(|pid| live.contains(pid))
            .collect();
        assert!(
            !server.is_empty(),
            "the server of {pane} runs under its tmux session"
        );
        for pid in &server {
            signal_group(*pid, "KILL");
        }
        assert!(wait_for(Duration::from_secs(10), || refused(port).then_some(())).is_some());
    }

    /// The end state: `doctor` reports no finding, the records of `panes` equal Herdr's
    /// snapshot of the scratch panes in `workspaces` (pane id and cell), and none of them is in
    /// `pane list`'s unmanaged table.
    pub fn assert_end_state(&self, panes: &[&str], workspaces: &[&str]) {
        let report = self.json(&["pane", "doctor"], 0).data();
        assert_eq!(
            report["findings"],
            json!([]),
            "doctor reports no finding: {report}"
        );
        let observed: BTreeSet<(String, String)> = self
            .herdr_snapshot()
            .into_iter()
            .filter(|p| workspaces.contains(&p.workspace.as_str()))
            .map(|p| (p.pane_id.as_str().to_owned(), json!(p.grid).to_string()))
            .collect();
        let recorded: BTreeSet<(String, String)> = panes
            .iter()
            .map(|pane| {
                let herdr = self.record(pane)["herdr"].clone();
                (
                    herdr["pane_id"].as_str().unwrap().to_owned(),
                    herdr["grid"].to_string(),
                )
            })
            .collect();
        assert_eq!(
            recorded, observed,
            "pane list equals Herdr's snapshot for the scratch panes"
        );

        // #832's unmanaged table, read through the real wiring (D33): Herdr is readable, and no
        // pane a record names is listed as unmanaged.
        let list = self.json(&["pane", "list"], 0).data();
        assert!(
            list.get("unmanaged_unavailable").is_none(),
            "Herdr is read: {list}"
        );
        let unmanaged: BTreeSet<String> = list["unmanaged"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|pane| pane["pane_id"].as_str().unwrap().to_owned())
            .collect();
        for (pane_id, _) in &recorded {
            assert!(
                !unmanaged.contains(pane_id),
                "{pane_id} has a record, so is not unmanaged: {list}"
            );
        }
    }
}
