//! The adoption rig (#831): the import rig, plus a Claude Code fake and a harness port that
//! reaches both harness fakes, so a pane whose tmux session runs `claude` can be adopted
//! through the fakes and a pane that runs OpenCode can be recorded beside it.
//!
//! The import rig's world has one harness fake, `FakeHarness`, which serves the OpenCode panes'
//! servers. [`AdoptRig`] keeps a `FakeClaude` of its own beside it and lends both to the one
//! router the wiring builds (`HarnessRouter`, with the test kit's `Lent`), so a call reaches the
//! fake its endpoint names and each fake keeps its call log. The two fakes and `FakeHost` mint
//! pids from tables of their own; [`AdoptRig::claude_process`] makes one process in all of
//! them, under the pid `FakeClaude` minted, which is the pid `agents --json` would list.
//!
//! `write_env` writes the `pane.env` of an adopted pane: the same variables the rig's default
//! has (the two kept names are `ENV_NAMES`), with the model and an optional agent chosen by the
//! test. Every value is made up, as in `rig.rs`. [`AdoptRig`] derefs to [`ImportRig`], so everything
//! the import tests do there works on it; its own `run_routed` family is the run over the
//! router.

use std::ops::Deref;

use holler_cli::output::Format;
use holler_pane::pane::{HarnessInfo, HarnessKind, Health, ModelSpec, PaneRole};
use holler_pane::{
    Actor, Argv, HarnessRouter, HostPort, Pane, PaneName, PaneStore, Ports, Profile, ProfileStore,
};
use holler_pane_testkit::claude::{ClaudeOp, FakeClaude};
use holler_pane_testkit::fixture::sample_profile;
use holler_pane_testkit::harnesses::Lent;
use serde_json::Value;

use super::rig::{ImportRig, EFFORT, FLEET};
use super::seams::ExtraHerdr;
use crate::doctor::rig::{Calls, Seed};
use crate::verb_harness::{run_verb_with, Outcome};

/// The switch under test.
pub(crate) const ADOPT: &str = "--adopt-claude-code";

/// A model the adoption takes as read: Anthropic's.
pub(crate) const ANTHROPIC_MODEL: &str = "anthropic/claude-opus-4";

/// The session a Claude Code process holds, which none of the old launcher's files names.
pub(crate) const CLAUDE_SESSION: &str = "f3b1c0de-0000-4000-8000-00000000c1a0";

/// The import rig and a `FakeClaude`.
pub(crate) struct AdoptRig {
    pub rig: ImportRig,
    pub claude: FakeClaude,
}

impl Deref for AdoptRig {
    type Target = ImportRig;

    fn deref(&self) -> &ImportRig {
        &self.rig
    }
}

/// Every call made through every port, the Claude fake's included.
pub(crate) struct Mark {
    world: Calls,
    claude: usize,
}

/// The calls made after a [`Mark`].
pub(crate) struct Since {
    pub world: Calls,
    pub claude: Vec<ClaudeOp>,
}

impl AdoptRig {
    /// The fleet of `seeds` (made unrecorded), labelled, with agreeing files, and a Claude fake
    /// with no process.
    pub fn new(seeds: &[Seed]) -> Self {
        Self {
            rig: ImportRig::new(seeds),
            claude: FakeClaude::new(),
        }
    }

    /// A pane directory is "Claude Code" to the import when its tmux session shows a process
    /// named `claude`. Make one in the session of `pane` that the Claude fake also runs, holding
    /// `session`, under the pid the Claude fake minted. Returns that pid.
    pub fn claude_process(&self, pane: &str, session: &str) -> u32 {
        let name = pane_name(pane);
        let pid = self.claude.run_by_hand(&name, session);
        self.rig
            .world
            .host
            .add_process(&name, "claude", pid)
            .expect("the pane's tmux session exists");
        pid
    }

    /// The fleet's real shape: a person typed `claude` into the pane's shell, so the session
    /// shows `bash` (whose pid only the host knows) and then the Claude process. Returns the
    /// Claude pid.
    pub fn typed_claude(&self, pane: &str, session: &str) -> u32 {
        let name = pane_name(pane);
        self.rig
            .world
            .host
            .run(&name, "/srv/demo", &Argv::new(vec!["bash".to_owned()]))
            .expect("start the shell");
        self.claude_process(pane, session)
    }

    /// A process named `claude` that only the host knows (`agents --json` lists no such pid).
    pub fn unlisted_claude(&self, pane: &str) {
        self.rig
            .world
            .host
            .run(
                &pane_name(pane),
                "/srv/demo",
                &Argv::new(vec!["claude".to_owned()]),
            )
            .expect("start the program");
    }

    /// Replace `pane`'s `pane.env` with the five read variables (model `model`, effort, the
    /// two ceilings) and `extra` lines, so it is Anthropic's unless the test says otherwise.
    pub fn write_env(&self, pane: &str, model: &str, extra: &[&str]) {
        let mut text = format!(
            "export OC_MODEL={model}\nexport OC_EFFORT={EFFORT}\nOC_CONTEXT_SOFT=100000\n\
             OC_CONTEXT_HARD=150000\nexport OC_KEY_FILE=/home/demo/.keys/demo.key\n\
             DEMO_TOKEN=zq-made-up-secret-7b1e\n"
        );
        for line in extra {
            text.push_str(line);
            text.push('\n');
        }
        self.rig.write(&format!("{pane}/pane.env"), &text);
    }

    // --- running over the router ------------------------------------------------------

    /// `holler pane import ... extra` over the rig's ports with the router as the harness.
    pub fn run_routed(&self, extra: &[&str], format: Format) -> Outcome {
        self.run_routed_over(&*self.rig.world.panes, &self.rig.world.host, extra, format)
    }

    /// As [`AdoptRig::run_routed`], with `store` as the pane store.
    pub fn run_routed_store(
        &self,
        store: &dyn PaneStore,
        extra: &[&str],
        format: Format,
    ) -> Outcome {
        self.run_routed_over(store, &self.rig.world.host, extra, format)
    }

    /// As [`AdoptRig::run_routed`], with `store` and `host` in place of the rig's.
    pub fn run_routed_over(
        &self,
        store: &dyn PaneStore,
        host: &dyn HostPort,
        extra: &[&str],
        format: Format,
    ) -> Outcome {
        self.verb_over(store, host, &self.rig.args(extra), format)
    }

    /// Any verb (`holler <argv...>`) over the router and the rig's stores.
    pub fn verb(&self, argv: &[&str], format: Format) -> Outcome {
        let argv: Vec<String> = argv.iter().map(|s| (*s).to_owned()).collect();
        self.verb_over(&*self.rig.world.panes, &self.rig.world.host, &argv, format)
    }

    fn verb_over(
        &self,
        store: &dyn PaneStore,
        host: &dyn HostPort,
        argv: &[String],
        format: Format,
    ) -> Outcome {
        let herdr = ExtraHerdr {
            herdr: &self.rig.world.herdr,
            extra: &self.rig.extra_herdr,
        };
        let router = HarnessRouter::new(Lent(&self.rig.world.harness), Lent(&self.claude));
        let ports = Ports {
            pane_store: store,
            herdr: &herdr,
            host,
            harness: &router,
            ..self.rig.world.ports()
        };
        let argv: Vec<&str> = argv.iter().map(String::as_str).collect();
        run_verb_with(&argv, format, ports)
    }

    // --- the call logs --------------------------------------------------------------------

    pub fn mark_all(&self) -> Mark {
        Mark {
            world: self.rig.mark(),
            claude: self.claude.faults().calls().len(),
        }
    }

    pub fn since(&self, mark: &Mark) -> Since {
        Since {
            world: self.rig.calls_since(&mark.world),
            claude: self.claude.faults().calls()[mark.claude..].to_vec(),
        }
    }

    // --- the expected record -----------------------------------------------------------

    /// The record an adoption writes for the Claude Code pane `name`: what `expected_record`
    /// gives an OpenCode pane of the rig, with the harness being the Claude process (`pid`, no
    /// port, health unknown), the session being the one that process holds, the model being
    /// `model`, no agent and no command (Holler cannot see its argv).
    pub fn expected_claude(
        &self,
        name: &str,
        role: PaneRole,
        pid: u32,
        session: &str,
        model: ModelSpec,
    ) -> Pane {
        Pane {
            harness: HarnessInfo {
                kind: HarnessKind::Claude,
                port: None,
                pid: Some(pid),
                health: Health::Unknown,
            },
            session_of_record: Some(session.to_owned()),
            model,
            opencode_agent: None,
            command: None,
            ..self.rig.expected_record(name, role)
        }
    }

    /// The model of `provider/model_id`, at the rig's effort.
    pub fn model(provider: &str, model_id: &str) -> ModelSpec {
        ModelSpec {
            provider: provider.to_owned(),
            model_id: model_id.to_owned(),
            effort: EFFORT.to_owned(),
        }
    }

    /// The Anthropic model of [`ANTHROPIC_MODEL`].
    pub fn anthropic() -> ModelSpec {
        Self::model("anthropic", "claude-opus-4")
    }

    /// Another actor created `fleet` with a spec for each of `panes` (none of which has a
    /// record); returns the profile as stored, at generation 1.
    pub fn seed_fleet(&self, panes: &[&str]) -> Profile {
        let actor = Actor::parse("someone-else").expect("an actor");
        let profile = sample_profile(FLEET, panes).expect("a profile");
        self.rig
            .world
            .profiles
            .concurrent_put(&profile, &actor)
            .expect("seed fleet")
    }

    /// The stored profile `fleet`, which the test says exists.
    pub fn fleet_now(&self) -> Profile {
        self.rig.fleet().expect("fleet exists")
    }

    /// The generation of the stored `fleet`, read through the port.
    pub fn fleet_generation(&self) -> u64 {
        self.fleet_now().generation
    }

    /// The actors in the change log of the profile `fleet`, oldest first.
    pub fn fleet_actors(&self) -> Vec<String> {
        let name = holler_pane::ProfileName::parse(FLEET).expect("a profile name");
        self.rig
            .world
            .profiles
            .log(&name)
            .expect("the change log")
            .iter()
            .map(|entry| entry.actor.as_str().to_owned())
            .collect()
    }
}

/// A pane name made of `text`.
pub(crate) fn pane_name(text: &str) -> PaneName {
    PaneName::parse(text).expect("a pane name")
}

/// The spec of the pane `name` in `data.profile`.
pub(crate) fn spec_of<'v>(data: &'v Value, name: &str) -> &'v Value {
    data["profile"]["panes"]
        .as_array()
        .unwrap_or_else(|| panic!("data.profile.panes is an array: {data}"))
        .iter()
        .find(|spec| spec["pane"] == name)
        .unwrap_or_else(|| panic!("no spec for {name}: {data}"))
}

/// The names of the specs in `data.profile`, in order.
pub(crate) fn spec_names(data: &Value) -> Vec<String> {
    data["profile"]["panes"]
        .as_array()
        .unwrap_or_else(|| panic!("data.profile.panes is an array: {data}"))
        .iter()
        .map(|spec| {
            spec["pane"]
                .as_str()
                .expect("a spec's pane name")
                .to_owned()
        })
        .collect()
}

/// Every finding of `data` about `pane`.
pub(crate) fn findings_about<'v>(data: &'v Value, pane: &str) -> Vec<&'v Value> {
    data["findings"]
        .as_array()
        .unwrap_or_else(|| panic!("data.findings is an array: {data}"))
        .iter()
        .filter(|f| f["pane"] == pane)
        .collect()
}

/// The pinned refusal of a real run with `n` panes that have problems.
pub(crate) fn blocked(n: usize) -> String {
    format!("import blocked: {n} pane(s) have problems; run with --dry-run for the report")
}
