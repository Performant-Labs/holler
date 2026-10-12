# Handoff-F (rework 1): Phase 6 - #803, S's REWORK items 1 to 4

**Date:** 2026-10-12, 10:54 MDT
**Branch:** `issue-803-implementation` at `7a9dbec` (S's REWORK commit), plus this cycle's uncommitted changes;
`origin/main` still `6eff9cc`; #839, #842 and #850 still open (re-checked with `gh issue view`), so the REFRESH rows stand
**Issue:** #803
**Input:** `handoff-S.md` (REWORK, `reworkKind: production`, items 1 to 4). Prior F handoff: `handoff-F.md`.

## What was done

- **Item 1, a pane verb around the door.** The user is given the door, never a bare pane verb.
  - `agent-skills/herdr-setup/SKILL.md`, Stage 10's final report: it now prints, with every value written out, the
    instance's `<herdr_env>` and the skill's absolute `lib` path. It also says why a pane verb with `HOLLER_STATE_DIR`
    alone, or with no environment, is never given to the user.
  - `SKILL.md`, Stage 11's first note: close and relaunch are now
    `<herdr_env> bash $WIZARD_LIB/herdr.sh holler pane close NAME --profile <profile>` and `... holler pane relaunch NAME`.
    The note also says what a bare `holler pane` verb does.
  - `docs/setup-wizard.md`, "Basic Herdr commands": the same two lines (with `<lib>`) and the same warning.
  - Same class, one sentence each: Stage 1's named-instance note (`SKILL.md`) and "Running an instance beside another"
    (`docs/setup-wizard.md`) said that `holler` calls in the user's own shells need `HOLLER_STATE_DIR`. They now add that
    a `holler pane` or `holler profile` call needs the door instead.
- **Item 2, a multi-line `layout`.** `agent-skills/herdr-setup/lib/panes.sh` takes S's option (a): a `layout` left open
  on its own line exits 1, with one message that names `layout` and says it goes on one line.
  - Its continuation lines are still skipped, so they cause no second message (such as "unknown table").
  - The header now states the subset (one value per line) and the reason for it.
  - The rule is stated where the config is described: the `LAYOUT` comment in `SKILL.md` and in `docs/setup-wizard.md`,
    Stage 1's list of `panes.sh check` refusals, and the CHANGELOG's Breaking Changes item.
- **Item 3, `pane reset` on the orchestrator.**
  - `SKILL.md` Stage 10 now runs `holler pane reset PANE --as-operator` through the door, with the user's yes.
  - It says what the reset leaves:
    - an OpenCode orchestrator gets a new session of record, so Stage 8's body config is derived again and the body is
      restarted before the briefing test;
    - the old session stays on its server, and doctor reports it as `stray-session`, which Stage 9's gate will show;
    - a Claude Code orchestrator is relaunched on a fresh session id, with no body row and no stray session.
  - `docs/setup-wizard.md`'s `AGENTS.md` bullet says the same.
- **Item 4, the creation order restated.** `docs/setup-wizard.md` "How the wizard builds panes" item 4 drops the
  parenthetical. Its section 6 pointer now names "Apply's creation order" with the exit codes. `SKILL.md` Stage 7's
  statement stays, as S said.
- **A-dup warn 4 (S advisory 7):** `panes.sh`'s `noslash` is renamed `strip_slash`, byte-identical to
  `instance.sh:43-44`, and is listed in the header with `trim` and `value_of`.
- `docs/handoffs/803/evidence.md`: seven entries (below). `docs/handoffs/803/decisions.md`: F's rework entry. This file.

## Design decisions

1. **Option (a), not (b), for item 2.**
   - **What (a) does.** It makes the header's claim true: `panes.sh` accepts exactly `instance.sh`'s subset, one value
     per line. That removes the whole class, not one key.
   - **What (b) would have done.** It refuses only when a top-level `workspace` follows. It keeps a parser that differs
     from `instance.sh`'s, and the next top-level key `instance.sh` learns to read would hit the same misread.
   - **The cost of (a).** It also refuses two layouts that `instance.sh` reads correctly:
     - a `layout` after `workspace`;
     - one whose continuation lines do not start with `[`.

     The rule stays one plain sentence ("on one line"). No documented example, fixture or test writes a multi-line
     `layout`: every fixture and doc example is one line (checked with grep). The wizard writes the config itself in
     Stage 1.
   - **Neither option reads `workspace` a second time** (A's W6).
2. **The continuation lines are skipped, then refused once.** Without the skip, each continuation line would add an
   "unknown table" or "neither key = value" message about the same mistake. An unterminated `layout` still also prints
   "no [[pane]] table", which it already did before this change.
3. **Item 1: Stage 10 prints the literals, Stage 11 shows the line.**
   - **Why print the literals.** The user's shell has none of the wizard's values, and a door line with placeholders is
     useless. Stage 10's report already prints what a later repair needs (the layout and the sessions of record), so the
     two literals go there.
   - **What the notes show.** Stage 11's note and the docs show the two verbs S named: `pane close NAME --profile <profile>`
     (so P's spec goes too, as the teardown does) and `pane relaunch NAME` (the command doctor names).
   - **Rejected: printing all five `HOLLER_*` values for the user's shell** (S's alternative). That would be a second
     spelling of the door's literals and would skip its scrub, which is D8's reason for the door.
4. **Item 3: the `stray-session` sentence agrees with Stage 9's gate.** D14 and Acceptance 17 fix Stage 9's gate as "no
   finding but `unregistered-herdr-pane`", so the text does not add an exception. When the gate shows the finding, the
   agent says the reset left it.
5. **Item 1's warning says what happens, from the code.** For example: "stops the processes of the tmux session of that
   name on tmux's default server, which may be another instance's pane, and only then fails on the missing Herdr
   session, leaving the record and the Herdr pane in place". The source is `wiring.rs:148,160`, `env.rs:86-94` and
   `close.rs:180-184` (evidence.md).

## Reuse / extend-vs-new

- **EXTEND `lib/panes.sh` only** (the new object D1 (c) justified). No new script, verb or variable.
- **The rename makes the one copied helper easy to find.** `strip_slash` now has the same name as in `instance.sh` and
  `stop-owned.sh`, as A-dup warn 4 asked.
- **The text routes the user through the existing door** (`herdr.sh holler`), which is unchanged. There is no second
  route and no copy of the door's list.

## Architecture notes for A

- **None at the architecture level.**
  - No module, interface or dependency direction changed.
  - `herdr.sh`, `instance.sh` and every other script are unchanged.
  - No Rust, wire, protocol or golden file changed.
- **Contract narrowing.** `panes.sh check` (and so every verb) refuses one more input shape: a `layout` written over
  several lines (exit 1, naming `layout`). Its printed output for every accepted config is unchanged.

## Deviations from spec / wireframe

- **Item 2: S's option (a)** (design decision 1). T's new test should expect the refusal to name `layout`, not
  `workspace`.
- **Beyond the four items, three small edits:**
  - A-dup warn 4 (the rename);
  - the one-line rule in the user-facing config text and the CHANGELOG (the new refusal is user-visible);
  - the two "own shells" sentences (item 1's class).
- **S's other advisory notes were not taken.** Each is non-blocking:
  - **1, the one-sided socket guard.** It needs a guard clause in `herdr.sh` and two new `herdr_803` cases. The door
    line the user now gets is complete, so a leak needs a stale exported `WIZARD_*_SOCKET` in the user's shell. That is
    worth a follow-up, not this rework.
  - **2, `[ext.*]` with a nested multi-line array.** It still fails closed as an "unknown table".
  - **3, the fake's missing exit-1 path.** Test-only, so T's.
  - **4, `--spec-only` offered for a live removed pane.**
  - **5, the default instance's tmux environment.** An observation for #778's repeat.
  - **6, ADR decision 34's wrapped span.**
- Wireframe: N/A (no UI surface).

## Tier 1 self-check (incl. tests now GREEN)

```
$ cargo test -p holler-cli --test wizard_scripts
test result: ok. 303 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
$ PATH=<mawk shim>:$PATH cargo test -p holler-cli --test wizard_scripts        # and the same with busybox awk
test result: ok. 303 passed; 0 failed                                           (both)
$ cargo test -p holler-cli --test docs_cli_test --test docs_pane_control_test
test result: ok. 3 passed; 0 failed      test result: ok. 19 passed; 0 failed
$ cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load   # the CI command
1st run: exit 101 at holler-hub's profile_rename_faults_test (1 failed), the run stopped there (fail-fast);
         a flaky inode assertion, unrelated to this change (Known issues)
$ cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load
167 binaries, every one "test result: ok"; 2624 passed, 0 failed; exit 0   (the same totals as F's and T-green's runs)
$ cargo clippy --workspace --all-targets -- -D warnings   # exit 0 (no Rust changed in this cycle)
$ bash scripts/lint.sh            # exit 0 (only the existing 600-line warnings)
$ bash scripts/changelog-check.sh # changelog-check: ok
$ bash -n panes.sh; shellcheck -s bash panes.sh                                  # clean
$ git diff --check -- agent-skills docs/setup-wizard.md CHANGELOG.md            # clean
```

**S's repro, by hand** (scratch configs in the session scratchpad):

| Case | Result |
|---|---|
| `layout = [` / `["o1", "a1"],` / `["b1"]` / `]`, then `workspace = "a=b"` | exit 1, one message: `layout must be written on one line, the line of its key (line 5 leaves it open): ...` (was: exit 0, `workspace holler`) |
| `workspace` first, then a `layout` closing on its second line | exit 1, the same message |
| a continuation line that does not start with `[` | exit 1, the same message |
| a one-line `layout` with a trailing comment, then `workspace = "my relay"` | exit 0, `workspace my relay, profile my relay` |
| a one-line `layout`, then `workspace = "a=b"` | exit 1, `instance.sh`'s workspace message (Acceptance 6 holds) |

Under gawk, mawk and busybox awk the output is the same: the refusal, and `plan`'s lines for the one-line config.

## Evidence appendix

`docs/handoffs/803/evidence.md`, section "Added by F (Phase 6 rework, S's REWORK items 1 to 3)", has seven entries:

- `instance.sh` reads a line starting with `[` as a header, and reads `workspace` only before one;
- a pane verb keeps configuration errors until a port's first use, and Herdr is lazy;
- an unset `HOLLER_TMUX_SOCKET` is tmux's default server;
- `close_live` stops the host's processes before the Herdr close;
- `refuse_orchestrator` and `--as-operator`;
- a reset leaves the old session as a stray;
- a Claude Code pane is reset by a relaunch and has no `stray-session`.

## Tests that look wrong (for T)

None is wrong. **S's item 2 asks T to add one test:**

- **The case.** A `panes_803` case with a multi-line `layout` followed by `workspace`, for example S's repro:
  `layout = [` / `["o1", "a1"],` / `["b1"]` / `]`, then `workspace = "a=b"`.
- **What it should pin.** `check` exits 1, and its stderr names `layout` and says "one line". With option (a) it does
  not name `workspace`.
- **A useful positive control.** The same panes with `layout` on one line pass and keep the label, as `grid`'s
  `workspace` line shows.

**Optional text pins, if T wants them.** No current test pins items 1, 3 or 4. For example:

- Stage 11's first note contains `herdr.sh holler pane close` and no longer `` `HOLLER_STATE_DIR` in your own shells ``;
- Stage 10 contains `pane reset PANE --as-operator` and `stray-session`;
- "Basic Herdr commands" in the docs contains `herdr.sh holler pane close`;
- the docs no longer contain "column 1 by row".

## Known issues

- **Unchanged from handoff-F.md.** U1 to U8 stay UNVERIFIED until the operator's repeat of #778. #839, #842 and #850 are
  still open, so Stage 9's DRIVEN sentence and D13's "no body row for a Claude Code pane" stand.
- **S's advisory 1 (the one-sided socket guard) is not taken** (Deviations). Item 1 now sends users through the door
  from their own shells. That makes the guard slightly more relevant, but only if the user's shell exports a stale
  `WIZARD_*_SOCKET`.
- **One `holler-hub` test is flaky in this session's environment, not because of this change.**
  - **The test.**
    `profile_rename_faults_test::a_failed_profile_save_puts_the_pane_file_back_and_the_same_rename_then_succeeds`
    asserts that the pane document has a new inode number after it is replaced and then saved back by two atomic
    renames (`crates/holler-hub/tests/profile_rename_faults_test.rs:250-254`, the inode read at line 236).
  - **Why it flakes here.** This session's `TMPDIR` is an ext4 directory (`~/.cache/tmp`), and ext4 hands a freed inode
    number straight back unless another file takes it first. A scratch replace-twice demo in that directory got the
    same inode back 10 times of 10.
  - **What happened.** The test fails 10 times of 10 run alone. It failed in my first full run, which then stopped
    there (fail-fast). It passed in the second, `--no-fail-fast` run, where other tests were creating files in
    parallel.
  - **Why it is not this change.**
    - `/tmp` on this machine is tmpfs, which numbers inodes from a counter.
    - CI on `main` at the merge base `6eff9cc` is green.
    - `crates/holler-hub` is byte-identical to `origin/main`.
    - F's and T-green's runs of this story passed it.
  - **A follow-up for the test's owner (#665).** Compare something a reused inode number cannot fake: for example, hold
    the old file open so its inode stays allocated, or compare its change time. I did not file it: an outward action is
    the orchestrator's call.

## Files changed

- `agent-skills/herdr-setup/lib/panes.sh`
- `agent-skills/herdr-setup/SKILL.md`
- `docs/setup-wizard.md`
- `CHANGELOG.md`
- `docs/handoffs/803/handoff-F-rework.md` (new), `docs/handoffs/803/decisions.md`, `docs/handoffs/803/evidence.md`
