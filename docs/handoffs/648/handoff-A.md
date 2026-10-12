# Handoff-A: Phase 3 - #648 the roster shows pane, SHOWN and DRIVEN, and a loud line when they differ  (up-front plan review)

**Date:** 2026-10-11
**Branch:** issue-648-implementation (worktree `.claude/worktrees/0648b-roster-shown-driven`, head `1c81e2d`; `origin/main` is `bda1069`, fetched during this pass, nothing merged since)
**Brief reviewed:** `docs/handoffs/648-brief.md` at `1c81e2d` (sha256 `1f12c6140e9e6508...`)   **Reuse map:** no separate file. The brief's Evidence (E1 to E20), the refresh decisions (R1 to R10) and "Decisions on the review's blocking points" serve as the map.   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

## Summary

**PASS, with eight binding warns (W-1 to W-8) and four that are not binding (W-9 to W-12).**

The plan extends the right objects in the right layer:
- `roster_cmd.rs` keeps the hub call and joins the hub's rows to the pane registry CLI-side.
- The cells come from `pane::list::PaneRow`, whose module doc already offers itself to the roster (`list.rs:9-10`).
- The registry is read through the `Ports` that `Wiring::connect` builds, which is ADR-0021 ruling 1's one construction site.
- `--profile` resolves through `ProfileScope`, and output goes through `output::emit` and `emit_error`.
- The hub's `Row`, the `control/roster` reply, the wire and `holler-pane` are untouched, and nothing persists.

**Why this is not a BLOCK.** Everything the brief leaves inconsistent or open can be settled without changing an acceptance criterion. So these are binding warns, as in the #649 precedent. A BLOCK on this brief would stop the run for an amendment that the warns already supply.

The binding warns:
- **W-1, the seam.** The brief states the seam between the hub call, `Wiring::connect` and the registry three ways that cannot all hold. The warn gives the shape that satisfies R2, R3, R8 and AC 11.
- **W-2, the pair rule.** The SHOWN/DRIVEN pair rule would get a third copy, and a second one inside the CLI crate. It must be one named predicate, pinned to its CLI twin by a test.
- **W-3, ADR 0003.** Two more ADR 0003 sentences (lines 91 and 99) and the global flags' `--help` become untrue. A Blast radius exception is approved.
- **W-4, the error boundary.** It is undecided for the three output modes. In particular, `RemotePolicyRefused` must stay exit 3 under `--format=json`.
- **W-5 to W-8:**
  - one source for the six cells (W-5);
  - the state dir of in-process tests, where the live hub is one unset variable away (W-6);
  - the text layout and where the "legend" lives (W-7);
  - three ADR-0021 sentences the change extends (W-8).

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| W-1 | warn (**binding on T and F**) | R2, R8; the decision "`roster_cmd::run`'s new signature" (brief line 148); AC 11 round 6 | layering | `run(roster, choice, ports)`, with `Ports` built "the way `run_with_stdio` does", forces `connect` before the hub call. R2 says `connect` comes only after a reply, never under `--server`, with `main.rs` as its one caller. | `run` takes a deferred source of ports, which `main.rs` builds around `Wiring::connect`. One pure function takes the reply and the registry as ports, error or "not read". Details below. |
| W-2 | warn (**binding on T and F**) | "The mismatch rule is the existing pair rule" (line 143), E11 | duplication | The rule already exists twice, each copy marked "keep the two equal" (`say_cmd.rs:358-364`, `dispatch.rs:77-103`, ADR-0021 line 708). An inline roster copy would be a third, unpinned one. | One named predicate in `roster_cmd.rs`, whose doc names its twins, pinned to `say_cmd::resolve_pane_target` by a test. Or, with an exception approved, one `pub fn` in `pane::list` that say_cmd's third arm also calls. |
| W-3 | warn (**binding on F**; Blast radius exception approved) | R10 ("nothing else in that file changes"); scope item 5 | ADRs | These become untrue for `roster`: ADR 0003 line 91 ("`--format=json` is `--json` on every verb"), line 99 (only pane and profile print the envelope), and `cli.rs:63` and `:71-75` (the `--json`/`--format` help). | Edit the two ADR sentences and the two help comments in the same change. These are factual edits, like R10's. |
| W-4 | warn (**binding on T and F**) | scope item 5, AC 4, AC 11, R3, R4; `roster_cmd.rs:50-57` | cross-cutting (error boundary) | Undecided:<ul><li>the envelope code of a failed hub call (`RemotePolicyRefused` exits 3 today; `unavailable` would make it exit 1 under `--format=json`);</li><li>what a bare `--json` prints for an error;</li><li>whether a usage error found after parsing is an envelope.</li></ul> | <ul><li>Three modes from `FormatChoice`.</li><li>Every roster `usage` is a stderr line, in every format.</li><li>Other errors go through `emit_error`.</li><li>Hub errors are `unavailable`, except `RemotePolicyRefused`, which gets a roster-owned open refusal code (exit 3).</li><li>R3's check runs first in `run`.</li></ul> |
| W-5 | warn (**binding on T and F**) | R5; R9's `PANE_COLUMNS` | duplication | Taking six of `PaneRow::cells()`'s nine by hard-coded index, or typing the six words a second time, breaks silently the day `pane list` moves a column. | Take each cell by its column's position in `pane::list::COLUMNS`. A unit test pins every `PANE_COLUMNS` entry to `COLUMNS`. |
| W-6 | warn (**binding on T**) | R8, AC 11 round 6; golden fixtures | cross-cutting (test isolation; no live fleet) | An in-process hub call reads `HOLLER_STATE_DIR` and, when it is unset, the default state dir, which is the live hub. LAST TURN and HOLD print ages from the wall clock. | Only AC 11 round 6 makes the hub call in-process, under a lock, with the variable on a scratch `StateDir`. Golden rows carry no wall-clock cell. |
| W-7 | warn (**binding on T and F**) | scope item 2 ("help and legend"), AC 1 | pattern consistency | The brief does not say how the columns are laid out after HOLD (today the last column, unpadded), or where the "legend" goes. AC 1 leaves stdout no room for a legend line. | Extend `render_table`'s fixed-width cadence. The legend is `--help` text, never output. |
| W-8 | warn (**binding on F**; Blast radius exception approved) | ADR-0021 lines 268-269, 321-322 and section 9's open codes | ADRs | ADR-0021 leaves "which verbs read a parked pane" to the roster story, and the brief does not answer it. Its list of the wiring's users omits the roster. W-4 adds an open code. | Three ADR-0021 sentences. Two code docs that become untrue go to a follow-up. |
| W-9 | warn (for O; not binding) | the `note:` lines; `data` has exactly two keys | pattern consistency | The CLI's stderr prefix is `warning:` (`main.rs:96`, `list.rs:102`). `pane list` reports a partial read in its JSON data (`unmanaged_unavailable`); the roster reports it only on stderr. | O decides at the next brief gate. F follows the brief as written. |
| W-10 | warn (not binding) | AC 10's join; the six keys written into the hub's row objects | pattern consistency | Label == pane name is a second relation between a row and a pane, beside session of record == bare session (ADR-0021 lines 705-709). The six keys share a namespace the hub owns. | One named join function whose doc cites AC 10 and #839. A test that the hub's row has none of the six keys. |
| W-11 | info | Blast radius line 133 | n/a | "Not changed, checked: ... ADR 0003 line 94" contradicts R10 (lines 36 and 118). | F follows R10 (and W-3). |
| W-12 | info (for T and S) | AC 7, 11 and 13; `--profile` | n/a | Spec observations outside A's dimensions (below). | See below. |

## Details for T and F

### W-1: the seam (binding)

**Why.** The brief states the seam three ways:
1. **R2's order.** `roster_cmd::run` makes today's hub call first. It calls `Wiring::connect()` only after a reply, and with `--server` it never reads the registry (R3).
2. **R2's caller.** "`main.rs`'s `run_roster` is the one place that does" call `connect`.
3. **Line 148 and R8.** `run(roster, choice, ports)` receives a `Ports` that `run_roster` builds "the way `run_with_stdio` does". `run_with_stdio` connects before the verb runs and turns a failed connect into exit 1 (`main.rs:256-270`).

These conflict in two ways:
- **(3) puts `connect` before the hub call, against (1).** A hung hub would cost an extra 5 s status wait before its own failure. A `--server` run on a machine with no local hub would need special-casing around a `connect` it must not make.
- **AC 11 round 6 needs both things in one entry.** It is in-process, with no live hub and an injected `FakePaneStore`, so it needs an entry that makes the hub call *and* takes injected ports. Only a deferred source of ports gives both.

**The binding shape.** Names are T's and F's.
- **The pure function (R8).**
  - It takes the hub's reply (`serde_json::Value`), the registry as one of three cases, the `Roster` flags, the `FormatChoice` and a `Sink`, and returns the exit code.
  - The three cases are:
    - the ports (`Ports<'_>`). The function makes the registry reads itself, so AC 7's empty call logs show something.
    - the `PaneError` a failed `connect` gave.
    - "not read" (`--server`).
  - AC 1 to 10 and AC 14 call it.
- **`run(roster, choice, registry, sink)`**, in this order:
  1. R3's check.
  2. Today's hub call.
  3. Only after a reply, and only without `--server`: `registry()`, a deferred source of `Result<Ports<'_>, PaneError>`.
  4. The pure function.

  AC 11 round 6 calls `run` with `|| Ok(rig.ports())` and checks that the source was never called.
- **`main.rs`'s `run_roster`** builds two things:
  - the source: a closure around `Wiring::connect()` that keeps the `Wiring` alive for the lent `Ports`, for example an `Option` or `OnceCell` declared before the call;
  - a `Sink` over stdout and stderr.

  `main.rs` stays the roster's only caller of `connect`, so `wiring.rs:3-7`'s list of callers stays true. No second construction site of ports appears (ADR-0021 ruling 1, lines 316-322).
- **A failed `connect` under `--profile` is R4's error, not R2's note.** It exits 1 through `emit_error`. `--profile P` needs the registry's answer, and a note over an empty listing would read as "P has no pane", which R4 forbids. R2's note is for the columns, without `--profile`.
- **Under `--profile`, the one registry read is `ports.scope.resolve(P, None)`.** Its `panes` are the join set, as in `pane list --profile` (`list.rs:133-136`). There is no `pane_store.list()` beside it: with a faulted store, that call would print a `note:` before AC 14's error, and AC 14 allows only the one `error:` line.
- **P is resolved even when the hub returned no rows.** So an unknown profile is `profile-not-found` (exit 3) whatever the roster holds.
- **`RosterResult` and the roster's use of `print_leaf_result` go.** The module doc of `roster_cmd.rs` and the doc of `print_leaf_result` (`main.rs:65-73`) are updated to say so.

### W-2: the pair rule's home (binding)

**Why.**
- **The two existing copies.**
  - The CLI's copy is inline in the third arm of `say_cmd::pane_state_refusal`, which is private (`say_cmd.rs:358-364`).
  - The hub's twin is `circuit::dispatch::pane_state_refusal` (`dispatch.rs:77-103`).
  - Both say "keep the two equal" (`dispatch.rs:86`, `say_cmd.rs:297-300`).
- **What justifies two copies.** ADR-0021 line 708 says "the two copies exist only because `holler-pane` is frozen and the hub cannot depend on the CLI". That reason does not cover a second copy inside the CLI crate.
- **#839 may change the rule.** It edits `say_cmd.rs` and `dispatch.rs`. An unpinned roster copy would drift with no test failing.

**Binding.**
- **One named predicate.** The roster's mismatch test is one named predicate in `roster_cmd.rs`, over the record's pair or `PaneRow`'s `shown` and `driven`, never inline in a renderer. Its doc names both twins.
- **A pinning test.** T pins the predicate to the CLI twin through the public `say_cmd::resolve_pane_target` (`say_cmd.rs:301`).
  - **The records:** not parked, not unhealthy, and with a session of record.
  - **The pairs:**
    - both set and equal;
    - both set and different;
    - one side `None`;
    - both `None`.
  - **The assertion:** the roster names the pane exactly when `resolve_pane_target` refuses with `pane-shown-driven-mismatch`.
- **No overlap with #839.** `say_cmd.rs` stays out of the change, so there is still no code overlap with #839.

**The alternative (Blast radius exception approved; F's choice).**
- **What it is.** One `pub fn` in `crate::pane::list`, the read verbs' shared code (`list.rs:4-10`). The roster calls it, and so does the third arm of `say_cmd::pane_state_refusal`, in place of its inline check (that hunk only).
- **What it changes.** The pinning test is then unnecessary, but the merge with #839 may need a hunk in `say_cmd.rs` resolved by hand.

`reconcile::shown_differs` stays unused here, as the brief decides (line 143).

### W-3: ADR 0003 and the global flags' help (binding; exception approved)

**Why.**
- **What changes.** After this story, `roster --format=json` prints the envelope and `roster --json` does not.
- **ADR 0003 then says otherwise twice:**
  - line 91: "`--format=json` is `--json` on every verb";
  - line 99: "Under `holler pane` and `holler profile` that object is the envelope ... Every other verb keeps its own `--json` shape".
- **So does the help of the two global flags,** which every subcommand prints:
  - `cli.rs:63`: "The same as `--format=json`."
  - `cli.rs:71-75`: "Under `holler pane` and `holler profile` JSON mode prints one envelope; the other verbs keep their own `--json` shape".
- **The project rule.** A change that contradicts an ADR updates the ADR in the same change. R10's "nothing else in that file changes" was decided about line 94 alone.

**Binding.**
- **The edits.** F edits ADR 0003 lines 91 and 99 to say that under `roster`, `--format=json` prints the envelope and `--json` keeps the roster's own shape (#648). The doc comments at `cli.rs:63` and `cli.rs:71-75` change to match.
- **Why they are factual.** They are edits of R10's kind. The decision itself belongs to #670 (`json_explicit`) and to ADR-0021 decision 6 ("Every pane and profile verb and the roster take `--format`").
- **Tests stay green.** `pane_verbs/process/docs_rows.rs`'s ADR 0003 checks still pass: `--format` is still named, and the global-flags line is untouched.
- **The guide needs nothing.** Its section 9 is scoped to `pane` and `profile`.

### W-4: the three output modes and the error boundary (binding)

**What the brief leaves undecided:**

**(a) The code of the failure envelope for a failed hub call under `--format=json` (AC 11).**
- `emit_error` takes the exit code from the class of the code (`output.rs:336-340`).
- Today `RemotePolicyRefused` exits 3 (`roster_cmd.rs:53-56`, #508's AC 13). No closed code gives that exit: `usage` gives 2.
- With `unavailable` for every hub error, a non-loopback `--server` would exit 1 under `--format=json` and 3 in text. That breaks AC 4 (the same exit in both formats) and ADR-0021 decision 6 (one envelope in JSON mode).

**(b) What a bare `--json` prints for an error** (no hub, or `profile-not-found`). The brief says only that it prints no envelope.

**(c) Whether a usage error found after parsing is an envelope under `--format=json`.**
- R3's check is a stderr line in every format.
- A malformed `--profile` name answers `usage`, through `pane::list::profile_name` (`list.rs:220-222`), which the roster should reuse.
- Through `emit_error` in JSON mode, that `usage` becomes an envelope. That breaks AC 4: "a usage error under `roster` writes no envelope in any format".

**Binding:**
- **Three modes from `FormatChoice`:**
  - text;
  - legacy: `format` is Json and `json_explicit` is false;
  - envelope: `json_explicit` is true.

  On success they print, in turn, the table, today's document with the six keys, and `emit` with `{rows, mismatches}`.
- **Every exit-2 `usage` of the roster** is `error: <message>` on stderr with nothing on stdout, in all three modes. That covers clap's errors and the ones found after parsing (R3's check, a malformed `--profile` name). `format_or_exit` already does this for `roster` (`main.rs:177-180`).
- **Every other error goes through `output::emit_error`:**
  - as `Format::Json` in the envelope mode;
  - as `Format::Text` in the text and legacy modes.

  So a bare `--json` keeps today's `error: <message>` line. `print_leaf_result` and `emit_text` write the same bytes.
- **A failed hub call keeps today's messages** (`roster_cmd.rs:50-57`).
  - **Code:** `unavailable` (exit 1), for `NoLiveHub` and every other `ControlError` except `RemotePolicyRefused`.
  - **`RemotePolicyRefused`:** gets a roster-owned open refusal code (exit 3). It is declared in `roster_cmd.rs` with `RefusalCode::from_static`, as `say_cmd.rs:218-223` declares its three, and is added to the open codes in ADR-0021 section 9's class table (see W-8).
  - **Test matrix:** T adds this case to AC 11, with exit 3 in every mode:
    - text: today's line;
    - `--format=json`: the failure envelope;
    - bare `--json`: today's line.
  - **If O prefers no new code,** the other consistent form is R3's: a stderr line and nothing on stdout, exit 3 in every format, recorded in the brief.
- **R3's check is the first thing `run` does,** before the hub call. It does not go in `format_or_exit` or `output::resolve_format`, which resolve the format of every verb.

### W-5: one source for the six cells (binding)

**Why.**
- **Where the cells are.** `PaneRow::cells()` returns nine cells in `COLUMNS` order (`list.rs:34-36`, `:268-280`). The roster's six are at positions 0, 1, 2, 4, 5 and 6.
- **What breaks.** Hard-coded positions, or the six words typed a second time, would put PROJECT under HEALTH the day `pane list` adds or moves a column, and nothing would fail.
- **What that costs.** It defeats R5's promise that a change "reaches the roster with no roster edit". #831 (a second harness) and #835 (import) both touch `pane list`'s neighbourhood.

**Binding.**
- **The text cells.** `PANE_COLUMNS` holds names that are in `pane::list::COLUMNS`. The roster takes each cell from `PaneRow::cells()` at that name's position in `COLUMNS`, either looked up or from a const table of positions.
- **The pinning test.** A unit test asserts that every `PANE_COLUMNS` entry is in `COLUMNS` at the position the roster uses.
- **The JSON values.** They come from the `PaneRow` fields (`name` as `pane`, then `pos`, `profile`, `health`, `shown`, `driven`), never from the `Pane` a second time.

### W-6: in-process tests and the state dir (binding on T)

**Why.**
- **Where the hub call goes.** `run` calls `transport::call(None, ..)`, which reaches `holler_hub::control::run`. That function reads `HOLLER_STATE_DIR` from the test process on every call, and uses the default state dir when the variable is unset. On the operator's machine, that is the live hub.
- **Process tests are safe; in-process tests are not.** Process tests set the variable on the child (`support/mod.rs:127`). In-process tests have to set it themselves.

**Binding.**
- **One in-process hub call.** Only AC 11 round 6 makes the hub call in-process. Any in-process test that can reach `run`'s hub call, local or with `--server`, runs with `HOLLER_STATE_DIR` set to an empty `support::StateDir`, under a process-wide lock.
- **The pattern to reuse.** `pane_integration/main.rs:51-67` (W-10 of #649). The workspace is edition 2021, so `set_var` adds no `unsafe`.
- **Everything else.** Every other in-process test calls the pure function, which makes no hub call.
- **No wall-clock cells in goldens.** LAST TURN and HOLD print an age from the system clock (`holler-hub/src/roster/rfc3339.rs:30-33`). Golden rows therefore have `last_turn: null` and no hold, or a timestamp far in the future, whose age saturates to `0s ago`.

### W-7: the text layout and the legend (binding)

**Why.**
- **Today's table.** HOLD is the last column and is not padded (`roster_cmd.rs:72`, `:82`).
- **What the brief leaves open.** It does not say how the six columns follow HOLD, or where scope item 2's "legend" goes.
- **What AC 1 allows.** AC 1 leaves stdout no room for a legend line: "the only differences are the new trailing columns".
- **Why it must be fixed now.** T's goldens will fix whatever T picks, so F needs the same rule.

**Binding.**
- **The layout.** Extend `render_table`'s own cadence:
  - HOLD padded to a fixed width, then one space;
  - the six columns at fixed widths;
  - DRIVEN last and unpadded, so no line ends in a space.

  Do not use `pane list`'s computed widths and two-space gap (`table_text`, private, `list.rs:189-216`). That would mix two table styles in one line.
- **The legend is help text** (`holler roster --help`), never a line of output.
  - It goes in the doc of `Command::Roster` (`cli.rs:107`), or in an `after_long_help` const kept in `roster_cmd.rs` next to `PANE_COLUMNS`.
  - `cli.rs` is 780 lines, so the const keeps a long legend out of it.

### W-8: ADR-0021 sentences the change extends (binding on F; exception approved)

**Why.** Three ADR-0021 passages are extended by this change and not covered by R9's two hunks:
- **Lines 268-269** leave "which verbs and prompts read a parked pane" to the roster story (#648), and the brief does not answer.
  - As planned, the roster lists a parked pane like any other and shows no pane hold.
  - Its HOLD column is the hub's session hold, and a park sets no hold entry (lines 713-714). So a parked pane's row shows HOLD `-` while the hub's gate refuses prompts to its session.
- **Lines 321-322** list the wiring's users ("every pane and profile verb ... as do `say`, `interrupt` and `answer` with `--pane`"). The roster becomes one.
- **Section 9's class table** lists the open codes so far, and W-4 adds one.

**Binding.** F edits these with R9's ADR-0021 hunks:
- **Lines 268-269 (one sentence):** the roster lists a parked pane like any other and shows no pane hold; `holler pane list` shows it.
- **Lines 321-322:** add the roster, which runs over the wiring after its hub call and never with `--server`.
- **Section 9's class table:** add the roster's open code to the list.

**Not binding: two code docs become untrue.**
- `shown_differs` names the roster among its readers (`holler-pane/src/reconcile.rs:181-183`).
- `PaneRow.driven` says "printed, never compared" (`list.rs:239-241`).

Both are outside the blast radius, so O files a follow-up for them. That follow-up should also ask whether the roster should flag a parked pane, which is a product question.

### W-9 to W-12 (not binding)

**W-9 (for O).**
- **The prefix.** The CLI's prefix for a non-fatal diagnostic beside a successful result is `warning:`: `main.rs:96` for `hold`, and `list.rs:102` ("warning: unmanaged panes not listed"). `note:` would be new to the binary.
- **The JSON signal.** `pane list` is the analogue the roster reuses. It reports the same situation (a second source could not be read, and the main listing still printed) in its JSON data, as `unmanaged_unavailable` (`list.rs:118-121`). The roster's `--format=json` reports it only on stderr. So a script cannot tell "registry unavailable" from "no pane joined" without reading stderr.
- **What O can decide** at the next brief gate: use `warning:`, and add an optional `data` key that is left out when the registry answered.
- **For F, nothing changes.** The brief pins `note:` and exactly two keys, both reviewed decisions, so F follows the brief as written.

**W-10.**
- **The join.** AC 10 joins a row to a pane by label == pane name. That is a second relation between a hub row and a pane. The hub's gate and `say --pane` use another: session of record == the bare session name (ADR-0021 lines 705-709). The brief decided this (AC 10, rounds 4 to 6), and #839 decides the address later (R7).
  - **Recommended:** compute the join key in one named function whose doc cites AC 10 and #839, so #839 has one place to change.
  - **How:** it can use `holler_proto::RoutableName`'s `has_label()` and `label()`, since the row's `name` is a `RoutableName`'s string form (`holler-hub/src/roster.rs:133`). An equivalent split, documented as such, also works.
- **The six keys.** They are written into the hub's row objects, whose key namespace the hub owns. None collides today (`roster.rs:137-180`).
  - **Recommended:** add a test that none of the six keys is a key of the hub's row. Then a hub field of the same name, added later, fails a test instead of being overwritten.

**W-11.** Blast radius line 133 still lists ADR 0003 line 94 as "Not changed, checked". R10 (line 36) and line 118 supersede it, and F follows R10.

**W-12, for T and S (spec observations outside A's dimensions):**
- **The logging banner.** Process tests always see the `logging_started` banner on stderr (`holler-proto/src/log.rs:321-339`; `pane_verbs/process/main.rs:19-21`). So AC 11's "today's one stderr line" and AC 13(b)'s "exactly the one line" mean "besides the banner".
- **AC 7's comparison with `pane list`.** An unscoped `pane list` reads Herdr (#832). Run over the same rig, it fills `FakeHerdr`'s call log. Either assert the roster's empty logs first, or compare with `PaneRow::from(&pane).cells()`, which is what `pane list` prints.
- **An empty `--profile` result.** A `--profile P` that joins no row prints `(no sessions on the hub)`, under today's rule for an empty table (`roster_cmd.rs:67-69`), and that is not true of the hub. The brief does not say what to print instead.
- **Whether the columns fill in production.** The PANE column fills only where a body's token label equals a pane's name. Nothing in the code mints a pane's body token under the pane's name, and the setup wizard mints one label per body pairing (`agent-skills/herdr-setup/SKILL.md:1340-1341`). With DRIVEN `-` until #839 (R6), the new columns may stay empty in production for a while. S should confirm that this matches the issue's intent.

## Notes for O

This is a PASS, so nothing is required for the run to continue. Open items:
- **W-4.** If O wants R3's form (no new code) rather than the binding default, record that in the brief.
- **W-8.** File the follow-up for the two code docs, and the product question of whether the roster should flag a parked pane.
- **W-9.** Decide at the next brief gate between `warning:` and `note:`, and whether `data` gets a key for a partial read. Otherwise the brief stands as decided.
- **R3's check and `conflicts_with`.** The codebase's usual tool for two flags that exclude each other is clap's `conflicts_with` (`cli.rs:611`, `pane/args.rs:77` and `:83`, `profile/create.rs:56`). It would print clap's own message instead of AC 13(a)'s pinned line, so it belongs to a later amendment, not this run.
- **The brief gate.** The brief's Pipeline section asks for a new brief gate after the refresh. This architecture pass is not that gate.

## Patterns referenced

- **The analogue.**
  - `crates/holler-cli/src/pane/list.rs`: `PaneRow`, `cells` and `COLUMNS`; `rows_in_scope` (130-155); `profile_name` (220-222); `PaneTable` and its partial-read field (111-125); the `warning:` line (102).
  - `crates/holler-cli/tests/pane_verbs/list.rs`: the rig, `ports()` and `assert_nothing_observed` (39-95).
- **A legacy verb over the wiring.** `crates/holler-cli/src/say_cmd.rs`: `route_target` (236-260), `resolve_pane_target` and `pane_state_refusal` (301-372), the verb-owned open codes (218-223).
- **The wiring.** `crates/holler-cli/src/pane/wiring.rs` (1-27, 101-128); `crates/holler-cli/src/pane/wiring/store.rs` (106-123, 337-390); ADR-0021 ruling 1 and "Wiring as built" (lines 316-322).
- **Output and dispatch.** `crates/holler-cli/src/output.rs` (49-82, 201-243, 336-340); `crates/holler-cli/src/main.rs` (160-184, 214-285).
- **The rule's twin and the ADRs.** `crates/holler-hub/src/circuit/dispatch.rs` (77-103); `docs/adr/ADR-0003.md` (lines 91, 94, 99, 105-106); `docs/adr/ADR-0021.md` (lines 256, 268-269, 705-718, 766-826, 996, 1039).
- **The in-process state-dir lock.** `crates/holler-cli/tests/pane_integration/main.rs` (51-67).
