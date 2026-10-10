#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #662
//! The snapshot of a live pane as a `ProfileSpec` (#662 AC 1): the mapping of
//! `profile_snapshot::spec_from_pane`, the port policy it writes, and
//! `profile_from_panes`. Pure functions over records: no port, no fake.

mod common;

use common::{pane, pane_name, profile_name};
use holler_pane::pane::{ContextCeilings, HarnessKind, ModelSpec, PaneRole};
use holler_pane::profile::{SpecHarness, SpecHerdr, SpecHost};
use holler_pane::profile_snapshot::{
    fixed_port_policy, profile_from_panes, spec_from_pane, FIXED_PORT_POLICY_PREFIX,
};
use holler_pane::{AgentKey, Argv, EnvVarName, GridPos, Pane, ProfileSpec};

fn argv(parts: &[&str]) -> Argv {
    Argv::new(parts.iter().map(|p| (*p).to_owned()).collect())
}

fn env(names: &[&str]) -> Vec<EnvVarName> {
    names
        .iter()
        .map(|n| EnvVarName::parse(n).unwrap())
        .collect()
}

/// An agent key (#700), the `env`/`name` helper style.
fn agent(text: &str) -> AgentKey {
    AgentKey::parse(text).unwrap_or_else(|e| panic!("{text}: {e}"))
}

/// `common::pane()` with every spec-relevant field set to a neutral value of this test.
fn full_pane() -> Pane {
    let mut p = pane();
    p.name = pane_name("demo-c1r2");
    p.herdr.workspace = "scratch".to_owned();
    p.herdr.grid = GridPos { row: 2, col: 1 };
    p.host.cwd = "/srv/demo".to_owned();
    p.harness.port = 48100;
    p.model = ModelSpec {
        provider: "demo-provider".to_owned(),
        model_id: "demo-model".to_owned(),
        effort: "high".to_owned(),
    };
    p.opencode_agent = Some(agent("orchestrator"));
    p.role = PaneRole::Orchestrator;
    p.env = env(&["ALPHA_TOKEN", "BETA_URL"]);
    p.context = ContextCeilings { soft: 1, hard: 2 };
    p.command = Some(argv(&["opencode", "serve"]));
    p.probe.check = Some(argv(&["curl", "-s", "http://127.0.0.1:48100/v1/models"]));
    p.probe.expect = vec!["qwen38".to_owned()];
    p
}

#[test]
fn snapshot_copies_every_spec_field_from_the_pane_record() {
    let want = ProfileSpec {
        pane: "demo-c1r2".to_owned(),
        herdr: SpecHerdr {
            workspace: "scratch".to_owned(),
            grid: GridPos { row: 2, col: 1 },
        },
        host: SpecHost {
            cwd: "/srv/demo".to_owned(),
        },
        harness: SpecHarness {
            kind: HarnessKind::Opencode,
            port_policy: "fixed:48100".to_owned(),
        },
        model: ModelSpec {
            provider: "demo-provider".to_owned(),
            model_id: "demo-model".to_owned(),
            effort: "high".to_owned(),
        },
        opencode_agent: Some(agent("orchestrator")),
        role: PaneRole::Orchestrator,
        env: env(&["ALPHA_TOKEN", "BETA_URL"]),
        context: ContextCeilings { soft: 1, hard: 2 },
        command: Some(argv(&["opencode", "serve"])),
        check: Some(argv(&["curl", "-s", "http://127.0.0.1:48100/v1/models"])),
        expect: vec!["qwen38".to_owned()],
    };
    assert_eq!(spec_from_pane(&full_pane()), want);

    // #700 AC 6, the None half: a record with no key snapshots to a spec with none.
    let mut none = full_pane();
    none.opencode_agent = None;
    assert_eq!(spec_from_pane(&none).opencode_agent, None);
}

#[test]
fn snapshot_ignores_the_fields_a_spec_does_not_hold() {
    // Session, pane id, host name, tmux, API version, pid, health, session of record,
    // hold, last observation, profile, generation and the last probe result are live
    // state, not spec: changing them changes nothing in the snapshot.
    let base = full_pane();
    let mut other = base.clone();
    other.generation += 40;
    other.herdr.session = "elsewhere".to_owned();
    other.host.name = "another-host".to_owned();
    other.host.tmux = "another-tmux".to_owned();
    other.host.herdr_api_version = None;
    other.harness.pid = None;
    other.session_of_record = None;
    other.profile = Some(profile_name("Other"));
    other.probe.last = None;
    assert_ne!(base, other, "the two records differ");
    assert_eq!(spec_from_pane(&base), spec_from_pane(&other));
    assert_eq!(spec_from_pane(&base).pane, "demo-c1r2");
}

#[test]
fn snapshot_takes_the_position_from_the_record_not_the_name() {
    // Each name spells a cell other than the one its record holds.
    for (name, row, col, want) in [("demo-c1r2", 2, 1, "r2c1"), ("demo-c2r1", 1, 1, "r1c1")] {
        let mut p = full_pane();
        p.name = pane_name(name);
        p.herdr.grid = GridPos { row, col };
        let spec = spec_from_pane(&p);
        assert_eq!(spec.pane, name);
        assert_eq!(spec.herdr.grid, GridPos { row, col }, "{name}");
        assert_eq!(spec.herdr.grid.to_string(), want, "{name}");
    }
}

#[test]
fn fixed_port_policy_is_the_prefix_and_the_port() {
    for port in [0_u16, 8095, 48100, u16::MAX] {
        assert_eq!(
            fixed_port_policy(port),
            format!("{FIXED_PORT_POLICY_PREFIX}{port}")
        );
    }
    assert_eq!(fixed_port_policy(48100), "fixed:48100");
    // A record at port 0 is copied as recorded (#644's `port_of_policy` refuses it).
    let mut p = full_pane();
    p.harness.port = 0;
    assert_eq!(spec_from_pane(&p).harness.port_policy, "fixed:0");
}

#[test]
fn profile_from_panes_keeps_order_and_starts_at_generation_zero() {
    let names = ["demo-c2r1", "demo-c1r1", "demo-c1r2"];
    let panes: Vec<Pane> = names
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let mut p = full_pane();
            p.name = pane_name(n);
            p.herdr.grid = GridPos {
                row: 1,
                col: u16::try_from(i).unwrap() + 1,
            };
            p.generation = 9;
            p
        })
        .collect();
    let name = profile_name("Some Profile");
    let profile = profile_from_panes(&name, &panes);

    assert_eq!(profile.name, name);
    assert_eq!(profile.slug, "some-profile");
    assert_eq!(profile.generation, 0);
    assert_eq!((profile.created, profile.updated), (0, 0));
    let got: Vec<&str> = profile.panes.iter().map(|s| s.pane.as_str()).collect();
    assert_eq!(got, names, "one spec per pane, in the order given");
    let want: Vec<ProfileSpec> = panes.iter().map(spec_from_pane).collect();
    assert_eq!(profile.panes, want);

    let empty = profile_from_panes(&name, &[]);
    assert!(empty.panes.is_empty());
    assert_eq!(empty.slug, "some-profile");
}
