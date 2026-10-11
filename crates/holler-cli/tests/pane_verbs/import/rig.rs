//! The import rig (#650): a `<state-root>` in a temporary directory, holding the files the
//! old launcher keeps for each pane, beside a live world on the test kit's fakes in which no
//! pane has a record yet.
//!
//! The live world is doctor's rig (`doctor/rig.rs`) with every seed unrecorded: for each pane
//! a Herdr pane, a tmux session, a harness server on its own port (48100 + index) and a
//! session on it. The rig labels each Herdr pane with the pane's name (the join key, #808),
//! then writes, for each pane, files that agree with that world: `pane.env`, `last-session`,
//! `sessions.toml` and `owned-sessions` naming the pane's live session, and one launcher
//! entry with the pane's live port. A freshly built rig is a fleet the import records whole.
//!
//! Every value is made up. [`SECRET`] stands for a value the import must never read.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use holler_cli::output::Format;
use holler_pane::pane::{
    ContextCeilings, HarnessInfo, HarnessKind, Health, Hold, HostInfo, LastObserved, ModelSpec,
    PaneProbe, PaneRole,
};
use holler_pane::{
    AgentKey, Argv, EnvVarName, HerdrPane, Pane, PaneName, PaneStore, Ports, ProfileName,
};
use holler_pane_testkit::envelope::check_envelope;
use holler_pane_testkit::harness::HarnessOp;
use holler_pane_testkit::herdr::{HerdrOp, PROTOCOL_22_VERSION};
use holler_pane_testkit::host::HostOp;
use holler_pane_testkit::pane_store::PaneStoreOp;
use holler_pane_testkit::profile_store::ProfileStoreOp;
use serde_json::Value;
use tempfile::TempDir;

use super::seams::ExtraHerdr;
use crate::doctor::rig::{Calls, Rig, Seed};
use crate::verb_harness::{run_verb_with, Outcome};

/// The Herdr session the rig's fake serves, and the one every run names.
pub(crate) const HERDR_SESSION: &str = "scratch";

/// A value in `pane.env` under a variable the import keeps by name only.
pub(crate) const SECRET: &str = "zq-made-up-secret-7b1e";

/// The profile a real run creates.
pub(crate) const FLEET: &str = "fleet";

/// The model of every pane's `pane.env`: split at the first `/`, so the model id keeps its
/// own `/`.
pub(crate) const OC_MODEL: &str = "demo-provider/demo/model-1";
pub(crate) const PROVIDER: &str = "demo-provider";
pub(crate) const MODEL_ID: &str = "demo/model-1";
pub(crate) const EFFORT: &str = "high";
pub(crate) const SOFT: u32 = 100_000;
pub(crate) const HARD: u32 = 150_000;
pub(crate) const AGENT: &str = "builder";

/// The variables kept by name: every one but the five whose values become fields, in file
/// order, de-duplicated.
pub(crate) const ENV_NAMES: [&str; 2] = ["OC_KEY_FILE", "DEMO_TOKEN"];

/// The `pane.env` every pane gets: `export` and bare lines, a blank and a `#` line, the five
/// read variables, and two kept by name (one repeated, one holding [`SECRET`]).
pub(crate) fn pane_env() -> String {
    format!(
        "# made-up values for a test (#650)\n\
         export OC_MODEL={OC_MODEL}\n\
         export OC_EFFORT={EFFORT}\n\
         \n\
         OC_CONTEXT_SOFT={SOFT}\n\
         export\tOC_CONTEXT_HARD={HARD}\n\
         export OC_DEFAULT_AGENT={AGENT}\n\
         export OC_KEY_FILE=/home/demo/.keys/demo.key\n\
         DEMO_TOKEN={SECRET}\n\
         export OC_KEY_FILE=/home/demo/.keys/other.key\n"
    )
}

/// `sessions.toml` with one attach row for `name` naming `session`.
pub(crate) fn sessions_toml(name: &str, port: u16, session: &str) -> String {
    format!(
        "[[session]]\n\
         name = \"{name}\"\n\
         harness = \"opencode\"\n\
         mode = \"attach\"\n\
         endpoint = \"http://127.0.0.1:{port}\"\n\
         session_id = \"{session}\"\n"
    )
}

/// `owned-sessions` listing `sessions`, after a comment and a blank line.
pub(crate) fn owned_sessions(sessions: &[&str]) -> String {
    let mut text = "# sessions this pane created\n\n".to_owned();
    for session in sessions {
        text.push_str(session);
        text.push('\n');
    }
    text
}

/// One launcher table entry.
#[derive(Debug, Clone)]
pub(crate) struct Entry {
    pub port: u16,
    pub cwd: String,
    /// The `command` value as TOML source (an array, or a string for a shell command).
    pub command: String,
}

impl Entry {
    /// The argv the default entry's `command` holds.
    pub fn argv(port: u16) -> Vec<String> {
        vec!["opencode".into(), "--port".into(), port.to_string()]
    }
}

/// The world and the files.
pub(crate) struct ImportRig {
    pub world: Rig,
    pub root: TempDir,
    /// The launcher table's entries by pane name; [`ImportRig::write_launcher`] writes them.
    pub launcher: BTreeMap<String, Entry>,
    /// Snapshot entries the rig's Herdr wrapper adds to the fake's own (a pane of the same
    /// label in another Herdr session, which `FakeHerdr`, serving one session, cannot hold).
    pub extra_herdr: Vec<HerdrPane>,
}

impl ImportRig {
    /// The fleet of `seeds` (made unrecorded), labelled, with agreeing files.
    pub fn new(seeds: &[Seed]) -> Self {
        let unrecorded: Vec<Seed> = seeds.iter().cloned().map(Seed::unrecorded).collect();
        let world = Rig::new(&unrecorded);
        let root = TempDir::new().expect("a temporary state root");
        let mut rig = Self {
            world,
            root,
            launcher: BTreeMap::new(),
            extra_herdr: Vec::new(),
        };
        for seed in seeds {
            rig.label(seed.name, seed.name);
            rig.write_agreeing_files(seed.name);
            let port = rig.world.live(seed.name).port;
            let entry = Entry {
                port,
                cwd: cwd_of(seed.name),
                command: toml_array(&Entry::argv(port)),
            };
            rig.launcher.insert(seed.name.to_owned(), entry);
        }
        rig.write_launcher();
        rig
    }

    /// The state root, as the runs pass it to `--from`.
    pub fn root_str(&self) -> String {
        self.root.path().to_str().expect("a UTF-8 path").to_owned()
    }

    pub fn path(&self, rel: &str) -> PathBuf {
        self.root.path().join(rel)
    }

    /// Write `text` at `rel` under the state root, making its directory.
    pub fn write(&self, rel: &str, text: &str) {
        let path = self.path(rel);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("make the directory");
        std::fs::write(&path, text).expect("write the file");
    }

    pub fn remove(&self, rel: &str) {
        std::fs::remove_file(self.path(rel)).expect("remove the file");
    }

    /// The four files of `name`, agreeing on its live session.
    pub fn write_agreeing_files(&self, name: &str) {
        let live = self.world.live(name);
        self.write(&format!("{name}/pane.env"), &pane_env());
        self.write(
            &format!("{name}/last-session"),
            &format!("{}\n", live.session),
        );
        self.write(
            &format!("{name}/sessions.toml"),
            &sessions_toml(name, live.port, &live.session),
        );
        self.write(
            &format!("{name}/owned-sessions"),
            &owned_sessions(&[&live.session]),
        );
    }

    /// The launcher table at its default path, from [`ImportRig::launcher`], written in
    /// reverse name order (so the import's order is its own, not the file's).
    pub fn write_launcher(&self) {
        let mut text = String::from("# made-up launcher table (#650)\n");
        for (name, entry) in self.launcher.iter().rev() {
            text.push_str(&format!(
                "\n[{name}]\nport = {}\ncwd = \"{}\"\ncommand = {}\n",
                entry.port, entry.cwd, entry.command
            ));
        }
        self.write("launcher.toml", &text);
    }

    /// The outside world labels the Herdr pane of seed `name` with `label`.
    pub fn label(&self, name: &str, label: &str) {
        self.world
            .herdr
            .rename(&self.world.pane_id(name), label)
            .expect("label the pane");
    }

    /// `pane import --from ROOT --herdr-session scratch` followed by `extra`.
    pub fn args(&self, extra: &[&str]) -> Vec<String> {
        let mut argv: Vec<String> = ["pane", "import", "--from"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        argv.push(self.root_str());
        argv.extend(["--herdr-session".to_owned(), HERDR_SESSION.to_owned()]);
        argv.extend(extra.iter().map(|s| (*s).to_owned()));
        argv
    }

    /// Run the import with `extra` arguments over the rig's ports.
    pub fn run(&self, extra: &[&str], format: Format) -> Outcome {
        self.run_argv(&self.args(extra), format)
    }

    /// Run `argv` (after `holler`) over the rig's ports.
    pub fn run_argv(&self, argv: &[String], format: Format) -> Outcome {
        self.run_with_store(&*self.world.panes, argv, format)
    }

    /// Run `argv` over the rig's ports with `store` in place of the fake pane store.
    pub fn run_with_store(
        &self,
        store: &dyn PaneStore,
        argv: &[String],
        format: Format,
    ) -> Outcome {
        let herdr = ExtraHerdr {
            herdr: &self.world.herdr,
            extra: &self.extra_herdr,
        };
        let ports = Ports {
            pane_store: store,
            herdr: &herdr,
            ..self.world.ports()
        };
        let argv: Vec<&str> = argv.iter().map(String::as_str).collect();
        run_verb_with(&argv, format, ports)
    }

    /// Every stored record, through the fake's port (so it is in the log: read it last).
    pub fn records(&self) -> Vec<Pane> {
        self.world.panes.list().expect("the store answers")
    }

    /// The stored record of `name`, if any.
    pub fn record(&self, name: &str) -> Option<Pane> {
        let name = PaneName::parse(name).expect("a pane name");
        self.world.panes.get(&name).expect("the store answers")
    }

    /// The stored profile `fleet`, if any.
    pub fn fleet(&self) -> Option<holler_pane::Profile> {
        use holler_pane::ProfileStore;
        let name = ProfileName::parse(FLEET).expect("a profile name");
        self.world.profiles.get(&name).expect("the store answers")
    }

    /// The record a real run writes for the pane `name` of this rig, from its live world and
    /// its default files: the session of record set, the profile `fleet`, generation 1.
    pub fn expected_record(&self, name: &str, role: PaneRole) -> Pane {
        let live = self.world.live(name);
        let entry = &self.launcher[name];
        Pane {
            name: PaneName::parse(name).expect("a pane name"),
            generation: 1,
            herdr: HerdrPane {
                label: None,
                ..live.herdr.clone()
            },
            host: HostInfo {
                name: "localhost".to_owned(),
                tmux: name.to_owned(),
                cwd: entry.cwd.clone(),
                herdr_api_version: Some(PROTOCOL_22_VERSION.to_owned()),
            },
            harness: HarnessInfo {
                kind: HarnessKind::Opencode,
                port: entry.port,
                pid: None,
                health: Health::Healthy,
            },
            session_of_record: Some(live.session.clone()),
            role,
            hold: Hold::None,
            last_observed: LastObserved {
                shown: None,
                driven: None,
                at: 0,
            },
            profile: Some(ProfileName::parse(FLEET).expect("a profile name")),
            model: ModelSpec {
                provider: PROVIDER.to_owned(),
                model_id: MODEL_ID.to_owned(),
                effort: EFFORT.to_owned(),
            },
            opencode_agent: Some(AgentKey::parse(AGENT).expect("an agent key")),
            env: ENV_NAMES
                .iter()
                .map(|n| EnvVarName::parse(n).expect("an env name"))
                .collect(),
            context: ContextCeilings {
                soft: SOFT,
                hard: HARD,
            },
            command: Some(Argv::new(Entry::argv(entry.port))),
            probe: PaneProbe {
                check: None,
                expect: Vec::new(),
                last: None,
            },
        }
    }

    /// Every file under the state root and its bytes.
    pub fn files(&self) -> BTreeMap<PathBuf, Vec<u8>> {
        let mut files = BTreeMap::new();
        collect_files(self.root.path(), &mut files);
        files
    }

    pub fn mark(&self) -> Calls {
        self.world.mark()
    }

    pub fn calls_since(&self, mark: &Calls) -> Calls {
        self.world.calls_since(mark)
    }
}

/// The project directory of a pane's launcher entry.
pub(crate) fn cwd_of(name: &str) -> String {
    format!("/srv/demo/{name}")
}

/// `items` as a TOML array of strings.
pub(crate) fn toml_array(items: &[String]) -> String {
    let quoted: Vec<String> = items.iter().map(|i| format!("\"{i}\"")).collect();
    format!("[{}]", quoted.join(", "))
}

fn collect_files(dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
    for entry in std::fs::read_dir(dir).expect("read the directory") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            collect_files(&path, out);
        } else {
            let bytes = std::fs::read(&path).expect("read the file");
            out.insert(path, bytes);
        }
    }
}

// --- Assertions --------------------------------------------------------------------------

/// The `data` of a JSON run that succeeded (exit 0, a valid envelope, nothing on err).
pub(crate) fn data_of(run: &Outcome) -> Value {
    assert_eq!(run.code, 0, "the run succeeds: {run:?}");
    assert!(
        run.err.is_empty(),
        "JSON mode writes nothing on err: {run:?}"
    );
    check_envelope(&run.out, run.code)
        .unwrap_or_else(|fault| panic!("{fault}: {run:?}"))
        .data
}

/// The code and message of a JSON run that failed (a valid failure envelope).
pub(crate) fn error_of(run: &Outcome) -> (String, String) {
    let envelope =
        check_envelope(&run.out, run.code).unwrap_or_else(|fault| panic!("{fault}: {run:?}"));
    let error = envelope
        .error
        .unwrap_or_else(|| panic!("the run failed: {run:?}"));
    (error.code, error.message)
}

/// The findings of `data` whose `kind` is `code`.
pub(crate) fn findings<'v>(data: &'v Value, code: &str) -> Vec<&'v Value> {
    data["findings"]
        .as_array()
        .unwrap_or_else(|| panic!("data.findings is an array: {data}"))
        .iter()
        .filter(|f| f["kind"] == code)
        .collect()
}

/// The one finding of `code` about `pane`; fails unless there is exactly one, at `level`.
pub(crate) fn one_finding<'v>(data: &'v Value, code: &str, pane: &str, level: &str) -> &'v Value {
    let found: Vec<&Value> = findings(data, code)
        .into_iter()
        .filter(|f| f["pane"] == pane)
        .collect();
    assert_eq!(found.len(), 1, "exactly one {code} about {pane}: {data}");
    assert_eq!(
        found[0]["level"], level,
        "the level of {code}: {}",
        found[0]
    );
    found[0]
}

/// The message of a finding.
pub(crate) fn message(finding: &Value) -> &str {
    finding["message"]
        .as_str()
        .unwrap_or_else(|| panic!("a finding's message is a string: {finding}"))
}

/// The `data.panes` entry of `name`.
pub(crate) fn pane_row<'v>(data: &'v Value, name: &str) -> &'v Value {
    data["panes"]
        .as_array()
        .unwrap_or_else(|| panic!("data.panes is an array: {data}"))
        .iter()
        .find(|p| p["name"] == name)
        .unwrap_or_else(|| panic!("no data.panes entry for {name}: {data}"))
}

/// The names in `data.panes`, in order.
pub(crate) fn pane_names(data: &Value) -> Vec<String> {
    data["panes"]
        .as_array()
        .unwrap_or_else(|| panic!("data.panes is an array: {data}"))
        .iter()
        .map(|p| p["name"].as_str().expect("a name").to_owned())
        .collect()
}

/// No store write, by either store: none of `cas_put`, `delete`, `rename` (E44).
pub(crate) fn assert_no_store_write(calls: &Calls) {
    let pane_writes: Vec<_> = calls
        .panes
        .iter()
        .filter(|op| matches!(op, PaneStoreOp::CasPut | PaneStoreOp::Delete))
        .collect();
    assert!(
        pane_writes.is_empty(),
        "no pane-store write: {:?}",
        calls.panes
    );
    let profile_writes: Vec<_> = calls
        .profiles
        .iter()
        .filter(|op| {
            matches!(
                op,
                ProfileStoreOp::CasPut | ProfileStoreOp::Delete | ProfileStoreOp::Rename
            )
        })
        .collect();
    assert!(
        profile_writes.is_empty(),
        "no profile write: {:?}",
        calls.profiles
    );
}

/// Only the reads the import may make, through every port (A's finding 4: an allow-list).
pub(crate) fn assert_only_allowed_reads(calls: &Calls) {
    let unexpected = |ok: bool, port: &str, log: String| assert!(ok, "{port} calls: {log}");
    let herdr = calls
        .herdr
        .iter()
        .all(|op| matches!(op, HerdrOp::Snapshot | HerdrOp::Version));
    unexpected(herdr, "Herdr", format!("{:?}", calls.herdr));
    let host = calls.host.iter().all(|op| matches!(op, HostOp::Ps));
    unexpected(host, "host", format!("{:?}", calls.host));
    let harness = calls
        .harness
        .iter()
        .all(|op| matches!(op, HarnessOp::Health | HarnessOp::ListSessions));
    unexpected(harness, "harness", format!("{:?}", calls.harness));
    let panes = calls
        .panes
        .iter()
        .all(|op| matches!(op, PaneStoreOp::Get | PaneStoreOp::List));
    unexpected(panes, "pane-store", format!("{:?}", calls.panes));
    let profiles = calls
        .profiles
        .iter()
        .all(|op| matches!(op, ProfileStoreOp::Get | ProfileStoreOp::List));
    unexpected(profiles, "profile-store", format!("{:?}", calls.profiles));
    assert_eq!(calls.probes, 0, "no probe runs");
}

/// `text` holds `needle` nowhere: the run's out and err.
pub(crate) fn assert_absent(run: &Outcome, needle: &str) {
    assert!(!run.out.contains(needle), "{needle:?} on out: {run:?}");
    assert!(!run.err.contains(needle), "{needle:?} on err: {run:?}");
}

/// The text form's line of the pane `name` (`NAME  ACTION  ROLE  POS  SESSION`), as words.
pub(crate) fn text_line(out: &str, name: &str) -> Vec<String> {
    let lines: Vec<&str> = out
        .lines()
        .filter(|l| l.split_whitespace().next() == Some(name))
        .collect();
    assert_eq!(lines.len(), 1, "one text line for {name}: {out}");
    lines[0].split_whitespace().map(str::to_owned).collect()
}
