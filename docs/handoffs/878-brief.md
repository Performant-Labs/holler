# Brief: #878 pane-compact

Repo: Performant-Labs/holler. Issue: #878 (part of epic #633; the operator says it gates release v0.5.0). Rigor: in-session. UI surface: no. Kind: feature.
Base: `origin/main` at ef881fc (re-checked 2026-10-12 after `git fetch`; unchanged). Branch `issue-878-implementation`. Nothing in this brief was run against a real OpenCode or Claude Code.

**Status: every open item is decided. No "Needs operator decision" item remains.** The operator's decisions of 2026-10-12 (issue comment) are in "Decided" below.

**Structure for the operator's session.**

- **PART A, the pipeline story. Launchable now.** Everything that needs no real server: contract amendment first, `HarnessPort::compact`, the refusal codes, the
  testkit fake and conformance, the OpenCode adapter (via `summarize`, with the model the server's own config pins), the CLI verb, and the 120 s bound.
  Claude Code stays `compact-unsupported` in Part A.
- **PART B, the Phase-0 spike. A later, separate step**, run by the operator's session after or alongside Part A, never by the pipeline, on scratch instances only.
  It writes `docs/research/compaction-spike.md`. It **decides only the Claude adapter** (what the old brief called Part 3). Its OpenCode half (B.1) is a
  confirmation of Part A's assumptions and a source of real fixtures, not a gate.

## Decided (operator, 2026-10-12)
- **N1 = option A.** The compaction model is the one the pane's own OpenCode server config already pins (`agent.compaction.model`). If there is none, the verb
  refuses `compact-model-unpinned` and sends nothing. No new spec field, no flag. The port's `model` is `None` for the verb; the adapter reads `GET /config`.
  Rejected: a new spec field plus flag (B), a required `--model` flag (C), the pane's recorded main model (D).
- **N2.** The spike uses, for OpenCode `summarize`, a local stub model server in a scratch home (no cost); for the Claude Code `/compact` probe, at most 3
  `haiku` turns **in total** on the operator's Claude login, never read or printed (I7); never the live fleet.
- **N3 = drop work item 5 (a generic ACP agent) from #878 entirely.** Out of scope: no ACP harness exists to build or test it on. Nothing is documented
  or filed for it beyond this one line.
- **N4.** Compaction has its own bound, **120 s by default**, separate from the ordinary verb bound (`Timeouts::call`, 10 s).
  Rejected: polling after a start (b), keeping 10 s (c).

## Problem
There is no way to compact a pane's session without typing `/compact` into the TUI. Typing was seen to fail (2026-10-12): the text sat
unsubmitted in four OpenCode panes because the slash-command menu was open and Enter was not accepted, and the caller then waited in a loop for a
compaction that never started; the watchdog flagged it after about eleven minutes. ADR-0021 I4 already forbids typing into a TUI to change a
session; compaction is the verb still missing. OpenCode's server lists `POST /session/{id}/summarize` (body `providerID`, `modelID`, optional
`auto`) and `POST /api/session/{id}/compact` (OpenAPI of OpenCode 1.19.0, read 2026-10-12; a dry run only, no real compaction has run). Whether Claude
Code accepts `/compact` as a prompt is not verified (Part B).

## Evidence (verbatim, `origin/main` at ef881fc)

### E1. The port: the two defaults `compact` copies, and the wrappers that must forward it
`crates/holler-pane/src/ports.rs:600-624`
```
    /// The default refuses with [`PROMPT_UNSUPPORTED`]: an impl that does not send
    /// prompts says so as a stable refusal, which the real adapter replaces (#642).
    fn send_prompt(
        &self,
        at: &HarnessEndpoint,
        session: &str,
        text: &str,
    ) -> Result<(), PaneError> {
        let _ = (at, session, text);
        Err(PaneError::Refused {
            code: PROMPT_UNSUPPORTED,
            message: "this harness does not queue a prompt".to_owned(),
        })
    }
...
    /// The default answers [`Activity::Idle`], so an impl that does not report activity
    /// refuses nothing.
    fn session_activity(&self, at: &HarnessEndpoint, session: &str) -> Result<Activity, PaneError> {
        let _ = (at, session);
        Ok(Activity::Idle)
    }
```
Open codes are declared as constants, `ports.rs:689-698`: `pub const PROMPT_UNSUPPORTED: RefusalCode = RefusalCode::from_static("prompt-unsupported");`
and `AGENT_UNSUPPORTED` the same way. The trait is `pub trait HarnessPort: Send + Sync` at `ports.rs:523`; `agent_view`, its last method, is `ports.rs:681`.
The endpoint type, `crates/holler-pane/src/ports/process.rs:54-60`: `pub enum HarnessEndpoint { Port(u16), Process { pane: PaneName, pid: u32 } }`;
`HarnessEndpoint::port()` refuses a process endpoint `process-unsupported`; `.process()` refuses a port `server-unsupported`.

A default-armed method is **not** found by the compiler in a wrapper that forgot it; the wrapper silently answers the default. The ones that forward
today (each has `send_prompt` and `session_activity` and gets a `compact` beside them): `crates/holler-pane/src/ports/router.rs:114,123` (routes by
`self.by(at)`), `crates/holler-pane-testkit/src/harnesses.rs:118,127`, `crates/holler-cli/src/pane/wiring/bind.rs:353,362` (`Wired`, whose doc at
`bind.rs:277-280` says "Every method is forwarded explicitly, so a method with a default ... reaches the inner port, never the default"),
`crates/holler-cli/src/pane/wiring.rs:381` (`Unwired`: it overrides most methods to answer `NotImplemented` (e.g. `abort`, line 407) but not `send_prompt` or `session_activity`, so it takes the defaults; `compact` gets the default `compact-unsupported` there unless the story adds an explicit `NotImplemented` arm, which it does, for symmetry with `abort`). The two real implementations that override: OpenCode
`crates/holler-adapter-opencode/src/lib.rs:420-431`, Claude `crates/holler-adapter-claude/src/lib.rs:302-330`. The fakes: `FakeHarness`
(`crates/holler-pane-testkit/src/harness.rs:368`, overrides neither `send_prompt` nor `session_activity`, so it is always `Idle` today) and `FakeClaude`
(`crates/holler-pane-testkit/src/claude.rs:380,391`).

### E2. What exists for a "pinned compaction model" (nothing in Holler; N1 decided: read the server's own pin)
`grep -ri compact` over `crates/*/src` finds no compaction code and no field. What exists:

- The pane's model, `crates/holler-pane/src/pane.rs:385-392`:
```
/// The model a pane runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelSpec {
    pub provider: String,
    pub model_id: String,
    pub effort: String,
}
```
  held by `Pane.model` (`pane.rs:438`) and `ProfileSpec.model` (`crates/holler-pane/src/profile.rs:185`). `Pane` and `ProfileSpec` are
  `#[serde(deny_unknown_fields)]` (`pane.rs:419`, `profile.rs:179`). It is the pane's **main** model, and for an OpenCode pane it is recorded and not
  applied, `docs/pane-control.md:87-92`: "no port carries them to an OpenCode server yet ... the model it uses is whatever its OpenCode configuration
  selects. Applying them is issue #829, not built."
- `ServeSpec` (`ports.rs:167-179`) carries `agent`, `model`, `cwd`; `model` is set only for a single process, "an OpenCode server gets `None` for both
  until #829" (`ports.rs:154-157`).
- The only compaction pin in the repo is OpenCode's own config, written by the restart script, `scripts/pane-restart:291-292`:
  `.provider[$p].options.apiKey = "{file:" + $kf + "}" | .agent.compaction.model = $m | if $e != "" then (.agent.compaction.variant = $e | ...`
  and tested at `scripts/tests/pane-restart-test.sh:298-299` (`.agent.compaction.model + " " + .agent.compaction.variant`). The OpenCode adapter
  already reads a running server's config: `crates/holler-adapter-opencode/src/server.rs:97-100` (`GET /config` answered as a JSON object, route `GET_CONFIG` at `lib.rs:482-485`). That the running server's `GET /config` exposes `agent.compaction` is **not yet observed** (it is read from the restart script's write, not from a reply); Part A's tests use the shape the script writes, and Part B.1 step 4 confirms it.
- The spec flags are one per field (ADR-0021 section 3, table at lines 281-297); there is no flag for a compaction model.

ADR-0021 section 2 has no sentence about an unintended or paid model (no hit for "unintended", "paid" or "intended model" in the ADR). The nearest
rules are: the agent is applied as the server's `default_agent`, set when the server starts (decision 1, `ADR-0021.md:133`); `send_prompt` "naming no
agent" so the pane's own default runs it (`ADR-0021.md:627`); and the model of an OpenCode pane being recorded, not applied (E2 above). The issue's
reason stands on its own: a compaction is a model call, so it must not run on a model nobody chose.

### E3. The busy check, the codes, and the single place a verb refuses on activity
`crates/holler-pane/src/tx_switch.rs:109-141`: `SESSION_BUSY` ("session-busy"), `SESSION_HOLDS_QUESTION`, `SESSION_HOLDS_PERMISSION` are open codes,
and `activity_refusal(activity)` maps `Activity` to `(code, words)`. `crates/holler-pane/src/tx_switch/plan.rs:250-` (the function; its doc starts at 246):
```
fn check_idle(ports: Ports<'_>, record: &Pane, at: &HarnessEndpoint) -> Result<(), PaneError> {
    let Some(session) = &record.session_of_record else {
        return Ok(());
    };
    let activity = ports.harness.session_activity(at, session)?;
    let Some((code, doing)) = activity_refusal(activity) else {
        return Ok(());
    };
    Err(PaneError::Refused { code, message: format!("session {} of {} {doing}", quoted(session), record.name) })
}
```
`check_health` is the sibling at `plan.rs:220` (refuses `server-unhealthy`). Both are private to `tx_switch::plan`. **`session-busy` already exists**
with the meaning "is running a turn"; compact reuses that constant. The new codes of this issue are only `compact-unsupported` and
`compact-model-unpinned`. The record to start from, with the pane-state refusals (`pane-parked`, `pane-unhealthy`, `pane-shown-driven-mismatch`):
`crates/holler-cli/src/say_cmd.rs:384-397` (`resolve_pane_record`). The endpoint of a record: `HarnessInfo::endpoint(&self, pane)`,
`crates/holler-pane/src/pane.rs:315`.

### E4. How the OpenCode adapter calls the server, and its time bound
`crates/holler-adapter-opencode/src/lib.rs:257-266`:
```
pub struct Timeouts {
    /// Every port method returns within this (I5). Default 10 s.
    pub call: Duration,
    /// One HTTP request, cut to what is left of `call`. Default 5 s.
    pub request: Duration,
    /// One health GET ...
    pub health: Duration,
    /// How long a switch, an attach or an abort may take to be observed. Default 5 s.
    pub settle: Duration,
}
```
`Call::send(route, path, body)` (`lib.rs:520`) sends one request within the smaller of `request` and what is left of `call`; routes are `Route`
constants (`lib.rs:450-505`; `Call` is `lib.rs:508`, built by `call`/`call_until` at `lib.rs:323,330` with `deadline` and `request` copied from `Timeouts`; `POST /session/:id/abort`, `GET /session/status`, `GET /session/:id/message`, `GET /config`). `send_prompt`
(`crates/holler-adapter-opencode/src/prompt.rs:31-44`) shows the shape: `known(&call, &path, session)?` (a `GET /session/:id`; an unknown session is
`session-not-found` and nothing is posted), then `call.send(PROMPT_ASYNC, ...)`, `Ok` only on the one status it expects, any other status named in a
message that quotes none of the reply. The adapter runs `session_activity` from `GET /session/status`, `GET /question`, `GET /permission` (`prompt.rs:49-70`).
**A summarize call is a model call and may outlast the 10 s `call` bound.** ADR-0021 section 2 already allows a method to be bounded otherwise
(it names two, `edit_spec` and `run_probe`, and a third through the wiring, `ADR-0021.md:97-100`; `compact` would be another).

### E5. The Claude adapter's prompt path, and its view of a status it does not know
`crates/holler-adapter-claude/src/lib.rs:302-322`: `send_prompt` checks the process still holds the session (`session-mismatch` otherwise), finds the
inbox socket by pid (`inbox::socket_of`), writes one frame `{"type":"user","message":{"role":"user","content":TEXT}}` (`inbox.rs:57-58`), then
`watch_held` (`inbound-held` when the status turns to a wait inside 600 ms). The spike note says the pane shows the text as "Another Claude session
sent a message:" with a fixed paragraph telling the model it did not come from its user (`docs/research/claude-code-pane-spike.md`, "How the model sees
it" under A). So `/compact` over this path arrives as peer text for the model, not as a typed command. Whether the client parses it as a slash command
is exactly what the spike must show. `crates/holler-adapter-claude/src/agents.rs:63-80` (`activity`) maps only `idle`, `busy`, `waiting`; any other
status is `unavailable`, pinned by `tests/hermetic_test.rs:474-486` (`("compacting", None, "unavailable")`). If Claude Code reports `compacting`
while it compacts, the adapter would currently refuse the pane as unavailable during the compaction; the spike records the real value.

### E6. The CLI: where a pane verb lives
`crates/holler-cli/src/pane/mod.rs:36-52` lists the verbs (`pub enum PaneCmd { List, Get, Watch, Launch, Relaunch, Switch, Reset, Park, Unpark, Close,
Doctor, Import }`), `acting_pane` (the verbs that act on one pane, so an unmanaged pane is refused `pane-unmanaged`) and `run` dispatch on it. The
file's header says it is frozen by #670, but `git log` shows #832 and #839 edited it, so a new variant is the normal way (three edits: the `pub mod`,
the enum variant, `acting_pane` and `run` arms). One verb is one file with its own clap `Args` and `run` (`park.rs`, `switch.rs`).
`say`/`interrupt`/`answer` are top-level (`crates/holler-cli/src/say_cmd.rs`, `interrupt_cmd.rs`, `answer_cmd.rs`), not `pane` verbs; `compact` is
`holler pane compact`, so it follows `switch.rs`, whose `execute` builds a request, runs an engine in `holler-pane` (`tx_switch::switch`) and
emits an outcome. Exit codes: 0 ok, 1 failure, 2 usage, 3 refusal (`docs/pane-control.md:163-164`). The mismatch after an act is the closed
`unavailable`, exit 1 (ADR-0021 "Switch and reset as built", `ADR-0021.md:552-600`).

### E7. The test kit
`crates/holler-pane-testkit/src/conformance/harness.rs:107` is `const CASES: [(&str, Case); 17]`, and the Claude suite
`conformance/claude.rs:128` is `[(&str, Case); 13]`; both are iterated by `run_cases` and listed by `harness_cases()`. Each has a `Mutant` wrapper
in `crates/holler-pane-testkit/tests/harness_conformance_test.rs:201` and `claude_conformance_test.rs:134` that breaks one behaviour so a case is proven
to catch it; a new case needs a mutant. `FakeHarness` has a call log per port method named by `HarnessOp` (`harness.rs:51-79`; inspection accessors such as
`aborts()` at `harness.rs:353`) and seed methods (`seed_session`, `seed_turns` at `harness.rs:287,294`). The cli verb tests use local doubles for activity
(`crates/holler-cli/tests/pane_verbs/switch/doubles.rs:164`, `SeamHarness`). OpenCode adapter fixtures live in `crates/holler-adapter-opencode/tests/fixtures/`
and `tests/support/{stub,fixtures}.rs`; `agent_view_test.rs` uses "the fixtures of a real 1.19.0 server".

Out of scope (N3): a generic ACP agent. `HarnessKind` has only `Opencode` and `Claude` (`pane.rs:140-148`); there is no ACP harness behind the port.

## PART A: the pipeline story (launchable now)

Launch condition: none open. Part A does not wait for the spike. Its OpenCode adapter is written from the OpenCode 1.19.0 OpenAPI document (**Documented**,
read 2026-10-12) and the config shape the restart script writes (E2), and its tests drive a stub server whose replies follow that document. Those replies
are a stand-in, not recorded ones. Part B.1 later runs the real server against the local stub model and is the check that the stand-in was right; if it
differs, the fix is a follow-up commit and the recorded replies replace the stand-in fixtures. Say this in the PR body.

### A1. Commit 1 (docs only, the amend-first commit)
ADR-0021 says a contract change goes first, so the commit order is the review boundary and the PR may be split after it if the reviewer asks.
`docs/adr/ADR-0021.md`: add `compact` to the verb list (line 250 and the "Failure modes by verb" table at 872-877: a new row `pane compact`); the
`HarnessPort` row of the ports table (line 111) gets `compact`; a new paragraph "Compaction (#878)" after "The prompt and activity seam" (lines 616-643) with
the signature, the codes, the shrink rule, the model rule and the bound; "Deferred to named stories" (line 1648) gets the Claude Code entry (decided by
Part B); a "Decisions taken" entry at **the next free number at merge time** (the list ends at 49 at `ADR-0021.md:1830`; do not hard-code a number).
`docs/pane-control.md`: the verb list (lines 150-151), a `### holler pane compact PANE` section between `reset` (276) and `park` (301), and section 12 "Later"
(line 1072). `CHANGELOG.md` `[Unreleased]` / `### Enhancements` (lines 8-10). Commits 2 on write tests (RED), then code.

### A2. The contract (what commit 1 states)
- New file `crates/holler-pane/src/ports/compact.rs` (beside `process.rs`, `agent_view.rs`), re-exported from `ports.rs`: `CompactOutcome { tokens_before: u64,
  tokens_after: u64 }`; `CompactModel { provider: String, model_id: String }` (a name pair, never a secret, I7); `COMPACT_UNSUPPORTED = "compact-unsupported"` and
  `COMPACT_MODEL_UNPINNED = "compact-model-unpinned"`, open codes, refusal, exit 3, declared with `RefusalCode::from_static` as at `ports.rs:689-698`.
  `session-busy` is `tx_switch::SESSION_BUSY` (E3), not redefined.
- `HarnessPort::compact(&self, at: &HarnessEndpoint, session: &str, model: Option<&CompactModel>) -> Result<CompactOutcome, PaneError>`, default-armed,
  refusing `compact-unsupported`. The verb always passes `None` (N1 = A): "use the compaction model the harness's own server config pins". A `Some` makes the
  harness use exactly that model; the verb has no way to send one, but the port keeps the parameter so a later source of a model (a spec field, if the fleet
  ever wants one) is an additive change. A harness never compacts on a model neither the caller nor its own pin named: it refuses `compact-model-unpinned`.
  (The issue's `compact(at, session, model)` is this signature; `Option` is the one refinement, said so in the ADR.) Contract rules: a busy session is refused
  `session-busy` and the harness sends no summarize; an unknown session is `session-not-found` and nothing is posted (as `abort` and `send_prompt` do); it
  never types into a TUI (I4); `tokens_after` is read from the harness after the act, never inferred; a count the harness cannot read is `unavailable`, never zero.
- The engine `holler_pane::tx_compact` (new `crates/holler-pane/src/tx_compact.rs`; no CAS write, a compaction records nothing in the registry): plan (the
  record and the pane-state refusals; `check_health`; `check_idle`), act (`compact` with `None`), observe: the outcome must satisfy `tokens_after < tokens_before`,
  otherwise `unavailable` (exit 1), the message naming both counts and saying the context did not shrink (the I3 mismatch rule: a failure observed after the act
  is `unavailable`, not an open code). `check_idle` and `check_health` become `pub(crate)` in `tx_switch::plan` (lines 250 and 220) and are called, not copied.
- CLI `holler pane compact PANE [--json]` (`crates/holler-cli/src/pane/compact.rs`, struct `PaneCompact`). **No other flag** (N1). Text: `compacted <pane>:
  <before> -> <after> tokens`. JSON `data`: `{"pane","session","tokens_before","tokens_after"}` (additive; `schema_version` stays 1). Exit 0 only when the context
  shrank; 3 for a refusal; 1 for no shrink or a failure; 2 for usage. No retry loop.
- **The bound (N4).** `Timeouts` (`crates/holler-adapter-opencode/src/lib.rs:257-266`) gets one field, `compact: Duration`, default **120 s** (set in
  `Default`, line 268; the CLI uses `Timeouts::default()`, `wiring.rs:229`). It replaces both `call` and `request` for the one `summarize` request: the compact
  method builds its `Call` with `deadline_after(timeouts.compact)` and `request: timeouts.compact` (via `call_until`, `lib.rs:330`), so the 5 s `request` bound
  does not cut it. Every other method keeps `call`/`request`. The cheap reads inside the compact method (`known`, activity, `GET /config`, token reads) use the
  same `Call`, so they are bounded by the 120 s too; that is acceptable and stated in the ADR (ADR-0021 section 2 already lets a method be bounded otherwise,
  `ADR-0021.md:97-100`; `compact` is another). The value is a named field and never a literal in the call.
- The OpenCode adapter's `compact` (`crates/holler-adapter-opencode`: new `compact.rs` beside `prompt.rs`, the `impl` method in `lib.rs` beside `send_prompt`, new
  `Route` constants for `POST /session/:id/summarize`). Order, in one `Call`: `known(...)` (unknown session `session-not-found`, nothing posted); activity via the
  existing `session_activity` (busy `session-busy`, pending question `session-holds-question`; the engine already checked, the adapter checks again because a turn
  can start in between); resolve the model (`Some` as given; `None` from `GET /config` `agent.compaction.model`; missing, empty or not a `provider/model` pair is
  `compact-model-unpinned`, nothing sent); read `tokens_before`; `POST summarize` with `providerID`/`modelID` split from the pin; accept only the one status the OpenAPI
  document gives for success (any other named in a message that quotes none of the reply, as `send_prompt` does); read `tokens_after`. Token source: the last
  assistant message's `info.tokens` from `GET /session/:id/message` (`prompt::messages`, `prompt.rs:138`) is the candidate, **Documented** and unrun; the brief does not
  assume it. T pins it from the OpenAPI schema; if the schema offers no readable count, the adapter answers `unavailable` (never zero) and Part B.1 step 3 settles it.
  The summarize variant (`variant`) and `auto` are not sent.
- Claude Code: the adapter inherits the default and answers `compact-unsupported`; the ADR and `docs/pane-control.md` say so, and that Part B decides whether it changes.

### A3. Work (T authors RED first, on the fakes)
1. `FakeHarness` learns `compact`: `HarnessOp::Compact` (`"harness.compact"`, fault-injectable like the others), a seedable context size and "compacts to", a
   seedable harness-side compaction pin (for `model: None`), an accessor returning the compact calls `(port, session, model)`, and a settable `session_activity`
   so a busy session can be seeded (today it is always `Idle`, E1). `FakeHarnesses` and `Wired` forward `compact`; the router forwards by endpoint; `Unwired`
   answers `NotImplemented`.
2. Forwarding tests: one per wrapper (`HarnessRouter`, `FakeHarnesses`, `Wired`) that a `compact` reaches the inner port and is not the default (the guard against
   the silent-default trap in E1).
3. Conformance: new cases in `conformance/harness.rs` (17 becomes 17 + n; update the count wherever it is asserted): unknown session is `session-not-found`;
   unserved port is `unavailable` checked first; a call either returns a `CompactOutcome` with `tokens_after < tokens_before` or is refused
   `compact-unsupported` / `compact-model-unpinned`; a busy session is `session-busy` and nothing was sent (where the rig can make one busy; otherwise the fake's own
   tests pin it). One Claude case (`conformance/claude.rs`, 13 becomes 14): `compact` is `compact-unsupported`. Each new case gets a mutant.
4. Engine and verb tests (acceptance below).
5. OpenCode adapter tests with `tests/support/stub.rs`: the stub asserts the requests it saw (the `known` GET, the `GET /config`, then exactly one POST, nothing else);
   a busy session sent no POST; no pin sent no POST; the request body names the pinned model; a stub that holds the reply for more than `request` (5 s) but less than
   `compact` still succeeds (the 120 s bound is wired; use a small `compact` in the test), and one that outlasts `compact` is `timeout`.

### A4. Acceptance criteria (from the issue, each observable)
1. `holler pane compact` on a fake harness that compacts: exit 0, prints both token counts; the fake logged exactly one `compact` call, with `model: None` and the
   fake's own pin as the model it ran. Test `compact_prints_before_and_after_on_the_harness_pinned_model` in `crates/holler-cli/tests/pane_verbs/compact.rs`.
2. A busy session: exit 3 with `session-busy`, and the fake logged no `compact` call (`compact_refuses_a_busy_session_and_sends_nothing`).
3. A harness with no pinned compaction model: exit 3 with `compact-model-unpinned`, no call to summarize. A harness that cannot compact (the Claude fake): exit 3 with
   `compact-unsupported`.
4. The context did not shrink (fake returns `tokens_after >= tokens_before`): exit 1, the message says the context did not shrink and names both counts; `--json` gives `ok:false`.
5. No keystroke: the fake Herdr records no `send_text`/`send_keys` during any of the above (I4).
6. The conformance suite (`run_harness_conformance`) has the `compact` cases, FakeHarness passes them, each mutant fails its case: `cargo test -p holler-pane-testkit`
   and `grep -c compact crates/holler-pane-testkit/src/conformance/harness.rs` is non-zero.
7. The OpenCode adapter's `compact` is covered by the stub tests of A3.5, including the 120 s bound and the "no pin, no POST" rule. The stub replies follow the
   OpenCode 1.19.0 OpenAPI document and are labelled as such in the test; replacing them by recorded replies is Part B.1's output.
8. A wrapper forwarding test exists for the router, `FakeHarnesses` and `Wired`; deleting a forward fails one of them.
9. `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load` is green (the `.aftersight/pipeline.config.json` command); `cargo clippy
   --workspace --all-targets -- -D warnings` clean on the files touched; ADR, `pane-control.md` and CHANGELOG carry the amendment.

### A5. Files
Production: `crates/holler-pane/src/{ports.rs, ports/compact.rs (new), ports/router.rs, tx_compact.rs (new), tx_switch/plan.rs (visibility only), lib.rs (module)}`,
`crates/holler-pane-testkit/src/{harness.rs, harnesses.rs, conformance/harness.rs, conformance/claude.rs, lib.rs}`,
`crates/holler-adapter-opencode/src/{compact.rs (new), lib.rs}`, `crates/holler-cli/src/pane/{compact.rs (new), mod.rs, wiring/bind.rs, wiring.rs}`.
`crates/holler-adapter-claude` is not touched. Docs: `docs/adr/ADR-0021.md`, `docs/pane-control.md`, `CHANGELOG.md`.
Tests: `crates/holler-cli/tests/pane_verbs/compact.rs` (new, registered in `pane_verbs/main.rs`), `crates/holler-pane/tests/` (engine), testkit tests
(`fake_harness_*`, `harness_conformance_test.rs` mutants, `claude_conformance_test.rs`), `crates/holler-adapter-opencode/tests/compact_test.rs` (new).
Reuse map (extend, do not duplicate): `check_idle` and `check_health` (E3); `resolve_pane_record` (E3); `activity_refusal`; `known()`, `Call::send`, `session_activity`,
`prompt::messages` and `unexpected_status` in the OpenCode adapter (E4); `HarnessInfo::endpoint`; the `switch.rs` verb shape and `emit_outcome`; the existing fixtures
support (`tests/support/{stub,fixtures}.rs`). No second poll, no second activity read, no second pane-state predicate.

### A6. Decisions already made
- Signature, outcome, the codes, default `compact-unsupported`, `holler pane compact PANE [--json]`, exit 0 only when the context shrank, no retry loop: the issue.
- `session-busy` is the existing constant. New codes: `compact-unsupported`, `compact-model-unpinned`. No code for "did not shrink": it is `unavailable` (exit 1).
- Compaction is a pane verb with its engine in `holler-pane`, beside `tx_switch`; it writes no record and calls no Herdr or host port (HTTP only for OpenCode), so I4
  reduces to a test that the fake Herdr saw nothing. The hub's control socket and the v2 wire are unchanged (verbs run CLI-side, ADR-0021 section 5).
- The pane-state refusals apply (`pane-parked`, `pane-unhealthy`, `pane-shown-driven-mismatch`), because a compaction is a model call on the session of record. The
  orchestrator guard of `switch`/`reset` does **not** apply (compaction keeps the session; the use case is an orchestrator compacting its agents). Both are defaults
  the operator can overturn at review.
- Out of scope: the Claude Code adapter's `compact` (Part B decides it); the generic ACP agent (N3, dropped); proposing `session/compact` to ACP; compaction before a
  new order (pfleet side); applying a pane's model to an OpenCode server (#829); a `compact` for a body or the hub; any `--all` or loop form of the verb.

### A7. Test plan
RED first, on the fakes: (1) the engine and verb tests of acceptance 1 to 5 fail because `HarnessPort` has no `compact` and the verb does not exist; (2) the conformance
cases fail against a `Mutant` that breaks each rule (answers `Ok` for an unknown session; sends when busy; returns `tokens_after` equal to `tokens_before`); (3) the
forwarding tests fail until the three wrappers forward; (4) the adapter tests of A3.5 fail until `compact` is written. Then GREEN with the minimum code. Final: the full
workspace test command above, and a doc sweep: `git grep -n "session_activity"` over `docs/` and the `bind.rs` doc for every place the default-armed list is restated
(ADR line 616, `bind.rs:277-280`); add `compact` there.

### A8. Risks
- **The stand-in replies may differ from a real server's.** Mitigation: Part B.1; the adapter's request and accepted status come from the OpenAPI document, and the adapter
  refuses anything else with a message that quotes no reply.
- **A default-armed method is not found by the compiler in a wrapper** (E1); the ADR chose required methods for reads for this reason (section 2, #835). The issue asks for
  a default. Mitigation: the forwarding tests (acceptance 8); if a reviewer prefers required, it is a compile-time change of the same size.
- **A busy-check race.** The engine reads activity, then calls; a turn can start in between. The adapter re-checks inside its call; the server's own answer for a busy session
  (B.1 step 6) decides whether more is needed.
- **Compaction is destructive** (the history becomes a summary). It runs only on a pane the operator names, never in a loop, and the spike runs on sessions it created.
- **An unintended model.** Mitigation: the port never picks one; no pin means `compact-model-unpinned` with nothing sent; the fake logs the model so a test can assert it.
- **A 120 s call holds the caller.** By decision (N4). The verb prints nothing until it ends; the ADR says so.
- **Public repo.** The issue, this brief, the evidence file, handoffs and the PR carry no host names, personal paths or fleet identifiers.

### A9. Hot-file rules
`docs/adr/ADR-0021.md` and `CHANGELOG.md` are edited by every story and conflict at their tails (the "Decisions taken" list, `[Unreleased]`). Rebase onto `origin/main` before
merge; on conflict **keep both sides**; the ADR decision number is **the next free number at merge time**. Never run `cargo fmt` workspace-wide; format only the files this
story touches (`rustfmt --edition 2021 <files>`). `docs/pane-control.md` section 12 and the ports table row (ADR line 111) are the other contested spots.

### A10. Clashes (running now)
- **#850** (a body driver for a Claude Code pane; amends ADR-0014 and ADR-0021 section 15). Overlap: ADR-0021 (Decisions tail, Deferred list at line 1648), CHANGELOG,
  `docs/pane-control.md` lines 60-85 and section 12. No code overlap in `holler-pane`.
- **#831 adoption step** (`pane import --adopt-claude-code`, a new `HostPort` method). Overlap: `crates/holler-cli/src/pane/wiring/bind.rs` and `wiring.rs` (both add forwarding
  methods in the same impls), `ports.rs` (a different trait, nearby hunks), the testkit `lib.rs` re-exports and `harnesses.rs`, ADR-0021 ports table rows 110-111, the
  "Failure modes by verb" table, the decision list. Resolve by rebasing last, keeping both sides, and re-running `cargo test --workspace` after the rebase.

## PART B: the Phase-0 spike (the operator's session; scratch instances only; not a pipeline phase; a later, separate step)

Runs after or alongside Part A. Decides **only the Claude adapter**: whether `compact` is built for Claude Code (its own brief and PR) or stays `compact-unsupported` for good.
Its OpenCode half confirms Part A. Never the live fleet: no live pane, server, tmux server, Herdr session or registry file is touched.

Output: `docs/research/compaction-spike.md` (add it to `docs/research/README.md`), plus probe scripts `scripts/spikes/opencode-compact.sh` and
`scripts/spikes/claude-compact.sh` built on `opencode-lib.sh` and `claude-lib.sh`. Each observation is one `PASS` / `FAIL` / `INFO` line, and every finding carries the
spike note's evidence label: **Verified** (observed by running), **Documented**, **Read**. Public repo: no host, user, pane, home-directory or key text anywhere in the file;
`<scratch-dir>` stands for a probe's temp directory. Reply bodies are pasted verbatim with ids replaced; they become `crates/holler-adapter-opencode/tests/fixtures/compact/`
(replacing Part A's stand-in replies) in a follow-up commit.

`scripts/spikes/claude-compact.sh` built on `opencode-lib.sh` and `claude-lib.sh`. Each observation is one `PASS` / `FAIL` / `INFO` line, and every
finding carries the spike note's evidence label: **Verified** (observed by running), **Documented**, **Read**. Public repo: no host, user, pane,
home-directory or key text anywhere in the file; `<scratch-dir>` stands for a probe's temp directory. Reply bodies are pasted verbatim with ids replaced; the
pipeline copies them into `crates/holler-adapter-opencode/tests/fixtures/compact/` (fixture-first: real artifacts, not hand-written ones).

**Rules for both probes (invariant I7 and the repo's secrets rule).** No key, token or login value is ever printed, put on an argv, or written to
a file; names of variables only. Anything sensitive goes in a file only after `git check-ignore -v <path>` confirms it is ignored. Compaction destroys
history, so it is run only on a session the probe created. No live pane, server, tmux server, Herdr session or registry file is touched; the probes
never read `HOME` state of the operator. Output reviewed before it is committed.

### B.1 OpenCode `summarize` (safe, no cost: a local stub model server, decided in N2)
Safety contract is the existing one in `scripts/spikes/opencode-lib.sh` (header, lines 1-25): `env -i`, `HOME` and `XDG_*` in a fresh `mktemp -d`, ports
from 48100-48199 on 127.0.0.1 only, a private tmux socket (`tmux -S`), an EXIT trap that kills only what the run started and removes the directory.
Its provider is a dead end that cannot answer a model call, which is not enough here: `summarize` needs a model reply. So the scratch config gets one
provider whose base URL is a stub on 127.0.0.1 started by the probe (OpenAI-compatible chat completions, streamed, with a fixed `usage` block, a
fixed summary text, a switch to hold a reply open, and a log of every request's `model` field). The probe refuses to continue unless the running
server's `GET /config` shows that stub as the only provider (the analogue of `oc_guard_no_model`).
1. Record `opencode --version` (the adapter was built against 1.19.0) and the verbatim `summarize` and `compact` route schemas from the server's own
   OpenAPI document (read-only GET).
2. Boot the scratch server under the guard. Create one session. Run three short turns against the stub so the session has context with known token counts.
3. **Where do the token counts come from?** Read candidates before and after and record which one tracks the context: the last assistant message's
   `info.tokens` in `GET /session/:id/message`, and the session object. Record the field names verbatim.
4. **Which model runs the summary, and does `GET /config` show the pin?** Read `GET /config` first and record the verbatim `agent.compaction` object that the scratch config pins. Then Call `POST /session/:id/summarize` (a) with an explicit `providerID`/`modelID`, (b) with the scratch config's
   `agent.compaction.model` pinned and the body naming a different model, (c) with no pin and a body, (d) with a body naming an unknown provider.
   The stub's request log is the proof of the model that ran. Record what `variant` does, and what `auto` means.
5. **Does the context shrink, and how is the reply shaped?** Record the status, the headers' content type, the body verbatim, how long the call
   took, whether it returns when the summary is finished or at once, the session id before and after (it must be unchanged), the TUI's title
   (private tmux, read-only `capture-pane`/title), and the message list after (the summary message).
6. **Busy session.** Hold the stub's reply open so a turn is in flight, call `summarize`: refused (which status/body), queued, or run on top. Record the
   stub's request count to prove which.
7. `POST /api/session/:id/compact`: same questions as 5 and 6, and how it differs from `summarize`.
8. **A failed summary.** Make the stub answer 500 mid-summary: is the history intact afterwards, and what does the call return.
9. Cleanup (the trap does it; verify): no process of the run left, the private tmux server gone, `<scratch-dir>` removed, nothing under the operator's
   real config touched.

### B.2 Claude Code `/compact` as a prompt (at most 3 `haiku` turns in total on the operator's login; decided in N2)
Follow the style of the #831 real-binary plan: it states what is asserted and what is unverified, then procedure, then rollback, then a results template.
The limits that plan found apply: the probe cannot use a scratch `HOME` (the login lives in the config directory under `HOME`), so isolation is a private
tmux socket, a fresh scratch working directory trusted by hand once, a scratch inbox-socket directory, a cleared environment, and a purge of the
scratch directory's transcripts afterwards.
- Run from a plain terminal, not from inside a Claude Code session (inside one, variables that change the login are set). Check by name only:
  `env | grep -o '^\(ANTHROPIC\|CLAUDE\)[A-Z_]*'` must print nothing; otherwise start the probe with `env -i` and an allow-list. Never print `env`
  unfiltered, never read the config directory.
- Version: record `claude --version` (the pane spike probed 2.1.296). The inbox frame is a **Read** interface that can drift per version.
- Model: the smallest (`--model haiku`), subscription login, no API-key variable set. Budget (the N2 cap, total across the whole probe, including any retry): at most 3 model turns (one seed turn "Reply with the
  single word: ok", the compaction's own summary call, one follow-up to see the conversation after). Subscription cost negligible. If a step would need a fourth turn, stop and record `INFO` instead.
1. Launch one process with the adapter's own argv shape (`--session-id`, `--settings` with `crossSessionInbound: accept`, the `SessionStart` hook that
   names the inbox socket), as `scripts/spikes/claude-drive.sh` does. Wait for `claude agents --json` to list it `idle`.
2. One seed turn through the inbox, so a conversation exists (compacting an empty one is probably refused, which would be a false negative).
3. Deliver `/compact` through the inbox (the adapter's `send_prompt` frame). Observe read-only: `claude agents --json` status values over time (is there a
   `compacting`? E5), the pane screen (`tmux capture-pane`), the transcript's size, the session id before/after, whether the model replied to it as ordinary text.
4. **Control (scratch only, a person's action, never an adapter action):** type `/compact` by hand into the scratch TUI, to see what a real compaction
   looks like in `agents --json` and the transcript, so step 3 can be judged. The adapter must never do this (I4).
5. **Can the token count be read without typing?** Record where, if anywhere (transcript usage fields). If there is no way to read before/after, the
   Claude adapter cannot return a `CompactOutcome` even if `/compact` works, and the answer is `compact-unsupported`.
6. Cleanup: kill the private tmux server; kill only pids tied to the run's start time (never `pkill claude`); purge the scratch directory's transcripts;
   confirm the temp directories are gone.

### What the evidence file must answer
| # | Question | Where the pipeline uses the answer |
|---|---|---|
| Q1 | Does `summarize` return when finished, or at once? How long? | confirms the 120 s bound and that no polling is needed (N4) |
| Q2 | Exact request body, reply status/body, route differences between `summarize` and `compact` | confirms Part A's request, accepted status and fixtures |
| Q3 | Which model runs the summary for each of the four calls of step 4 (a)-(d); what a missing or unknown model does | confirms the `compact-model-unpinned` rule and that the pin in `GET /config` is the model that ran (N1) |
| Q4 | Busy session: refused, queued or run | whether the adapter needs more than its own re-check |
| Q5 | Token source before/after; session id unchanged | confirms the token source Part A uses |
| Q6 | Failure: history intact or not | confirms the adapter's error mapping |
| Q7 | Claude: does `/compact` over the inbox compact; status value; token source | the Claude adapter, or `compact-unsupported` for good (the hand-off table) |

### Hand-off from Part B to the code
| Answer | Effect |
|---|---|
| B.1 confirms Part A (request, status, token source, `agent.compaction` in `GET /config`) | swap the stand-in fixtures for the recorded replies; nothing else |
| B.1 contradicts Part A | a fix commit on the OpenCode adapter (the contract in A2 is unchanged unless the ADR text is wrong) |
| B.2 shows `/compact` over the inbox works and a token source exists (Q7) | a new brief for the Claude adapter: `compact` writes `/compact` through the inbox frame, waits on the observed status, reads counts from the named source; `agents::activity` must first learn the `compacting` value |
| B.2 shows it does not work, or no token source | Claude Code stays `compact-unsupported`; the docs say why |

## Rejected (one line each)
New spec field plus flag (N1 B); required `--model` flag (N1 C); the pane's recorded main model (N1 D); polling after a start (N4 b); keeping the 10 s bound (N4 c);
a generic ACP agent (N3, dropped from #878).
