# [feat(pane): holler pane import leaves panes Holler does not drive alone, and --adopt-claude-code adopts them](https://github.com/Performant-Labs/holler/issues/835)

Part of epic #633 (amended 2026-10-11). Follows the operator decision recorded on #654 (the Claude Code panes the import would adopt), option (b), changed to a default plus a switch.

## Problem
`holler pane import` registers every pane directory under the state root as an OpenCode pane and has no way to leave a pane out. Some fleet panes run a Claude Code CLI session instead of an OpenCode server, and the old per-pane files do not say so: they look identical to OpenCode panes. A real import would record those panes with the wrong harness. #654 lists this as an open operator decision before the real import.

## Work
1. **Default: leave a pane Holler does not drive alone.** `pane import` does not register a pane whose harness is not OpenCode. It reports an info finding for it (a new finding code, named in the contract amendment) and writes nothing for that pane. This is not a block: the import proceeds for the other panes, and a dry run shows the same finding. A pane left alone shows as `unregistered-herdr-pane` in a whole-fleet `doctor` pass, which is expected.
2. **Switch: `--adopt-claude-code`.** With it, the import also adopts those panes. Until the Claude Code harness (#831) has merged, the switch is refused as `usage` (exit 2), naming #831, and nothing is written. After #831 it registers them with that harness.
3. **How the import tells the harness apart is NOT decided here.** The old files cannot tell, and `HostPort::ps` returns process ids only (no command line). The first step of the story is the contract amendment (the epic's amend-first rule) that adds the read-only signal, for example the pane's foreground command name, and the rule that uses it. A pane whose OpenCode server is merely down must still be adopted and flagged `server-down`, not skipped; so health on the recorded port is not the signal.
4. **Surface.** The flag is added to the verb's `cli-surface.txt` line and to ADR 0003's row for this verb. `--format=json` shows the new finding like every other finding.

## Fixture
A scratch state root with one OpenCode pane directory and one pane directory whose pane runs a different foreground command, on fakes from the test kit: default run records the first and reports the second as left alone; with `--adopt-claude-code` before #831, exit 2 naming #831; a dry run records nothing in both cases.

## Pipeline
`rigor: in-session`. Wave 1 of this amendment. Builds on #650 (merged). Needs no real ports, so it does not wait for #649. Not a dependency of #654 while option (a) of #654 (an operator-made state root of OpenCode panes only) stays available as the fallback; if the operator chooses to wait for this story instead, it becomes one.

## Rules
See the epic (#633): public repository (no personal host, tailnet or account names), the amend-first rule for the contract, `in-session` rigor, no story touches a live fleet, a real Herdr session, a running pane or a real Claude or OpenCode session: verification is the test kit and scratch instances. A subscription login is never read, copied or logged; env var NAMES only (invariant I7).

Filed by an AI agent (Claude Code) at the operator's direction, 2026-10-11, from the real-fleet dry run of `pane import` that showed Claude Code panes cannot be told apart from OpenCode panes in the old files.

---

## Contract amendment (amend-first; resolves the BLOCK in `docs/handoffs/835/handoff-A.md`)

Everything above this line is the issue, verbatim, and stays the source of truth. This section is the contract amendment the issue's Work item 3 puts first. It decides what the issue leaves open, in the order of handoff-A's findings. Where this section and the issue differ, it says so and why; the issue wins everywhere it is explicit. The amendment lands first: ADR-0021 and the port contract change before any test or code (the pipeline's T and F follow it).

### 1. The signal (handoff-A finding 1)

- **A new read-only `HostPort` method beside `ps`:** `foreground_commands(&self, name: &PaneName) -> Result<Vec<String>, PaneError>` (F matches the exact signature style of `ps` in `crates/holler-pane/src/ports.rs`). It answers the command **name** of the foreground process of every live pane in the named tmux session, one entry per pane, in the order tmux lists them. A session with no pane process answers an empty list. A missing session is `pane-not-found`, exactly as `ps` answers.
- **Names only.** It reads command names, never an argv, a command line, or Claude Code's configuration directory (spike decision 4; invariant I7). The tmux adapter reads `#{pane_current_command}` from the same `list-panes -s -t =NAME:` listing `ps` uses, with a strict parser beside `parse_ps`, and maps errors through the adapter's existing `Call` and `refused()` path. It does not use Herdr's `pane.process_info`, which returns full argv and command lines.
- **Default-armed, not required** (the #645 precedent). A required method would force edits to `Unwired` in `wiring.rs` (owned by #649, running now), the launch and apply rigs and `ports_test.rs`. The default **fails loudly**: it returns a refusal with an open code in the style of `send_prompt`'s `prompt-unsupported` (F matches that default's pattern and names the code, e.g. `command-names-unsupported`, exit 3). It never answers an empty list, because an implementation that does not override it would then import a Claude Code pane as OpenCode.
- **`ps` is not changed** (it would reach #647's frozen `reconcile/observe.rs`).
- **The records.** Add the method to ADR-0021 section 2's port table and its "amended since" sentence. The epic's contract block is a GitHub issue body: the operator edits it (not the pipeline); the brief only flags it.
- **The fake and its pins.** `FakeHost` answers, for a session, the basename of argv[0] of each process `run` started in it (as tmux reports `sleep` for `env -- sleep`), and an empty list when none; a new `HostOp` variant whose `as_str` equals the adapter's `OP_*` constant; one conformance case in the test kit's host suite and one `Mutant` break, with the `Mutant` wrapper delegating the new method so `the_unbroken_host_wrapper_passes` still holds. The issue's fixture then needs no new scenario method: `host.run(name, cwd, ["claude", ...])` in the left-alone pane's session.

### 2. The rule, for every answer (finding 2)

- A pane whose session has a live pane whose command name is exactly **`claude`** is **left alone** by default, and **adopted** with `--adopt-claude-code` once #831 has merged.
- **Any other answer is imported as OpenCode, as today**: `bash` (the old launcher's OpenCode panes run a bash script, `oc-holler`, so they report `bash`), `opencode`, or an empty session. This keeps a pane whose OpenCode server is merely down adopted and flagged `server-down` (issue Work item 3), keeps #650's fixtures (which start no process in any pane's session) green, and matches the old launcher's own test for a Claude Code process (`claude` on argv[0]).
- **Only `claude` is left alone.** No other command name is classified.
- The name lives in **one constant in `holler-pane`** (so #831 and #832 reuse it); nothing classifies command names anywhere else.
- **An unreadable signal fails closed.** If the new read returns an error other than `pane-not-found` (which the existing `ps` path already turns into `no-tmux-session`), including the default's loud refusal, the pane gets a new `problem` finding `command-unreadable` and no record, and a real run is refused as `import-blocked` like any other problem. It is never recorded as OpenCode on a guess. (This follows the import's fail-closed stance, e.g. `sources-disagree`.) The finding's message never quotes observed text except through `findings::quoted`.

### 3. The finding (finding 3)

- **Code: `left-alone-not-driven`** (final spelling in the ADR). **Level: `info`, as the issue says** ("an info finding"), not handoff-A's suggested `skipped`; the issue is the source of truth, `info` already exists, and the code differs from `harness-not-opencode` because the switch adopts these panes and not that one's. **Action: `skipped`** for the pane's row, built through `Outcome::left_alone` with a constructor beside `not_opencode`. No new `Action` and no new `Level`. One finding and no others for such a pane; the import makes no harness-health or session calls for it.
- `harness-not-opencode` is untouched. Handoff-A noted the two outcomes differ in level; this is a deliberate, recorded difference and S should treat it as such.
- It is a new code, stable once merged; add it to the finding table in ADR-0021 section 13 and the verb's help text. `--format=json` shows it like every finding (`level` `info`, `action` `skipped`).
- A pane left alone shows as `unregistered-herdr-pane` in a whole-fleet `doctor` pass: expected (issue Work item 1).

### 4. The switch (finding 4)

- **Keep `usage` (exit 2), as the issue says.** Reason: until #831 merges the value is a refused argument for this build, and the issue chose it. Handoff-A noted the CLI's one "story not landed" answer is `output::not_implemented(story)` (code `not-implemented`, exit 1); the brief records that the issue's `usage` was chosen over it. **Reuse the helper's wording**: build the "#831" text with `not_implemented_message(831)` so it has one spelling.
- **One site:** `Input::read`, step 1, before any file is read or any port is called, beside the existing `usage` checks in `crates/holler-cli/src/pane/import.rs`, so a dry run is refused too and nothing is written. #831 replaces that one place.
- **Documented in:** ADR-0021 section 9's `pane import` row, the `PaneImport` help text, ADR 0003's row for the verb, and `crates/holler-cli/tests/fixtures/cli-surface.txt` (a line with the flag).

### 5. After #831 (finding 5): out of scope here

No story owns the post-#831 adoption, and #835 cannot build it (`HarnessKind` has only `Opencode`). This story builds the **refusal**, not the adoption. Recorded for the owner: adopting into an existing `fleet` profile needs a change to the import's write step (`profile-exists` today refuses adding records to a profile that exists) or another route such as `pane launch --profile fleet` with #831's harness. The operator should add a line to #831 (or file a follow-up); the pipeline cannot, and does not.

### 6. Blast radius, by owner (finding 6)

- `crates/holler-pane/src/ports.rs` (frozen, #637's): the new default-armed method only.
- `holler-pane-testkit`: `FakeHost`, `HostOp`, the host conformance suite, the `Mutant`.
- `holler-adapter-host`: `src/lib.rs`, `src/tmux.rs`; **new adapter tests go in a new file**, not `tests/fake_tmux_test.rs` (818 lines, past the ~800 guard).
- `holler-pane` import: `import/assess.rs`, `import/observe.rs` (one more read on the same per-pane thread; update its "at most three port calls" note), `import/findings.rs` (two new kinds in `ALL`), `import.rs`.
- `holler-cli`: `pane/import.rs` (flag, `Input::read`, help), the import rig's read allow-list (`tests/pane_verbs/import/rig.rs`, which admits `HostOp::Ps` only), `cli-surface.txt`.
- Docs: ADR-0021 sections 2, 9 and 13 and the closing lists only; ADR 0003's row; `CHANGELOG.md` (an `## [Unreleased]` entry linking #835). Do **not** touch `wiring.rs` (#649), and keep ADR-0021 edits to those sections because #828 edits section 8 and `CHANGELOG.md`; #652's guide describes the import and should be told of the new default and flag.
- Files near the 900-line lint: check `wc -l` on every touched file before extending it.

### 7. #832 (finding 7)

#832 (same wave) must reuse the `HostPort` read and the one rule constant rather than add a second signal (such as Herdr's screen-detected agent). A note on #832 is for the operator.

### Fixture additions

On top of the issue's fixture: a pane whose command read errors (not `pane-not-found`) gives `command-unreadable`, no record, and a real run refused as `import-blocked`; a session with no process (the #650 default world) is imported as OpenCode; a pane whose command is `bash` is imported as OpenCode; every pane left alone writes nothing in a real run and in a dry run.

---

## Contract amendment, round 2 (resolves the second BLOCK in `docs/handoffs/835/handoff-A.md`)

This section supersedes the earlier sections wherever they differ. The issue text at the top still wins where it is explicit.

### R2-1. The port method is REQUIRED, not default-armed (supersedes section 1's "Default-armed" bullet and its open code)

Reason: #649's production wiring (`BindingHost` over `WiredHost` over `TmuxHost`, in #649's `crates/holler-cli/src/pane/wiring.rs` and `wiring/bind.rs`) forwards only the four `HostPort` methods that exist. A default-armed method is not forwarded by those decorators, so over the real wiring the import would get the default's refusal for every pane, every pane would be `harness-unreadable`, and every real run would be refused `import-blocked`; no compile error would say so and no test over `FakeHost` would catch it. A required method makes the compiler find every implementation (ADR-0021 decision 12, the #802 precedent) and keeps #649's own rule that every decorator forwards every port method.

- **No default, no new open code.** Delete the default and the `command-names-unsupported` code from section 1. The signature stays `foreground_commands(&self, name: &PaneName) -> Result<Vec<String>, PaneError>`.
- **Implementations to update (the compiler lists them):** `TmuxHost`, `FakeHost`, the conformance `Mutant`, `TestHost`, `Unwired`, the launch and apply rigs' linked hosts, and, after #649 has merged, `BindingHost` and `Wired<dyn HostPort>`.
- **Lift the "do not touch `wiring.rs`" rule** (section 6) for exactly these forwards, and no other change to #649's files.
- **Pin each decorator:** a test per decorator that the new method reaches the inner host (follow the pattern of #649's forwarding tests in `tests/pane_integration/bind.rs`), and the host conformance suite run over the production decorator if #649 already does that for the existing methods.
- **Sequencing: this story is built after #649 has merged**, in a worktree based on that `main`. (It also needs the current `main`, which includes #832.) Until then it is parked.

### R2-2. The records (supersedes the ADR 0021 section 13 "finding table" wording)

ADR-0021 section 13 has no finding table. Add a **"Decided (#835)" paragraph** to section 13 recording the port method, the rule and its limits, and the two new codes. Edit sections 2 and 9 and the closing lists as before. Add `crates/holler-pane/tests/import_test.rs` to the blast radius (its code table `NEW_CODES` and levels, and the prose counts, change). The missing-session code in section 2 is spelled **`tmux-session-missing`** (the real code, pinned in `import_test.rs`), not `no-tmux-session`.

### R2-3. Files and coexistence (supersedes the `fake_tmux_test.rs` sentence in section 6)

`crates/holler-adapter-host/tests/fake_tmux_test.rs` must change by about three lines: its `METHODS` array and exhaustive `call` match will not compile with a new `HostOp` variant, and its `OP_*` pin needs the new constant. It goes from 818 to about 821 lines; new adapter tests still go in a new file. #832 (merged as PR #836) edited `ports.rs`, `wiring.rs` and ADR-0021 sections 2, 9 and 13, the same places this story edits: work from the current `main`, and note the coexistence in the PR.

### R2-4. The two codes, renamed (supersedes `left-alone-not-driven` and `command-unreadable`)

- **`harness-claude-code`** (level `info`, row action `skipped`, built through `Outcome::left_alone`): names the condition, and stays true after #831, when the same panes are adopted with the switch.
- **`harness-unreadable`** (level `problem`): names what could not be read, and does not clash with this verb's existing "command" (the launcher argv).

### R2-5. The rule's limits, recorded (supersedes section 2's "matches the old launcher's own test ... `claude` on argv[0]")

tmux reports `#{pane_current_command}`, the argv[0] basename of the pane's foreground process, whereas the old launcher matches any argv element. Record the limits in the "Decided (#835)" paragraph and in the verb's help text:

- A Claude Code pane whose `claude` process has exited shows its shell (`bash`) and is imported as OpenCode.
- A Claude Code pane started in a way that makes argv[0] something else (a wrapper) is not recognised.
- The rule is a default for the first real import, not a proof of what a pane runs.

### R2-6. Every path that would record a pane on a guess is the unreadable problem (extends section 2)

Besides an error from the new read, these are also `harness-unreadable` (problem, no record, `import-blocked` on a real run): a lost chain (`Observed::Lost`, which today is only an `observe-failed` warning in `assess.rs`), and a failed `ps` that is not `pane-not-found`. A pane is never recorded as OpenCode unless the signal was read and was not `claude`.

### R2-7. Home of the rule

The one constant (`"claude"`) and its predicate are public in `crates/holler-pane/src/pane.rs`, beside `HarnessKind`, so #831 and #832 can reuse them. Nothing compares a command name with `"claude"` anywhere else.

### R2-8. Rules for the conformance case (for T)

The new host conformance case must hold on a real tmux as well as on `FakeHost`: never assert an empty list (the session's shell is itself listed), and poll within a bound for the expected name, because right after `run` the foreground name can still be `env`. The case asserts the name the process settles to, and the `Mutant` break makes that assertion fail.

### Out of scope, for the operator

#832's `HerdrPort::observe_pane` is default-armed too, and #649's `BindingHerdr` and `LazyHerdr` do not forward it, so over the real wiring `pane list` would silently show no working directory and no last screen change. That is #832's to settle, and the reason this story takes the required form.

---

## Contract amendment, round 3: fold in the `observe_pane` flaw from #832 (operator decision)

The operator decided to fold the #832 flaw into this story. The issue text at the top is unchanged; this section adds scope on the operator's word and S treats it as in scope. It supersedes nothing in rounds 1 and 2.

**Correction of the name.** The method is **`HerdrPort::observe_pane`** (added by #832, merged as PR #836), not a `HostPort` method: it lives in `crates/holler-pane/src/ports.rs` on `HerdrPort`, with its batch form `observe_panes`. The `HostPort` method this story adds (`foreground_commands`) is separate.

### R3-1. `observe_pane` becomes REQUIRED on `HerdrPort`

- **Why.** Today `observe_pane` is default-armed: the default answers an empty `PaneObservation`. #649's production wiring (`BindingHerdr` and `LazyHerdr`, in #649's `wiring.rs` and `wiring/bind.rs`) forwards only the methods that existed when it was written, so over the real wiring the default is used: `pane list` would silently show no working directory and no last screen change for an unmanaged pane, with no compile error and no failing test (the tests use `FakeHerdr`). Making it required lets the compiler find every implementation, the same reason `foreground_commands` is required (round 2, R2-1).
- **What changes.** Remove the default body of `HerdrPort::observe_pane`. `observe_panes` stays a defaulted batch form that delegates to `observe_pane` (the Herdr adapter overrides it to read the snapshot once). Implementations the compiler lists: the Herdr adapter and `FakeHerdr` (already implement it), `Unwired` (already answers `NotImplemented`), the test wrappers in `tests/pane_verbs/import/seams.rs`, `launch/displaced.rs` and the conformance test's wrapper (already forward it), plus, once #649 has merged, **`BindingHerdr` and `LazyHerdr`**, which must forward `observe_pane`, and, so a pane list stays one snapshot read and not N, `observe_panes` too.
- **Lift the "do not touch `wiring.rs`" rule** for exactly these forwards (with the `foreground_commands` forwards of round 2), and nothing else in #649's files.
- **ADR-0021.** Record the change in section 2's `HerdrPort` row and the "amended since" sentence, and in the "Decided (#835)" paragraph of section 13: `observe_pane` is required, with the reason above.
- **Sequencing is unchanged:** this story is built after #649 has merged, on the current `main`.

### R3-2. Pin each forward with a test (the same way as `foreground_commands`)

- One test per decorator (`BindingHerdr`, `LazyHerdr`) that `observe_pane` reaches the inner Herdr and returns its observation unchanged, and one that `observe_panes` does too and reads the inner snapshot once (follow the pattern of #649's forwarding tests in `tests/pane_integration/bind.rs`); the Herdr conformance suite run over the production decorator if #649 already does that for the existing methods.
- No decorator or test double may rely on a default for either method.

### R3-3. New acceptance item

**Over the real wiring, `pane list` shows an unmanaged pane's working directory, and its last screen change when the Herdr adapter reports one.** In a default-run test: build the production Herdr decorator stack (the same constructors #649 exposes) over `FakeHerdr`, give an unlisted Herdr pane a working directory and, in one case, a last-screen-change time, and assert that `holler pane list` (text and `--format=json`) shows the directory, and the change time in the case that has one. In the case where the adapter reports none, the change time stays absent (`-` in text, `null` in JSON): **real Herdr reports no last screen change today (#837), so null stays honest**; the test must not invent a value. A second check, with the same stack over a fake that does **not** forward (a deliberately non-forwarding decorator in the test), is not needed: the compile error and R3-2 cover that.

### R3-4. Blast radius additions

`crates/holler-pane/src/ports.rs` (the `observe_pane` default removed), #649's `wiring.rs` and `wiring/bind.rs` (the forwards only), the `pane list` test file for the new item (a new file or a sibling module if `tests/pane_verbs/list.rs` is near the line guard; check `wc -l`), ADR-0021 sections 2 and 13, `CHANGELOG.md` (the entry for #835 also names the follow-up to #832). Do not change #832's behaviour: the Herdr adapter's `observe_pane` already reports the working directory and no last screen change; only the trait's default goes away.

### R3-5. Out of scope

The source of an unmanaged pane's last screen change is #837 (an open spike). This story makes the existing value reach `pane list` over the real wiring; it does not look for a new source.

## Round 5: merge `origin/main` into the branch (operator-authorised extra run, 2026-10-12)

**Why.** S's third audit (`docs/handoffs/835/handoff-S.md`, cycle 3, verdict REWORK) found that `main` moved while the story was parked. PR #857 (part of #831) landed this story's own names (`HostPort::foreground_commands`, `CLAUDE_CODE_COMMAND`, `is_claude_code_command`, host conformance case 10, the forwards) and ADR-0021 section 15, whose item 17 and decision 23 decide the adoption route (a re-run of `pane import --adopt-claude-code` that extends the existing profile). This changes no contract and no scope: the issue, rounds 1 to 3 and the behaviour stand. It is branch upkeep plus prose.

**The work list is S's REWORK in `docs/handoffs/835/handoff-S.md`, section "Verdict", items 1 (a to e) and 2 (a to i). Do exactly that list, nothing else.**

- Run `git fetch origin`, then `git merge --no-commit --no-ff origin/main` and resolve all 10 conflicts. Leave the merge uncommitted: the pipeline's phase commit records it as a two-parent commit. Stage by explicit path.
- Keep #857's names and positions. There must be exactly one `CLAUDE_CODE_COMMAND` and one `is_claude_code_command`. Renumber this story's ADR-0021 decisions 22 and 23 to follow `main`'s. Keep one adoption entry under "Deferred to named stories".
- Resolving the merge conflicts in test files named in S's item 1d (`wired.rs`, `ports_test.rs`, `claude_code_rule_test.rs`, `docs_pane_control_test.rs`) is authorised as part of the merge; take both sides' meaning as S's table says. This is not new test authorship. T-green reviews the merged tests.
- The statements of the rule's limits (foreground process, the command name only, an exact `claude`, the three limits, "a default, not a proof") stay exactly as they are on the branch. Do not reword them.
- Prose that says the adoption waits for the Claude Code harness must say it waits for #831's adoption step (ADR-0021 section 15, item 17 and decision 23), per S's item 2.
- After the merge, `git merge-tree --write-tree HEAD origin/main` must be clean, `cargo build --workspace --all-targets` must pass, and the full Tier 1 must be green.
