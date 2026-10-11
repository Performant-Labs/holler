# [feat(adapter-opencode): the OpenCode adapter (serve, health, create and switch sessions, abort, attach a TUI, report the shown session)](https://github.com/Performant-Labs/holler/issues/642)

Part of epic #633. **Parts 1 and 2 are merged** (#705, bd5e825); #802 merges before this part starts. This text is the complete statement of **part 3** (one run) and replaces every earlier version of this issue body; where an older comment disagrees, this text decides. Consolidated 2026-10-11 from the earlier amendments and the operator's decisions of the same day.

## Scope (final)

Implement the rest of `HarnessPort` for OpenCode in `crates/holler-adapter-opencode`, so the pane's OpenCode agent (`Pane.opencode_agent`, an `AgentKey`; none means the server's own default) is **applied and verified**, and the two methods #645 part 2 left as defaults become real:

1. **`serve_command` applies the agent.** With `spec.agent == Some(key)` the argv it returns starts the server with `default_agent` set to `key`; with `None` it returns exactly the argv it returns today. It no longer refuses an agent with `agent-unsupported`.
2. **`verify_serving` checks the running server.** It returns `Ok(())` only when the server really runs the agent the spec asks for, and refuses with the adapter's own open code `agent-unusable` otherwise. The launch engine (#802) calls it after the health wait and before `create_session`, and its rollback then stops the server (a process of the pane's tmux session).
3. **`send_prompt`** queues a prompt through OpenCode's HTTP API, never by typing into a TUI (I4).
4. **`session_activity`** reports `Idle`, `Busy` or `HoldingQuestion` from the server's live state.
5. The docs that say "not applied yet" are corrected (crate docs, ADR-0021, the stale doc sentences on `AGENT_UNSUPPORTED`, the real-test rig's docs, `CHANGELOG.md`).

The spike (#635) facts the adapter relies on stay as they were (`docs/research/opencode-pane-spike.md`); the conformance suite of #638/#684 must still pass for `OpenCodeHarness`.

## Operator decisions (2026-10-11)

Each is final; the reason follows it.

- **D1. The agent is applied as the server's `default_agent`, once, when the server starts. Not per prompt.** Hub-delivered messages never pass through this adapter: they go hub `send_prompt` -> `session/prompt` -> `holler-body` -> `prompt_async` on that pane's own server. Per prompt would need a protocol field, a hub change and a `holler-body` change.
- **D2. The mechanism is the environment variable `OPENCODE_CONFIG_CONTENT`, set in the server's argv `env` prefix, value `{"default_agent":"<KEY>"}`** (compact JSON built with `serde_json::json!`; no other key, never an agent's definition: tmux refuses a command around 20,000 bytes, E22). `opencode serve` has no agent option and a config file would bring in persistence rules. **Placement and precedence:** the assignment is appended to the prefix after the prefix's own words (`env -u OPENCODE_DISABLE_TERMINAL_TITLE OPENCODE_CONFIG_CONTENT=... <bin> serve ...`, or `env -i K=V ... OPENCODE_CONFIG_CONTENT=... <bin> serve ...`), and `env` applies assignments in order, so the adapter's value replaces both a same-named pair configured in `ProcessEnv::Isolated` and a value inherited from the tmux server's environment. With `None` the adapter sets nothing. **One helper:** extend `ProcessEnv::prefix` with `ProcessEnv::prefix_with(&self, extra: &[(String, String)]) -> Vec<String>` (`prefix()` calls it with no extra pairs, so `tui_argv` and the TUI's argv are unchanged); `serve_command` is its only caller with a pair. Reason: one prefix builder, no second one.
- **D3. The variable was probed live on 2026-10-11 (E21): OpenCode 1.19.0 started with `OPENCODE_CONFIG_CONTENT='{"default_agent":"plan"}'` reports `"default_agent":"plan"` at `GET /config`, and a prompt sent to it produced messages whose `agent` is `plan`.** The spike's version was 1.18.35, so the real test (acceptance 11) proves it again against whatever `opencode` is installed. **If the variable does not reach the server, stop and escalate; do not switch to a config file or to a per-prompt agent.**
- **D4. `verify_serving` asks the running server, in this order, within `Timeouts.call` and with the adapter's one request bound (`Timeouts.request`):** with `spec.agent == None` it returns `Ok(())` and sends **nothing**. With `Some(key)`: (a) `GET /config` must answer a JSON object whose `default_agent` equals `key`: this proves the variable reached the server (if it did not, the pane would run as `build`, which cannot dispatch subagents: the incident this epic exists to prevent); (b) `GET /agent` must answer a JSON array, which is the agent set (the decision: the adapter learns the set by asking the server, not by reading any file), in which an element has `name == key`, `mode == "primary"` and `hidden` not `true`. Anything else is a refusal `agent-unusable` whose one-line message names the key and the reason: `the server's default agent is not <key>`, `no agent named <key>`, `agent <key> is a subagent (mode <mode>)`, `agent <key> is hidden`. The key is a name and never a secret (I7), so the message may carry it; it carries no body of the reply. A `200` that is not the JSON the step needs (OpenCode answers an unserved route with its web app's HTML page and a 200) is `unavailable`, in the adapter's existing one-line form (E11, `Call::unexpected`); a refused connection is `unavailable`, no answer in time is `timeout` with `op` `harness.verify_serving`.
- **D5. `agent-unusable` is an open refusal code (exit 3), a constant in the adapter's own file** (`pub const AGENT_UNUSABLE: RefusalCode = RefusalCode::from_static("agent-unusable")` in `crates/holler-adapter-opencode/src/server.rs`, built through `PaneError::Refused`, the way the Herdr adapter owns `grid-unreachable`). It is distinct from `AGENT_UNSUPPORTED`, which stays in `holler-pane` for a harness that cannot apply an agent at all. The ADR-0021 code tables list it wherever they list `agent-unsupported`. Reason: "this harness cannot" and "this agent cannot run here" are different refusals for the operator.
- **D6. `send_prompt(port, session, text)`** first asks `GET /session/:id` the way `abort` does (`known`): a 404 or 400 is `session-not-found` and nothing is posted. Then it sends `POST /session/:id/prompt_async` with the body `{"parts":[{"type":"text","text":TEXT}]}` and **no `agent` field**, the exact body and the bare route `holler-body` uses (E15, E16). **`Ok(())` exactly when the reply is `204`**; any other status or an HTML `200` is `unavailable` in the adapter's one-line form, and no message ever contains the prompt text. The `/api/session/:id/prompt_async` form is not a route and is never used. Reason: `prompt_async` answers 204 at once and the turn runs on; the agent was verified at start (D4), so a second check per prompt would add latency for nothing.
- **D7. `session_activity(port, session)`:** (a) `GET /question` and `GET /permission` (global lists, E17); an item whose `sessionID` is `session` means the turn is waiting on a human: **`HoldingQuestion`** (checked first; a permission request counts, because the turn waits for an answer just as for a question). (b) Otherwise `GET /session/status` (E12, E14): when it answers a JSON object, the session is **`Busy`** if the object has an entry for `session` whose `type` is not `idle`, else **`Idle`** (absence means idle: spike line 73, E13); the status route is authoritative whenever it answers an object. (c) Only when `/session/status` does not answer an object (older server, HTML page) does the adapter fall back to `GET /session/:id/message`: the last message whose `info.role` is `assistant` with no `info.time.completed` is **`Busy`**, otherwise **`Idle`**. If the fallback is not usable either, the call is `unavailable`. Reason: the status route is the server's own account of a running turn; the message list is only the fallback the operator asked for.
- **D8. Where the code goes (file size).** `crates/holler-adapter-opencode/src/lib.rs` is 599 lines and the lint fails at 900, `tests/hermetic_test.rs` is 813: the agent code (the `OPENCODE_CONFIG_CONTENT` pair, `serve_command`, `verify_serving`, `AGENT_UNUSABLE`) goes in `src/server.rs` (76 lines), `send_prompt` and `session_activity` go in a **new `src/prompt.rs`**, and `lib.rs` only wires the three trait methods and gets its doc comment corrected. `refuse_an_agent` and the `AGENT_UNSUPPORTED` import are deleted from the adapter (the constant stays in `holler-pane`).
- **D9. Stale words.** Comment-only edits (no code) are allowed in `crates/holler-pane/src/ports.rs` (the `serve_command`/`verify_serving` docs and the `AGENT_UNSUPPORTED` doc, which say the OpenCode adapter refuses every agent "until #642 part 3") and in the module doc of `crates/holler-pane/src/lib.rs` where it says the same. Reason: after this part they would lie.
- **D10. A pane's body posts to that pane's own server** (each pane has its own port, ADR-0021 section 2 fact 4), so a `default_agent` set on one server never affects another pane.
- **D11. The real-OpenCode rig stays hermetic about models.** A prompt is sent only by the agent test, to the rig's dead-end provider (127.0.0.1:9 refuses), so no model is reached; the assistant message may carry an `error`, which the test tolerates. The rig's documentation ("No prompt is ever sent") is rewritten to say so.

## Acceptance (tests first)

Every test is named in a file below. "Stub" is the hermetic HTTP stub of `tests/support/stub.rs` (as `hermetic_test.rs` uses it). Existing tests that pin the OpenCode adapter's `agent-unsupported` refusal (`tests/hermetic_test.rs` near line 647, `tests/real_opencode_test.rs` near line 333) are changed in place to the new behaviour. **RED note for T:** these tests call methods that exist, so RED is an assertion failure (`serve_command` with an agent still answers `agent-unsupported`, `send_prompt` still answers `prompt-unsupported`), not a compile error.

1. **`tests/agent_test.rs` (new, hermetic):** `serve_command` with `Some(plan)` under `ProcessEnv::Inherit` returns exactly `env -u OPENCODE_DISABLE_TERMINAL_TITLE OPENCODE_CONFIG_CONTENT={"default_agent":"plan"} <bin> serve --port P --hostname 127.0.0.1 <serve_args>` (pinned as a whole argv); under `ProcessEnv::Isolated` the pair comes after the configured pairs; with a configured `OPENCODE_CONFIG_CONTENT=x` pair in `Isolated` the adapter's assignment is the later one; `Some(key)` for several valid keys (`plan`, `orchestrator`, `a-b_c`) carries the key in exactly one element.
2. **`tests/agent_test.rs`:** `serve_command` with `None` returns the same argv as before this part, byte for byte (the pin of the previous behaviour), and sends nothing (the stub saw no request).
3. **`tests/agent_test.rs`:** `verify_serving` with `None` returns `Ok(())` and the stub saw **no** request.
4. **`tests/agent_test.rs` (Stub):** with `Some(plan)` and the stub answering `GET /config` `{"default_agent":"plan"}` and `GET /agent` `[{"name":"plan","mode":"primary","native":true,"hidden":null}, ...]`: `Ok(())`, and exactly those two requests were made.
5. **`tests/agent_test.rs` (Stub):** each of these is `Refused` with code `agent-unusable` and a one-line message naming the key and the reason of D4: `/config` default is another agent; the agent is absent from `/agent`; its `mode` is `subagent`; its `hidden` is `true`. No reply body appears in any message.
6. **`tests/agent_test.rs` (Stub):** `/config` or `/agent` answering the web app's HTML with a 200 is `unavailable` (one line naming the route and port); a refused port is `unavailable`; a server that accepts and never answers is `timeout` with `op` `harness.verify_serving`, within `Timeouts.call` plus the suite's usual slack.
7. **`tests/prompt_test.rs` (new, Stub):** `send_prompt` posts to `/session/:id/prompt_async` exactly `{"parts":[{"type":"text","text":TEXT}]}` (no `agent` key, bare route) and returns `Ok(())` on `204`; a 404 on `GET /session/:id` is `session-not-found` and nothing was posted; a `200` HTML answer to the post is `unavailable`; a `500` is `unavailable`; the prompt text appears in no error message (a distinctive text is searched for).
8. **`tests/prompt_test.rs` (Stub):** `session_activity` is `HoldingQuestion` for a `/question` item with this session's `sessionID` and for a `/permission` item with it; an item for another session does not count; `Busy` for `/session/status` `{"<id>":{"type":"busy"}}`; `Idle` for `{}`; `Idle` for an entry of type `idle`; with `/session/status` answering HTML, the message-list fallback gives `Busy` for a last assistant message without `time.completed` and `Idle` for a completed one; with the fallback unusable too, `unavailable`.
9. **`tests/hermetic_test.rs` (changed in place):** the test near line 647 now asserts the new `serve_command`/`verify_serving` behaviour for an agent instead of `agent-unsupported`; no test file passes 900 lines.
10. **Conformance:** `OpenCodeHarness` passes `run_harness_conformance` from `holler-pane-testkit` (including #802's agent case, which now takes the "argv carries the key" branch), through the launcher of `tests/real_opencode/rig.rs` for the opt-in run, and `tests/hermetic_test.rs`' existing conformance use for the stub run.
11. **`tests/real_opencode_test.rs` (opt-in `#[ignore]`, `HOLLER_TEST_OPENCODE=1`, no model call):** `agent_default_reaches_the_first_message`: start one server through `serve_command` with `plan` and one with `None` (the rig's launcher), `verify_serving` is `Ok` for the first; send a **raw** `POST /session/:id/prompt_async` whose body is exactly `{"parts":[{"type":"text","text":"hello"}]}` (the rig's raw helper, not the adapter) and assert the first assistant message's `info.agent` and `info.mode` are `plan` for the first server and the server's default for the second (tolerate its `error`); a second test asserts `verify_serving` refuses, `agent-unusable`, `explore` (a subagent), `title` (hidden) and an unknown key on real servers started with them; a third that `send_prompt` answers `Ok` and `session_activity` reads `Idle` once the turn has ended (poll within `Timeouts.settle`). The rig keeps its dead-end provider check after every server it starts.
12. **Docs:** the crate doc comment in `src/lib.rs` (the "agent ... until then ... refuse" sentences and the "No per-pane environment" decision) says the agent is applied through `OPENCODE_CONFIG_CONTENT`, verified by `verify_serving`, and that `send_prompt` and `session_activity` are real; the rig's header ("No prompt is ever sent", `rig.rs` lines 7-9) and `tests/real_opencode_test.rs` lines 10-12 say a prompt is sent only by the agent test; ADR-0021 section 2 (the "harness server as amended" decision 5, "HarnessPort as built" fact 7), section 8 ("The prompt and activity seam": the adapter now replaces both defaults) and the Deferred bullets ("Applying `Pane.opencode_agent`...", "The refusal of a pane that is not idle...") are updated, and `agent-unusable` is listed where the code tables list `agent-unsupported`; `CHANGELOG.md` has an `## [Unreleased]` entry linking this issue.

## Blast radius

- `crates/holler-adapter-opencode/**` (source and tests; new files `src/prompt.rs`, `tests/agent_test.rs`, `tests/prompt_test.rs`; the declared `[[test]]` entries in its `Cargo.toml` if the crate sets `autotests = false`).
- `docs/adr/ADR-0021.md`, `CHANGELOG.md`.
- Comment-only edits in `crates/holler-pane/src/ports.rs` and `crates/holler-pane/src/lib.rs` (D9). No other crate, no hub, protocol or `holler-body` change.

## Out of scope

`--agent` wiring on `pane launch` and `relaunch` and the spec-level refusal of an undefined key (#644 part 2); the wiring of this adapter into the CLI and the provider credential reaching the server through the pane's tmux session (#649); stopping the server (#802 did: `HostPort::stop_owned`); model, effort, environment-variable names and context ceilings (they join `ServeSpec` later).

## Depends on

#802 **merged** (it replaces `serve` with `serve_command`, adds `verify_serving`, `ServeSpec` and `HostPort::run`'s `cwd`), #700 (`Pane.opencode_agent`, `AgentKey`, merged), #637, #638, #635, #684. Blocks #644 part 2's real end-to-end use and #649.

## Dependency note

#802 is not merged at the time of writing (PR pending, tree `3fe111d`). The signatures and file:line quotes below (E1 to E10) are the **pre-merge reading of that tree**; this story is built after #802 merges, and if the merged code differs from the quote, the merged code wins and the brief is patched before the run builds.

## Pipeline

`rigor: second-opinion`. Fresh worktree, branch `issue-642-implementation` off `origin/main` after #802 merged.

## Rules for this story

See the epic (#633): the contract section there is fixed. Issue first, tests first, Holler's pipeline and hooks apply (branch `issue-<N>-implementation`, fresh worktree, Conventional Commits, `cargo fmt`, clippy clean, no new `unsafe`, no `unwrap`/`expect`/`panic` outside tests). Edit only the files in the Blast radius. No story touches a live fleet, a running pane or a real Herdr session; verification is the test kit's fakes, the hermetic stub, and the opt-in scratch `opencode serve` with its own `HOME` and ports 48100-48199. Never print or commit a credential; no personal hostname, path or account name in code, tests or docs (use placeholders).

## AI assistance

This issue was drafted by Claude (Sonnet 5.5) from the operator's decisions in session; the maintainer filing it is accountable for it, per `CONTRIBUTING.md`.

## Evidence (verbatim)

Every excerpt is cut by `sed -n A,Bp` from the file at the stated line numbers of the #802 tree (`3fe111d`, pre-merge) or of the repository, never retyped. E21 is the output of commands run against a scratch `opencode serve`.

### E1. ServeSpec (the spec serve_command and verify_serving take)
`crates/holler-pane/src/ports.rs:99-133`

~~~~
    pub grid: GridPos,
}

/// What a pane's harness server must run with: the part of the pane's spec that
/// [`HarnessPort::serve_command`] applies to the server's argv and
/// [`HarnessPort::verify_serving`] checks on the running server (#802).
///
/// `#[non_exhaustive]`: the model, the effort, the environment names and the context
/// ceilings join it later with no change to a signature. A new field of the pane's spec
/// for its server goes here, never into a new parameter. Outside this crate a spec is
/// built only from [`ServeSpec::default`] and its builders, so a struct literal does not
/// compile there, with or without `..Default::default()`; the launch engine, inside this
/// crate, builds it with every field named.
///
/// ```compile_fail,E0639
/// let spec = holler_pane::ServeSpec { agent: None };
/// ```
/// ```compile_fail,E0639
/// let spec = holler_pane::ServeSpec { agent: None, ..Default::default() };
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ServeSpec {
    /// The OpenCode agent the server runs as its default (`Pane.opencode_agent`); `None`
    /// leaves the server's own default.
    pub agent: Option<AgentKey>,
}

impl ServeSpec {
    /// This spec, with `agent` as the server's agent.
    #[must_use]
    pub fn with_agent(mut self, agent: AgentKey) -> Self {
        self.agent = Some(agent);
        self
    }
~~~~

### E2. HostPort::run with cwd, and stop_owned
`crates/holler-pane/src/ports.rs:196-216`

~~~~
/// The machine a pane lives on: tmux sessions and the processes in them (the adapter
/// is `holler-adapter-host`, #641).
///
/// **Blocking.** Every method is synchronous. Call from `spawn_blocking` (or a
/// thread) in async code. Every method returns within I5's bound (default 10 s) or
/// with [`PaneError::Timeout`]. An implementation is `Send + Sync`.
pub trait HostPort: Send + Sync {
    /// Make the tmux session `name` exist, working in `cwd`.
    fn ensure_session(&self, name: &PaneName, cwd: &str) -> Result<(), PaneError>;

    /// Run `argv` (never through a shell) in the session `name`, starting it in `cwd`,
    /// which need not be the directory the session was made in (#802: the launch engine
    /// passes the pane spec's directory, for the spec's command and for the harness
    /// server alike).
    fn run(&self, name: &PaneName, cwd: &str, argv: &Argv) -> Result<(), PaneError>;

    /// Stop the processes the session `name` owns: what `run` started there, the pane's
    /// harness server among them (#802).
    fn stop_owned(&self, name: &PaneName) -> Result<(), PaneError>;

    /// The process ids running in the session `name`.
~~~~

### E3. HarnessPort: serve_command and verify_serving (#802)
`crates/holler-pane/src/ports.rs:220-258`

~~~~
/// The harness server of a pane and its sessions (the adapter is
/// `holler-adapter-opencode`, #642). **Final:** the spike #635 confirmed it unchanged;
/// ADR-0021 section 2, "`HarnessPort` as built (#642)", records what the adapter decided,
/// and #802 amended how a server starts: the port builds its argv, and [`HostPort::run`]
/// starts it in the pane's tmux session, so [`HostPort::stop_owned`] stops it.
///
/// **Blocking.** Every method is synchronous. Call from `spawn_blocking` (or a
/// thread) in async code. Every method returns within I5's bound (default 10 s) or
/// with [`PaneError::Timeout`]. An implementation is `Send + Sync`.
pub trait HarnessPort: Send + Sync {
    /// The argv that starts the harness server of `name` on `port` with `spec` applied,
    /// for [`HostPort::run`] to start. Pure: it starts no process, binds no port, writes
    /// no file and sends nothing.
    ///
    /// It never returns an argv that would start a server without what `spec` asks for.
    /// When `spec.agent` is `Some(key)`, some element of the argv contains the key's
    /// text; an implementation that cannot apply the agent refuses with
    /// [`AGENT_UNSUPPORTED`].
    fn serve_command(
        &self,
        name: &PaneName,
        port: u16,
        spec: &ServeSpec,
    ) -> Result<Argv, PaneError>;

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

    /// Whether the server on `port` answers.
~~~~

### E4. HarnessPort: the #645 defaults send_prompt and session_activity, AGENT_UNSUPPORTED, Activity
`crates/holler-pane/src/ports.rs:278-323`

~~~~

    /// Queue `text` as a prompt to `session` on the server at `port`, through the
    /// harness API, never by typing into a TUI (I4). `pane reset --first` calls it (#645).
    ///
    /// The default refuses with [`PROMPT_UNSUPPORTED`]: an impl that does not send
    /// prompts says so as a stable refusal, which the real adapter replaces (#642).
    fn send_prompt(&self, port: u16, session: &str, text: &str) -> Result<(), PaneError> {
        let _ = (port, session, text);
        Err(PaneError::Refused {
            code: PROMPT_UNSUPPORTED,
            message: "this harness does not queue a prompt".to_owned(),
        })
    }

    /// What `session` on the server at `port` is doing now (#645: `pane reset` refuses a
    /// conversation that is not [`Activity::Idle`]).
    ///
    /// The default answers [`Activity::Idle`], so an impl that does not report activity
    /// refuses nothing.
    fn session_activity(&self, port: u16, session: &str) -> Result<Activity, PaneError> {
        let _ = (port, session);
        Ok(Activity::Idle)
    }
}

/// `prompt-unsupported`: the default of [`HarnessPort::send_prompt`], for a harness that
/// does not queue a prompt. Refusal, exit 3.
pub const PROMPT_UNSUPPORTED: RefusalCode = RefusalCode::from_static("prompt-unsupported");

/// `agent-unsupported`: [`HarnessPort::serve_command`] or [`HarnessPort::verify_serving`]
/// of a harness that cannot apply the agent a [`ServeSpec`] asks for (#802; the OpenCode
/// adapter answers it for every agent until #642 part 3). Refusal, exit 3: the harness
/// understood the request and declines it; nothing went wrong.
pub const AGENT_UNSUPPORTED: RefusalCode = RefusalCode::from_static("agent-unsupported");

/// What a harness session is doing, as [`HarnessPort::session_activity`] reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activity {
    /// Waiting for a prompt.
    Idle,
    /// Running a turn.
    Busy,
    /// Waiting for an answer to a question it asked.
    HoldingQuestion,
}

~~~~

### E5. AgentKey: what a key may contain
`crates/holler-pane/src/argv.rs:129-160`

~~~~
/// default agent.
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

~~~~

### E6. server.rs as of #802: serve_command, refuse_an_agent, healthy
`crates/holler-adapter-opencode/src/server.rs:1-76`

~~~~
//! A pane's OpenCode server: the argv that starts it (`serve_command`), whether the running
//! one honours its spec (`verify_serving`), and whether a server is healthy (`health`).
//!
//! The adapter starts no server (#802). The host adapter's `run` starts the argv in the
//! pane's tmux session, in the pane spec's directory, and its `stop_owned` stops it; the
//! launch engine waits for its health. So nothing here spawns, waits on or signals a
//! process.

use std::time::Duration;

use holler_pane::ports::AGENT_UNSUPPORTED;
use holler_pane::{Argv, PaneError, ServeSpec};
use serde_json::Value;

use crate::http::{self, Reply};
use crate::{json_of, OpenCodeConfig};

/// The route a health check asks.
const HEALTH: &str = "/global/health";

/// `HarnessPort::serve_command`: `<env prefix> <opencode_bin> serve --port P --hostname
/// 127.0.0.1 <serve_args>`, behind the `env` prefix the TUI's argv also starts with
/// ([`crate::ProcessEnv::prefix`]). The server listens on the loopback address only, and
/// the pane's name is not on the argv: the host runs it in the pane's own tmux session and
/// directory. Pure: it reads the configuration and nothing else. A spec with an agent is
/// refused ([`refuse_an_agent`]), so no argv starts a server without the agent asked for.
pub(crate) fn serve_command(
    config: &OpenCodeConfig,
    port: u16,
    spec: &ServeSpec,
) -> Result<Argv, PaneError> {
    refuse_an_agent(spec)?;
    let mut argv = config.env.prefix();
    argv.extend([
        config.bin()?.to_owned(),
        "serve".to_owned(),
        "--port".to_owned(),
        port.to_string(),
        "--hostname".to_owned(),
        "127.0.0.1".to_owned(),
    ]);
    argv.extend(config.serve_args.iter().cloned());
    Ok(Argv::new(argv))
}

/// `agent-unsupported` for a spec that asks for an agent, and `Ok` for one that does not,
/// with nothing sent. Until #642 part 3 the adapter applies no agent, so no argv it builds
/// could carry one (`serve_command`), and it cannot check a running server for one:
/// `verify_serving` answers this too, and fails closed.
pub(crate) fn refuse_an_agent(spec: &ServeSpec) -> Result<(), PaneError> {
    match spec.agent {
        None => Ok(()),
        Some(_) => Err(PaneError::Refused {
            code: AGENT_UNSUPPORTED,
            message: "the OpenCode adapter does not apply an agent yet (#642)".to_owned(),
        }),
    }
}

/// Whether the server on `port` answers `GET /global/health` with `{"healthy": true}`
/// within `within`. Refused, timed out, garbled or anything else is `false`.
pub(crate) fn healthy(port: u16, within: Duration) -> bool {
    matches!(
        http::request(port, "GET", HEALTH, None, within),
        Ok(reply) if is_healthy(&reply)
    )
}

/// A `200` whose JSON is an object with `"healthy": true`.
fn is_healthy(reply: &Reply) -> bool {
    json_of(reply)
        .as_ref()
        .and_then(|health| health.get("healthy"))
        .and_then(Value::as_bool)
        == Some(true)
}
~~~~

### E7. lib.rs: header docs that name the agent and the order of work
`crates/holler-adapter-opencode/src/lib.rs:8-16`

~~~~
//! **Every method of the port is built** (#642): the server side (`health`,
//! `create_session`, `list_sessions` and `abort`, part 1; `serve_command` and
//! `verify_serving`, #802) and the TUI side (`attach_tui`, `select_session` and
//! `shown_session`, part 2), whose TUI runs in the pane's tmux session, as the server does.
//! The last part of #642 applies the pane's OpenCode agent (`Pane.opencode_agent`, after
//! #700); until then `serve_command` and `verify_serving` refuse a spec that asks for one
//! (`agent-unsupported`). Nothing wires the adapter in yet (#649).
//!
//! **Blocking**, as the port is: every method is synchronous (call it from `spawn_blocking`
~~~~

### E8. lib.rs: the 'no per-pane environment' decision and the port facts to update
`crates/holler-adapter-opencode/src/lib.rs:62-96`

~~~~
//!   `#{pane_title}` of the pane's tmux session, addressed by its exact name as `=<name>:`,
//!   which tmux resolves to the active pane of the session's current window. That pane is the
//!   TUI's: the session's current window must be the TUI's. Since #802 every pane's session
//!   also holds its server's window, which the host's `run` creates detached; a client that
//!   switches the session to that window would move `attach_tui` and `shown_session` onto the
//!   server's pane. `shown_session` is `None` whenever it cannot tell (fact 3): the home
//!   screen, a deleted session, a title that is not a whole id, a dead pane, another program,
//!   no tmux session or server. It never guesses and never asks the server.
//! - **A TUI's port is observed, not remembered**: `select_session` reads it from the
//!   pane's `#{pane_start_command}`, so a pane with no TUI is `unavailable` before anything
//!   is sent. `attach_tui` keeps the pane when its program exits (`remain-on-exit on`) and
//!   replaces that program with the TUI. Both answer `Ok` only once the title shows the
//!   session; the TUI is never sent a keystroke.
//! - **One TUI per server is the caller's precondition** (fact 4): `select-session` reaches
//!   every TUI of a server, and the registry gives each pane its own port.
//! - **`list_sessions` lists top-level sessions only** (fact 5). A child (subagent)
//!   session, one whose `parentID` is a string, is left out, because reconcile (#647) treats
//!   every listed session except the session of record as a stray.
//! - **The lookup is wiring's job** (fact 6, see [`Resolver`]). `tui_session` must answer
//!   for a pane that is being launched, before the pane's record exists, so #649 cannot back
//!   it with a registry lookup alone. The server's directory is no lookup (#802): it comes
//!   from the pane spec, through the host's `run`.
//! - **One divergence from the test kit's `FakeHarness`** (fact 6), which the conformance
//!   suite pins neither way: a `PaneId` the `tui_session` lookup does not know is the
//!   lookup's own error (`pane-not-found`) from `select_session` and `shown_session`, where
//!   the fake answers `unavailable` and `None`.
//! - **No per-pane environment, and none of the adapter's own environment on an argv**
//!   (`ps` and tmux show it): no per-pane environment names, model or effort are applied.
//!   tmux starts both the server and the TUI, so under [`ProcessEnv::Inherit`] both see the
//!   tmux server's environment: a provider credential reaches the server only through the
//!   pane's tmux session (#649's to set, ADR-0021 section 2), never through an argv. tmux
//!   limits a command's size (3.7c refused a 20,000-byte argument), so #642 part 3's
//!   `OPENCODE_CONFIG_CONTENT` carries only `{"default_agent": KEY}`, never an agent.
//!
//! [`http`] is the adapter's own transport, and [`tui`] holds the tmux configuration and the
~~~~

### E9. lib.rs: ProcessEnv and its prefix (the one env-prefix helper)
`crates/holler-adapter-opencode/src/lib.rs:145-186`

~~~~
/// Precondition (ADR-0021 section 2, "`HarnessPort` as built (#642)", fact 6): a resolver
/// must answer for a pane that is being launched, before its registry record exists (#644
/// writes the record after the act). Meeting it is wiring's job (#649), not a registry
/// lookup's.
pub type Resolver<K, V> = Arc<dyn Fn(&K) -> Result<V, PaneError> + Send + Sync>;

/// The environment the adapter's OpenCode processes, the server and the TUI, start with.
/// Both are started by tmux (the server by the host adapter's `run`, #802), and both argvs
/// begin with the one `env` prefix it gives (`ProcessEnv::prefix`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessEnv {
    /// Each inherits the tmux server's environment (tmux starts it, so not the adapter's),
    /// with `OPENCODE_DISABLE_TERMINAL_TITLE` removed (`env -u`), because the adapter reads
    /// the TUI's title. A provider credential reaches the server only that way, through the
    /// pane's tmux session (#649), never through an argv.
    Inherit,
    /// Each gets exactly these variables (`env -i` semantics). Tests use it for the scratch
    /// `HOME` and `XDG_*`. It must not set `OPENCODE_DISABLE_TERMINAL_TITLE`, which turns
    /// off the title the adapter reads.
    ///
    /// **Never put a secret here.** Every pair goes on the server's and the TUI's command
    /// lines, which `ps` shows and tmux reports as `#{pane_start_command}`. No message of
    /// the adapter echoes a start command or an environment value.
    Isolated(Vec<(String, String)>),
}

impl ProcessEnv {
    /// The `env` prefix of every OpenCode argv the adapter builds, the server's and the
    /// TUI's: `env -u OPENCODE_DISABLE_TERMINAL_TITLE` under [`ProcessEnv::Inherit`], and
    /// `env -i K=V ...` with exactly the configured pairs under [`ProcessEnv::Isolated`].
    /// Nothing of the adapter's own environment goes into it.
    pub(crate) fn prefix(&self) -> Vec<String> {
        let mut prefix = vec!["env".to_owned()];
        match self {
            ProcessEnv::Inherit => prefix.extend(["-u".to_owned(), TITLE_OFF.to_owned()]),
            ProcessEnv::Isolated(vars) => {
                prefix.push("-i".to_owned());
                prefix.extend(vars.iter().map(|(key, value)| format!("{key}={value}")));
            }
        }
        prefix
    }
~~~~

### E10. lib.rs: OpenCodeConfig, the HarnessPort impl for serve_command and verify_serving
`crates/holler-adapter-opencode/src/lib.rs:213-290`

~~~~
/// How the adapter reaches OpenCode and tmux.
#[derive(Clone)]
pub struct OpenCodeConfig {
    /// The `opencode` binary, for the server and the TUI. Its path must be valid UTF-8,
    /// since it goes on their command lines. tmux starts both, so a bare name is looked up
    /// on the `PATH` of the tmux server; wiring (#649) passes an absolute path.
    pub opencode_bin: PathBuf,
    /// Appended after `serve --port P --hostname 127.0.0.1`; the tests pass `["--pure"]`.
    pub serve_args: Vec<String>,
    /// The environment of the server and of the TUI's `attach` (see [`ProcessEnv`]).
    pub env: ProcessEnv,
    /// The tmux server the panes' TUIs run on.
    pub tmux: TmuxConfig,
    /// The tmux session name a pane's TUI runs in, never a tmux target (#649:
    /// `Pane.host.tmux`, which equals `Pane.name`, ADR-0021 line 37). The adapter alone
    /// turns it into a target.
    pub tui_session: Resolver<PaneId, PaneName>,
    /// The bounds.
    pub timeouts: Timeouts,
}

impl OpenCodeConfig {
    /// The `opencode` binary as an argv's text; `unavailable` when it is not valid UTF-8.
    pub(crate) fn bin(&self) -> Result<&str, PaneError> {
        self.opencode_bin
            .to_str()
            .ok_or_else(|| PaneError::Unavailable {
                what: "the path of the opencode binary is not valid UTF-8".to_owned(),
            })
    }
}

/// The `HarnessPort` over OpenCode.
pub struct OpenCodeHarness {
    config: OpenCodeConfig,
}

impl OpenCodeHarness {
    /// An adapter with `config`. It starts nothing until a method is called.
    pub fn new(config: OpenCodeConfig) -> Self {
        Self { config }
    }

    /// A call of the method `op` to the server on `port`, its deadline starting now.
    fn call(&self, port: u16, op: &'static str) -> Call {
        self.call_until(port, op, deadline_after(self.config.timeouts.call))
    }

    /// A call of the method `op` to the server on `port` that must end by `deadline`, the
    /// one the method took at its entry: `select_session` learns its port only from tmux,
    /// after its deadline has started.
    fn call_until(&self, port: u16, op: &'static str, deadline: Instant) -> Call {
        Call {
            port,
            op,
            deadline,
            request: self.config.timeouts.request,
        }
    }
}

impl HarnessPort for OpenCodeHarness {
    fn serve_command(
        &self,
        _name: &PaneName,
        port: u16,
        spec: &ServeSpec,
    ) -> Result<Argv, PaneError> {
        server::serve_command(&self.config, port, spec)
    }

    fn verify_serving(&self, _port: u16, spec: &ServeSpec) -> Result<(), PaneError> {
        server::refuse_an_agent(spec)
    }

    fn health(&self, port: u16) -> Result<bool, PaneError> {
        let timeouts = &self.config.timeouts;
        Ok(server::healthy(port, timeouts.health.min(timeouts.call)))
~~~~

### E11. lib.rs: Route, Call::send_by and Call::unexpected (how every request and its failure read)
`crates/holler-adapter-opencode/src/lib.rs:345-440`

~~~~
const CREATE: Route = Route {
    method: "POST",
    label: "/session",
};
const RETITLE: Route = Route {
    method: "PATCH",
    label: "/session/:id",
};
const DELETE: Route = Route {
    method: "DELETE",
    label: "/session/:id",
};
const LIST: Route = Route {
    method: "GET",
    label: "/session",
};
const GET_SESSION: Route = Route {
    method: "GET",
    label: "/session/:id",
};
const ABORT: Route = Route {
    method: "POST",
    label: "/session/:id/abort",
};
const STATUS: Route = Route {
    method: "GET",
    label: "/session/status",
};
const SELECT: Route = Route {
    method: "POST",
    label: "/tui/select-session",
};

/// One port call in flight: the server's port, the method's `op` and its deadline.
struct Call {
    port: u16,
    op: &'static str,
    /// [`Timeouts::call`] from the method's entry (I5).
    deadline: Instant,
    /// [`Timeouts::request`]: one request's own bound.
    request: Duration,
}

impl Call {
    /// One request of this call, within the smaller of `request` and what is left of the
    /// call.
    fn send(&self, route: Route, path: &str, body: Option<&Value>) -> Result<Reply, PaneError> {
        self.send_by(route, path, body, self.deadline)
    }

    /// [`Call::send`], also ended by `until` (a poll's own end inside the call's bound).
    /// Refused is `unavailable`; no reply in time is `timeout` with the call's `op`; a
    /// reply that is not HTTP is `unavailable` naming the route.
    fn send_by(
        &self,
        route: Route,
        path: &str,
        body: Option<&Value>,
        until: Instant,
    ) -> Result<Reply, PaneError> {
        let within = budget(self.request, until.min(self.deadline));
        match http::request(self.port, route.method, path, body, within) {
            Ok(reply) => Ok(reply),
            Err(HttpError::Refused) => Err(PaneError::Unavailable {
                what: format!("the harness server on port {}", self.port),
            }),
            Err(HttpError::TimedOut) => Err(self.timeout()),
            Err(HttpError::Garbled(why)) => Err(PaneError::Unavailable {
                what: format!(
                    "{} {} on port {}: {}",
                    route.method,
                    route.label,
                    self.port,
                    one_line(&why)
                ),
            }),
        }
    }

    fn timeout(&self) -> PaneError {
        PaneError::Timeout {
            op: self.op.to_owned(),
        }
    }

    /// The `unavailable` of a reply that is not the `wanted` answer of `route`: one line
    /// naming the route, the port and the status, quoting at most 60 bytes of the body.
    fn unexpected(&self, route: Route, reply: &Reply, wanted: &str) -> PaneError {
        PaneError::Unavailable {
            what: format!(
                "{} {} on port {} answered {}, not {wanted}: \"{}\"",
                route.method,
                route.label,
                self.port,
                reply.status,
                excerpt(&reply.body)
~~~~

### E12. lib.rs: settled and poll (the existing use of GET /session/status)
`crates/holler-adapter-opencode/src/lib.rs:489-525`

~~~~
/// Poll `GET /session/status` until `id` is absent from it or not `busy`, within `settle`:
/// an abort is trusted only once it is seen. Still busy at the end is `timeout`.
fn settled(call: &Call, id: &str, settle: Duration) -> Result<(), PaneError> {
    poll(settle, call.deadline, call.op, |until| {
        let reply = call.send_by(STATUS, STATUS.label, None, until)?;
        let status = json_of(&reply)
            .filter(Value::is_object)
            .ok_or_else(|| call.unexpected(STATUS, &reply, "an object"))?;
        Ok((string_at(status.get(id), "type") != Some("busy")).then_some(()))
    })
}

/// The adapter's one poll, for whatever must be seen within [`Timeouts::settle`] (an
/// abort, a switch, an attach): `step` runs every [`SETTLE_POLL`] until it answers, within
/// `settle` and `deadline`, the method's own, whichever ends first. `step` gets that end, to
/// bound its own work. The poll sleeps only while a whole interval is left; when the end
/// comes first, the answer is `timeout` with `op`.
fn poll<T>(
    settle: Duration,
    deadline: Instant,
    op: &str,
    mut step: impl FnMut(Instant) -> Result<Option<T>, PaneError>,
) -> Result<T, PaneError> {
    let until = deadline_after(settle).min(deadline);
    loop {
        if let Some(seen) = step(until)? {
            return Ok(seen);
        }
        if Instant::now() + SETTLE_POLL >= until {
            return Err(PaneError::Timeout { op: op.to_owned() });
        }
        std::thread::sleep(SETTLE_POLL);
    }
}

/// The ids of a session list's top-level sessions, in the order given: an element whose
/// `parentID` is a string is a child session and is left out (ADR-0021's `HarnessPort` note,
~~~~

### E13. spike notes: /session/status
`docs/research/opencode-pane-spike.md:60-75`

~~~~
POST /session  {"title":"…"}   -> 200 {"id":"ses_…","directory":"<project dir>","title":"…",…}
POST /session?directory=<dir>  -> 200 (same; the directory query parameter selects the project instance)
GET  /session                  -> 200 [ …sessions, most recently updated first… ]
GET  /session/:id              -> 200 session | 404 {"name":"NotFoundError",…}
PATCH /session/:id {"title":…} -> 200 session (renames it)
DELETE /session/:id            -> 200 true | 404 when already gone
GET  /session/status           -> 200 {} when nothing runs; {"ses_…":{"type":"busy"}} while busy
```

**Observed (verified).** The server is healthy 630-740 ms after spawn, warm or cold, including with a fresh config
directory. A fresh server has **no** session of its own: no "ping" session. A created session keeps the title
Holler gives it and is placed in the server's working directory. After the server is killed and restarted on a new
port with the same data directory, `GET /session/:id` still returns it. Idle sessions are absent from
`/session/status`, so absence means idle. `GET /api/session/active` (the v2 API) returned `{"data":{}}`.

**Observed (verified): two servers that share a data directory share one session store.** A session created on the
~~~~

### E14. spike notes: a busy session without a model call
`docs/research/opencode-pane-spike.md:165-170`

~~~~
**Observed (verified).**
- Idle session: `200 true`, no effect.
- Busy session: `POST /session/:id/shell {"agent":"build","command":"sleep <n>"}` makes `/session/status` report
  `busy` without a model call. `abort` then answers `200 true`, the session leaves `/session/status` (idle) within
  22-28 ms, the `sleep` child is gone, and the shell call returns `200` with tool output
  `User aborted the command`.
~~~~

### E15. holler-body's real-OpenCode HTTP facts: the prompt route
`crates/holler-body/src/http_attach_driver.rs:36-45`

~~~~
//!   — fail closed, never fall through to `POST /session` (verified this is
//!   never called by [`HttpAttachDriver::attach`]).
//! - **Prompt**: confirmed live that `POST {endpoint}/session/{session_id}/prompt_async`
//!   (bare, **not** `/api`-prefixed) is the real route — `204` immediately,
//!   genuinely fire-and-forget. `POST {endpoint}/api/session/{id}/prompt_async`
//!   is **not** a real route: it fell through to the SPA's catch-all and
//!   returned a `200` of the web UI's own `index.html`, not a driver success.
//!   This driver therefore always calls the bare form, regardless of which
//!   prefix answered the existence check.
//! - **Interrupt**: confirmed the exact mirror image — `POST
~~~~

### E16. holler-body: the prompt_async request as sent
`crates/holler-body/src/http_attach_driver/connection.rs:270-300`

~~~~
async fn send_prompt(
    client: &reqwest::Client,
    endpoint: &str,
    session_id: &str,
    text: &str,
    shared: &Arc<Mutex<Shared>>,
) {
    // Always bare — confirmed live that `/api/session/{id}/prompt_async` is
    // not a real route (see `http_attach_driver.rs`'s module doc).
    let url = format!("{}/session/{session_id}/prompt_async", endpoint.trim_end_matches('/'));
    let body = serde_json::json!({ "parts": [{"type": "text", "text": text}] });
    log_debug(
        "prompt_async",
        vec![("method", "POST".to_string()), ("path", format!("/session/{session_id}/prompt_async"))],
        holler_proto::log::frame_at_noisy(&body.to_string()),
    );
    lock(shared).status = Status::Working;
    let started = Instant::now();
    match client.post(&url).json(&body).send().await {
        Ok(response) => {
            log_debug(
                "prompt_async",
                vec![
                    ("status", response.status().as_str().to_string()),
                    ("ms", elapsed_ms(started)),
                ],
                None,
            );
        }
        Err(err) => {
            log_warn("prompt_async", vec![("event", format!("error: {err}")), ("ms", elapsed_ms(started))]);
~~~~

### E17. holler-body: the shape of a pending question and a pending permission
`crates/holler-body/src/http_attach_driver/wire.rs:84-126`

~~~~
}

/// One pending item from `GET {endpoint}/question` (issue #133/#382), per
/// OpenCode's real `QuestionRequest` schema — the JSON field is `id`, not
/// `requestID` (that name is only the URL path parameter on the reply
/// endpoint).
#[derive(Debug, Deserialize)]
pub(super) struct OcQuestionRequest {
    pub(super) id: String,
    #[serde(rename = "sessionID")]
    pub(super) session_id: String,
    pub(super) questions: Vec<OcQuestionInfo>,
}

#[derive(Debug, Deserialize)]
pub(super) struct OcQuestionInfo {
    #[serde(default)]
    pub(super) question: String,
    pub(super) options: Vec<OcQuestionOption>,
}

#[derive(Debug, Deserialize)]
pub(super) struct OcQuestionOption {
    pub(super) label: String,
}

/// One pending item from `GET {endpoint}/permission` (issue #133/#382).
/// Only `id`/`sessionID` are needed — the reply vocabulary
/// (`once`/`always`/`reject`) is fixed, not derived from anything else in
/// this shape.
#[derive(Debug, Deserialize)]
pub(super) struct OcPermissionRequest {
    pub(super) id: String,
    #[serde(rename = "sessionID")]
    pub(super) session_id: String,
    #[serde(default)]
    pub(super) permission: String,
}

/// Maps a `holler-server answer` `choice` to OpenCode's real permission
/// reply enum (`once`/`always`/`reject`), accepting a few obvious
/// operator-facing aliases since the wire's `choice` is free text.
pub(super) fn normalize_permission_reply(choice: &str) -> Option<&'static str> {
~~~~

### E18. holler-body: how GET /question is read
`crates/holler-body/src/http_attach_driver/connection.rs:383-405`

~~~~
    child_sessions: &[String],
) -> Option<PendingBlock> {
    let url = format!("{}/question", endpoint.trim_end_matches('/'));
    log_debug("question_poll", vec![("path", "/question".to_string())], None);
    let response = client.get(&url).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    let requests: Vec<OcQuestionRequest> = response.json().await.ok()?;
    let request = requests
        .into_iter()
        .find(|r| session_in_scope(&r.session_id, session_id, child_sessions))?;
    if request.questions.is_empty() {
        return None;
    }
    let prompt = request
        .questions
        .first()
        .map(|q| q.question.clone())
        .unwrap_or_default();
    let options: Vec<Vec<String>> = request
        .questions
        .iter()
~~~~

### E19. the real-OpenCode rig: header and its provider guard
`crates/holler-adapter-opencode/tests/real_opencode/rig.rs:1-36`

~~~~
//! The rig of `real_opencode_test.rs` (#642b, the brief's "Real OpenCode" rules): one per
//! test, and one per conformance case. It is the spike's shell rig
//! (`scripts/spikes/opencode-lib.sh`) in Rust:
//!
//! - a fresh scratch dir in `/tmp`, short enough for a tmux socket path; `HOME` and the
//!   `XDG_*` dirs inside it, so no real OpenCode config, auth or session store is read;
//! - the dead-end provider config, verbatim from the spike, so no model can be reached:
//!   127.0.0.1:9 must refuse, and after every server the rig launches is healthy it must
//!   report exactly that provider, or the test fails without going on. No prompt is ever
//!   sent;
//! - a launcher (#802): `serve_command` only builds the server's argv, so the rig runs that
//!   argv itself, in a process group of its own, in the scratch project dir, waits for its
//!   health with the engine's own wait (`holler_pane::tx_launch::wait_healthy`), then runs
//!   the model guard. This crate may not depend on the host adapter (ADR-0021 section 5), so
//!   the server does not run under tmux here;
//! - two ports from 48100-48199 only, each refused and bindable when picked and not yet
//!   handed out in this process;
//! - a private tmux server (`-S <scratch>/tmux.sock -f /dev/null`) with sessions
//!   `demo-c1r1` and `demo-c2r1` running a placeholder (`sleep 3600`), for the panes
//!   `w9:p1` and `w9:p2`; never the default tmux server.
//!
//! The [`Guard`] is built before the tmux server starts. On drop it SIGKILLs the process
//! group of every server the launcher started through [`Recording`], kills the private tmux server
//! by its socket (`kill-server`, never a signal), and removes the scratch dir. It never
//! signals a process it did not start and never looks processes up.

use std::collections::HashSet;
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant};

use holler_adapter_opencode::http::{self, Reply};
use holler_adapter_opencode::{
~~~~

### E20. ADR-0021 section 2: the amendment decisions, the tmux size limit and the as-built facts (lines to correct)
`docs/adr/ADR-0021.md:116-137`

~~~~
section 8) and by #802, and `HostPort` by #802 (see "The harness server as amended (#802)" below).

**The harness server as amended (#802).** The operator's decisions of 2026-10-11 (1 to 4), and three more decided in
review round 1 under the operator's rules (change signatures freely while in development; one source of truth; a wedged
process can always be killed with extreme prejudice):

1. **Decided (operator, 2026-10-11): the agent is applied as the OpenCode server's `default_agent`, set when the server
   starts**, not per prompt: per prompt would need a protocol field, a hub change and a `holler-body` change.
2. **Decided (operator, 2026-10-11): it reaches the port by changing the port**, not by an additive default-armed method.
   An omitted agent is a compile error at every call site, and every implementation is in this workspace.
3. **Decided (operator, 2026-10-11): `ServeSpec`**, a `#[non_exhaustive]` struct in `holler-pane`'s `ports.rs` that holds
   `agent: Option<AgentKey>` today. The model, the effort, the environment names and the context ceilings join it later
   with no signature change, and a new field of the pane's spec for its server goes there, never into a new parameter.
   Outside `holler-pane` it is built only from `ServeSpec::default()` and its builder (`with_agent`); the launch engine
   builds it with every field named, so a field added later is a compile error there until it is passed on.
4. **Decided (operator, 2026-10-11): the harness server runs inside the pane's tmux session, started through the host
   adapter.** `HostPort` (`run`, and `stop_owned`: TERM, a shared grace, then KILL) is the one place that starts and stops a
   pane's processes. So `HarnessPort::serve` became `serve_command(name, port, &ServeSpec) -> Argv`, which builds the
   server's argv and starts nothing (no process, no bound port, no file); the OpenCode adapter no longer spawns `opencode
   serve`; and `harness.pid` stops being a second source of truth: launch no longer writes it. This closes #695 (a wedged
   server could not be stopped). When the spec has an agent, some element of the argv contains the key's text; a harness
   that cannot apply it refuses `agent-unsupported` (an open code, exit 3: the harness understood the request and
~~~~

### E22. ADR-0021: the tmux size limit and the as-built facts 2 and 7
`docs/adr/ADR-0021.md:166-175`

~~~~

What moves with the server: `stop_owned` stops the server's process group, not the shell commands OpenCode starts in
sessions of their own (`scripts/spikes/opencode-lib.sh`), which the deleted `kill_group` did not stop either. A provider
credential never goes on an argv (`ps` and tmux show it): under tmux the server inherits the tmux server's environment, not
the adapter's, so a credential reaches it only through the pane's tmux session. That is #649's to set, and `HostPort`
has no environment parameter, so #649 may need a `HostPort` amendment of its own. tmux also limits a command's size
(tmux 3.7c refused a 20,000-byte `new-window` argument and took 15,000), so #642 part 3's `OPENCODE_CONFIG_CONTENT`
carries only `{"default_agent": KEY}`, never an agent's definition.

**`HarnessPort` as built (#642).** `holler-adapter-opencode` implements every method, over OpenCode's HTTP API on
~~~~

### E23. ADR-0021 section 8: the prompt and activity seam
`docs/adr/ADR-0021.md:469-482`

~~~~

**The prompt and activity seam (#645 part 2, the operator's carve-out).** `HarnessPort` grows two default-armed methods,
the first amendment since #637 froze it (the second is #802's, section 2): `send_prompt(port, session, text)` and
`session_activity(port, session)`, which
answers the closed `Activity` (`Idle`, `Busy`, `HoldingQuestion`; in `ports.rs`, never on the wire). A default keeps every
impl that does not override it as it was: `session_activity` answers `Idle`, so nothing is refused where no port reports
activity, and `send_prompt` refuses with the open code `prompt-unsupported` (exit 3). That code departs from the
`not-implemented` the skeleton answers for unlanded methods on purpose: by the time `reset --first` sends the prompt the
reset itself has succeeded, so "understood and declined" (a refusal) is the honest class, not a runtime failure. The real
OpenCode adapter replaces both defaults (#642 part 3), and the test kit's fake records them (#684). Two choices differ from
what this ADR had proposed for 645b, and #646 and #649 build on them rather than beside them: the first message goes from the
verb straight to `HarnessPort::send_prompt`, not through the hub's one prompt path (so the prompt gate of #646, park state
included, does not see it); and the activity read is of the pane's session of record, not of its DRIVEN session, which
until #649 is not kept in step with it.
~~~~

### E24. ADR-0021: the Deferred bullets this part makes false
`docs/adr/ADR-0021.md:808-828`

~~~~
## Deferred to named stories

- The roster's `--json` shape beside the envelope: #648.
- How the hub's DRIVEN session follows `session_of_record`: #649 and #654.
- The exact file layouts of `panes.json` and `profiles.json`, the long-poll window, and the feed's retained window: #639 and
  #661.
- Which closed failure code a mismatch observed after `act` carries: decided, `unavailable` (exit 1), section 8: #644 for
  launch and relaunch; #645 for switch and reset.
- The refusal of a pane that is not idle or holds a question, and `reset --first`: decided, #645 part 2, through two
  default-armed `HarnessPort` methods (section 8, "The prompt and activity seam"). Their real implementation is #642
  part 3's, and the fake's recording #684's; whether the first message later moves to the hub's one prompt path is #646's
  and #649's to decide.
- `HerdrPort`: #640 implements it as merged; a pane with no cell (inside a nested split, or in another tab) has no place in
  `HerdrSnapshot`, which waits for a contract amendment before #647 and #650 need it.
- Applying `Pane.opencode_agent` in the OpenCode adapter: #642's last part, after #700, through the `ServeSpec` that #802
  added to `HarnessPort::serve_command` and `verify_serving` (section 2).
- Stopping the harness server: decided, #802 (closing #695). The server runs inside the pane's tmux session, started by
  `HostPort::run`, so `HostPort::stop_owned` (TERM, a grace, then KILL) stops it, and launch records no pid (section 2,
  "The harness server as amended (#802)").
- Applying a project's agent in `pane launch` and `pane relaunch` (`--agent KEY`, refusing a key the project does not define,
  and recording `Pane.opencode_agent` in the record write): #644's last part, after #700.
~~~~

### E25. tests that pin the OpenCode adapter's agent-unsupported refusal today: hermetic_test.rs
`crates/holler-adapter-opencode/tests/hermetic_test.rs:636-680`

~~~~
        stub.lines().is_empty(),
        "nothing was sent: {:?}",
        stub.lines()
    );
}

fn agent(text: &str) -> AgentKey {
    AgentKey::parse(text).unwrap()
}

fn is_agent_unsupported<T: std::fmt::Debug>(answer: &Result<T, PaneError>) -> bool {
    matches!(answer, Err(e @ PaneError::Refused { .. }) if e.code() == "agent-unsupported")
}

/// #802 AC 5 and plan review W-3: until #642 part 3 the adapter applies no agent, so a spec
/// with one is refused `agent-unsupported` by `serve_command` (no argv at all) and by
/// `verify_serving` (fail closed); a spec without one verifies `Ok` and sends nothing.
#[test]
fn ac8_a_spec_with_an_agent_is_refused_and_one_without_verifies() {
    let stub = Stub::start();
    let h = serving(ProcessEnv::Inherit);
    let with = ServeSpec::default().with_agent(agent("feature-implementor"));
    let built = h.serve_command(&pane_name(), stub.port, &with);
    assert!(is_agent_unsupported(&built), "{built:?}");
    let verified = h.verify_serving(stub.port, &with);
    assert!(is_agent_unsupported(&verified), "{verified:?}");
    assert_eq!(h.verify_serving(stub.port, &ServeSpec::default()), Ok(()));
    assert!(stub.lines().is_empty(), "sent: {:?}", stub.lines());
    let message = match built {
        Err(PaneError::Refused { message, .. }) => message,
        other => panic!("{other:?}"),
    };
    assert!(!message.contains('\n'), "one line: {message:?}");
}

/// Run by [`ac8_no_credential_of_the_adapters_environment_reaches_an_argv`] as a re-executed
/// child whose own environment holds [`SECRET_VAR`]; returns at once otherwise. It fails when
/// any argv the adapter builds, under either `ProcessEnv`, holds the secret's name or value.
#[test]
fn credential_probe_child() {
    if std::env::var_os(CREDENTIAL_PROBE).is_none() {
        return;
    }
    assert_eq!(
        std::env::var(SECRET_VAR).as_deref(),
~~~~

### E26. tests that pin it today: real_opencode_test.rs
`crates/holler-adapter-opencode/tests/real_opencode_test.rs:318-345`

~~~~
    }
    let rig = Rig::start();
    let p0 = rig.ports[0];
    let name = PaneName::parse(SESSIONS[0]).unwrap();
    let argv = rig
        .harness
        .serve_command(&name, p0, &ServeSpec::default())
        .unwrap();
    assert!(!argv.as_slice().is_empty());
    std::thread::sleep(Duration::from_millis(500));
    assert!(refused(p0), "#802: serve_command started nothing on {p0}");
    let key = AgentKey::parse("feature-implementor").unwrap();
    let with = ServeSpec::default().with_agent(key);
    let refusal = rig.harness.serve_command(&name, p0, &with);
    assert!(
        matches!(&refusal, Err(e) if e.code() == "agent-unsupported"),
        "{refusal:?}"
    );
}

#[test]
#[ignore = "needs opencode and tmux: HOLLER_TEST_OPENCODE=1 cargo test -p holler-adapter-opencode --test real_opencode_test -- --ignored"]
fn ac19a_a_prefix_of_a_session_name_never_reaches_its_pane() {
    if !opted_in("ac19a") {
        return;
    }
    let mut rig = Rig::start();
    rig.serve(0);
~~~~

### E27. hermetic_test.rs helpers the new tests reuse
`crates/holler-adapter-opencode/tests/hermetic_test.rs:28-112`

~~~~

/// A session id of OpenCode's shape: `ses_` and 26 of `[0-9A-Za-z]`.
const ID: &str = "ses_0123456789abcdefABCDEFghij";
/// What a bound may overrun by on a loaded CI runner.
const SLACK: Duration = Duration::from_millis(300);
const HEALTHY: &str = r#"{"healthy":true,"version":"1.18.35"}"#;

/// Short bounds so the frozen cases finish quickly.
fn quick() -> Timeouts {
    Timeouts {
        call: Duration::from_secs(2),
        request: Duration::from_millis(800),
        health: Duration::from_millis(300),
        settle: Duration::from_millis(400),
    }
}

/// A scratch directory of its own per test, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    /// A scratch dir holding an executable `opencode` script that leaves a `spawned` marker
    /// next to itself: a stand-in binary that shows whether anything ran it.
    fn with_fake_opencode() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("hlr802-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("opencode");
        let body = format!(
            "#!/bin/sh\ntouch '{}'\nexit 42\n",
            dir.join("spawned").display()
        );
        std::fs::write(&script, body).unwrap();
        let mut mode = std::fs::metadata(&script).unwrap().permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut mode, 0o755);
        std::fs::set_permissions(&script, mode).unwrap();
        Scratch(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn spawned(&self) -> bool {
        self.0.join("spawned").exists()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A config whose tmux half is never reached in 642a: a binary and socket that do not exist,
/// and a `tui_session` that knows no pane.
fn config(opencode_bin: impl Into<PathBuf>, timeouts: Timeouts) -> OpenCodeConfig {
    OpenCodeConfig {
        opencode_bin: opencode_bin.into(),
        serve_args: Vec::new(),
        env: ProcessEnv::Inherit,
        tmux: TmuxConfig {
            tmux_bin: PathBuf::from("/nonexistent/hlr642-tmux"),
            socket: TmuxSocket::Path(PathBuf::from("/nonexistent/hlr642-tmux.sock")),
        },
        tui_session: Arc::new(|pane: &PaneId| {
            Err(PaneError::PaneNotFound {
                what: pane.as_str().to_string(),
            })
        }),
        timeouts,
    }
}

/// An adapter for the calls that never spawn.
fn harness(timeouts: Timeouts) -> OpenCodeHarness {
    OpenCodeHarness::new(config("/nonexistent/hlr642-opencode", timeouts))
}

fn pane_name() -> PaneName {
    PaneName::parse("demo-c1r1").unwrap()
}

fn unavailable<T: std::fmt::Debug>(result: Result<T, PaneError>, what: &str) -> String {
~~~~

### E28. tui_argv: the TUI's argv shares the prefix
`crates/holler-adapter-opencode/src/tui.rs:154-183`

~~~~
    escape_arg(&dir.replace('#', "##"))
}

/// The TUI's argv, unescaped. Under [`ProcessEnv::Isolated`] it is
///
/// ```text
/// env -i K=V ... <bin> attach http://127.0.0.1:<port> --dir <dir> --session <id>
/// ```
///
/// and under [`ProcessEnv::Inherit`] it starts `env -u OPENCODE_DISABLE_TERMINAL_TITLE`
/// instead of `env -i K=V ...`, so the title the adapter reads stays on. The prefix is the
/// one the server's argv starts with too (`ProcessEnv`'s, #802).
pub fn tui_argv(
    opencode_bin: &str,
    env: &ProcessEnv,
    port: u16,
    dir: &str,
    session_id: &str,
) -> Vec<String> {
    let mut argv = env.prefix();
    argv.extend([
        opencode_bin.to_owned(),
        "attach".to_owned(),
        format!("{LOOPBACK_URL}{port}"),
        "--dir".to_owned(),
        dir.to_owned(),
        "--session".to_owned(),
        session_id.to_owned(),
    ]);
    argv
~~~~

### E21. A live probe of OpenCode's behaviour (run 2026-10-11, `opencode --version` = 1.19.0, scratch HOME and XDG dirs, ports 47911-47913, no provider configured, no model reached)

Server started as: `env -i PATH=$PATH HOME=$S/home XDG_CONFIG_HOME=$S/cfg XDG_DATA_HOME=$S/data XDG_STATE_HOME=$S/state XDG_CACHE_HOME=$S/cache OPENCODE_CONFIG_CONTENT='{"default_agent":"plan"}' opencode serve --pure --port 47911 --hostname 127.0.0.1` (project dir empty).

~~~~
GET /global/health            -> {"healthy":true,"version":"1.19.0"}
GET /config                   -> {"command":{},"plugin":[],"default_agent":"plan","username":"unknown","mode":{},"agent":{}}
GET /agent | jq -c '[.[]|{name,mode,native,hidden}]'
  -> [{"name":"plan","mode":"primary","native":true,"hidden":null},{"name":"build","mode":"primary","native":true,"hidden":null},{"name":"compaction","mode":"primary","native":true,"hidden":true},{"name":"explore","mode":"subagent","native":true,"hidden":null},{"name":"general","mode":"subagent","native":true,"hidden":null},{"name":"summary","mode":"primary","native":true,"hidden":true},{"name":"title","mode":"primary","native":true,"hidden":true}]
GET /agent | jq -c '.[0]|keys' -> ["description","mode","name","native","options","permission"]
   (so `hidden` is absent unless it is true; the jq above printed null for an absent key)
GET /session/status           -> {}
GET /question                 -> []
~~~~

An unusable `default_agent` is not refused by the server at start (it answers healthy and `/config` echoes the value), which is why `verify_serving` must check:

~~~~
OPENCODE_CONFIG_CONTENT='{"default_agent":"general"}'     (a subagent)
  /global/health {"healthy":true,...}; /config default_agent "general"; /agent lists general first, mode "subagent"
OPENCODE_CONFIG_CONTENT='{"default_agent":"nonexistent"}'
  /global/health {"healthy":true,...}; /config default_agent "nonexistent"; /agent has no such element (build is first)
OPENCODE_CONFIG_CONTENT='{"default_agent":"title"}'       (hidden)
  /global/health {"healthy":true,...}; /config default_agent "title"; /agent lists title first, "hidden":true
~~~~

A prompt sent to a server started with `default_agent` `plan` (`POST /session` -> `ses_...`; then the bare route holler-body uses):

~~~~
POST /session/<id>/prompt_async  {"parts":[{"type":"text","text":"hello"}]}   -> 204
GET  /session/<id>/message | jq -c '[.[]|{role:.info.role,agent:.info.agent,mode:.info.mode,error:(.info.error.name // null),completed:(.info.time.completed // null)}]'
  -> [{"role":"user","agent":"plan","mode":null,"error":null,"completed":null},{"role":"assistant","agent":"plan","mode":"plan","error":null,"completed":1791722748214}]
GET  /session/status          -> {}
~~~~

## Comments from the issue thread

### aangelinsf, 2026-10-10 20:00:51 MDT (2026-10-11T02:00:51Z)

Scope left (2026-10-10): parts 1 and 2 are merged (#705, bd5e825). What remains is the 'agent' amendment only: apply `Pane.opencode_agent` (merged in #721) as the server's `default_agent` when the adapter starts it, or per prompt if the API takes an agent per message (the #635 spike says which), with the extra conformance case. The `HarnessPort::serve` signature gains the agent; #644's `--agent` wiring waits for this.

AI assistance: drafted by Claude (Sonnet 5.5); the maintainer is accountable, per CONTRIBUTING.md.

### aangelinsf, 2026-10-11 03:21:30 MDT (2026-10-11T09:21:30Z)

Correction (2026-10-11): the line in my scope comment above, "The `HarnessPort::serve` signature gains the agent", was a draft assumption of mine, not an operator decision. The operator's decisions are in #802: the agent is applied as the server's `default_agent` at start, and `HarnessPort::serve` itself changes to `serve(name, port, &ServeSpec)` (amended first, in its own PR, step 1). #642 part 3 (the adapter applies the agent) waits for #802.

AI assistance: drafted by Claude (Sonnet 5.5); the maintainer is accountable, per CONTRIBUTING.md.

### aangelinsf, 2026-10-11 04:39:54 MDT (2026-10-11T10:39:54Z)

A request for this story, while it is still open (from the pfleet watchdog work in [pfleet#396](https://github.com/Performant-Labs/pfleet/issues/396); see the note on #633).

The adapter's session state already includes `HoldingQuestion`. A watcher also needs the **content** of what is held, read through the adapter instead of from each pane's server:

- a `held_questions()` on the harness port returning, per held item: kind (question or permission), the session it belongs to (including a sub-agent's child session), header, options (label and description), and when it started;
- the same carried into the pane record's JSON (`holler pane get/list`), so the roster (#648) can show a held question and its age instead of `idle` with PENDING `-`.

Why it matters, from a real case: a sub-agent's question about the scope of a test run was held for 3 h 33 min while the roster showed the pane as `idle`, PENDING `-`. pfleet could only find it by reading the pane's own session store.

An open check for the story's tests: pfleet's issues could not establish whether the harness's question list includes a question held by a **child** session or only the top-level one. A fixture with a held child-session question, and a decision on what the adapter returns, would settle it for every watcher.
