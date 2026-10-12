# Brief: #878 pane-compact

Repo: Performant-Labs/holler. Issue: #878 (part of epic #633; the operator says it gates release v0.5.0). Rigor: in-session. UI surface: no. Kind: feature.
Base: `origin/main` at ef881fc. Branch `issue-878-implementation`. Nothing in this brief was run against a real OpenCode or Claude Code.

**How this brief is organised.** The issue's first step is a spike and says "no code until it is read". So the work is split in two:

- **Phase 0, the spike.** Run by the operator's session, on scratch instances only, never by the pipeline. It produces one evidence file,
  `docs/research/compaction-spike.md`. Section "Phase 0" lists the exact safe steps.
- **The pipeline story.** Everything after: contract amendment, port, fakes, conformance, engine, CLI verb, OpenCode adapter, docs. Part 1 does
  not need the spike result, because the issue already fixes its design. Part 2 (the OpenCode adapter) reads the evidence file. Part 3 (Claude
  Code) exists only if the spike shows `/compact` works, and is a later step.

Launch condition for the pipeline: the operator has answered **N1** (where the compaction model comes from) in "Needs operator decision". N2 to N4
do not block Part 1. Part 2's phase F does not start until `docs/research/compaction-spike.md` is committed on `main` or in the worktree; if it is
missing the run stops with the reason `blocked-on-phase-0`, and does not guess a reply shape.

## Problem
There is no way to compact a pane's session without typing `/compact` into the TUI. Typing was seen to fail (2026-10-12): the text sat
unsubmitted in four OpenCode panes because the slash-command menu was open and Enter was not accepted, and the caller then waited in a loop for a
compaction that never started; the watchdog flagged it after about eleven minutes. ADR-0021 I4 already forbids typing into a TUI to change a
session; compaction is the verb still missing. OpenCode's server lists `POST /session/{id}/summarize` (body `providerID`, `modelID`, optional
`auto`) and `POST /api/session/{id}/compact` (OpenAPI of OpenCode 1.19.0, read 2026-10-12; a dry run only, no real compaction has run). The Agent
Client Protocol has no request that compacts. Whether Claude Code accepts `/compact` as a prompt is not verified.

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
`crates/holler-cli/src/pane/wiring.rs:381` (`Unwired`, answers `NotImplemented`). The two real implementations that override: OpenCode
`crates/holler-adapter-opencode/src/lib.rs:420-431`, Claude `crates/holler-adapter-claude/src/lib.rs:302-330`. The fakes: `FakeHarness`
(`crates/holler-pane-testkit/src/harness.rs:368`, overrides neither `send_prompt` nor `session_activity`, so it is always `Idle` today) and `FakeClaude`
(`crates/holler-pane-testkit/src/claude.rs:380,391`).

### E2. What exists for a "pinned compaction model" (nothing in Holler)
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
  already reads a running server's config: `crates/holler-adapter-opencode/src/server.rs:87-103` (`GET /config`, route `GET_CONFIG` at `lib.rs:483-486`).
- The spec flags are one per field (ADR-0021 section 3, table at lines 281-297); there is no flag for a compaction model.

ADR-0021 section 2 has no sentence about an unintended or paid model (no hit for "unintended", "paid" or "intended model" in the ADR). The nearest
rules are: the agent is applied as the server's `default_agent`, set when the server starts (decision 1, `ADR-0021.md:133`); `send_prompt` "naming no
agent" so the pane's own default runs it (`ADR-0021.md:627`); and the model of an OpenCode pane being recorded, not applied (E2 above). The issue's
reason stands on its own: a compaction is a model call, so it must not run on a model nobody chose.

### E3. The busy check, the codes, and the single place a verb refuses on activity
`crates/holler-pane/src/tx_switch.rs:109-141`: `SESSION_BUSY` ("session-busy"), `SESSION_HOLDS_QUESTION`, `SESSION_HOLDS_PERMISSION` are open codes,
and `activity_refusal(activity)` maps `Activity` to `(code, words)`. `crates/holler-pane/src/tx_switch/plan.rs:246-262`:
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
constants (`lib.rs:450-505`; `POST /session/:id/abort`, `GET /session/status`, `GET /session/:id/message`, `GET /config`). `send_prompt`
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

### E8. There is no ACP harness behind the port
`HarnessKind` has two variants, `Opencode` and `Claude` (`pane.rs:140-148`), and its doc says Claude is "Not the ACP body harness of the same id (ADR
0012), which only `holler-body` reads". `grep available_commands` finds nothing in `crates` or `docs`. So the issue's item 5 ("a generic ACP agent")
has no `HarnessPort` to live in.

## Phase 0: the spike (operator's session; scratch instances only; not a pipeline phase)

Output: `docs/research/compaction-spike.md` (add it to `docs/research/README.md`), plus probe scripts `scripts/spikes/opencode-compact.sh` and
`scripts/spikes/claude-compact.sh` built on `opencode-lib.sh` and `claude-lib.sh`. Each observation is one `PASS` / `FAIL` / `INFO` line, and every
finding carries the spike note's evidence label: **Verified** (observed by running), **Documented**, **Read**. Public repo: no host, user, pane,
home-directory or key text anywhere in the file; `<scratch-dir>` stands for a probe's temp directory. Reply bodies are pasted verbatim with ids replaced; the
pipeline copies them into `crates/holler-adapter-opencode/tests/fixtures/compact/` (fixture-first: real artifacts, not hand-written ones).

**Rules for both probes (invariant I7 and the repo's secrets rule).** No key, token or login value is ever printed, put on an argv, or written to
a file; names of variables only. Anything sensitive goes in a file only after `git check-ignore -v <path>` confirms it is ignored. Compaction destroys
history, so it is run only on a session the probe created. No live pane, server, tmux server, Herdr session or registry file is touched; the probes
never read `HOME` state of the operator. Output reviewed before it is committed.

### 0.A OpenCode `summarize` (safe, no cost with a local stub; see N2)
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
4. **Which model runs the summary?** Call `POST /session/:id/summarize` (a) with an explicit `providerID`/`modelID`, (b) with the scratch config's
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

### 0.B Claude Code `/compact` as a prompt (small, real cost; needs N2 consent)
Follow the style of the #831 real-binary plan: it states what is asserted and what is unverified, then procedure, then rollback, then a results template.
The limits that plan found apply: the probe cannot use a scratch `HOME` (the login lives in the config directory under `HOME`), so isolation is a private
tmux socket, a fresh scratch working directory trusted by hand once, a scratch inbox-socket directory, a cleared environment, and a purge of the
scratch directory's transcripts afterwards.
- Run from a plain terminal, not from inside a Claude Code session (inside one, variables that change the login are set). Check by name only:
  `env | grep -o '^\(ANTHROPIC\|CLAUDE\)[A-Z_]*'` must print nothing; otherwise start the probe with `env -i` and an allow-list. Never print `env`
  unfiltered, never read the config directory.
- Version: record `claude --version` (the pane spike probed 2.1.296). The inbox frame is a **Read** interface that can drift per version.
- Model: the smallest (`--model haiku`), subscription login, no API-key variable set. Budget: at most 3 model turns (one seed turn "Reply with the
  single word: ok", the compaction's own summary call, one follow-up to see the conversation after). Subscription cost negligible.
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

### What the evidence file must answer (the pipeline reads these as inputs)
| # | Question | Where the pipeline uses the answer |
|---|---|---|
| Q1 | Does `summarize` return when finished, or at once? How long? | the adapter's bound and whether it polls (N4) |
| Q2 | Exact request body, reply status/body, route differences between `summarize` and `compact` | the adapter's request and its fixtures |
| Q3 | Which model runs the summary for each of 4(a)-(d); what a missing or unknown model does | N1 and the `compact-model-unpinned` rule |
| Q4 | Busy session: refused, queued or run | whether the adapter relies on the server's refusal or only on the engine's pre-check |
| Q5 | Token source before/after; session id unchanged | `CompactOutcome` and the shrink check |
| Q6 | Failure: history intact or not | the adapter's error mapping |
| Q7 | Claude: does `/compact` over the inbox compact; status value; token source | Part 3 or `compact-unsupported` for good |

## The pipeline story

### Part 1: contract, port, fakes, conformance, engine, CLI (independent of the spike)
**Commit 1 (docs only, the amend-first commit; ADR-0021 text says a contract change goes in its own PR, so the commit order is the review boundary and
the PR may be split after it if the reviewer asks).** `docs/adr/ADR-0021.md`: add `compact` to the verb list (line 250 and the "Failure modes by verb"
table at 872-877: a new row `pane compact`); the `HarnessPort` row of the ports table (line 111) gets `compact`; a new paragraph "Compaction (#878)"
after "The prompt and activity seam" (line 616-643) with the signature, the three codes, the shrink rule and the ACP rule (below); "Deferred to named
stories" (line 1648) gets the ACP and Claude Code entries; a "Decisions taken" entry at **the next free number at merge time** (the list ends at 49 at
`ADR-0021.md:1830`; do not hard-code a number). `docs/pane-control.md`: the verb list (lines 150-151), a `### holler pane compact PANE` section between
`reset` (276) and `park` (301), and section 12 "Later" (line 1072). `CHANGELOG.md` `[Unreleased]` / `### Enhancements` (lines 8-10).
Commit 1 declares the contract below; commits 2 on write tests (RED), then code.

**The contract (what commit 1 states):**
- New file `crates/holler-pane/src/ports/compact.rs` (beside `process.rs`, `agent_view.rs`), re-exported from `ports.rs`: `CompactOutcome { tokens_before: u64,
  tokens_after: u64 }`; `CompactModel { provider: String, model_id: String }` (a name pair, never a secret, I7; it carries no effort, see N1);
  `COMPACT_UNSUPPORTED = "compact-unsupported"` and `COMPACT_MODEL_UNPINNED = "compact-model-unpinned"`, open codes, refusal, exit 3, declared with
  `RefusalCode::from_static`. `session-busy` is `tx_switch::SESSION_BUSY` (E3), not redefined.
- `HarnessPort::compact(&self, at: &HarnessEndpoint, session: &str, model: Option<&CompactModel>) -> Result<CompactOutcome, PaneError>`, default-armed,
  refusing `compact-unsupported`. `model: None` means "use the harness's own configured compaction model if it has one" and is how N1's option A is
  expressed; with a `Some` the harness uses exactly that model. A harness never runs a compaction on a model that neither the caller nor the harness's own pin
  named: it refuses `compact-model-unpinned`. (The issue's `compact(at, session, model)` is this signature; `Option` is what lets one signature serve every
  N1 answer.) Contract rules: a busy session is refused `session-busy` and the harness sends no summarize; an unknown session is `session-not-found` and
  nothing is posted (as `abort` and `send_prompt` do); it never types into a TUI (I4); `tokens_after` is read from the harness after the act, never
  inferred; a count the harness cannot read is `unavailable`, never zero.
- The engine `holler_pane::tx_compact` (new file `crates/holler-pane/src/tx_compact.rs`; no CAS write, a compaction records nothing in the registry): plan
  (the record and the pane-state refusals; the model per N1; `check_health`; `check_idle`) then act (`compact`) then observe: the outcome must satisfy
  `tokens_after < tokens_before`, otherwise `unavailable` (exit 1), the message naming both counts and saying the context did not shrink. This is the
  decision of the I3 mismatch rule (a failure observed after the act is `unavailable`, not an open code). `check_idle` and `check_health` become
  `pub(crate)` in `tx_switch::plan` and are called, not copied (reuse map).
- CLI `holler pane compact PANE [--profile P] [--json]` (`crates/holler-cli/src/pane/compact.rs`, struct `PaneCompact`). Text: `compacted <pane>: <before> -> <after> tokens`.
  JSON `data`: `{"pane","session","tokens_before","tokens_after"}` (additive; `schema_version` stays 1). Exit 0 only when the context shrank; 3 for a refusal;
  1 for no shrink or a failure; 2 for usage. No retry loop. No flag in Part 1 (see N1 for a possible `--model`).
- The ACP rule is written into the ADR and `docs/pane-control.md` as a rule for a future harness, with no code (E8, N3).

**Work in Part 1 (T authors RED first on the fakes):**
1. `FakeHarness` learns `compact`: `HarnessOp::Compact` (`"harness.compact"`, fault-injectable like the others), a seedable context size and "compacts to",
   a seedable harness-side compaction pin (for `model: None`), an accessor returning the compact calls `(port, session, model)`, and a settable
   `session_activity` so a busy session can be seeded (today it is always `Idle`, E1). `FakeHarnesses` and `Wired` forward `compact`; the router forwards by endpoint.
2. Forwarding tests: a test per wrapper (`HarnessRouter`, `FakeHarnesses`, `Wired`) that a `compact` reaches the inner port and is not the default. This is the
   guard against the silent-default trap in E1.
3. Conformance: new cases in `conformance/harness.rs` (17 becomes 17 + n; update the count wherever it is asserted) such that an adapter passes or refuses by an
   open code: unknown session is `session-not-found`; unserved port is `unavailable` checked first; a call either returns a `CompactOutcome` with
   `tokens_after < tokens_before` or is refused `compact-unsupported` / `compact-model-unpinned`; a busy session is `session-busy` and nothing was sent
   (where the rig can make one busy; otherwise the fake's own tests pin it). One Claude case (`conformance/claude.rs`, 13 becomes 14): `compact` is
   `compact-unsupported` unless Part 3 is built. Each new case gets a mutant.
4. Engine and verb tests (below).

### Part 2: the OpenCode adapter (reads the evidence file)
`crates/holler-adapter-opencode`: a new `compact.rs` (beside `prompt.rs`), `impl HarnessPort ... fn compact` in `lib.rs` beside `send_prompt`, new `Route`
constants for `summarize` (and `compact`, whichever Q2 selects). Order inside the call, each within the call's bound: `known(...)` (unknown session
`session-not-found`); activity (busy refused `session-busy`, a pending question `session-holds-question`, using the existing `session_activity`);
resolve the model (`Some` as given; `None` from the server's `GET /config` `agent.compaction` if N1 is A; otherwise `compact-model-unpinned`, nothing sent);
read `tokens_before`; `POST summarize` with the body Q2 fixes; accept only the status Q2 fixes (any other named in a message that quotes no reply, as
`send_prompt` does); read `tokens_after` the way Q5 fixes. Tests replay the evidence file's verbatim replies through `tests/support/stub.rs`; the
real call is covered by that recorded evidence and not by a hand-written server (issue acceptance). The bound (N4) is a field of `Timeouts`, never a
literal in the call.

### Part 3: the Claude Code adapter (only if Q7 says `/compact` works; a separate, later step)
Default stays `compact-unsupported` (the adapter inherits the default; add an explicit override that names the reason only if the conformance case needs
one). If the spike shows it works and a token source exists: `compact` writes `/compact` through the existing inbox frame, waits on the status the
spike observed (the `compacting` value must first become known to `agents::activity`, which today refuses it `unavailable`, E5), and reads the counts
from the source Q7 names. That is its own brief, its own PR, and updates `FakeClaude` and the Claude conformance case. Until then the Claude pane
answers `compact-unsupported` and the docs say so.

## Acceptance criteria (from the issue, each observable)
1. `holler pane compact` on a fake harness that compacts: exit 0, prints both token counts; the fake logged exactly one `compact` call, and its model is
   the pinned one. Test `compact_prints_before_and_after_and_the_fake_saw_the_pinned_model` in `crates/holler-cli/tests/pane_verbs/compact.rs`.
2. A busy session: exit 3 with `session-busy`, and the fake logged no `compact` call (`compact_refuses_a_busy_session_and_sends_nothing`).
3. A pane with no pinned compaction model: exit 3 with `compact-model-unpinned`, no call logged. A harness that cannot compact (the Claude fake): exit 3
   with `compact-unsupported`.
4. The context did not shrink (fake returns `tokens_after >= tokens_before`): exit 1, the message says the context did not shrink and names both counts;
   `--json` gives `ok:false`.
5. No keystroke: the fake Herdr records no `send_text`/`send_keys` during any of the above (I4).
6. The conformance suite (`run_harness_conformance`) has the `compact` cases, FakeHarness passes them, each mutant fails its case:
   `cargo test -p holler-pane-testkit` and `grep -c compact crates/holler-pane-testkit/src/conformance/harness.rs` is non-zero.
7. The OpenCode adapter's `compact` is covered by tests that replay `docs/research/compaction-spike.md`'s verbatim replies; no hand-written reply shape.
8. A wrapper forwarding test exists for the router, `FakeHarnesses` and `Wired`; deleting a forward fails one of them.
9. `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load` is green (the `.aftersight/pipeline.config.json` command);
   `cargo clippy --workspace --all-targets -- -D warnings` clean on the files touched; ADR, `pane-control.md` and CHANGELOG carry the amendment.

## Files
Production: `crates/holler-pane/src/{ports.rs, ports/compact.rs (new), ports/router.rs, tx_compact.rs (new), tx_switch/plan.rs (visibility only), lib.rs (module)}`,
`crates/holler-pane-testkit/src/{harness.rs, harnesses.rs, conformance/harness.rs, conformance/claude.rs, lib.rs}`,
`crates/holler-adapter-opencode/src/{compact.rs (new), lib.rs}`, `crates/holler-cli/src/pane/{compact.rs (new), mod.rs, wiring/bind.rs, wiring.rs}`.
`crates/holler-adapter-claude` only in Part 3. Docs: `docs/adr/ADR-0021.md`, `docs/pane-control.md`, `docs/research/{compaction-spike.md, README.md}`, `CHANGELOG.md`.
Tests: `crates/holler-cli/tests/pane_verbs/compact.rs` (new, registered in `pane_verbs/main.rs`), `crates/holler-pane/tests/` (engine), testkit tests
(`fake_harness_*`, `harness_conformance_test.rs` mutants, `claude_conformance_test.rs`), `crates/holler-adapter-opencode/tests/compact_test.rs` (new) and
`tests/fixtures/compact/`.
Reuse map (extend, do not duplicate): `check_idle` and `check_health` (E3); `resolve_pane_record` (E3); `activity_refusal`; `known()`, `Call::send`,
`session_activity` and `unexpected_status` in the OpenCode adapter (E4); `HarnessInfo::endpoint`; the `switch.rs` verb shape and `emit_outcome`; the existing
fixtures support (`tests/support/{stub,fixtures}.rs`). No second poll, no second activity read, no second pane-state predicate.

## Decisions already made
- Signature, outcome, the three codes, default `compact-unsupported`, `holler pane compact PANE [--json]`, exit 0 only when the context shrank, no retry
  loop: the issue. `Option<&CompactModel>` in place of `&CompactModel` is this brief's one refinement (see the contract); say so in the ADR.
- `session-busy` is the existing constant. New codes: `compact-unsupported`, `compact-model-unpinned`. No code for "did not shrink": it is `unavailable`
  (exit 1) by the I3 rule.
- Compaction is a pane verb with its engine in `holler-pane`, beside `tx_switch`; it writes no record and calls no Herdr or host port (HTTP only for
  OpenCode), so I4 reduces to a test that the fake Herdr saw nothing. The hub's control socket and the v2 wire are unchanged (verbs run CLI-side, ADR-0021
  section 5).
- The pane-state refusals apply (`pane-parked`, `pane-unhealthy`, `pane-shown-driven-mismatch`), because a compaction is a model call on the session of
  record. The orchestrator guard of `switch`/`reset` does **not** apply (compaction keeps the session; the use case is an orchestrator compacting its agents).
  Both are defaults the operator can overturn at review.
- Part 3 (Claude) and the ACP rule are not built here. Out of scope per the issue: proposing `session/compact` to ACP; compaction before a new order
  (pfleet side). Also out of scope: applying a pane's main model to an OpenCode server (#829).

## Needs operator decision
**N1. Where the compaction model comes from.** Today Holler has no field for it (E2). The issue says "the model the pane's profile pins", which does
not exist. The fleet already pins it in OpenCode's own config, `agent.compaction.model` (written by the restart script). The refusal rule is the same in
all options: no named model, no compaction (`compact-model-unpinned`). Options:
- **A. Ask the running server for the pin it already has.** To someone running the fleet: no new setting, no re-launch, panes already pinned by the restart
  script work at once; a pane with no pin in OpenCode's config is refused. Holler cannot show or diff the pin (`profile show` will not mention it), and it
  depends on OpenCode's config shape. Contract change: none to the record, flags or wire; the port's `model` is `None`. The adapter reads `GET /config`.
- **B. A new spec field plus a flag, `--compaction-model PROVIDER/ID`.** Holler becomes the source of truth and `profile show` and drift see it. Cost: a new
  optional field on `Pane` and `ProfileSpec` (both reject unknown fields, so an older binary cannot read a newer record), the profile file format,
  `profile_diff`, `profile_snapshot`, the import table, and every existing pane is refused until its profile is re-applied. It also creates a second place
  to pin the same fact the restart script already writes, against the one-source-of-truth rule. This is the large option and the one most exposed to #831 and #850.
- **C. A required flag on the verb, `holler pane compact PANE --model PROVIDER/ID`.** Nothing stored; the caller (the orchestrator) names the model each
  time. Contract change: one flag on the CLI, `model` is `Some`; no record change. Risk: the caller can name a model different from the pane's own
  pin, which is the failure the issue wants to avoid.
- **D (not recommended). Use the pane's recorded main model.** No new anything, but for an OpenCode pane that value is recorded and not applied (#829), so
  it can differ from what the server runs: the unintended-model risk exactly.
- **Recommendation: A** for v0.5.0, with B left as a later additive step if the fleet wants Holler to own the pin. Reason: smallest blast radius, no
  data migration, no collision with #831/#850, and it keeps the pin where the fleet already maintains it.
  The brief's Part 1 holds for every option; only the model-resolution step in `tx_compact` and, for B or C, one flag differ.

**N2. The spike's model and cost** (Phase 0). OpenCode: (i) a local stub provider on 127.0.0.1 (recommended: no cost, no key, deterministic token counts and a
request log that proves which model ran; costs about an hour to write), (ii) the cheapest hosted model with a scratch key, real cost per turn and a key to
protect, or (iii) a self-hosted model endpoint if you run one, free but its token counts and speed are not deterministic. Claude Code: consent to at most
3 `haiku` turns on the subscription login (recommended; negligible), or skip the Claude probe and ship `compact-unsupported` for Claude Code now.

**N3. The ACP item (issue item 5) has nowhere to land** (E8): no pane runs an ACP agent through `HarnessPort`. Options: (a) write the rule in the ADR and pane
docs as a rule for a future harness and file a follow-up issue (recommended; nothing to build, nothing to test); (b) add an ACP harness kind, a large change
of its own that amends ADR-0012 and the registry; not for v0.5.0.

**N4. How long a compaction may take** (I5 says a port call returns within 10 s by default). A summary is a model call and may outlast that. Options:
(a) a separate longer bound for `compact` (a new `Timeouts` field; ADR-0021 section 2 lists `compact` as a third method bounded otherwise; the CLI call
simply takes longer), (b) the call starts the compaction and returns, and the verb polls until the context shrank or a deadline (more moving parts, needs
the server to report progress), (c) keep 10 s and fail with `timeout` on a slow compaction (a compaction that did start would then be reported as failed).
Recommendation: decide after Q1; if `summarize` blocks until finished, (a) with a default of 120 s. Until answered, Part 1 uses the named field and
changes nothing else.

## Out of scope
Proposing `session/compact` to ACP; compaction before a new order; applying the pane's model to an OpenCode server (#829); a `compact` for a body or
the hub; any `--all` or loop form of the verb; the Claude Code adapter's `compact` unless the spike shows it works (Part 3).

## Test plan
RED first, on the fakes: (1) the engine and verb tests of acceptance 1 to 5 fail because `HarnessPort` has no `compact` and the verb does not exist;
(2) the conformance cases fail against a `Mutant` that breaks each rule (answers `Ok` for an unknown session; sends when busy; returns `tokens_after`
equal to `tokens_before`); (3) the forwarding tests fail until the three wrappers forward. Then GREEN with the minimum code. Part 2's RED: the adapter
tests, replaying the evidence file's replies, fail until `compact` is written; they also assert the requests the stub saw (the `known` GET, then one POST,
nothing else) and that a busy session sent no POST. Final: the full workspace test command above, and a doc sweep: `git grep -n "session_activity"` over `docs/` and the `bind.rs` doc
for every place the default-armed list is restated (ADR line 616, `bind.rs:277-280`); add `compact` there.

## Risks
- **The spike changes the shape.** Summarize may block, may be refused when busy, may need `variant`, may report tokens elsewhere. Mitigation: the adapter is Part 2, written from the evidence file; Part 1 does not depend on it.
- **A default-armed method is not found by the compiler in a wrapper** (E1); the ADR chose required methods for reads for this reason (section 2, #835). The issue
  asks for a default. Mitigation: the forwarding tests (acceptance 8); if a reviewer prefers required, it is a compile-time change of the same size.
- **A busy-check race.** The engine reads activity, then calls; a turn can start in between. The server's own answer for a busy session (Q4) decides whether the adapter adds a second guard.
- **Compaction is destructive** (the history becomes a summary). It runs only on a pane the operator names, never in a loop, and the spike runs on sessions it created.
- **An unintended model.** Mitigation: the port never picks one; no pin means `compact-model-unpinned` with nothing sent; the fake logs the model so a test can assert it.
- **Claude Code's `compacting` status** is unknown to the adapter today and would be `unavailable` (E5); only relevant if Part 3 is built, and must be fixed there.
- **Public repo.** The issue, this brief, the evidence file, handoffs and the PR carry no host names, personal paths or fleet identifiers; the evidence file is read before commit.

## Hot-file rules
`docs/adr/ADR-0021.md` and `CHANGELOG.md` are edited by every story and conflict at their tails (the "Decisions taken" list, `[Unreleased]`). Rebase onto
`origin/main` before merge; on conflict **keep both sides**; the ADR decision number is **the next free number at merge time**, never the one seen when
writing. Never run `cargo fmt` workspace-wide (it rewrites unrelated files); format only the files this story touches (`rustfmt --edition 2021 <files>` or
`cargo fmt -- --check` to inspect). `docs/pane-control.md` section 12 and the ports table row (ADR line 111) are the other contested spots.

## Clashes (running now)
- **#850** (a body driver for a Claude Code pane; holler-body and the hub; amends ADR-0014 and ADR-0021 section 15). Overlap: ADR-0021 (Decisions tail,
  Deferred list at line 1648, the section-15 text our Claude paragraph points to), CHANGELOG, `docs/pane-control.md` lines 60-85 and section 12 (it removes
  "does not reach a Claude Code pane yet"). No code overlap in `holler-pane`; `holler-body` and the hub are untouched here.
- **#831 adoption step** (`pane import --adopt-claude-code`, a new `HostPort` method `foreground_pids`; host adapter, testkit and wiring). Overlap:
  `crates/holler-cli/src/pane/wiring/bind.rs` and `wiring.rs` (both add forwarding methods in the same impls), `crates/holler-pane/src/ports.rs` (a
  different trait, nearby hunks), the testkit `lib.rs` re-exports and `harnesses.rs`, ADR-0021 ports table rows 110-111 (adjacent rows), the "Failure modes
  by verb" table, and the decision list. Resolve by rebasing last, keeping both sides, and re-running `cargo test --workspace` after the rebase.
