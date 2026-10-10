# Decision journal — #645 part 2 (in-session)

## 2026-10-10 ~12:55 AM MDT — Phase 0/1 (survey-brief), O

- **Run opened.** Issue #645 (epic #633, wave 3), rigor `in-session` (issue + operator), no UI
  surface → D and U N/A. Pre-flight PASS (15/15) at `<primary>`; the provisioner **resumed** the
  stale by-hand-era worktree path `<run-worktree>` and re-pointed it to the plugin branch
  `issue-0645-switch-reset` at main's tip — the old era's branch and state were gone; zero stray
  files (first odd thing this run; resolved cleanly).
- **Survey finding — the story is one bullet wide.** Part 1 (a sibling pane, `bff4dbe`, "#645 part
  1 of 2") delivered the session-switch transaction (`tx_switch`), both verbs' surface and —
  verified test-by-test — every acceptance bullet the frozen contract permits: deleted-session
  switch changes nothing; named refusals incl. `orchestrator-pane`, `server-unhealthy`,
  `pane-not-in-profile`, `pane-not-found`; the I4 no-keystroke assertion on both verbs (the shared
  `both` runner); envelope + exit parity on every case in both formats.
- **The remaining bullet is port-blocked.** `--first TEXT` on reset (queue the first message to
  the new session, in the transaction) and the not-idle / held-question refusals need a
  `HarnessPort` the fixed epic contract does not list; `ports.rs` is frozen to #637's slice; the
  harness fakes are #684's; the real HTTP prompt API is unverified (the #635 spike could not run
  it without a model call) and the real adapter is #642 part 3 (open). Part 1 documented exactly
  this deferral at `tx_switch.rs:41-42`. The epic's own integration scenario assigns "reset with a
  first message" to #649.
- **OPERATOR DECISION (threshold escalation, this run): surgical carve-out, the #700 precedent.**
  The epic gains ONE exception: `HarnessPort` may grow **default-armed** seam methods inside
  #645's radius — `send_prompt` and `session_activity` — so no other impl breaks (the only in-tree
  `HarnessPort` impls are test doubles and the CLI's `Unwired` placeholder; a default-armed method
  compiles for all of them untouched). #645 part 2 delivers `--first`, the "wrong session"
  incident test, and the busy/held-question refusals; the **real** HTTP impl stays with #642
  part 3 and the **testkit fake's** recording with #684 (this run fakes through its own test
  doubles, the `WriterInSelect` wrapper pattern already in the verb's test file).
- **Scope reading flagged for override:** the operator's chosen option left the busy/held-question
  refusals as decidable "in the follow-up"; this run **includes** them (the issue's scope paragraph
  names them explicitly, "as pfleet epic #266 story 271 demands of reset") so one run closes #645
  complete. A reviews the shape; the operator may trim to `send_prompt` only.
- **Boundaries (issue blast radius + the carve-out).** F: `holler-pane/src/ports.rs` (two
  default-armed methods ONLY — the carve-out's whole extent), `holler-pane/src/tx_switch.rs`,
  `holler-cli/src/pane/reset.rs`, `holler-cli/src/pane/switch.rs`, plus this verb's own rows in
  `ADR-0003.md` and `tests/fixtures/cli-surface.txt` (the one-verb-one-file ruling; part 1 set the
  precedent touching both for its rows). T: `tests/pane_verbs/reset.rs` and `switch.rs` (their
  shared runner) only. NOT touched, ever: the testkit (#684's), `adapter-opencode` (#642's),
  `pane/wiring.rs` (#649's), `doctor/**` incl. `doctor/rig.rs` (#647's), `launch/**` (#644's),
  `cli.rs`/`main.rs` (#670's), `Cargo.toml` (any). No mutation testing; one cargo at a time;
  never a live fleet, a running pane or a real session; never print or commit a credential.
- **Standing rules applied:** handoffs in this PUBLIC repo carry placeholders only (`<primary>`,
  `<run-worktree>`, "a sibling pane"); first real commit as soon as T's tests are staged; rebase
  on origin/main before the PR; on green the pane self-merges per the in-session rule.

## 2026-10-10 ~1:10 AM MDT — architecture-review: PASS (1 warn, 5 notes), O

- **A passed the carve-out plan.** Verified: all nine in-tree `HarnessPort` impls compile
  untouched with default-armed methods; prompt-after-record is the issue's own order and the only
  one consistent with I2; the activity check slots after `check_health`/`refuse_orchestrator`,
  gated `Target::Fresh` + a current session-of-record, switch path untouched; the trait default
  bypasses the fake's call log so part 1's op-sequence tests stay green; default=idle correct.
- **W-1 applied before the record:** F's doc boundary now includes ADR-0021's switch/reset rows
  (the `PROPOSED (645b)` codes row, the deferred item, §8's seven-port-calls/no-prompt line) —
  this story's rows, part 1's precedent. N-1 applied: the carve-out wording names the closed
  `Activity` enum and the default's `RefusalCode` const.
- **Carried to T/F (a-dup re-checks):** N-2 the prompt-failure-after-record is a THIRD
  `SwitchFailure` decoration (never `created`/`acted`); N-3 the `prompt-unsupported` default's
  departure from the `not-implemented` convention is named in the ADR amendment; N-4 T's RED is
  compile-level (wrappers override not-yet-existing methods → the whole `pane_verbs` binary is
  RED — honest, journaled with evidence) and `switch.rs:648`'s no---first pin flips; N-5 the
  no-session-of-record case is proven with a wrapper that answers busy for any query and asserts
  no call happened.

## 2026-10-10 (morning) — Phase 4, T-red: the suite, the contract names, an environment hazard, O+T

- **Decided (T names the contract, none existed):** the stable refusal codes are
  **`session-busy`** and **`session-holds-question`** (the ADR-0021 PROPOSED 645b row's own
  wording, "not idle or holds a question"); the `Activity` enum's pinned variants are
  **`Idle | Busy | HoldingQuestion`**; the honest-failure message carries the brief's own
  phrase **"first message did not land"**; tests import `holler_pane::ports::Activity` (no
  root re-export — `lib.rs` is outside every boundary). The "no port call added" reading of
  AC 1 is **`send_prompt`-only**: A's approved gate is `Target::Fresh`-scoped (it queries on
  every reset with a session of record, `--first` or not), so the `--first`-absent case pins
  an empty prompt log and part 1's exact fake-log sequences, not a zero-activity-query run.
- **Decided (engine-side unit test NOT added):** `holler-pane` has no dev-dependency on the
  testkit (manifest is Never), a hand-rolled double set would be the second-`FakeHarness`
  shape a-dup forbids, and part 1 shipped zero unit tests beside `tx_switch` — the ordering
  pin rides the CLI wrapper's pane-store snapshot at prompt time instead.
- **Assumed:** `SeamHarness` (one wrapper, WriterInSelect-shaped: delegate the eight, override
  the seam, answer-activity-from-a-table, fault-prompt-optionally) is the right single double
  for all seam cases; if F's `Activity` answer needs richer shapes the wrapper grows a table,
  not state.
- **Hedged:** none of the nine cases can be shown "failing at runtime" individually — the RED
  is compile-level by design (N-4); the six-error rustc set IS the evidence. Clippy on the
  touched target is deferred to T-green (cannot run against a compile-RED binary).
- **Evidence:** baseline `cargo test -p holler-cli --test pane_verbs` green (219/0) at
  `3d95aec`; after the tests, six errors, each naming `Activity`/`send_prompt`/
  `session_activity`; `cargo test -p holler-pane --lib` still 12/0.
- **ENVIRONMENT HAZARD (O and F must read):** this worktree's `target` symlinks to
  `<primary>`'s `target`, shared with sibling sessions. Mid-run a sibling's build of an
  unmerged branch (`ProfileSpec.opencode_agent` — absent at `origin/main`, which equals this
  run's HEAD) poisoned the cache and injected a phantom `E0063` into an untouched file
  (`launch/rig.rs`). All RED evidence was captured with a run-private
  `CARGO_TARGET_DIR=<cache>/tmp/opencode/0645-target`; **F's builds and the plugin's stage
  runs need the same isolation (or a forced rebuild of `holler-pane` from this tree) until
  the sibling artifact is displaced.**

## 2026-10-10 ~1:25 AM MDT — t-red authored: honest compile-RED; shared-target hazard, O

- **T authored the suite** (2 cases + seam doubles in switch.rs, 5 cases + `reset_over_seam` in
  reset.rs, the `:648` surface pin flipped; baseline 219/0 green captured first). RED is
  **compile-level by design** (A N-4): the wrappers override `send_prompt`/`session_activity`,
  which no trait carries — six rustc errors name the seam; the whole `pane_verbs` binary measures
  RED. T names the refusal codes the docs never fixed: `session-busy`, `session-holds-question`.
  No engine unit test beside `tx_switch` — deliberate: `holler-pane` has no testkit dev-dep (a
  manifest edit is Never) and hand-rolled doubles would be the second-fake shape a-dup forbids.
- **Second odd thing: phantom E0063 through the shared target.** The worktree's `target` symlinks
  the primary's warm cache; a sibling session's unmerged `ProfileSpec.opencode_agent` artifact
  made UNTOUCHED `launch/rig.rs` fail E0063 mid-run. **Isolation decision:** every plugin suite
  run and F's stage in this run carry `CARGO_TARGET_DIR=<cache>/tmp/opencode/0645-target` (T's
  warm dir) until the sibling artifact is displaced. Env-inheritance reaches the plugin's cargo
  and F's children.
- **First real commit landed immediately after staging** (standing rule): the suite + handoffs.
