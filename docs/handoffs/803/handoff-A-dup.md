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
