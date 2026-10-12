# Handoff-A-dup: #835 - `holler pane import` leaves Claude Code panes alone, with `--adopt-claude-code`  (anti-duplication gate)

**Date:** 2026-10-11 (19:48 MDT)
**Branch:** `issue-835-implementation`
**Diff base:** `bda1069` (`origin/main` at 19:42 MDT after `git fetch`, and the merge base)   **Diff head:** `b488a33`
**Reuse map:** `docs/handoffs/835/handoff-A.md` (pass 4: "Reuse map" and its list of parallel paths to flag)
**Verdict:** PASS

## Summary

PASS: 0 `block`, 1 `warn`. F extended every object the map named, and the diff has no parallel path.

- **Scope of the review.** This is the run's first A-dup. Cycle 1's code (`9bd101b`) never reached this gate, because its T-green returned BLOCK. So this pass reviews the whole feature diff (`bda1069..b488a33`): 37 files outside `docs/handoffs`.
- **Production code.** Each item extends the object the map names, in the map's layer. The read is a `HostPort` method beside `ps`. The rule is one constant and one predicate in `pane.rs`. The chain gains one read on the same per-pane thread. The left-alone outcome is one constructor through `left_alone`. The switch sits in step 1 of `Input::read`. See the table below.
- **The rework added no drift.** F's cycle 2 (`526bbd3..4604693`) touched three Rust files, and only comment lines changed in them. Its other edits are the guide and ADR-0021.
- **The merge is clean.** `526bbd3` is a two-parent merge of `bda1069`. Its tree is `git merge-tree --write-tree 529467f bda1069` plus two things:
  - T-red's three test-file edits;
  - the run's handoff files.

  No file from `main` changed beyond the sentences A's W2 to W4 named.
- **The warn is about tests only:** a rig method was copied instead of shared.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-cli/tests/pane_verbs/import/rig.rs:258` | `ImportRig::run_with_host` (T's, #835) copies `run_with_store` (`:237`) line for line and changes only which port it replaces. The copied lines are the `ExtraHerdr` build, the argv conversion and the `run_verb_with` call. It extends the rig, so it is not the map's "new rig". But it extends it by copying.<br>A smaller twin: `text_row` (`tests/pane_integration/observe.rs:279`) is the same six lines as the import rig's `text_line` (`rig.rs:528`), in another test binary. | Optional, T's, and not needed for this PR:<ul><li>one private rig method that takes the port overrides, for example `run_over(argv, format, \|ports\| Ports { host, ..ports })`, called by both `run_with_store` and `run_with_host`;</li><li>`text_line` could move to `verb_harness`, which both test binaries include through `#[path]`.</li></ul> |

No duplication of production code. The extension is clean.

## The extension, object by object (against the map)

| The map's object | In the diff | Parallel path? |
|---|---|---|
| **The port read.** The map named:<ul><li>`HostPort` beside `ps`;</li><li>in the adapter, a listing beside `panes()` and `stop_listing()`;</li><li>a strict parser beside `parse_ps`;</li><li>`Call` and `refused()`;</li><li>an `OP_*` constant.</li></ul> | **The port.** `ports.rs:348` is required, with no default.<br>**The adapter.** `TmuxHost::foreground_commands` (`lib.rs:406`) has `ps`'s shape: `Call::start`, `run_tmux`, a strict parse, and `refused()` for a failed call. The constant is `OP_FOREGROUND_COMMANDS`.<br>**The listing and the parser.**<ul><li>`tmux::foreground_listing` (`tmux.rs:146`) sits beside `panes` and `stop_listing`.</li><li>`parse_foreground` (`:253`) sits beside `parse_ps`, and is just as strict: one bad line fails the read.</li></ul> | **None.**<br>The adapter now has three `list-panes -s -t =NAME: -F` builders that differ only in their format. That follows the module's own rule, "one pure function per command line" (`tmux.rs:1-2`), and the map's shape, so it is not flagged. |
| **The rule.** One public constant and one predicate, beside `HarnessKind`. | `CLAUDE_CODE_COMMAND` (`pane.rs:147`) and `is_claude_code_command` (`:157`).<br>This is the workspace's only comparison of a command name with `claude`. Every other `"claude"` in `src` is a protocol harness id: `holler-proto`'s vocabulary, the hub and the body.<br>In `unmanaged.rs`, only the doc comments changed. The rule is not folded into `is_unmanaged`. | none |
| **The observation.** The chain in `observe()`, with one read after `ps`, on the same per-pane thread. `tmux_finding` and `failed_message` for its errors. | **Unchanged:** `observe_all` and `joined`.<br>**Added:** `read_tmux` (`observe.rs:117`), which runs inside `observe` on the pane's own thread.<br>**Extended in place:**<ul><li>`tmux_finding` (`:207`) now takes `Unread`, the failed call and its error. Its one mapping is the same for both reads: `pane-not-found` gives `tmux-session-missing`, and any other error gives `harness-unreadable`.</li><li>`failed_message` is used for both reads.</li></ul>**Moved:** `lost_finding` (`:228`) is #650's inline "not observed" finding, moved out of `assess.rs` and given the new kind. It is not a copy. | none |
| **A pane left alone.** `Outcome::left_alone`, with one constructor beside `not_opencode`. `Action::Skipped`. One kind in `ImportFinding::ALL`. | `Outcome::claude_code` (`assess.rs:103`) goes through `left_alone`. `assess_all` routes the pane (`:133-137`).<br>Two kinds are added to `ALL`, `code` and `level`. There is no new `Action` and no new `Level`. | none |
| **The unreadable signal.** One more `Level::Problem` kind, through `assess`'s `blocked`. | `HarnessUnreadable` is a `Problem`. It blocks through the existing `blocked`, and a real run is refused through `Plan::refusal`. There is no second refusal path. | none |
| **The forwards and their pins.**<ul><li>The forwards: `BindingHost`, `Wired<dyn HostPort>` and `Unwired`.</li><li>The pins: the host suite over `BindingHost` and over `WiredHost`.</li></ul> | **The forwards.** `bind.rs:185` and `:357` forward to the inner host. `wiring.rs:328` answers `NotImplemented`, as `Unwired`'s other methods do.<br>**#649's files.** They changed only by these forwards and by the module doc that describes them, as brief R2-1's lift allows.<br>**The production path.** The import reads the host through `ports.host`. The wiring builds it as `BindingHost` over `WiredHost` over `TmuxHost` (`wiring.rs:153`, `:172`), so no read goes around `BindingHost`.<br>**The pins.** `WiredHost`'s new suite run sits beside `BindingHost`'s existing one. | none |
| **The test doubles.** None may answer without reaching the inner host. | All 11 `HostPort` implementations answer the method. The `compile_fail` doctest's `PidsOnly` leaves it out on purpose.<ul><li>The three `LinkedHost`s and `LostChain` forward it.</li><li>The `Mutant` forwards it, except under its two new breaks.</li><li>`TestHost` (`ports_test.rs`) is a stub, not a wrapper.</li></ul> | none |
| **The fake and its pins.** `FakeHost` names each process. One `HostOp` variant. | **The fake.** `Session.pids` became `processes` (a pid and a name). `ps` and `foreground_commands` share one helper, `each_process` (`host.rs:250`).<br>**The pins:**<ul><li>one `HostOp` variant;</li><li>conformance case 10 and two `Mutant` breaks;</li><li>`METHODS` and `call` in `fake_tmux_test.rs` (3 more lines).</li></ul>**The new adapter tests** are in the new `foreground_test.rs`, which uses `tests/common`. | none |
| **The switch.** Step 1 of `Input::read`, `usage()` and `output::not_implemented_message`. | `pane/import.rs:175-181` uses the file's own `usage()` and `not_implemented_message(CLAUDE_CODE_STORY)`.<br>`CLAUDE_CODE_STORY` follows the existing story constants `ROUTING_STORY` and `ROSTER_STORY`. | none |
| **The tests.** No copies of D33's `observe_*` tests, and no new rig. | **The two R3-3 tests** (`pane_integration/observe.rs`) run `pane list` end to end over the production Herdr chain. D33's tests check the port calls with a recording double, so these are not copies of them.<br>They overlap `pane_verbs/unmanaged.rs`'s list tests in what they assert, with the chain added. Brief R3-3 asks for exactly that, so the overlap is a reviewed choice. They use `seed_unmanaged_pane` and `run_verb_with`.<br>**The import tests** extend `ImportRig`: `run_with_host` (finding 1) and the read allow-list. | none (finding 1) |
| **The guide.** One findings table. Link to ADR-0021 section 13, and do not restate it. | Section 3's one table gains two rows.<br>Section 10's fact gives the operator the rule, its limits and the check, and links section 13 as the contract (`pane-control.md:704`). The port read, "never on a guess" and the switch's reasons stay in the ADR.<br>`main`'s files changed only in the sentences W2 to W4 named. | none |

**Beyond the map:**

- **Hub and protocol.** The overlay's Phase 7 list (the token store, `Lockout`, `Roster`, `log`, and the hub test helpers) is untouched. No protocol, hub or body file changed, and no golden file.
- **File sizes.** Every touched `.rs` file is under 900 lines. The largest are `pane_verbs/launch/rig.rs` (839, one forward) and `fake_tmux_test.rs` (820, `METHODS`). Both are as the plan said.
- **Names.** The non-handoff diff contains no personal host, account or network names.

## Notes for F

None: the verdict is PASS.

## For S and the operator (outside this gate)

1. **For S: what the docs say is read.** The user-facing text says the import reads "the command name of each process in the pane's tmux session":
   - `pane/import.rs:68`;
   - `pane-control.md:696`;
   - `CHANGELOG.md:70`;
   - the guide's row at `:299`, which says "A process in its tmux session".

   The contract is narrower: the foreground process of each live pane (`ports.rs:340`, ADR-0021 `:953`). A `claude` that a script starts as a child, while the script stays in the foreground, is not seen. The limits sentence covers this case as "a wrapper", so the wording may be enough. The call is S's.
2. **For #831's owner.** #831 replaces the refusal at one code site, `Input::read` (`pane/import.rs:175`). But "refused until #831" is also written in prose, and no test checks that prose:
   - ADR-0021: section 9's row, section 13's "The switch", and "Deferred to named stories";
   - the guide, sections 3 and 12;
   - the help text of `PaneImport` and of the flag;
   - the CHANGELOG.

## Patterns referenced

- `docs/handoffs/835/handoff-A.md` (pass 4): the reuse map and its list of parallel paths.
- `crates/holler-adapter-host/src/{lib,tmux}.rs`: `ps`, `panes`, `stop_listing`, `parse_ps` and `refused`.
- `crates/holler-pane/src/import/{observe,assess,findings}.rs`, at the base and at the head (`git diff bda1069 HEAD`).
- `crates/holler-cli/src/pane/{wiring.rs, wiring/bind.rs, import.rs}`, and `crates/holler-cli/src/output.rs` (`not_implemented_message`).
- Tests:
  - `crates/holler-cli/tests/verb_harness/mod.rs`;
  - `tests/pane_verbs/{unmanaged.rs, import/rig.rs}`;
  - `tests/pane_integration/{observe,bind}.rs`;
  - `crates/holler-pane-testkit/src/{host.rs, conformance/host.rs}`.
