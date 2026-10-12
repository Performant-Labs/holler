# Handoff-A-dup: Phase 8 - #803, the setup wizard builds panes through `holler pane` and `holler profile apply`  (anti-duplication gate)

**Date:** 2026-10-12
**Branch:** `issue-803-implementation`
**Diff base:** `6eff9cc` (`origin/main`)   **Diff head:** `2698960`
**Reuse map:** `docs/handoffs/803/handoff-A.md`, section "Reuse & Analogous-Feature map" (A-derived: this automated run has no `survey.md`)
**Verdict:** PASS

## Summary

PASS, with 0 `block` and 4 `warn` findings. F extended every object the map named. The one new
object, `lib/panes.sh`, is the one the brief justified in writing (D1 (c)): no existing script parses
`[[pane]]` or `layout`.

- **`herdr.sh` was extended, not paralleled.**
  - One allow-list lives in one function, `base_env` (`herdr.sh:211-225`).
  - `server-start` adds `HERDR_*` to it, as before. The new `holler` verb adds the instance's `HOLLER_*`
    literals and nothing of `HERDR_*`.
  - There is no second copy of the list. `run` became an allow-list, and `label` went.
- **`panes.sh` reads `[instance]` and `workspace` only through `instance.sh`.**
  - It skips both in the file (`panes.sh:213`, `457`). It takes them from `instance.sh`'s printed lines
    (`panes.sh:404-422`).
  - It reads the first instance's port base from `instance.sh /dev/null`, not from a second `47001`.
  - Its copies are the ones the map accepted:
    - `trim` and `value_of`, byte-identical to `instance.sh:29-42`;
    - the port rule, the same as `instance.sh:165-189`, applied to OpenCode panes.
- **The test harness is reused through `pub(super)`, not copied.**
  - `herdr_803` imports `Env`, `script` and `text` from `herdr.rs`.
  - `proof_803` imports `Host`, `Inst`, `first_up`, `snapshot`, `start_detached`, `record`,
    `holler_calls`, `ok`, `stdout` and `text` from `proof.rs`. Apart from those visibility changes,
    `proof.rs` is unchanged (still 825 lines).
  - `skilltext_803` imports `code_lines`, `section`, `flat`, `read`, `all_texts`, `skill`, `docs` and
    `readme` from `skilltext.rs`.
- **The fake `holler` matches the code.** I checked these against `tx_apply.rs`, `profile/apply.rs`,
  `get.rs`, `list.rs`, `profile/list.rs` and `doctor.rs`:
  - the action and outcome words (`planned` in a dry run);
  - the step shape `{pane, action, outcome, code, diff, detail}`;
  - `cell_key = (workspace, col != 1, row, col)`;
  - rows a to f (its check order is equivalent, because rows c/d and e/f are disjoint on health);
  - apply's text lines and its exit-3 `apply refused ...` line;
  - the JSON of `pane get`, `pane list` (`unmanaged` left out when empty), `profile list` and
    `pane doctor`.

  Its planner rule is one function that cites `plan.rs` (E40). `proof_803`'s module doc says it tests
  the fake, not the adapter.
- **No code block runs a pane or profile verb outside the door.** That holds in `SKILL.md`,
  `docs/setup-wizard.md` and the README. `skilltext.rs`'s scoping test now enforces it (W7d).

The four warns are small. Two are in user-facing text, one is in the door's guard, and one is a copied
helper the header does not name. None needs an operator decision.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `agent-skills/herdr-setup/SKILL.md:1510-1513`; `docs/setup-wizard.md:646` | **A second, incomplete route to the pane verbs, around the door.**<ul><li>**What the text says.** Stage 11's note tells the user to close or relaunch a pane with `holler pane close` / `holler pane relaunch` "with the instance's `HOLLER_STATE_DIR` in your own shells". The docs name no environment at all.</li><li>**What a pane verb needs.** Also `HOLLER_HERDR_SESSION`, `HOLLER_HERDR_WORKSPACES` and the Herdr socket, plus `HOLLER_TMUX_SOCKET` for a named instance (`crates/holler-cli/src/pane/wiring/env.rs:66-74`, `86-94`, `168-176`).</li><li>**What happens.** As written, the call stops with `usage` ("no Herdr session is configured"). A user who completes it by hand and leaves out `HOLLER_TMUX_SOCKET` reaches tmux's default server for a named instance. That is exactly the case `holler_guard` refuses (`herdr.sh:237-244`).</li><li>**Scope.** The agent's own path is correct everywhere.</li></ul> | Do one of these, one sentence in each file:<ul><li>point the note at the door: `<herdr_env> bash $WIZARD_LIB/herdr.sh holler pane close NAME --profile <profile>`;</li><li>or have Stage 10's report print the instance's five `HOLLER_*` values for the user's own shell, and name them in both files.</li></ul> |
| 2 | warn | `agent-skills/herdr-setup/lib/herdr.sh:300-301` (the guard: `229-254`) | **The door's socket guard is one-sided.** I agree with T-green's advisory 2.<ul><li>`holler_guard` requires the right socket for each kind of instance. But it passes the other kind's socket whenever it is set.</li><li>Example: the default instance's `WIZARD_HERDR_SOCKET` pasted into a named instance's `<herdr_env>` reaches `holler` as `HOLLER_HERDR_SOCKET`.</li><li>The adapter prefers that variable over the named session's own socket (`env.rs:168-176`). So the named instance's panes would be built in another Herdr server.</li><li>That is the cross-instance leak that W1 closed for `HERDR_SOCKET_PATH`.</li></ul> | In `holler_guard`, either:<ul><li>refuse (exit 2, nothing run) a set `WIZARD_HERDR_SOCKET` for a named instance and a set `WIZARD_TMUX_SOCKET` for the default instance;</li><li>or leave those two out of `keep`.</li></ul>Add one `herdr_803` case for each. XS. |
| 3 | warn | `docs/setup-wizard.md:160-161` | **The wizard's docs state the creation order a second time:** "in its creation order (a workspace's column 1 by row, then the rest by row and column)".<ul><li>The brief's Amendment (d) asks for pointers, not copies: "the creation order is stated once, where #853 states it" (`docs/pane-control.md` section 6, ADR-0021 section 8).</li><li>The restatement in `SKILL.md` Stage 7 is needed: under D3 the agent compares the dry run's steps against it. The docs' copy is not needed.</li></ul> | Drop the parenthetical. The link to section 6, two sentences later, already covers it. XS. |
| 4 | warn | `agent-skills/herdr-setup/lib/panes.sh:118` (header: `39-41`) | **A copied helper that the header does not name.**<ul><li>`noslash` is `strip_slash` (`instance.sh:44`, and also `stop-owned.sh:171`) under a new name. That is one helper beyond what W6 allowed.</li><li>The header lists only `trim` and `value_of` as copies.</li><li>Each lib script carrying its own helpers is the lib's pattern (there is no shared awk library), so this is not a block.</li></ul> | Rename it `strip_slash` and add it to the header's list of copies, so a grep finds all three. XS. |

**No drift from rework.** F had no rework round, and T-green's repair touched only the matching in
`skilltext_803.rs`.

**Checked and accepted (no finding):**

- **The port rule in `panes.sh` (W6).**
  - It is the same rule as `instance.sh`'s, including "a pane with its own port keeps its index".
  - The default range is read from `instance.sh`, so the two cannot drift apart.
  - Brief D1 (c) and D11 justify the copy.
- **The XDG rule in `panes.sh`'s `config_home`** mirrors `env.rs:191-198`. That is Phase 3's follow-up 3,
  which needs Rust and is not this story's work.
- **`proof.rs`'s old `build()` scenario stays beside `proof_803`.**
  - The brief justifies this in writing ("scripts-level and stay valid as a test of the scripts"), and
    Acceptance 19 freezes `proof.rs`.
  - `proof_803::stage4_hub` is the new Stage 4 shape: no `--advertise`, no `tailscale serve`, and
    `--stage 4`. It is not a copy of the private `stage6_hub`.
- **Per-module test helpers follow the target's established pattern** (W7c allowed them):
  - `panes_803`'s `lib`, `out` and `err`, and `skilltext_803`'s `stage` and `teardown`;
  - `preflight.rs` already has a `lib_dir` identical to `proof.rs`'s, `stage(n)` is in two other
    modules, and `between` is in seven;
  - `proof_803::Door::panes` and `panes_803::Scratch::cmd` are two eight-line `panes.sh` runners. The
    first runs against a config at a given path, and the second writes its own config.
- **The two shell-word splitters do different jobs.** `panes_803::words` splits bash-quoted words, to
  check `plan`'s quoting contract. `skilltext.rs`'s `script_words` splits on whitespace, for the verb
  check.
- **The empty session maps to `default` in both `panes.sh` and `herdr.sh`** (W12c), and `proof_803` pins
  that the two agree.
- **`SKILL.md` Stage 6's example launch line** matches what `panes.sh`'s `spec()` prints, flag for flag.
  It only illustrates the plan: the agent copies the printed lines.
- **Nothing that must stay unchanged was edited:**
  - no Rust production code, wire, protocol or golden file;
  - none of `instance.sh`, `collide.sh`, `inventory.sh`, `ledger.sh` or `stop-owned.sh`;
  - ADR-0021 is amended in the same change: section 11, item 16, "Not built here" and decision 34.
- **Sizes:** `proof.rs` is still 825 lines, `herdr.rs` went from 874 to 751, and every new test module is
  under 600 lines.

## Notes for F

Not required, since the verdict is PASS. If F or a follow-up takes the warns, all four are XS:

- 1 and 3 are text edits.
- 2 is a guard clause and two test cases.
- 4 is a rename and one header line.

## Patterns referenced

- `agent-skills/herdr-setup/lib/herdr.sh` (`base_env`, `holler_guard`, the `holler` verb) and
  `agent-skills/herdr-setup/lib/instance.sh`
- `crates/holler-pane/src/tx_apply.rs` and `crates/holler-cli/src/profile/apply.rs` (the fake's contract)
- `crates/holler-cli/src/pane/wiring/env.rs` (what a pane verb reads from the environment)
- `crates/holler-cli/tests/wizard/proof.rs`, `herdr.rs` and `skilltext.rs` (the reused harness)
- `docs/handoffs/803/handoff-A.md` (the map, and W1 to W12)

---

# Handoff-A-dup (rework 1): Phase 8 - #803, after F's rework of S's REWORK items 1 to 4  (anti-duplication gate)

**Date:** 2026-10-12, 11:23 MDT
**Branch:** `issue-803-implementation`
**Diff base:** `6eff9cc` (`origin/main`); this cycle is `f2ecfae..43f9047` (F's `3a631d0`, T's `43f9047`)   **Diff head:** `43f9047`
**Reuse map:** `docs/handoffs/803/handoff-A.md`, section "Reuse & Analogous-Feature map"
**Verdict:** PASS

## Summary

PASS, with 0 `block` and 2 `warn` findings. The rework extends the objects it touches and adds no new one.

- **`panes.sh` is extended inside its own `layout` handling.**
  - `top_key` records the line that leaves the layout open (`panes.sh:217`), and `check_layout` refuses it, naming
    `layout` (`:364`).
  - Nothing reads `workspace` a second time, so W6 holds: the label still comes only from `instance.sh`'s printed lines.
  - The refusal sits in the script that owns `layout` (D1 (c)), and `instance.sh` stays frozen.
- **The user's route to a pane verb is the existing door.** There is no new script, verb, variable or allow-list.
  - Stage 10's report, Stage 11's first note and the docs' "Basic Herdr commands" give
    `<herdr_env> bash <lib>/herdr.sh holler pane close|relaunch ...`.
  - `herdr.sh` is unchanged. Its `holler` verb passes the words through as given, so `pane close`, `pane relaunch` and
    `pane reset --as-operator` all get the scrub, `holler_guard` and `check_pane`.
  - F rejected printing the five `HOLLER_*` values: that would be a second spelling of the door's literals, and it would
    skip the scrub. That is the reuse-first choice.
  - `<herdr_env>` (`SKILL.md:685`) and `$WIZARD_LIB` (`:324`) are the existing definitions.
- **My warn 4 is closed.** `strip_slash` (`panes.sh:124`) is byte-identical to `instance.sh:44`, comment line included,
  and the header lists it with `trim` and `value_of`, which are still byte-identical to `instance.sh:29-42`.
- **My warns 1 and 3 are closed** by S's items 1 and 4. Warn 2 is carried forward as finding 2.
- **The new tests reuse what exists.**
  - The `panes_803` case uses only that module's helpers: `Scratch`, `refuses`, `verb`, `stdout`, `keyed`, `pane`,
    `DEFAULTS` and `out`.
  - The three `skilltext_803` pins use `skilltext.rs`'s `flat`, `section` and `docs` through `pub(super)`, plus the
    module's own `stage`.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `agent-skills/herdr-setup/SKILL.md:1491-1498`, `:1531-1538`; `docs/setup-wizard.md:656-663`. The root: `crates/holler-cli/src/pane/wiring.rs:148-160`, `pane/close.rs:181-183` | **The wizard's text compensates, in three places, for an ordering in the CLI.**<ul><li>**What the text says.** All three passages describe what a bare `holler pane close` does: it stops the processes of the tmux session of that name on tmux's default server, then fails on the missing Herdr session.</li><li>**Where that comes from.** `from_env` parses the configuration up front (`wiring.rs:152`) but keeps each error for its port's first use (`:148`, the lazy Herdr at `:160`; ADR-0021 section 5). `close_live` then stops the host's processes before its first Herdr call (`close.rs:181-183`).</li><li>**Why it matters.** The text is the right fix for this story: Rust is out of scope, and the wizard's own calls already go through the door. But the hazard sits at the CLI's choke point and reaches any bare `holler pane close`, from the wizard or not. And the three passages restate Rust's internal order, which Amendment (d)'s rule (pointers, not copies) argues against. A CLI fix would make all three wrong.</li></ul> | **O:** consider a follow-up on epic #633 (ADR-0021 section 5).<ul><li>A verb that acts answers its configuration errors (`usage`: an unset `HOLLER_HERDR_SESSION`, a malformed socket) before its first act.</li><li>The configuration is already parsed eagerly, so this needs no I/O, and connectivity stays lazy.</li><li>When that lands, cut the three passages to "never a bare pane verb: use the door".</li></ul>No change in this story. |
| 2 | warn | `agent-skills/herdr-setup/lib/herdr.sh:300-301` (the guard: `229-254`) | **The door's socket guard is still one-sided.** This carries forward my warn 2 (S's advisory 1, T-green's advisory), which F declared as not taken.<ul><li>`holler_guard` requires each instance kind's own socket, but the verb passes the other kind's whenever it is set.</li><li>**What changed this cycle.** The user's own shell is now a sanctioned caller of the door (Stage 11's note, the docs' basics). An interactive shell is long-lived, unlike the agent's fresh tool shells.</li><li>`<herdr_env>` sets only the instance's own kind (`SKILL.md:685-689`, `panes.sh:503-514`).</li><li>**Still a warn.** A leak needs an *exported* `WIZARD_HERDR_SOCKET` (named instance) or `WIZARD_TMUX_SOCKET` (default instance). A plain shell assignment of the printed lines never reaches `bash herdr.sh`.</li></ul> | In `holler_guard`, refuse (exit 2, nothing run) a set `WIZARD_HERDR_SOCKET` for a named instance and a set `WIZARD_TMUX_SOCKET` for the default instance. Add one `herdr_803` case for each. XS, here or in the first follow-up that touches `herdr.sh`. |

**No drift from rework.**

- F's changes stay inside `panes.sh`, `SKILL.md`, the docs and the CHANGELOG. T's stay inside `panes_803.rs` and
  `skilltext_803.rs`.
- `git diff f2ecfae..HEAD` is empty for:
  - Rust production code, `holler-proto`, `docs/adr/` and `docs/protocol/`;
  - `herdr.sh`, `instance.sh`, `collide.sh`, `inventory.sh`, `ledger.sh` and `stop-owned.sh`;
  - `proof.rs` and `proof_fakes/`.
- `git diff --check` on the cycle's deliverables is clean.

**Checked and accepted (no finding):**

- **The `layout` refusal's message names the key first and gives the reason,** as `panes.sh`'s neighbouring messages
  do (the retired-table messages).
- **The continuation lines are still gathered** (`panes.sh:459-461`), but only to find where the open layout ends, so
  the mistake yields one message, not one "unknown table" per line. That is not a second parser: `check_layout`
  refuses before `parse_layout` could see a multi-line value.
- **The two door rules in the tests are different predicates, not copies.**
  - `skilltext.rs:172-177` requires `herdr.sh holler` on any code-block line that holds a pane or profile verb.
  - `skilltext_803::pane_verbs_outside_the_door` (7 lines) requires every `holler pane ` in the flattened prose of the
    two user-facing notes to follow `herdr.sh ` directly.
  - There is no shared helper to reuse: the code-block rule is inline in its test.
- **The remaining bare `holler pane ...` mentions** in `SKILL.md` and the docs are the agent's own calls, described in
  prose: Stage 7's Gate, "If a stage fails" and the teardown.
  - Each names the door, or is covered by the skill's rule that every pane call goes through it (`SKILL.md:1173`,
    `1190`).
  - None is a route the user runs from their own shell. T scoped its bare-verb pin to the user's text on the same
    reading.
- **The docs' new `holler pane reset PANE --as-operator` span** is parsed by `docs_cli_test` against the real clap
  tree, and the flag exists (`pane/reset.rs:1,32`).
- **The docs' `<lib>` placeholder** is introduced in its own sentence ("the skill's absolute `lib` path"). The docs'
  other `agent-skills/herdr-setup/lib/...` paths name sources, not commands to run.
- **Sizes:**
  - `skilltext_803.rs` is now 625 lines (+81), past `lint.sh`'s 600-line warning but under the 800 this stack
    flags. T noted it should be split before it nears 900.
  - `panes_803.rs` is 560 lines, and `panes.sh` is 525 (+9).

## Notes for F

Not required, since the verdict is PASS. Finding 2 is XS if it is taken in this story.

## Notes for O

These are follow-ups to consider. None blocks this story.

1. **Finding 1's CLI follow-up,** an issue on epic #633: a verb that acts answers its configuration errors before its
   first act.
2. **Finding 2,** if it is not taken in this story.
3. **Phase 3's follow-ups 1 to 4** (`handoff-A.md`, "Notes for O") still stand. This rework was not expected to
   address them.

## Patterns referenced

- `agent-skills/herdr-setup/lib/herdr.sh` (`holler_guard`, the `holler` verb) and `lib/instance.sh` (`trim`,
  `value_of`, `strip_slash`, and the rule that a line starting with `[` is a header)
- `crates/holler-cli/src/pane/wiring.rs` (`from_env`), `pane/close.rs` (`close_live`) and ADR-0021 section 5
- `crates/holler-cli/tests/wizard/skilltext.rs` (the code-block door rule) and `panes_803.rs` (`Scratch`)
- `docs/handoffs/803/handoff-A.md` (the map) and `handoff-S.md` (items 1 to 4)
