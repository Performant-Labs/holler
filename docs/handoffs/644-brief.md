# [feat(cli): `holler pane launch` and `relaunch`, a transaction that creates the session of record](https://github.com/Performant-Labs/holler/issues/644)

Part of epic #633. This is the complete statement of the story (consolidated 2026-10-11; where an older comment or amendment disagrees, this text is the decision).

## Status

- **Part 1 is merged** (#718): `holler pane launch` and `relaunch` as a transaction (plan, Herdr pane, tmux session, server, health, session of record through the API, TUI, one CAS write, observe), profile spec-editing, probe, argv rule, grid forms.
- **Part 2 is this run: the OpenCode agent through `launch` and `relaunch`.** #700 (merged) declared the field `Pane.opencode_agent`, its `AgentKey` guard and the `--agent` flag, and the verbs already overlay the flag on the spec and record it (E1 to E8 below). #802 (PR open when this was written) makes the engine pass the in-flight spec's agent to `serve_command` and `verify_serving` (E11 to E14). What is left is small and is listed in Scope.

## Dependency note

Built after **#802** and **#642 part 3** have merged. Every `#802` line quoted below is the pre-merge reading, taken from #802's branch at `3fe111d` (`origin/main` is `263a2d7`); the contract names used here (`serve_command`, `verify_serving`, `ServeSpec`, `AGENT_UNSUPPORTED`) are fixed by #802's issue body. #642 part 3 supplies the OpenCode adapter's real `verify_serving` and its refusal code `agent-unusable`; the fakes in this story stand in for it.

## Scope (final)

1. **Output.** A successful `launch` and `relaunch` report the agent the pane was recorded with:
   - **text:** the one-line result gains `, agent KEY` after `harness port N` and the optional `(pid P)`, before any `; profile ...` clause (E7). A pane with no agent prints exactly the line it prints today (a golden test).
   - **JSON:** `data` gains the key `agent` after `spec_only` (additive; every existing key and its order are unchanged): the stored record's `opencode_agent` as a string, or `null` when the pane has none and with `--spec-only`.
2. **The spec field table.** `opencode_agent` joins `SpecField` (E19): a new variant `OpencodeAgent`, path `opencode_agent`, placed in `ALL` after `Effort` and before `Role` (`ALL` has 16 entries), `value()` reads `spec.opencode_agent`, and `FieldValue` gains `Key(Option<String>)` (serializes as the string or `null`; `Display` prints `none` for `None`, else the key with control characters escaped like `Text`); `differs` is plain equality. `profile show` and `profile apply` therefore see an agent change; `spec_from_pane` already maps the field (E21), so a pane compared with its own snapshot never differs.
3. **The undefined-key refusal is `verify_serving`'s, and the verbs add no check of their own.** The engine asks the running server (E14), after the health wait and before any session is created, whether it honours the spec's agent; a refusal (the adapter's code `agent-unusable` once #642 part 3 lands, `agent-unsupported` for an adapter that cannot apply an agent, the fake's `agent-mismatch`) is exit 3 with the code and message unchanged, goes through the engine's rollback, and records nothing. Only a running server can say which agents it has, so this cannot be moved earlier; the plan's `serve_command` refusal (E11) already covers a harness that cannot apply any agent. The CLI reads no project file.
4. **What needs no change** (stated so the brief does not re-plan it, each shown in the evidence): the flag and its `agent-key-invalid` refusal (E1, E2, E3); the overlay and the rule that a relaunch without `--agent` keeps the recorded agent, and with `--agent` replaces it (E5, E8, E26); the same CAS write that records the pane records the agent, because the record is built from the spec (E12); the engine's plumbing of the spec's agent into `ServeSpec` on launch and relaunch (E11, E24).

## Operator decisions (2026-10-11)

1. **Refuse an undefined key from `verify_serving`'s answer, with the adapter's code and a rollback; no file is read in the CLI.** One source of truth (the server). The refusal happens in the act, not the plan, because only a running server can answer (Scope 3).
2. **`relaunch` without `--agent` keeps the recorded agent.** A relaunch never changes behaviour silently; the existing test pins it (E26).
3. **No clear flag in this story; `--agent` only sets.** Removing an agent is a separate decision and is not planned.
4. **`opencode_agent` joins `SpecField::ALL`** (a small amend of #662's `profile_diff.rs`, in the Blast radius), so `profile show` and `apply` see an agent change.
5. **`agent` appears in the result line and as an additive JSON key** (Scope 1).

## Acceptance (each is a test on the test kit's fakes; paths are real or named sibling modules)

1. **Output, text and JSON.** `crates/holler-cli/tests/pane_verbs/launch/agent.rs` (new sibling module, declared next to `mod serve;` in `launch.rs`): `launch --agent KEY` prints `..., harness port N, agent KEY` and `data.agent == "KEY"`; a launch without `--agent` prints the golden line it prints today with `data.agent == null`; `--spec-only` prints `data.agent == null`; a `--format=json` run passes the envelope checker with every pre-existing key in its pre-existing order.
2. **Launch records the agent.** The same module: after `launch --agent KEY` the stored record has `opencode_agent == Some(KEY)`, written in the same CAS write as the model and effort (one generation bump, one `cas_put`), and the fake harness recorded a `ServeSpec` with that agent for both `serve_command` and `verify_serving`.
3. **Relaunch with a different `--agent`, without `--agent`.** `crates/holler-cli/tests/pane_verbs/relaunch.rs` (existing tests at E26 stay as they are) plus, in the new module, a relaunch result that reports `agent` for a replaced key and for a kept key.
4. **An undefined key is refused, nothing is recorded and nothing runs.** In the new module, on a launch and on a relaunch: the fake `verify_serving` answers a refusal with an open code (`agent-unusable` as the adapter will answer it) and a second case with the fake's own `agent-mismatch`; the verb exits 3 with that code and message unchanged; the registry equals what it was (same generation, no record for a launch); the call log shows the session's processes were stopped by the rollback and no session of record was created; the reconcile step is printed once. (The launch case with `agent-unsupported` is #802's `a_verify_refusal_rolls_back_and_stops_the_server`, E25; this criterion adds the other codes and the relaunch.)
5. **`--agent` with `--profile P`.** `crates/holler-cli/tests/pane_verbs/launch/profiles.rs`: `launch --profile P --agent KEY` writes the key into P's spec for the pane and bumps P's generation once; a launch that fails at `verify_serving` leaves P unchanged.
6. **The field table.** `crates/holler-pane/tests/profile_diff_test.rs`: `PATHS` has 16 entries in `ALL` order with `opencode_agent` after `model.effort`; `spec_field_values_serialize_like_the_spec` holds for a spec with an agent and one without (absent in the spec JSON, `null` in the value); `diff_spec` reports `opencode_agent` as a difference when the spec has `KEY` and the live pane has none, and when they differ, and never when equal; `FieldValue::Key(None).to_string() == "none"`.
7. **`profile show` and `apply` see it.** `crates/holler-cli/tests/profile_verbs/show.rs` (and `apply.rs` where it lists spec fields or differences): a spec with an agent prints `opencode_agent: KEY` and one without prints `opencode_agent: none`; a live pane whose agent differs from its profile's spec is reported as a difference of that field. Every golden or literal list of spec fields in `crates/holler-cli/tests/profile_verbs/**` and `crates/holler-pane-testkit/tests/fake_profile_store_test.rs` is updated for the 16th field (found by grepping for `harness.port_policy` and `SpecField`), and only those lines change.
8. **Docs.** ADR-0021's launch/relaunch row records the agent output and the `verify_serving` refusal; the README or `docs/` text that shows a launch result line (grep for `harness port`) shows the agent form; `CHANGELOG.md` has an `## [Unreleased]` entry linking this issue.

## Blast radius

- `crates/holler-cli/src/pane/launch.rs` (the `Report` struct and its text only; the overlay code stays)
- `crates/holler-cli/src/pane/relaunch.rs` only if its docs need the agent sentence
- `crates/holler-pane/src/profile_diff.rs` (the `SpecField` row and `FieldValue::Key`; amends #662's file)
- the tests named in Acceptance, including the new `crates/holler-cli/tests/pane_verbs/launch/agent.rs`; no test file may pass 900 lines (`launch.rs` is 687, `relaunch.rs` 596)
- `docs/adr/ADR-0021.md` (the launch/relaunch row), `README.md` / `docs/` result-line text, `CHANGELOG.md`

Not in this story: `HarnessPort` and `HostPort` (#802), the adapter and its refusal code (#642 part 3), `Pane.opencode_agent`, `AgentKey` and the flag declaration (#700), `tx_launch.rs`, any hub, protocol or `holler-body` change.

## Depends on

#802 and #642 part 3 merged. (Part 2 also needs #700, merged.)

## Pipeline

`rigor: second-opinion`. Fresh worktree, branch `issue-644-implementation` off `origin/main`.

## Rules for this story

See the epic (#633): the contract section there is fixed. Issue first, tests first, Holler's pipeline and hooks apply (branch `issue-<N>-implementation`, fresh worktree, Conventional Commits, `cargo fmt`, clippy clean, no new `unsafe`). Edit only the files in the Blast radius. No story touches a live fleet, a running pane or a real Herdr session; verification is the fakes from the test kit and temporary directories. Never print or commit a credential.

## AI assistance

This issue was drafted by Claude (Sonnet 5.5) from the operator's decisions in session; the maintainer filing it is accountable for it, per `CONTRIBUTING.md`.

## Evidence (verbatim)

Cut by command from the files named, never retyped. Source: #802's branch at `3fe111d` (which is `origin/main` `263a2d7` plus #802's unmerged commits), so every `tx_launch*`, `ports.rs` and test-kit quote is the pre-merge reading of #802.

- **E1** `crates/holler-cli/src/pane/args.rs:56-66` — the `--agent` flag as declared (#700)
```rust
    pub effort: Option<String>,
    /// The pane's role: agent or orchestrator.
    #[arg(long, value_name = "ROLE")]
    pub role: Option<String>,
    /// The OpenCode agent the pane's messages run as: letters, digits, - and _. Without it, the
    /// server's default agent.
    #[arg(long, value_name = "KEY")]
    pub agent: Option<String>,
    /// An environment variable NAME the pane gets (a name only, never NAME=value). Repeatable.
    #[arg(long, value_name = "NAME")]
    pub env: Vec<String>,
```

- **E2** `crates/holler-cli/src/pane/args.rs:100-136` — the typed value: `AgentKey::parse`, the `agent-key-invalid` refusal
```rust
    pub workspace: Option<String>,
    pub grid: Option<GridPos>,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub role: Option<PaneRole>,
    pub agent: Option<AgentKey>,
    pub env: Vec<EnvVarName>,
    pub ctx_soft: Option<u32>,
    pub ctx_hard: Option<u32>,
    pub port_policy: Option<String>,
    pub command: Option<Argv>,
    pub check: Option<Argv>,
    pub expect: Vec<String>,
}

impl SpecFlags {
    /// Type the flags with the guards of `holler-pane`, or refuse with the guard's own code:
    ///
    /// - `--grid`: `grid-ambiguous` (not a cell, or ambiguous) or `grid-out-of-range`;
    /// - `--env`: `profile-secret-refused` for `NAME=value` (the value is never echoed) and
    ///   `env-name-invalid` for an empty name or one with whitespace;
    /// - `--command-json` and `--check-json`: `command-not-argv` for JSON that is not an array of
    ///   strings, and `usage` for text that is not JSON;
    /// - `--role`: `usage` for anything but `agent` or `orchestrator`;
    /// - `--agent`: `agent-key-invalid` for a key that is empty or holds anything but ASCII
    ///   letters, digits, `-` and `_` (the key is never echoed).
    pub fn validate(&self) -> Result<SpecValues, PaneError> {
        Ok(SpecValues {
            project: self.project.clone(),
            workspace: self.workspace.clone(),
            grid: self.grid.as_deref().map(GridPos::parse).transpose()?,
            model: self.model.clone(),
            effort: self.effort.clone(),
            role: self.role.as_deref().map(parse_role).transpose()?,
            agent: self.agent.as_deref().map(AgentKey::parse).transpose()?,
            env: self
                .env
```

- **E3** `crates/holler-pane/src/argv.rs:130-160` — `AgentKey` and its guard
```rust
///
/// Parsing is the guard (see the module docs). It serializes as a plain string and
/// reads back through [`AgentKey::parse`], so a record holding a malformed key does
/// not load.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct AgentKey(String);

impl AgentKey {
    /// Check `text` and keep it as an agent key: non-empty, and only ASCII letters,
    /// digits, `-` and `_` (so no whitespace, newline, `=` or `/`). Anything else is
    /// [`AGENT_KEY_INVALID`], in a message that states the rule and not the text.
    pub fn parse(text: &str) -> Result<Self, PaneError> {
        let allowed = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
        if text.is_empty() || !text.chars().all(allowed) {
            return Err(PaneError::Refused {
                code: AGENT_KEY_INVALID,
                message: "an OpenCode agent key must be non-empty and use only ASCII letters, \
                          digits, '-' and '_'"
                    .to_owned(),
            });
        }
        Ok(Self(text.to_owned()))
    }

    /// The key.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

- **E4** `crates/holler-cli/src/pane/launch.rs:131-145` — `effective_spec` doc: the flag overlay
```rust
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
```

- **E5** `crates/holler-cli/src/pane/launch.rs:173-180` — the overlay: a flag given replaces the base's agent, else the base's (the record's on a relaunch) is kept
```rust
        .or(base.and_then(|b| b.command.clone()));
    let check = values.check.clone().or(base.and_then(|b| b.check.clone()));
    let opencode_agent = values
        .agent
        .clone()
        .or(base.and_then(|b| b.opencode_agent.clone()));
    let env = replaced(&values.env, base.map(|b| &b.env));
    let expect = replaced(&values.expect, base.map(|b| &b.expect));
```

- **E6** `crates/holler-cli/src/pane/launch.rs:205-224` — the spec built, `opencode_agent` set
```rust
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
```

- **E7** `crates/holler-cli/src/pane/launch.rs:343-412` — the result data and its one-line text (no agent today)
```rust

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
```

- **E8** `crates/holler-cli/src/pane/relaunch.rs:58-88` — relaunch: base is the profile's spec or the record's own; the overlay is `effective_spec`
```rust
        }
        Err(error) => emit_error(&mut ctx.sink, ctx.format, ErrorBody::from(&error)),
    }
}

/// The request the engine runs (see the module docs for the order of the reads).
fn relaunch_request(args: &PaneRelaunch, ports: Ports<'_>) -> Result<RelaunchRequest, PaneError> {
    let name = PaneName::parse(&args.pane)?;
    let values = args.spec.validate()?;
    let profile = profile_name(&args.profile)?;
    let record = ports
        .pane_store
        .get(&name)?
        .ok_or_else(|| PaneError::PaneNotFound {
            what: name.to_string(),
        })?;
    let profile = stored_profile(ports, profile.as_ref())?;
    let from_record = spec_from_pane(&record);
    let base = profile
        .as_ref()
        .and_then(|profile| spec_of(profile, &name))
        .unwrap_or(&from_record);
    let spec = effective_spec(Some(base), &name, &values)?;
    Ok(RelaunchRequest {
        record,
        spec,
        grid_given: values.grid.is_some(),
        profile: profile.map(|profile| profile.name),
        spec_only: args.spec_only.spec_only,
    })
}
```

- **E9** `crates/holler-pane/src/pane.rs:244-252` — the record's field (#700)
```rust
    /// Recorded by launch/relaunch (#644) and import (#650), so
    /// `profile create --from-current` copies them from the store.
    pub model: ModelSpec,
    /// The OpenCode agent the pane's hub-delivered messages run as; `None` means the
    /// server's default agent (#700). A name, never a secret (I7).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opencode_agent: Option<AgentKey>,
    /// Environment variable NAMES only, never values.
    #[serde(default, deserialize_with = "crate::argv::deserialize_env_names")]
```

- **E10** `crates/holler-pane/src/profile.rs:186-190` — the spec's field (#700): absent when `None`
```rust
    /// The OpenCode agent the pane's hub-delivered messages run as; `None` means the
    /// server's default agent (#700). A name, never a secret (I7).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opencode_agent: Option<AgentKey>,
    pub role: PaneRole,
```

- **E11** `crates/holler-pane/src/tx_launch.rs:265-340` — #802: the plan builds the `ServeSpec` from the in-flight spec and calls `serve_command`
```rust
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
```

- **E12** `crates/holler-pane/src/tx_launch.rs:376-392` — the record is built from the spec, `opencode_agent` included
```rust
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
```

- **E13** `crates/holler-pane/src/tx_launch.rs:620-660` — the act: `server::start`
```rust

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
```

- **E14** `crates/holler-pane/src/tx_launch/server.rs:1-101` — #802: the health wait and `verify_serving`, whose refusal goes through the rollback
```rust
//! The harness server's steps of a launch and a relaunch, and the one health wait (#802).
//!
//! The plan built the server's argv (`HarnessPort::serve_command`, pure). The act runs exactly
//! that argv in the pane's tmux session through `HostPort::run`, the one place that starts and
//! stops a pane's processes, so `HostPort::stop_owned` stops the server too (TERM, a grace,
//! then KILL) and the engine records no pid of it. Each step after a budget check (I5):
//!
//! - **The old server** ([`stop_old`], a relaunch's B1 and B2): what the pane owns is stopped,
//!   and a server that still answers on the old port fails the run, loudly, never adopted.
//! - **The new one** ([`start`]), after the tmux session exists and the spec's command runs:
//!   1. **The port again.** A server that started answering on the port since the plan checked
//!      it is refused (`port-in-use`), never adopted and never stopped: with the plan's check,
//!      this is what keeps a launch from taking over a server it did not start.
//!   2. **The run**, in the spec's directory.
//!   3. **The health wait** ([`wait_healthy`]): the server is asked every `boot_poll` until it
//!      answers or the run's budget ends, and a server that never answers ends the run as the
//!      budget's `timeout` (`pane.launch` or `pane.relaunch`). A server that exits at once is
//!      seen the same way: the wait has no process to watch.
//!   4. **The spec**: `verify_serving` asks the running server whether it honours the plan's
//!      `ServeSpec`, before any session is created on it. A refusal is the run's failure, and
//!      the rollback stops the server.

use std::thread;
use std::time::{Duration, Instant};

use crate::{HarnessPort, Pane, PaneError, PaneName, Ports};

use super::{port_in_use, unavailable, Budget, Plan, Seen};

/// Wait for the harness server on `port` to answer: ask `harness.health` every `poll` until it
/// answers `true` (`Ok(true)`) or `deadline` passes (`Ok(false)`). A `health` error is returned
/// as it is. It asks at least once, and once more when the deadline comes during a sleep, so a
/// call takes at most until `deadline` plus one `health` call's own bound.
///
/// The launch engine's wait, and the one the test kit's `HarnessPort` conformance suite and the
/// OpenCode adapter's opt-in real tests wait with.
pub fn wait_healthy(
    harness: &dyn HarnessPort,
    port: u16,
    poll: Duration,
    deadline: Instant,
) -> Result<bool, PaneError> {
    loop {
        if harness.health(port)? {
            return Ok(true);
        }
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Ok(false);
        }
        thread::sleep(left.min(poll));
    }
}

/// B1 and B2 of a relaunch: stop what the pane owns, its old server among them, then check
/// that the old server no longer answers. One that still answers fails the run, loudly, and
/// is never adopted.
pub(super) fn stop_old(
    ports: Ports<'_>,
    name: &PaneName,
    old: &Pane,
    budget: &Budget,
) -> Result<(), PaneError> {
    budget.check()?;
    ports.host.stop_owned(name)?;
    budget.check()?;
    let port = old.harness.port;
    if ports.harness.health(port)? {
        return Err(unavailable(format!(
            "the harness server on port {port} still answers after the processes of {name} \
             were stopped; a running server is never adopted"
        )));
    }
    Ok(())
}

/// The new server's step of the act (see the module docs): the port is checked again, the
/// planned argv runs in the spec's directory, the server's health is waited for within what is
/// left of the budget, and the running server is asked whether it honours the plan's
/// `ServeSpec`.
pub(super) fn start(
    ports: Ports<'_>,
    plan: &Plan<'_>,
    seen: &Seen,
    budget: &Budget,
    poll: Duration,
) -> Result<(), PaneError> {
    budget.check()?;
    if ports.harness.health(plan.port)? {
        return Err(port_in_use(plan.port));
    }
    budget.check()?;
    ports
        .host
        .run(plan.name, &plan.spec.host.cwd, &seen.server)?;
    if !wait_healthy(ports.harness, plan.port, poll, budget.deadline())? {
        return Err(budget.expired());
    }
    budget.check()?;
    ports.harness.verify_serving(plan.port, &plan.serve)
}
```

- **E15** `crates/holler-pane/src/ports.rs:245-256` — #802: `verify_serving`'s contract
```rust
    /// Whether the server running on `port` honours `spec`. The launch engine asks once
    /// the server is healthy, before any session is created on it (#802). `Ok` only when
    /// the server honours every field of `spec`; a server that does not is a refusal with
    /// an open code the implementation owns, and one that cannot be reached answers as any
    /// call to it does (`unavailable`, `timeout`). The engine answers every error by
    /// rolling back (the host then stops the server). An implementation that cannot check
    /// a field it cannot apply refuses it, as [`HarnessPort::serve_command`] does
    /// ([`AGENT_UNSUPPORTED`] for an agent).
    ///
    /// Required, with no default: a harness that starts servers must say whether the
    /// running one honours the spec.
    fn verify_serving(&self, port: u16, spec: &ServeSpec) -> Result<(), PaneError>;
```

- **E16** `crates/holler-pane/src/ports.rs:305-312` — `AGENT_UNSUPPORTED`
```rust
pub const PROMPT_UNSUPPORTED: RefusalCode = RefusalCode::from_static("prompt-unsupported");

/// `agent-unsupported`: [`HarnessPort::serve_command`] or [`HarnessPort::verify_serving`]
/// of a harness that cannot apply the agent a [`ServeSpec`] asks for (#802; the OpenCode
/// adapter answers it for every agent until #642 part 3). Refusal, exit 3: the harness
/// understood the request and declines it; nothing went wrong.
pub const AGENT_UNSUPPORTED: RefusalCode = RefusalCode::from_static("agent-unsupported");
```

- **E17** `crates/holler-pane-testkit/src/harness.rs:36-45` — the fake's own open code `agent-mismatch`
```rust
const PANE_FLAG: &str = "--pane";
const PORT_FLAG: &str = "--port";
const AGENT_FLAG: &str = "--agent";

/// `agent-mismatch`: the fake's [`HarnessPort::verify_serving`] of a spec whose agent is not
/// the one the running server was started with. The fake's own open code: a refusal, exit 3.
pub const AGENT_MISMATCH: RefusalCode = RefusalCode::from_static("agent-mismatch");

/// A method of the `HarnessPort` port, as a fault targets it and the call log records
/// it.
```

- **E18** `crates/holler-pane-testkit/src/harness.rs:125-175` — the fake's `verify_serving`
```rust
/// aborts the servers acknowledged. Every port shares the data directory `"default"`,
/// as the live fleet's servers share one (opencode-pane-spike.md:75-77), until
/// [`FakeHarness::set_data_dir`] gives a port its own.
///
/// A call that *reaches* a port goes on when the server there runs. When the server is
/// frozen, the call answers `timeout` with the method's [`HarnessOp`] as its `op`, the
/// shape a wedged port answers; when it was killed or never started, the call answers
/// `unavailable` ("the harness server on port N"). Each method, in check order:
///
/// - `serve_command(name, port, spec)`: pure. It records `spec`
///   ([`FakeHarness::serve_specs`]) and returns an argv that describes the server,
///   `holler-fake-harness-server --pane <name> --port <port>`, then `--agent <key>` when
///   the spec has one. It starts nothing; [`FakeHarness::start_server`] starts the server
///   such an argv describes, as a host's `run` would.
/// - `verify_serving(port, spec)` reaches the port. `Ok` when `spec.agent` is the agent the
///   running server was started with; otherwise the fake's refusal [`AGENT_MISMATCH`].
/// - `health(port)`: `true` when the server runs, `false` when it is frozen, killed or
///   never started, as a timed request that times out or is refused would tell.
/// - `create_session(port)` and `list_sessions(port)` reach the port, then mint an id
///   (`ses_` and 26 hex digits, 30 characters; treat it as opaque) in the port's data
///   directory, or list that directory's sessions in creation order.
/// - `abort(port, session)` reaches the port. A session outside its data directory is
///   `session-not-found`; otherwise the abort is logged ([`FakeHarness::aborts`]).
/// - `attach_tui(pane, port, session)` reaches the port. A session outside its data
///   directory is `session-not-found`, and the pane keeps the TUI it had; otherwise the
///   pane's TUI, replacing any earlier one, shows the session.
/// - `select_session(pane, session)`: a pane with no TUI is `unavailable` ("no TUI in
///   pane P"), checked first because the TUI is what names the server. It then reaches
///   the TUI's port. A session outside that port's data directory is
///   `session-not-found`, and the screen stays; otherwise that TUI, and no other, shows
///   the session.
/// - `shown_session(pane)`: the session the TUI shows, or `None` on its home screen or
///   with no TUI. It does not reach the server: a TUI keeps its screen while its server
///   is frozen or dead (opencode-pane-spike.md:193-194).
///
/// Every port method first passes [`FakeHarness::faults`], which models a wedged
/// *adapter*: under `Fault::Wedged` every method answers `timeout`, `health` and
/// `shown_session` included. [`FakeHarness::freeze`] models a wedged *server* behind a
/// working adapter. A method that fails changes nothing. The configuration, scenario
/// and inspection methods bypass the faults and the call log.
pub struct FakeHarness {
    world: Mutex<World>,
    faults: FaultSwitch<HarnessOp>,
}

/// What the fake holds, behind its lock.
#[derive(Default)]
struct World {
    servers: HashMap<u16, ServerView>,
    /// Every spec `serve_command` was asked for, oldest first.
    serve_specs: Vec<ServeSpec>,
```

- **E19** `crates/holler-pane/src/profile_diff.rs:28-135` — `SpecField`, `ALL` (15 fields), `as_str`, `value`, `differs`
```rust
use crate::profile::{Profile, ProfileName, ProfileSpec};
use crate::profile_snapshot::spec_from_pane;

/// A field of a `ProfileSpec`, named by its dotted JSON path. Serializes as that path
/// ([`SpecField::as_str`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpecField {
    Workspace,
    Grid,
    Cwd,
    HarnessKind,
    PortPolicy,
    Provider,
    ModelId,
    Effort,
    Role,
    Env,
    ContextSoft,
    ContextHard,
    Command,
    Check,
    Expect,
}

impl SpecField {
    /// Every field, in the order `profile show` lists a spec and [`diff_spec`] compares
    /// it. `harness.port_policy` is compared like every other field: the live side is
    /// the policy of the port the pane's harness uses (ADR-0021 section 3).
    pub const ALL: [SpecField; 15] = [
        SpecField::Workspace,
        SpecField::Grid,
        SpecField::Cwd,
        SpecField::HarnessKind,
        SpecField::PortPolicy,
        SpecField::Provider,
        SpecField::ModelId,
        SpecField::Effort,
        SpecField::Role,
        SpecField::Env,
        SpecField::ContextSoft,
        SpecField::ContextHard,
        SpecField::Command,
        SpecField::Check,
        SpecField::Expect,
    ];

    /// The dotted path of the field in a `ProfileSpec`'s JSON (`"herdr.grid"`), which is
    /// also its serde form.
    pub const fn as_str(self) -> &'static str {
        match self {
            SpecField::Workspace => "herdr.workspace",
            SpecField::Grid => "herdr.grid",
            SpecField::Cwd => "host.cwd",
            SpecField::HarnessKind => "harness.kind",
            SpecField::PortPolicy => "harness.port_policy",
            SpecField::Provider => "model.provider",
            SpecField::ModelId => "model.model_id",
            SpecField::Effort => "model.effort",
            SpecField::Role => "role",
            SpecField::Env => "env",
            SpecField::ContextSoft => "context.soft",
            SpecField::ContextHard => "context.hard",
            SpecField::Command => "command",
            SpecField::Check => "check",
            SpecField::Expect => "expect",
        }
    }

    /// The value of this field in `spec`. `harness.kind` and `role` take their text from
    /// their serde form, so no variant name is written a second time.
    pub fn value(self, spec: &ProfileSpec) -> FieldValue {
        match self {
            SpecField::Workspace => FieldValue::Text(spec.herdr.workspace.clone()),
            SpecField::Grid => FieldValue::Grid(spec.herdr.grid),
            SpecField::Cwd => FieldValue::Text(spec.host.cwd.clone()),
            SpecField::HarnessKind => FieldValue::Text(serde_text(&spec.harness.kind)),
            SpecField::PortPolicy => FieldValue::Text(spec.harness.port_policy.clone()),
            SpecField::Provider => FieldValue::Text(spec.model.provider.clone()),
            SpecField::ModelId => FieldValue::Text(spec.model.model_id.clone()),
            SpecField::Effort => FieldValue::Text(spec.model.effort.clone()),
            SpecField::Role => FieldValue::Text(serde_text(&spec.role)),
            SpecField::Env => {
                FieldValue::List(spec.env.iter().map(|n| n.as_str().to_owned()).collect())
            }
            SpecField::ContextSoft => FieldValue::Number(spec.context.soft),
            SpecField::ContextHard => FieldValue::Number(spec.context.hard),
            SpecField::Command => FieldValue::Argv(spec.command.clone()),
            SpecField::Check => FieldValue::Argv(spec.check.clone()),
            SpecField::Expect => FieldValue::List(spec.expect.clone()),
        }
    }

    /// Whether `spec` and `live`, two values of this field, differ: `env` and `expect` as
    /// sets (an order or a repeat is not a difference), every other field by equality
    /// (an argv in order).
    fn differs(self, spec: &FieldValue, live: &FieldValue) -> bool {
        match (self, spec, live) {
            (SpecField::Env | SpecField::Expect, FieldValue::List(a), FieldValue::List(b)) => {
                as_set(a) != as_set(b)
            }
            _ => spec != live,
        }
    }
}

impl Serialize for SpecField {
    /// The dotted path ([`SpecField::as_str`]).
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
```

- **E20** `crates/holler-pane/src/profile_diff.rs:160-200` — `FieldValue` and its `Display`
```rust
pub enum FieldValue {
    /// `herdr.workspace`, `host.cwd`, `harness.kind`, `harness.port_policy`, the three
    /// `model` fields and `role`.
    Text(String),
    /// `herdr.grid`.
    Grid(GridPos),
    /// `context.soft`, `context.hard`.
    Number(u32),
    /// `env` (the names) and `expect`, in stored order.
    List(Vec<String>),
    /// `command`, `check`; `None` serializes as `null`.
    Argv(Option<Argv>),
}

impl fmt::Display for FieldValue {
    /// The text form:
    ///
    /// - `Text` with every control character escaped by `char::escape_default`, so ESC
    ///   prints as `\u{1b}` and a newline as `\n` (the rule Holler's text log lines
    ///   escape a field value with, in `holler_proto::log`); every other character as is.
    /// - `Grid` as `r2c1`, and `Number` in decimal.
    /// - `List` and `Argv(Some)` as a compact JSON array of strings, `["opencode","serve"]`,
    ///   never a line joined for a shell; `Argv(None)` as `none`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldValue::Text(text) => write_escaped(f, text),
            FieldValue::Grid(grid) => write!(f, "{grid}"),
            FieldValue::Number(number) => write!(f, "{number}"),
            FieldValue::List(items) => write_json_strings(f, items),
            FieldValue::Argv(Some(argv)) => write_json_strings(f, argv.as_slice()),
            FieldValue::Argv(None) => f.write_str("none"),
        }
    }
}

/// `text` with each control character written as its `char::escape_default`.
fn write_escaped(f: &mut fmt::Formatter<'_>, text: &str) -> fmt::Result {
    for c in text.chars() {
        if c.is_control() {
            write!(f, "{}", c.escape_default())?;
        } else {
```

- **E21** `crates/holler-pane/src/profile_snapshot.rs:28-66` — `spec_from_pane` already maps `opencode_agent`
```rust
/// |---|---|
/// | `pane` | `name`, as text |
/// | `herdr.workspace`, `herdr.grid` | `herdr.workspace`, `herdr.grid` (never parsed from the name) |
/// | `host.cwd` | `host.cwd` |
/// | `harness.kind` | `harness.kind` |
/// | `harness.port_policy` | [`fixed_port_policy`] of `harness.port` |
/// | `model`, `role`, `context` | `model`, `role`, `context` |
/// | `opencode_agent` | `opencode_agent` (absent stays absent) |
/// | `env` | `env`: the names, in order |
/// | `command` | `command` |
/// | `check`, `expect` | `probe.check`, `probe.expect` (in order) |
///
/// Not copied, because a spec does not hold them: `generation`, `herdr.session`,
/// `herdr.pane_id`, `host.name`, `host.tmux`, `host.herdr_api_version`, `harness.pid`,
/// `harness.health`, `session_of_record`, `hold`, `last_observed`, `profile` and
/// `probe.last`.
///
/// The port is copied as recorded: a record at port 0 gives `fixed:0`, which #644's
/// `port_of_policy` refuses (`usage`).
pub fn spec_from_pane(pane: &Pane) -> ProfileSpec {
    ProfileSpec {
        pane: pane.name.as_str().to_owned(),
        herdr: SpecHerdr {
            workspace: pane.herdr.workspace.clone(),
            grid: pane.herdr.grid,
        },
        host: SpecHost {
            cwd: pane.host.cwd.clone(),
        },
        harness: SpecHarness {
            kind: pane.harness.kind,
            port_policy: fixed_port_policy(pane.harness.port),
        },
        model: pane.model.clone(),
        opencode_agent: pane.opencode_agent.clone(),
        role: pane.role,
        env: pane.env.clone(),
        context: pane.context,
        command: pane.command.clone(),
```

- **E22** `crates/holler-cli/src/profile/show.rs:134-142` — `profile show` prints every field of `SpecField::ALL`
```rust
/// `spec PANE`, then one `FIELD: VALUE` line per field.
fn push_spec(lines: &mut Vec<String>, spec: &ProfileSpec) {
    lines.push(format!("spec {}", text(&spec.pane)));
    lines.extend(
        SpecField::ALL
            .into_iter()
            .map(|field| format!("  {}: {}", field.as_str(), field.value(spec))),
    );
}
```

- **E23** `crates/holler-pane/tests/profile_diff_test.rs:199-246` — the field table is public API: `PATHS` and the serialize-like-the-spec test
```rust

/// The dotted paths, in `ALL` order: the field table is public API (#664, #665).
const PATHS: [&str; 15] = [
    "herdr.workspace",
    "herdr.grid",
    "host.cwd",
    "harness.kind",
    "harness.port_policy",
    "model.provider",
    "model.model_id",
    "model.effort",
    "role",
    "env",
    "context.soft",
    "context.hard",
    "command",
    "check",
    "expect",
];

#[test]
fn spec_field_values_serialize_like_the_spec() {
    let got: Vec<&str> = SpecField::ALL.iter().map(|f| f.as_str()).collect();
    assert_eq!(got, PATHS);

    let mut bare = spec();
    bare.command = None;
    bare.check = None;
    bare.role = PaneRole::Orchestrator;
    for one in [spec(), bare] {
        let whole = serde_json::to_value(&one).unwrap();
        for field in SpecField::ALL {
            assert_eq!(
                serde_json::to_value(field).unwrap(),
                Value::String(field.as_str().to_owned()),
                "{field:?} serializes as its path"
            );
            let pointer = format!("/{}", field.as_str().replace('.', "/"));
            let want = whole.pointer(&pointer).cloned().unwrap_or(Value::Null);
            assert_eq!(
                serde_json::to_value(field.value(&one)).unwrap(),
                want,
                "{field:?} at {pointer}"
            );
        }
    }
}
```

- **E24** `crates/holler-cli/tests/pane_verbs/launch/serve.rs:165-265` — #802's verb tests: the in-flight agent reaches `serve_command`, on launch and on a relaunch with a different `--agent`
```rust
/// AC 3 and AC 4: the in-flight spec's agent reaches `serve_command`; the act ensures the
/// session, runs the spec's command, then exactly the argv the plan built, both in the
/// spec's directory, waits for the server's health, verifies the spec, and only then creates
/// the session of record and attaches the TUI.
#[test]
fn the_act_runs_the_planned_argv_after_the_command_then_waits_and_verifies() {
    let rig = Rig::new();
    let (at_start, at_verify) = (Arc::new(Mutex::new(None)), Arc::new(Mutex::new(None)));
    rig.after_server_start(snapshot_into(&at_start));
    rig.after(HarnessOp::VerifyServing, snapshot_into(&at_verify));
    let extra = [
        "--agent",
        AGENT,
        "--command-arg",
        "sleep",
        "--command-arg",
        "600",
    ];
    let run = rig.run(&launch_with(&extra), Format::Json);
    data_of(&run);

    let start = taken(&at_start, "the server's start");
    let host = [HostOp::EnsureSession, HostOp::Run, HostOp::Run];
    assert_eq!(
        start.host, host,
        "the session, the command, then the server"
    );
    assert_eq!(
        steps(&start.harness),
        [HarnessOp::ServeCommand, HarnessOp::Health],
        "nothing after the plan's reads (and the port's re-check) yet"
    );
    let verify = taken(&at_verify, "verify_serving");
    assert!(
        count(&verify.harness, HarnessOp::Health) > count(&start.harness, HarnessOp::Health),
        "the health wait ran between the start and the verify: {verify:?}"
    );
    assert_eq!(
        steps(&verify.harness),
        [
            HarnessOp::ServeCommand,
            HarnessOp::Health,
            HarnessOp::VerifyServing
        ]
    );
    let all = [
        HarnessOp::ServeCommand,
        HarnessOp::Health,
        HarnessOp::VerifyServing,
        HarnessOp::CreateSession,
        HarnessOp::AttachTui,
        HarnessOp::ShownSession,
    ];
    assert_eq!(steps(&run.calls.harness), all);

    let spec = with_agent(AGENT);
    assert_eq!(rig.fakes.harness.serve_specs(), std::slice::from_ref(&spec));
    let runs = rig.fakes.host.runs();
    let planned = rig
        .fakes
        .harness
        .serve_command(&name(PANE), PORT, &spec)
        .unwrap();
    let expected = vec![
        (name(PANE), "/srv/demo".to_owned(), argv(&["sleep", "600"])),
        (name(PANE), "/srv/demo".to_owned(), planned),
    ];
    assert_eq!(
        runs, expected,
        "exactly the planned argv, in the spec's cwd"
    );
    let server = rig.fakes.harness.server(PORT).unwrap();
    assert_eq!(server.agent, Some(agent(AGENT)), "{server:?}");
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 3: a relaunch passes the in-flight spec's agent: a `--agent` that differs from the
/// recorded one reaches `serve_command` (the fake recorded the new key), and without the flag
/// the recorded key does.
#[test]
fn relaunch_passes_the_in_flight_agent_to_serve_command() {
    let rig = Rig::new();
    let mut theirs = rig.live(false);
    theirs.opencode_agent = Some(agent("orchestrator"));
    rig.fakes.panes.concurrent_put(&theirs).unwrap();

    let run = rig.run(&["pane", "relaunch", PANE, "--agent", AGENT], Format::Json);
    data_of(&run);
    let asked = rig.fakes.harness.serve_specs();
    assert_eq!(asked.last(), Some(&with_agent(AGENT)), "{asked:?}");
    let server = rig.fakes.harness.server(PORT).unwrap();
    assert_eq!(server.agent, Some(agent(AGENT)));
    rig.assert_matches(&run, PANE);

    let run = rig.run(&["pane", "relaunch", PANE], Format::Json);
    data_of(&run);
    let asked = rig.fakes.harness.serve_specs();
    assert_eq!(asked.last(), Some(&with_agent(AGENT)), "the stored key");
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
```

- **E25** `crates/holler-cli/tests/pane_verbs/launch/serve.rs:325-360` — #802's verb test: a `verify_serving` refusal rolls back and stops the server
```rust
    );
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 7: a `verify_serving` refusal after a healthy start goes through the rollback: the
/// session's processes are stopped (the server among them), the Herdr pane the run made is
/// closed, no session of record is created and nothing is recorded; the act had begun, so the
/// reconcile step is printed once.
#[test]
fn a_verify_refusal_rolls_back_and_stops_the_server() {
    let rig = Rig::new();
    let refusal = PaneError::Refused {
        code: AGENT_UNSUPPORTED,
        message: "the running server does not apply the agent".to_owned(),
    };
    rig.fakes
        .harness
        .faults()
        .fail_next(HarnessOp::VerifyServing, refusal);
    let run = rig.run(&launch_with(&["--agent", AGENT]), Format::Json);
    assert_eq!(run.code, 3, "{run:?}");
    let (code, message) = error_of(&run);
    assert_eq!(code, "agent-unsupported");
    assert_eq!(occurrences(&message, STEP_PHRASE), 1, "{message}");
    assert!(message.contains(STEP), "{message}");
    assert!(
        run.calls.host.contains(&HostOp::StopOwned),
        "{:?}",
        run.calls.host
    );
    assert!(!run.calls.harness.contains(&HarnessOp::CreateSession));
    assert!(
        run.calls.herdr.contains(&HerdrOp::Close),
        "{:?}",
        run.calls.herdr
```

- **E26** `crates/holler-cli/tests/pane_verbs/relaunch.rs:530-596` — #700's tests: an invalid key is refused first; relaunch without `--agent` keeps the stored key; with `--agent` replaces it
```rust
#[test]
fn pane_relaunch_accepts_every_spec_flag() {
    assert_spec_flags_accepted("relaunch");
}

/// #700 AC 4: an invalid `--agent` key is refused before anything else runs — no port
/// call, no record write — with `agent-key-invalid`, which does not echo the key.
#[test]
fn relaunch_refuses_an_invalid_agent_key_before_anything() {
    let rig = Rig::new();
    rig.live(false);
    let run = rig.run(&relaunch_with(&["--agent", "a=b"]), Format::Json);
    assert_eq!(run.code, 3, "{run:?}");
    let (code, message) = error_of(&run);
    assert_eq!(code, "agent-key-invalid", "{message}");
    assert!(!message.contains("a=b"), "never echoes the key: {message}");
    assert_untouched(&run.calls);
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// #700 AC 5 (A's DECISION 1, the relaunch landmine): without the flag a relaunch keeps
/// the stored key — its base spec is `spec_from_pane` of the record, which copies the
/// key, and the record literal writes the spec's key back rather than resetting it.
#[test]
fn relaunch_without_agent_keeps_the_stored_key() {
    let rig = Rig::new();
    let mut theirs = rig.live(false);
    theirs.opencode_agent = Some(agent("orchestrator"));
    rig.fakes.panes.concurrent_put(&theirs).unwrap();
    let run = rig.run(RELAUNCH, Format::Json);
    data_of(&run);
    assert_eq!(
        rig.record(PANE)
            .unwrap()
            .opencode_agent
            .as_ref()
            .map(|key| key.as_str()),
        Some("orchestrator")
    );
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// #700 AC 5: given, the flag replaces the stored key, like its siblings.
#[test]
fn relaunch_with_agent_replaces_the_stored_key() {
    let rig = Rig::new();
    let mut theirs = rig.live(false);
    theirs.opencode_agent = Some(agent("orchestrator"));
    rig.fakes.panes.concurrent_put(&theirs).unwrap();
    let run = rig.run(
        &relaunch_with(&["--agent", "feature-implementor"]),
        Format::Json,
    );
    data_of(&run);
    assert_eq!(
        rig.record(PANE)
            .unwrap()
            .opencode_agent
            .as_ref()
            .map(|key| key.as_str()),
        Some("feature-implementor")
    );
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}
```

