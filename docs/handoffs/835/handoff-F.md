# Handoff-F: #835 - `holler pane import` leaves Claude Code panes alone, with `--adopt-claude-code`

**Date:** 2026-10-11 (18:40 MDT)
**Branch:** `issue-835-implementation` at `36b0de8` (base `2d217dd`, #649 merged), F's changes uncommitted
**Issue:** #835

## What was done

The port, the fake and the adapter:

- `crates/holler-pane/src/ports.rs`: `HostPort::foreground_commands(&self, name: &PaneName) -> Result<Vec<String>, PaneError>`,
  required (R2-1), beside `ps`. `HerdrPort::observe_pane` lost its default (R3-1); `observe_panes` keeps its default. The
  module docs list #832's and #835's amendments (A's W2).
- `crates/holler-pane/src/pane.rs`: `pub const CLAUDE_CODE_COMMAND: &str = "claude"` and `pub fn is_claude_code_command(name: &str) -> bool`
  (exact match), beside `HarnessKind` (R2-7). Their docs say the name is a process name, not the protocol's harness id,
  and record the rule's limits (R2-5).
- `crates/holler-pane-testkit/src/host.rs`: `HostOp::ForegroundCommands` (`host.foreground_commands`). `FakeHost` keeps
  each process's command name (the basename of its argv's first element) beside its pid, so `exit_process`, `stop_owned`
  and `end_session` take the name with the process; `foreground_commands` answers them in start order, `pane-not-found`
  for a missing session, through the faults and the call log like every method.
- `crates/holler-adapter-host/src/tmux.rs`: `foreground_listing` (`list-panes -s -t =NAME: -F "#{pane_dead} #{pane_current_command}"`)
  and the strict `parse_foreground`, beside `panes`/`parse_ps`.
- `crates/holler-adapter-host/src/lib.rs`: `TmuxHost::foreground_commands` through `run_tmux`, `Call` and `refused()`;
  `OP_FOREGROUND_COMMANDS`; the crate docs' error table gains the column, and a paragraph says what is read (I7).

The forwards (the compiler listed them):

- `crates/holler-cli/src/pane/wiring/bind.rs`: `BindingHost` and `Wired<dyn HostPort>` forward `foreground_commands`.
  The module docs' sentence that listed `observe_pane` among the defaulted methods is corrected (one sentence).
- `crates/holler-cli/src/pane/wiring.rs`: `Unwired::foreground_commands` answers `NotImplemented`.

The import:

- `crates/holler-pane/src/import/findings.rs`: `HarnessClaudeCode` (`harness-claude-code`, `info`) and `HarnessUnreadable`
  (`harness-unreadable`, `problem`), after `HarnessNotOpencode` in `ALL`.
- `crates/holler-pane/src/import/observe.rs`: the chain is `ps`, then (only once `ps` answered) `foreground_commands`,
  then the server. `Observed` is now `ClaudeCode`, `Seen(Chain)` or `Lost`. A session that runs Claude Code ends the
  chain: no harness call. `tmux_finding` maps a `pane-not-found` from either read to `tmux-session-missing` and any other
  error to `harness-unreadable` naming the call through `failed_message`; `lost_finding` is the lost chain's
  `harness-unreadable`.
- `crates/holler-pane/src/import/assess.rs`: `Outcome::claude_code`, beside `not_opencode`, through `left_alone`
  (`Action::Skipped`, one finding). `assess_all` routes `ClaudeCode` to it before any join, so such a pane has exactly
  one finding even when it would have had a join problem. A lost chain is `harness-unreadable` (was `observe-failed`).
- `crates/holler-pane/src/import.rs`: module docs (the reads, the rule) and `Action::Skipped`'s doc.

The switch:

- `crates/holler-cli/src/pane/import.rs`: `--adopt-claude-code` (`adopt_claude_code: bool`), refused as `usage` at the top
  of `Input::read` (step 1, before any file or port), with the message
  `--adopt-claude-code: not implemented (story #831); without it the import leaves a pane that runs Claude Code alone`
  built from `not_implemented_message(831)`. The verb's help text gains the rule, its limits, the two codes and the switch
  (R2-5 asks for the limits in the help text).

The docs:

- `docs/adr/ADR-0021.md`: section 2 (the `HerdrPort` and `HostPort` rows, the "amended since" sentence), section 9 (the
  `pane import` row), section 13 (a "Decided (#835)" paragraph: the signal, the rule, never on a guess, the limits, the
  switch, and why the registry rule of `unmanaged` is not used), section 14 (the `observe_pane` bullet, amended in
  place), "Deferred to named stories" (the post-#831 adoption, no owner yet) and "Decisions taken" (22 and 23).
- `docs/adr/ADR-0003.md`: the verb's line gains `[--adopt-claude-code]` and `#835`.
- `CHANGELOG.md`: one `## [Unreleased]` Enhancements entry linking #835 (and #831, #832).
- `docs/handoffs/835/evidence.md`: six facts in unchanged code.

## Design decisions

1. **A pane whose tmux session cannot be read still has its server asked.** Only a pane known to run Claude Code skips
   `harness.health` and `harness.list_sessions` (brief section 3; A's note 4: "health and the session list only for a pane
   that is not `claude`"). A pane whose `ps` or names read failed is not known to be Claude Code, so it is observed as #650
   observed every pane, and its report keeps the server's findings. This keeps `tmux-session-missing` exactly as #650
   reported it (a server finding beside it when the server is down). The alternative, asking no server for any pane
   whose session was not read, would cut a few reads of a pane that is blocked anyway, but would change #650's report for
   a missing session and add a second branch to the chain. A harness read is harmless, and none is made of a Claude
   Code pane.
2. **`harness-unreadable` replaces `observe-failed` for a failed `ps` and a lost chain; it is not added beside it.** One
   finding per cause: the message is `what the pane's tmux session runs could not be read, so the pane is not recorded:
   host.ps failed (unavailable): ...`, the call named through `failed_message` (A's W4), so the report still says which
   read failed. The lost chain's message ends `its check did not finish`, as #650's did. `observe-failed` stays for the
   harness reads only.
3. **The types: `Observed::{ClaudeCode, Seen(Chain), Lost}`.** `Chain` holds `tmux: Result<(), Unread>` (the op and the
   error of the read that failed) and the server. `assess` takes `Option<&Chain>` (`None` = lost), so a Claude Code pane
   cannot reach the join at all, rather than being one more arm the join must ignore.
4. **The adapter's format is `#{pane_dead} #{pane_current_command}`, no pid.** The dead flag lets the parser drop a dead
   pane as `ps` does, and makes the listing strict (T's `a_malformed_listing_is_unavailable`); the name comes last, so it
   is the rest of its line and cannot shift the flag. tmux cuts the name at its first space, so a line has one space
   before the name. A pid adds nothing the method answers.
5. **The fake names a process by `Path::file_name` of its program**, which is POSIX basename's reading (`/usr/bin/` is
   `bin`); a program with no file name (`/`) is named as given. It never sees a shell, so a fresh session names nothing,
   as before.
6. **The finding order:** `HarnessClaudeCode` and `HarnessUnreadable` sit after `HarnessNotOpencode` in `ImportFinding`,
   so a pane's harness findings come first in its list, before the file and join problems. No test pins the order.
7. **The switch is checked first in `Input::read`**, before `--herdr-session`: it refuses the request whatever the other
   arguments are, so it is said first.
8. **Merge-friendly placement against `origin/main`** (which has moved past this branch's base; see Known issues). In
   ADR-0021 section 14 the `observe_pane` bullet keeps its first line, so this hunk is one unchanged line away from
   #840's hunk there; the CHANGELOG entry goes after #650's entry, away from the lines #652 and #840 changed.

## Reuse / extend-vs-new

Extended, per A's reuse map; no parallel path:

- `HostPort` beside `ps`; in the adapter, a listing beside `panes()`/`stop_listing()`, a parser beside `parse_ps`, the one
  `run_tmux`/`Call`/`refused()` path and an `OP_*` constant.
- The import's one per-pane thread scaffold (`observe_all`), one more read on the same thread; `tmux_finding` and
  `failed_message` reused for the read's errors.
- `Outcome::left_alone` through a constructor beside `not_opencode`; `Action::Skipped`; two kinds in `ImportFinding::ALL`.
  No new `Action` or `Level`. A problem kind blocks through `assess`'s existing `blocked` and `Plan::refusal`.
- The rule is one constant and one predicate in `pane.rs`; the only comparison with `"claude"` is in that predicate. It is
  not `unmanaged`'s registry rule (section 13's last bullet says why).
- The decorators forward to the inner port (`BindingHost`, `Wired`); `Unwired` answers not-implemented, as for every method.
- The switch reuses `Input::read`'s `usage()` and `output::not_implemented_message`.

## Architecture notes for A

- **Public interface changes:** `HostPort` gains a required method; `HerdrPort::observe_pane` is required (no default);
  `holler_pane::pane::{CLAUDE_CODE_COMMAND, is_claude_code_command}`; `HostOp::ForegroundCommands`;
  `PaneImport.adopt_claude_code` (the CLI surface, `cli-surface.txt` line already T's). Every implementation in the
  workspace now answers both methods; T added the ones in test files.
- **Layers:** the rule lives in `holler-pane` (domain); the tmux read in `holler-adapter-host`; the switch in
  `holler-cli`. `holler-pane` knows no CLI flag: the switch's text is the CLI's, the finding's message is the import's.
- **No new dependency.** No protocol change, no new error code (the two codes are findings), no hub state.
- **#649's files:** only the forwards, `Unwired`, and the one module-doc sentence of `bind.rs` that named `observe_pane`
  among the defaulted methods (it is not, after #835).

## Deviations from spec / wireframe

None in behaviour. Two readings, recorded here for S:

- R2-6's "a failed `ps` ... is `harness-unreadable`" is read as the one finding for that cause (it replaces #650's
  `observe-failed`), and the pane's server is still asked (design decision 1).
- R2-2 asked for ADR-0021 sections 2, 9, 13 and the closing lists; A's W2 added section 14 and `ports.rs`'s module docs.
  All are done; section 14 is amended in place, as W2 says.
- R3-1 asked to record `observe_pane`'s new form in section 13's "Decided (#835)" paragraph too. A's W2 (the PASS review,
  later than the brief's round 3) keeps that paragraph to the import and puts the port change in sections 2 and 14. F
  followed W2: section 2's row and "amended since" sentence, section 14's bullet, and "Decisions taken" item 23.

## Tier 1 self-check (incl. tests now GREEN)

```
$ cargo build --workspace --all-targets
    Finished `dev` profile [unoptimized + debuginfo] target(s)            (no warning)
$ cargo clippy --workspace --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s)            (exit 0)
$ bash scripts/lint.sh            -> exit 0 (warnings only for files this change does not touch; every changed
                                     source file is under 600 lines, the largest crates/holler-adapter-host/src/lib.rs at 521)
$ bash scripts/changelog-check.sh -> changelog-check: ok
$ git diff --check                -> clean
$ cargo doc --no-deps -p holler-pane -p holler-pane-testkit -p holler-adapter-host   -> 0 warnings

$ cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load
  153 result groups: 2375 passed, 2 failed, 32 ignored. The only failures:
    import::harness::a_dry_run_reports_the_pane_left_alone_and_writes_nothing                 harness.rs:75:43 "a spec name"
    import::harness::a_pane_running_claude_is_left_alone_and_the_others_import_as_opencode    harness.rs:75:43 "a spec name"
  (T's `spec_names()` reads the key `name`; see "Tests that look wrong".)

T's RED tests, now GREEN:
  holler-pane       import_test (5), claude_code_rule_test (2), ports_test (8), doctests ports.rs (4, both
                    compile_fail,E0046 pass)
  holler-pane-testkit  fake_host_test (incl. the three foreground_commands tests and the wedged/op-name ones),
                    host_conformance_test (14, incl. the two new Mutant breaks)
  holler-adapter-host  fake_tmux_test (29, incl. every_method_is_bounded_and_reaps_its_child and
                    a_missing_tmux_binary_is_unavailable_from_every_method), foreground_test (4)
  holler-cli        cli_surface_test (3), pane_integration (69 passed, 4 ignored: incl.
                    the_wired_host_passes_the_host_conformance_suite, the wired.rs foreground line and the two R3-3
                    pins), pane_verbs import:: (44 of 46: the_adrs_name_the_switch_and_the_two_new_codes, and
                    import::harness 5 of 7; the other 2 are the spec-key test bug)

After the last (doc-comment and help-text) edits, re-run: clippy (exit 0); holler-pane, testkit and adapter-host
(380 passed, 0 failed, 9 ignored); holler-cli pane_verbs, cli_invocation_test, cli_surface_test, docs_cli_test (the
same two failures only).

F's spot check on a real tmux (a private scratch server; not T's Tier 2):
$ cargo test -p holler-adapter-host --test real_tmux_test -- --ignored the_tmux_host_passes_the_host_conformance_suite
  test the_tmux_host_passes_the_host_conformance_suite ... ok          (tmux 3.7c; case 10 names `sleep`)
```

## Evidence appendix

`docs/handoffs/835/evidence.md` (six facts: the problem level blocks the row and the real run, the import skips a
recorded pane before observing, the adapter's `refused()` mapping, `not_implemented_message`'s wording, `usage` is exit 2).

## Tests that look wrong (for T)

1. **`crates/holler-cli/tests/pane_verbs/import/harness.rs:75`, `spec_names()`**, reads `spec["name"]`, but a profile
   spec's JSON key for its pane is `pane` (`ProfileSpec.pane`, `crates/holler-pane/src/profile.rs:181`, no serde rename).
   It panics `a spec name` in two tests, after every assertion before it has passed:
   `a_pane_running_claude_is_left_alone_and_the_others_import_as_opencode` and
   `a_dry_run_reports_the_pane_left_alone_and_writes_nothing`. Test-only: with the key changed to `spec["pane"]` (a
   temporary local edit, reverted at once and checked byte-identical with `sha256sum -c`), all seven tests of
   `import::harness` pass, including the harness-call count `(2, 2)` and the text form. F did not edit the test.
2. Cosmetic: `rustfmt --check` would reformat lines in several of the new or edited test files (for example the long
   import line of `crates/holler-pane/tests/ports_test.rs:19`). CI runs no `rustfmt`, so nothing fails.

## Known issues

1. **The branch is behind `origin/main`.** Base `2d217dd`; `origin/main` is `bda1069` (18:16 MDT), with `7fc00f1` (#655
   docs), `0e62b51` (#652, the operator guide `docs/pane-control.md`) and `bda1069` (#840, #832's review fixes). F does
   not commit, so nothing was merged. Before or at the PR:
   - **#840's two claims about #835 (A's W3).** On main, `crates/holler-pane/src/unmanaged.rs` (the docs of
     `unregistered` and `is_unmanaged`) and ADR-0021 section 14 (`... that #831 and #835 call instead of writing their
     own`) say #835 calls the registry rule. It does not, and cannot: section 13's "Not the registry's rule" bullet says
     why. Those sentences are not on this branch, so F could not correct them; they need a one-line edit each once main
     is merged in (drop "#835" or point at `holler_pane::pane::is_claude_code_command`).
   - **#652's guide** describes `pane import` and does not yet know the new default, the two codes or the switch (brief
     section 6: "#652's guide ... should be told"). It is not on this branch.
   - **Expected merge overlap:** ADR-0021 section 14 (this branch's hunk is one unchanged line from #840's), `CHANGELOG.md`
     (no overlap expected) and T's `crates/holler-cli/tests/pane_verbs/launch/rig.rs` (#840 changed it too; the T forward
     is 4 lines at the `LinkedHost` impl).
2. The epic's contract block (#633's issue body) is the operator's to edit (brief section 1, "The records"); nothing here
   changes it.

## Files changed

Production:

- `crates/holler-pane/src/ports.rs`
- `crates/holler-pane/src/pane.rs`
- `crates/holler-pane/src/import.rs`
- `crates/holler-pane/src/import/assess.rs`
- `crates/holler-pane/src/import/findings.rs`
- `crates/holler-pane/src/import/observe.rs`
- `crates/holler-pane-testkit/src/host.rs`
- `crates/holler-adapter-host/src/lib.rs`
- `crates/holler-adapter-host/src/tmux.rs`
- `crates/holler-cli/src/pane/import.rs`
- `crates/holler-cli/src/pane/wiring.rs`
- `crates/holler-cli/src/pane/wiring/bind.rs`
- `docs/adr/ADR-0021.md`
- `docs/adr/ADR-0003.md`
- `CHANGELOG.md`

Run artifacts: `docs/handoffs/835/handoff-F.md`, `docs/handoffs/835/evidence.md`, `docs/handoffs/835/decisions.md`
(appended).
