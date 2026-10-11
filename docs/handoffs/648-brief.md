# [feat(cli): the roster shows pane, SHOWN and DRIVEN, and a loud line when they differ](https://github.com/Performant-Labs/holler/issues/648)

Part of epic #633. This is the complete statement of the story (consolidated 2026-10-11 from earlier amendments; where an older note disagreed, this text is the decision).

## Scope

Extend `holler roster` with the pane's place in the registry, so a mismatch between what a pane **shows** and what the hub **drives** is loud.

1. **New columns**, appended after the existing ones, in this order: `PANE` (the pane's name, `-` for an agent with no pane: the io body, the alpha and beta agents), `POS` (the pane's grid position), `PROFILE` (the pane's profile, `-` for none), `HEALTH` (the pane's harness health: `healthy`, `unhealthy`, `unknown`, `-` when no pane), `SHOWN` (the session the pane's TUI shows, `-` when unknown or no pane) and `DRIVEN` (the session the hub drives, `-` when no pane).
   - *Noted at the brief gate (round 4; facts only, E18 quotes them; the decision "Where every new column comes from" below is unchanged):* a pane's name is a `SessionName`, which never contains a `/` (E10, E18). The hub names every row through `qualified_name`: `<label>/<session>` once the advertising token's label is bound, and the hub binds it when the body authenticates, before its first `session/presence`; a label minted by the hub is 1 to 32 characters. A row's name is the bare session name only when no label is bound, which the hub's own doc gives as a unit test or a presence that outraces the bind. So a row's `name` equals a pane's name only for a row with no label. No code today matches a row's `name` with a pane's name. What relates a pane record to a hub session today is its `session_of_record`, which the hub's prompt gate matches with the bare session name being prompted and to which `say --pane` sends.
2. **`POS` is rowcol**: text `r2c1`; JSON `{"row":2,"col":1,"pos":"r2c1"}` with `pos` equal to `r{row}c{col}`. A pane named `hj-c1r2` is a name, not a position: it sits at `r2c1` and is not renamed. The roster's help and legend say so.
   - *Clarified at the brief gate:* POS is read from the pane's registry record ("the pane's place in the registry" above), never from its name. The record holds where the pane sits in Herdr as `herdr.grid`, a `GridPos` (`crates/holler-pane/src/pane.rs:91-100`, `:232`), the field `pane list` (#643) prints (`crates/holler-cli/src/pane/list.rs:174`, `crates/holler-cli/src/pane/list.rs:193`). `GridPos` prints as `r{row}c{col}` and serializes as `{"row":R,"col":C,"pos":"rRcC"}`, row first (`crates/holler-pane/src/grid.rs:68-73`, `crates/holler-pane/src/grid.rs:129-138`). ADR-0021 section 10 (lines 614-615): "A pane's position comes from Herdr's snapshot, never from its name: `hj-c1r2` sits at `r2c1`." So the fixture's pane is named `hj-c1r2` and its record's `herdr.grid` is row 2, column 1. E10 quotes them.
   - *Clarified at the brief gate (round 3):* today's help does not say this, so the story adds it. The roster's help is the doc comment of `Command::Roster`, `/// List the roster. (hub-only, top-level daily verb)` (`crates/holler-cli/src/cli.rs:107`), and the doc comments of the `Roster` struct's fields (`cli.rs:464-484`, quoted in full in E4); none of them says that a pane's name is not its position. The roster prints no legend today: `render_table` writes a header line and the rows and nothing else (E1), and a search of `cli.rs` and `roster_cmd.rs` at `92fbc6e` finds no `legend`, `long_about` or `after_help` (the one `about` in `cli.rs`, at line 26, is the root command's). So the legend is new text as well.
3. **The mismatch summary line** (text output only): when at least one pane's `SHOWN` differs from its `DRIVEN` (both known and not equal), the first line of the output is `N pane(s) show a different session than the hub drives: <names>`, names in roster order, separated by `, `. When every pane matches, or no pane has both values, no such line is printed.
   - *Clarified at the brief gate:* "both known and not equal" means that a pane whose SHOWN is `-` is never counted or named, whatever its DRIVEN, and neither is an agent with no pane. That is the rule the hub's pane-state prompt gate and `say --pane` already apply to a pane's recorded pair: "one side absent is not a mismatch" (`crates/holler-cli/src/say_cmd.rs:341-342`, `:358-364`; the hub's twin, `crates/holler-hub/src/circuit/dispatch.rs:80-81`, `:95-101`). Acceptance 2 tests it; E11 quotes the rule.
   - *Clarified at the brief gate (round 3):* the line is the byte format of the decision "Summary line, byte format" below for every N, one included: only N and the names vary, and `pane(s)` and `show` are written literally, so one mismatched pane gives `1 pane(s) show a different session than the hub drives: <name>`. Acceptance 2 pins N = 1 as well as N = 2.
   - *Clarified at the brief gate (round 3):* "roster order" is the order in which the roster prints its rows, a name taking the place of the row it is printed on. The roster prints the rows of the hub's `control/roster` reply in the reply's order (`render_table`'s `for row in &rows`, `crates/holler-cli/src/roster_cmd.rs:75`, in E1), and the hub sorts them by row name (`crates/holler-hub/src/roster.rs:553`, E16). Acceptance 1 holds each of those lines byte-identical to today's up to its `HOLD` cell, so the pane store's name order (E8) is not roster order and does not reorder the rows.
4. **`--profile P`** lists only P's panes, through the `ProfileScope` trait (#637). A profile that does not exist is a **refusal** (`profile-not-found`, exit 3, one line, nothing else printed).
   - *Clarified at the brief gate:* with no pane named, `ProfileScope::resolve(P, None)` answers P and every pane of P, and a P that does not exist is `profile-not-found` (`crates/holler-pane/src/profile.rs:384-393`). The CLI's implementation, `StoreScope`, takes every pane whose record names P, in name order (`crates/holler-cli/src/pane/profile_scope.rs:84-90`, `:159-174`); the tests use the test kit's `FakeProfileScope` (`crates/holler-pane-testkit/src/profile_scope.rs:1-4`, `:190-200`). A verb holds the scope as `Ports.scope` (`crates/holler-pane/src/ports.rs:278`), and `pane list --profile` calls it that way (`crates/holler-cli/src/pane/list.rs:98-99`). `profile-not-found` is a refusal with exit 3, the same in text and JSON (`class_of`, `crates/holler-pane/src/error.rs:289-291`; `ErrorClass::exit_code`, `:235-246`). E6 and E8 quote them.
   - *Clarified at the brief gate (round 4), the order under `--profile`:* `--profile P` changes which rows are printed, never their order. The rows it prints come in the order of the hub's `control/roster` reply, as every roster's rows do (scope item 3, round 3), and not in the name order in which `ProfileScope::resolve` returns P's panes (`StoreScope`'s `members`, E8). The names of the summary line and of `data.mismatches` (scope item 5) follow the same order.
5. **`--format=text|json`** (default text). `--format=json` prints the shared envelope (`output::emit()`, #670) with the same new fields in `data`; in JSON the mismatch is `data.mismatches` (an array of pane names) and there is no summary line.
   - *Clarified at the brief gate:* `--format=json` is told from a bare `--json` by `FormatChoice.json_explicit`, which only `--format=json` sets (`crates/holler-cli/src/output.rs:49-82`); `main.rs` already reserves the bit for this story (`crates/holler-cli/src/main.rs:216-219`), and today `dispatch` hands `roster` only the resolved `json` (`:221`, `:229`). `output::emit` writes one envelope (`schema_version` 1, `ok`, `data`, `error`) in JSON mode and the text otherwise, and in both returns 0 or the exit code of the error's class (`output.rs:150-171`, `:201-217`, `:336-340`; `SCHEMA_VERSION`, `:33-34`). A usage error that clap reports under `roster` is clap's own text on stderr with exit 2 in every format, since only `pane` and `profile` get usage envelopes (`output.rs:36-38`; `main.rs:115-120`), and `--json --format=text` is `error: ...` with exit 2 for `roster` (`main.rs:160-184`). `check_envelope(stdout, exit_code)` is in the test kit, which `holler-cli` already has as a dev-dependency. E5, E6 and E7 quote them.
   - *Clarified at the brief gate (round 3; E14 quotes the lines not quoted elsewhere):* with this story `roster` prints the envelope, and only for an explicit `--format=json`, the case #670 reserved for it: "`roster` prints the envelope only for this, so `roster --json` keeps its legacy shape" (`FormatChoice.json_explicit`, `output.rs:54-57`, in E5). Under `--format=json` the roster's success (`ok` true, `data` as the decision "`--format=json` `data` shape" below says) and its `profile-not-found` refusal (`ok` false, `error.code` `profile-not-found`, exit 3) are each one envelope on stdout, written by `output::emit` (`output.rs:207-217`, in E6) and `output::emit_error` (`:240-243`). Both are `pub` and take a `Sink`; `emit` takes any `Serialize` data and `emit_error` an `ErrorBody`, and neither names a verb, so the roster calls them as the `pane` and `profile` verbs do, with no change to `output.rs` (not in the blast radius). In text mode the roster prints its table, and a bare `--json` prints no envelope (scope item 6). `ENVELOPE_NAMESPACES` stays `["pane", "profile"]` (`output.rs:36-38`): its one reader is `RawArgs::envelope_namespace` (`:362-369`), which only `usage_envelope` calls, for a command line clap could not parse (`main.rs:140-158`), so it decides usage errors and nothing else. A usage error under `roster` is therefore what it is today in every format, `--format=json` included: clap's own message on stderr with exit 2 (`--format=xml` is one, since `--format` is a `value_enum`, `cli.rs:80-81` in E5), or `error: --json cannot be combined with --format=text` on stderr with exit 2 (`main.rs:160-184`, in E6), and nothing on stdout. Acceptance 4 says what its test checks for each class.
   - *Clarified at the brief gate (round 4), the one shape of `data` (the decision "`--format=json` `data` shape" below is unchanged; this spells out what it gives):* on success, `data` is one object with exactly two keys, `rows` and `mismatches`: `{"rows": [ROW, ...], "mismatches": [NAME, ...]}`. `data.rows` is the array that `roster --json` prints as its own top-level `rows` for the same hub reply, pane registry and flags: the same rows in the same order, each with today's keys (E2) and the six new keys of scope item 6. `data.mismatches` is an array of pane names: the names the text summary line gives for the same input (scope item 3's rule), in roster order. The decision makes `mismatches` a key of the object, so it is in every success envelope, and it is `[]` when no pane is a mismatch, which is when the text output has no summary line. `data` has no other key: `rows` is not moved out of `data` or nested deeper, and `roster --json` stays `{"rows": [...]}`, with no `mismatches` (scope item 6). So with two mismatched panes `a` and `b`, in that roster order, the text output's first line is `2 pane(s) show a different session than the hub drives: a, b`, and the `data` of `--format=json` is `{"rows": [...], "mismatches": ["a", "b"]}`. This object is "the roster payload" of acceptance 4.
6. **`roster --json` keeps its shape and gains the new fields additively** (`pane`, `pos`, `profile`, `health`, `shown`, `driven` as extra keys). ADR 0003 lines 105-106: a changed `--json` shape is breaking, but "Additive = MINOR", so there is **no ADR 0003 amendment and no deprecation notice**. Every existing key and its value is unchanged. (Operator decision, 2026-10-11.)
   - *Evidence added at the brief gate (the operator decision above is unchanged):* ADR 0003 lines 105-106 are quoted verbatim in E9. Today `roster --json` prints the hub's `control/roster` reply as it arrives, `{"rows": [ROW, ...]}` (`crates/holler-cli/src/roster_cmd.rs:44-45`; the hub builds it at `crates/holler-hub/src/control_server.rs:465-474`), each ROW being the serde form of the hub's `Row` with its `hold` flattened (`crates/holler-hub/src/roster.rs:131-180`). Those are the keys acceptance 3 keeps. E1 and E2 quote them.
7. Tests use only #638's fakes and envelope helper; no new dependency. `SHOWN` and `DRIVEN` are the pane record's `last_observed.shown` and `last_observed.driven`, the same values `pane list` prints (see "Decisions on the review's blocking points"); the roster makes no adapter call.
   - *Clarified at the brief gate (facts only; E2 and E3 quote them):* today a roster call reads one thing, the hub's reply to `control/roster` (`crates/holler-cli/src/roster_cmd.rs:39-49`), which the hub builds on each call from its in-process roster, filling `hold` from hub state at read time (`crates/holler-hub/src/control_server.rs:461-475`); the hub's `Row` has no pane field (`crates/holler-hub/src/roster.rs:131-180`). A pane's sessions are in three places. (a) The port `HarnessPort::shown_session`, "The session the TUI of `pane` shows, if it can tell" (`crates/holler-pane/src/ports.rs:205-206`), which the OpenCode adapter answers with `None` whenever it cannot tell (ADR-0021 lines 126-127); only the CLI process can make that call, because the hub runs no adapter (ADR-0021 line 648; `crates/holler-hub/src/panes/mod.rs:8-10`). (b) The record's `last_observed { shown, driven, at }`, "What reconcile last observed" (`crates/holler-pane/src/pane.rs:180-191`): `pane list` prints these two as SHOWN and DRIVEN (`crates/holler-cli/src/pane/list.rs:12-16`, `:178-179`), and nothing writes `driven` before #649 (ADR-0021 lines 641-644). (c) The record's `session_of_record`, which I2 makes "the only session a pane's TUI shows and the hub drives" (ADR-0021 line 204; `pane.rs:235-237`); how the hub's own DRIVEN session is pointed at it is deferred to #649 and #654 (ADR-0021 lines 639-640). Of the three, only (a) is an observation made at the moment of the call. In the CLI, `Wiring::connect` hands out `Unwired`, whose every port answers `not-implemented`, until #649 (`crates/holler-cli/src/pane/wiring.rs:8-13`, `:29-33`). The issue does not say which of these is "the hub's current drive state" for DRIVEN, nor what the roster prints when a port call for a pane fails rather than answering `None`.

## Acceptance (each is a test; "golden" means a committed expected-output file)

*Cross-reference added at the brief gate (the numbering is the issue's):* scope item 1 is tested by acceptance 1 and 6, item 2 by 5, item 3 by 2 and 7, item 4 by 6, item 5 by 4, item 6 by 3, and item 7 by 7.

1. **Old columns unchanged.** A golden test over a fixture roster (including an agent with no pane) shows that every pre-existing column of every row is byte-identical to today's output; the only differences are the new trailing columns, which show `-` for an agent with no pane.
   - *Clarified at the brief gate:* the fixture also has a pane with a profile and a pane without one, and one pane for each HEALTH value of scope item 1 (`healthy`, `unhealthy`, `unknown`), so the golden file pins every PROFILE and HEALTH value as well as the `-` cells of the agent with no pane. None of its panes shows a session other than the one driven, so no summary line comes before the table.
   - *Clarified at the brief gate:* today's table has nine columns, `SESSION` to `HOLD` (E1). Each but `HOLD` is left-aligned to a fixed width and followed by one space; `HOLD` is the last and is not padded, so each line ends right after its `HOLD` cell (`crates/holler-cli/src/roster_cmd.rs:71-74`, `:81-92`). The new columns therefore follow the `HOLD` cell, and "byte-identical" covers each line from its start through the end of that cell. An empty roster prints `(no sessions on the hub)` and no header (`:67-69`).
2. **Text mismatch.** A fixture with two panes whose SHOWN differs from DRIVEN prints the summary line first with both names in roster order (golden); a fixture with none prints no such line (golden).
   - *Clarified at the brief gate:* both fixtures also hold a pane whose SHOWN is `-` and whose DRIVEN is known. It is never named, and N counts only the panes with both values known and different (scope item 3), so in the fixture with none the line stays absent although that pane's two cells differ.
   - *Clarified at the brief gate (round 3):* a third golden fixture has exactly one pane whose SHOWN and DRIVEN are both known and differ. Its output starts with the line `1 pane(s) show a different session than the hub drives: <name>`, `<name>` being that pane's name, and then the table (scope item 3: `pane(s)` is written literally for every N).
3. **`--json` additive.** The `--json` output of the fixture parses; every key present in today's output is present with an equal value; the new keys are present.
4. **Envelope.** `--format=json` output parses with #681's `check_envelope`, `ok` is true, `schema_version` is 1, `data` equals the roster payload including the new fields, and exit codes are identical for `--format=text` and `--format=json` for the same input (0 on success, 2 on a usage error, 3 on a refusal).
   - *Clarified at the brief gate (round 3):* the outputs of `--format=json` that are envelopes (scope item 5) are the success and the `profile-not-found` refusal, and the test passes each to `check_envelope` with its exit code (0 and 3; E7 lists the rules it checks). A usage error under `roster` writes no envelope in any format (scope item 5), so for that class the test checks exit 2 and an empty stdout, and does not call `check_envelope`.
   - *Clarified at the brief gate (round 4), what `data` is compared with:* "the roster payload including the new fields" is the `data` object that scope item 5 (round 4) states, `{"rows": [...], "mismatches": [...]}`, with no other key. The test runs `roster --json` and `roster --format=json` over the same fixture and, comparing parsed JSON values, checks three things. `data` has exactly the keys `rows` and `mismatches`. `data.rows` equals the `rows` of the `--json` output, whose keys acceptance 3 checks. `data.mismatches` holds the names of the text summary line for that fixture, in the same order: `[]` for acceptance 1's fixture and for acceptance 2's fixture with no mismatch, the two names for acceptance 2's fixture with two, and the one name for its third fixture.
5. **POS.** In the text table, in the JSON `pos` and `row`/`col`, and in `--format=json`, a position is never written as `c<col>r<row>`; a golden test shows the pane named `hj-c1r2` at `r2c1`. (Pane **names** and the explanatory help text may contain `c1r2`: this criterion is about position values only.)
6. **PROFILE and `--profile`.** PROFILE shows the profile or `-`; `--profile P` lists only P's panes; an unknown profile is refused `profile-not-found` with exit 3.
   - *Clarified at the brief gate (round 2):* the process test that pins today's refusal of `roster --profile` (exit 1 and `error: not implemented (story #648)`, E4) is deleted with the refusal (Blast radius); after this story, this criterion's tests are what pin `--profile`.
7. **Same values as `pane list`; read fresh on each call.** For every pane of the fixture the roster's SHOWN and DRIVEN cells equal the cells `pane list` prints for the same record; when a record's `last_observed` changes between two roster calls, the second call shows the new values (and the summary line appears or disappears accordingly). The roster makes no `HarnessPort`, `HerdrPort` or `HostPort` call (a test that gives it ports whose every method fails the test).
   - *Clarified at the brief gate (round 3):* in that last test, `harness`, `herdr` and `host` are the only ports a call to which fails the test. They are the test kit's `FakeHarness`, `FakeHerdr` and `FakeHost` (scope item 7 allows only #638's fakes), and after the run the test asserts that each one's call log, which records every call made through the port, failed ones included, is empty (`faults().calls()`, E15). Every other port is a working test-kit fake: `pane_store` is a `FakePaneStore` seeded with the fixture's records and `scope` a `FakeProfileScope` over that store and a `FakeProfileStore`, and the same test asserts that the new columns show those records' values, so a roster that skips the store fails it too.
   - *Clarified at the brief gate (round 3), pinning the decision that DRIVEN is `last_observed.driven`:* the fixture has a pane whose `session_of_record` and `last_observed.driven` are both set and differ, and a pane whose `session_of_record` and `last_observed.shown` are set and differ while its `last_observed.driven` is `None`. The first pane's DRIVEN cell is its `last_observed.driven`, never its `session_of_record`. The second pane's DRIVEN cell is `-`, as `pane list` prints a `None` (E17), and the summary line does not name it: the pair rule decides a mismatch, not `shown_differs` (E11).
   - *Clarified at the brief gate (round 3), the tests of the decision "What the roster shows before #649 wires the real store":* two tests run the hub rows of acceptance 1's fixture against a `FakePaneStore` whose `list` fails, set through its fault switch (`Fault::Fail`, or `fail_next` with `PaneStoreOp::List`; E15). They are separate from acceptance 1's golden test, whose store holds the fixture's records and answers. In text mode: (i) when `list` answers `not-implemented`, stdout is the fixture's hub rows with `-` in each of the six new columns and no summary line (a golden file), stderr has no `note:` line (the decision gives the note to a different store error), and the exit code is 0; (ii) when `list` answers `unavailable`, stdout is the same, the exit code is 0, and stderr has exactly one line, `note: pane registry unavailable (unavailable)`, the decision's `note: pane registry unavailable (<code>)` with `<code>` the error's stable code (`PaneCode::Unavailable => "unavailable"`, `crates/holler-pane/src/error.rs:120`). Under `--json` and `--format=json` both tests also exit 0, since neither store error is a roster failure.

## Blast radius

- `crates/holler-cli/src/roster_cmd.rs` (`crates/holler-cli/src/roster.rs` does not exist and is not created; if the pane-side merge does not fit `roster_cmd.rs` under the 900-line lint, put it in one new sibling file `crates/holler-cli/src/roster_panes.rs`)
- `crates/holler-cli/src/main.rs`: pass #670's "explicit `--format=json`" bit to `roster_cmd::run` and remove #670's `roster --profile P` refusal ("not implemented (story #648)") from the extracted roster helper; if that needs more than the roster helper's lines, say so in the brief and keep the edit inside that helper
  - *Clarified at the brief gate (as this bullet asks; E4 quotes each line):* the refusal is `crates/holler-cli/src/main.rs:275-282`, inside `run_roster` (`:272-285`); it prints `error: not implemented (story #648)` on stderr and exits 1. Removing it and passing the bit needs more than the helper's lines. `ROSTER_STORY` (`main.rs:43-44`) has no use but the refusal (`main.rs:277`), so it becomes dead code, which `dead_code = "deny"` (`Cargo.toml:32-33`) makes a build error. The bit is in `choice`, which `dispatch` holds, and `dispatch` hands `run_roster` only `json` (`main.rs:221`, `:229`), so its `Command::Roster` arm changes too. Outside `main.rs`, the existing test `roster_profile_is_refused_with_the_roster_story` (`crates/holler-cli/tests/pane_verbs/process/legacy_verbs.rs:95-105`, its constant at `crates/holler-cli/tests/pane_verbs/process/stub.rs:35-38`) asserts that refusal with no format flag, with `--json` and with `--format=json`, so it no longer passes once the refusal is gone; it is not one of this story's new tests. The comment on `Roster`'s `--profile` field (`crates/holler-cli/src/cli.rs:480-481`) also describes the refusal.
- `crates/holler-hub/src/roster.rs`: **no change** (the hub's `Row` is untouched; every new column comes from the pane registry, see "Decisions on the review's blocking points")
- the new tests under `crates/holler-cli/tests/` and their golden files; `docs/` and `README.md` only where the roster's documented output appears; `CHANGELOG.md`
- *Added at the brief gate (round 2; each file follows from a change listed above, and E4 quotes the lines):*
  - `crates/holler-cli/tests/pane_verbs/process/legacy_verbs.rs` and `crates/holler-cli/tests/pane_verbs/process/stub.rs`: the story deletes the test `roster_profile_is_refused_with_the_roster_story` (`legacy_verbs.rs:95-105`), the import of its constant (`legacy_verbs.rs:13`, `use crate::stub::ROSTER_PROFILE_REFUSAL as ROSTER_REFUSAL;`) and that constant (`stub.rs:35-38`, documented as "The refusal of `roster --profile`, until story #648."). Both pin the refusal this story removes, and the test is the constant's only use. Acceptance 6 tests what `roster --profile` does instead. A search of the tree at `92fbc6e` for `ROSTER_PROFILE_REFUSAL` and `story #648` finds no other test that pins the refusal.
  - `crates/holler-cli/src/cli.rs`: the comment on `Roster`'s `--profile` field (`:480-481`), which says `main.rs` refuses the flag, and the roster's help, which scope item 2 requires to say that a pane's name is not its position. That help is the doc comment of `Command::Roster` (`:107`, `/// List the roster. (hub-only, top-level daily verb)`) and those of the `Roster` struct's fields (`:464-484`, in E4).
  - ADR 0003 line 94 also states the refusal, in words that end it at this story: "Until #648, `roster --profile` is refused with exit 1 and `error: not implemented (story #648)`." It is not added here, since scope item 6 records no ADR 0003 amendment.

## Evidence the brief must quote

The reviewers cannot see the repo. The brief quotes, with file and line: #670's roster helper and the refusal in `crates/holler-cli/src/main.rs`; the format resolver's "explicit json" bit in `crates/holler-cli/src/output.rs`; `ProfileScope` in `crates/holler-pane/src/profile.rs`; `check_envelope` from #681 in `holler-pane-testkit`; the existing `render_table` header in `roster_cmd.rs`; and the hub's roster row type in `crates/holler-hub/src/roster.rs`.

## Decisions on the review's blocking points (final, 2026-10-11, taken under the operator's standing rules; they override any earlier wording above, including the "current drive state" and "live observation" phrases)

- **Where every new column comes from: the pane registry, read through `Ports.pane_store.list()`.** One source of truth, the one `pane list` already uses (E-list in the evidence). `PANE` = the record's name, matched to an agent row by the agent's name equalling the pane's name; `POS` = the record's `herdr.grid`, printed rowcol; `PROFILE` = the record's `profile`; `HEALTH` = the record's recorded `harness.health`; `SHOWN` = `last_observed.shown`; `DRIVEN` = `last_observed.driven`. **The roster makes no live adapter call**: a roster is cheap and bounded (I5), and "observe, don't infer" (I6) is reconcile's job, which writes `last_observed`. The hub's `Row` and `holler-hub` are not touched.
- **What the roster shows before #649 wires the real store:** in the real binary `Wiring::connect` hands out `Unwired`, whose `PaneStore::list` answers `not-implemented`. The roster then still prints the hub's rows exactly as today, with `-` in every new column, no summary line, and exit 0 (a `not-implemented` from the pane store is not a roster failure). The new columns light up when #649 wires the store. A second, different store error (`timeout`, `unavailable`) is also not a roster failure: it prints `-` columns and one line on stderr `note: pane registry unavailable (<code>)`, exit 0. Tests cover `not-implemented` and `unavailable`.
- **The mismatch rule is the existing pair rule:** a pane is a mismatch only when both `last_observed.shown` and `last_observed.driven` are `Some` and unequal; one side absent is never a mismatch (`say_cmd.rs` and the hub's `dispatch.rs` apply the same rule to a recorded pair, E11). `reconcile::shown_differs` answers a different question (shown against `session_of_record`) and is not used here.
- **Summary line, byte format:** `N pane(s) show a different session than the hub drives: a, b` (colon, one space, names in roster order separated by `, `), followed by a newline, then the table. Pinned in the golden files.
- **`--format=json` `data` shape:** the same object `roster --json` prints (`{"rows": [...]}`, each row with the new keys added) plus one more key, `"mismatches": [pane names]`, next to `rows`. `roster --json` itself does not get `mismatches`.
- **Exit codes:** `roster` has exit 0 (success, in both formats), 2 (a usage error such as `--format=xml`) and 3 (the `profile-not-found` refusal); it has no exit-1 path of its own. Acceptance 4 is tested once per class in both formats (a success fixture, `--format=xml`, an unknown `--profile`).
- **The `--profile` refusal output** goes through the same shared helper every refusing verb uses (`output.rs`): text mode prints the one-line `error:` form on stderr, `--format=json` prints the failure envelope; nothing else is printed.
- **`roster_cmd::run`'s new signature** is chosen by the story and stated in the brief: it receives the resolved format choice (with #670's `json_explicit` bit) and a `Ports`; `main.rs`'s `run_roster` builds the `Ports` the way `run_with_stdio` does for the pane verbs.

## Evidence (brief gate, rounds 1 to 4)

Added at the brief gate, with every line above whose italic label says "at the brief gate", to answer the outside review; the rest of this brief is the issue's text. Round 2 added E12, E13 and the lines labelled "round 2"; round 3 added E14 to E17 and the lines labelled "round 3"; round 4 added E18 and the lines labelled "round 4". Every excerpt is verbatim from this worktree at its base, `92fbc6e` (`origin/main`); once an item names a file, a bare `:N` is a line of that file. (The worktree's base is now `3c139dd`, which differs from `92fbc6e` only in files no excerpt comes from; rounds 3 and 4 read their excerpts there.) The "E-list in the evidence" that the first decision above cites is this list: `pane list` reads the registry through `ports.pane_store.list()` in E8 (`rows_in_scope`) and turns a record into its cells in E3 (`PaneRow::from`) and E17. The six quotes the section above asks for are E4 (#670's roster helper and its refusal), E5 (the format resolver's "explicit json" bit), E8 (`ProfileScope`), E7 (`check_envelope`), E1 (the `render_table` header) and E2 (the hub's roster row type).

- **E1, the roster today** (`crates/holler-cli/src/roster_cmd.rs`). `run` reads the hub's roster once per call and sets the exit code (`:39-59`):

  ```rust
  pub fn run(roster: &Roster, json: bool) -> RosterResult {
      let state_root = holler_hub::state::resolve_state_dir().unwrap_or_default();
      let call = holler_hub::control::ControlCall::roster(roster.all, roster.prefix.as_deref());
      match crate::transport::call(roster.server.as_deref(), &call) {
          Ok(doc) => {
              if json {
                  ok(doc.to_string())
              } else {
                  ok(render_table(&doc))
              }
          }
          Err(holler_hub::control::ControlError::NoLiveHub) => {
              err(format!("no live holler hub reachable at {}", state_root.display()), 1)
          }
          // Issue #508 AC 13: `--server ws://…` to a non-loopback host, exit 3
          // (the same fail-closed policy `body join` applies) — checked before
          // any dial, never reachable when `roster.server` is `None`.
          Err(holler_hub::control::ControlError::RemotePolicyRefused(msg)) => err(msg, 3),
          Err(e) => err(e.to_string(), 1),
      }
  }
  ```

  `render_table`, the header and the row format (`:61-95`):

  ```rust
  /// Render the roster reply (`{rows: [...]}`) as the spec's table. Columns are
  /// the row's display fields; a held permission is the `PENDING` column (the
  /// human view shows just its count — `--json` carries the full array). An
  /// empty roster prints a friendly one-liner instead of a bare header.
  fn render_table(doc: &serde_json::Value) -> String {
      let rows = doc.get("rows").and_then(|r| r.as_array()).cloned().unwrap_or_default();
      if rows.is_empty() {
          return "(no sessions on the hub)\n".to_string();
      }
      let mut out = String::new();
      out.push_str(&format!(
          "{:<20} {:<10} {:<8} {:<10} {:<10} {:<8} {:<8} {:<20} HOLD\n",
          "SESSION", "HARNESS", "MODE", "STATE", "CONN", "HOSTNAME", "PENDING", "LAST TURN"
      ));
      for row in &rows {
          let pending = row
              .get("pending")
              .and_then(|p| p.as_array())
              .map(|items| items.len().to_string())
              .unwrap_or_else(|| "-".into());
          out.push_str(&format!(
              "{:<20} {:<10} {:<8} {:<10} {:<10} {:<8} {:<8} {:<20} {}\n",
              row.get("name").and_then(|v| v.as_str()).unwrap_or("-"),
              row.get("harness").and_then(|v| v.as_str()).unwrap_or("-"),
              row.get("mode").and_then(|v| v.as_str()).unwrap_or("-"),
              row.get("state").and_then(|v| v.as_str()).unwrap_or("-"),
              row.get("conn_state").and_then(|v| v.as_str()).unwrap_or("-"),
              row.get("hostname").and_then(|v| v.as_str()).unwrap_or("-"),
              pending,
              last_turn_display(row.get("last_turn")),
              hold_display(row)
          ));
      }
      out
  }
  ```

  `main.rs` prints a success as it is, adding no newline: `print_leaf_result(&result.message, result.to_stderr, result.exit_code, false)` (`crates/holler-cli/src/main.rs:284`; the printing, `:80-89`, is in E4).
- **E2, the hub's roster row and the reply it builds.** The row type (`crates/holler-hub/src/roster.rs:131-180`):

  ```rust
  /// One roster row: one routable session name a body advertises.
  ///
  /// `name` keys the row (a `RoutableName`'s string form, `label/session`). The
  /// transport's liveness (`conn_state`) is separate from the A2A session state
  /// (`state`) — a body can be `connected` but `idle`, or `gone` while still
  /// `working` in the last presence.
  #[derive(Debug, Clone, PartialEq, serde::Serialize)]
  pub struct Row {
      /// The routable name (`label/session`).
      pub name: String,
      /// The harness hosting this session.
      pub harness: String,
      /// How the session was established: `spawn` / `attach`.
      pub mode: String,
      /// The A2A session state (`idle` / `working` / `input-required`) — the
      /// *stored* value. [`Roster::rows`] displays `stalled` in its place when a
      /// `working` row's `last_update_at` has aged past the stall threshold.
      pub state: String,
      /// The transport's liveness: `connected` / `reconnecting` / `gone`.
      pub conn_state: &'static str,
      /// The token that currently holds the row.
      pub token_id: String,
      /// The body's client id.
      pub client_id: String,
      /// The body's hostname.
      pub hostname: String,
      /// The attached harness session id (attach mode only).
      pub harness_session_id: Option<String>,
      /// When the row was first seen (epoch seconds).
      pub first_seen: u64,
      /// When the token was last heard from (epoch seconds) — the TTL's input.
      pub last_seen: u64,
      /// When the in-flight turn last produced a `session/update` (epoch
      /// seconds), or `None` for a non-`working` row — the `stalled` input.
      pub last_update_at: Option<u64>,
      /// The held permission(s)/elicitation(s) (issue #151) — the `PENDING`
      /// column; `--json` carries the full array.
      pub pending: Option<Vec<PendingItem>>,
      /// The JSON-RPC id of the current / most recent `session/prompt` (issue
      /// #142) — the `holler wait --after` basis.
      pub turn_id: Option<String>,
      /// The most recently completed turn (issue #142) — the `LAST TURN` column.
      pub last_turn: Option<LastTurn>,
      /// The session hold (issue #442), filled in from the hub's hold registry
      /// when the row is read by `control/roster`; the roster itself never holds
      /// it (a hold belongs to the session name, not to a row). Flattened, and
      /// absent for a session that is not held.
      #[serde(flatten)]
      pub hold: holler_proto::SessionHold,
  }
  ```

  The reply to `control/roster`, built on each call (`crates/holler-hub/src/control_server.rs:461-475`):

  ```rust
  async fn roster_control(cid: &holler_proto::CorrelationId, obj: &serde_json::Value, roster: &Roster, registry: &Registry) -> String {
      let params = obj.get("params");
      let all = params.and_then(|p| p.get("all")).and_then(|v| v.as_bool()).unwrap_or(false);
      let prefix = params.and_then(|p| p.get("prefix")).and_then(|v| v.as_str());
      let mut rows = roster.rows_matching(Option::from(all), prefix);
      // Issue #442: the hold is hub state keyed by session name, so it is read
      // here from the registry rather than stored on the (presence-driven) row.
      for row in &mut rows {
          row.hold = registry.holds().row_hold(&row.name);
      }
      encode_response(
          cid,
          serde_json::json!({ "rows": rows }),
      )
  }
  ```

  *Added at the brief gate (round 2), the keys acceptance 3 keeps, in full:* `Row` has no `skip_serializing_if`, so each of its fifteen fields other than `hold` is a key of every row, `null` when `None`. `hold` is flattened from `holler_proto::SessionHold`, whose five fields (`hold`, `hold_reason`, `held_since`, `hold_kind`, `hold_default`) are keys only when set, so a row that is not held has none of them (`crates/holler-proto/src/hold.rs:28-50`). None of the six new keys of scope item 6 is one of these twenty.

  ```rust
  #[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
  pub struct SessionHold {
      /// `true` while the session is held.
      #[serde(default, skip_serializing_if = "is_false")]
      pub hold: bool,
      /// Why, in the operator's words. Absent when none was given.
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub hold_reason: Option<String>,
      /// When the hold was set (RFC 3339).
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub held_since: Option<String>,
      /// Which hold the three fields above describe (issue #460): `operator`
      /// (`holler hold`) or `default` (the session joined held). Present whenever
      /// `hold` is; a hub that predates default holds never sends it, which means
      /// `operator`. When both holds are on a session it names the operator hold
      /// (the one that beats a release grant).
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub hold_kind: Option<String>,
      /// `true` only when a default hold sits under an operator hold (releasing
      /// the operator hold then leaves the session held by default).
      #[serde(default, skip_serializing_if = "is_false")]
      pub hold_default: bool,
  }
  ```

  The hub's pane registry, which its prompt gate reads (`crates/holler-hub/src/live.rs:475-478`, in `impl Registry`, `:450`), and what the hub does with panes (`crates/holler-hub/src/panes/mod.rs:8-10`):

  ```rust
      /// The pane registry the pane-state gate reads, if one is attached (story #646).
      pub fn panes(&self) -> Option<&Arc<crate::panes::PaneState>> {
          self.panes.as_ref()
      }
  ```

  ```text
  //! carried as a JSON-RPC result. The hub is the store only: it keeps what a verb recorded and
  //! what reconcile observed. It never infers a pane's session, stamps nothing and runs no
  //! adapter.
  ```
- **E3, where a pane's SHOWN and DRIVEN are.** The record (`crates/holler-pane/src/pane.rs:180-191`, and `:235-243`, fields of `Pane`, which is `:224-259`):

  ```rust
  /// What reconcile last observed about which session the pane shows and which the hub
  /// drives. Written by reconcile, never inferred.
  #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  #[serde(deny_unknown_fields)]
  pub struct LastObserved {
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub shown: Option<String>,
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub driven: Option<String>,
      /// When it was observed (milliseconds since the Unix epoch).
      pub at: i64,
  }
  ```

  ```rust
      /// THE session: the TUI is attached to it and the hub drives it.
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub session_of_record: Option<String>,
      pub role: PaneRole,
      pub hold: Hold,
      pub last_observed: LastObserved,
      /// The profile the pane belongs to; a pane belongs to at most one.
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub profile: Option<ProfileName>,
  ```

  The port that observes SHOWN (`crates/holler-pane/src/ports.rs:205-206`):

  ```rust
      /// The session the TUI of `pane` shows, if it can tell.
      fn shown_session(&self, pane: &PaneId) -> Result<Option<String>, PaneError>;
  ```

  `docs/adr/ADR-0021.md`: lines 126-127 (section 2, "`HarnessPort` as built (#642)"), line 204 (I2), lines 639-644 (section 11) and lines 648-649 (section 12):

  ```text
  3. `shown_session` is `None` whenever it cannot tell: the home screen, a deleted session, a title that is not a whole id,
     no TUI; it never guesses.
  ```

  ```text
  | I2 | `session_of_record` is the only session a pane's TUI shows and the hub drives. | After each of launch, relaunch, switch and reset on the fakes, the fake TUI's shown session, the driven session and `session_of_record` are equal. |
  ```

  ```text
  **Deferred to #649 (wiring) and #654 (cutover):** how the hub's DRIVEN session is pointed at `session_of_record` for each
  registered pane, and when it moves after a switch or reset.
  Until #649 wires DRIVEN, reconcile (#647) compares SHOWN with `session_of_record`, which I2 makes the session the hub
  drives, and leaves `last_observed.driven` as stored.
  A switch or reset (#645) likewise leaves `last_observed.driven` as stored; the hub's DRIVEN session follows the new
  `session_of_record` once #649 wires it.
  ```

  ```text
  **Decided:** the hub runs no adapters (ruling 1), so it executes no long work. The CLI process that runs a verb executes
  every step, and every port call is bounded by I5 (default 10 s) or ends in `timeout`. A verb that times out stops,
  ```

  How `pane list` (#643) reads them, and its offer to the roster (`crates/holler-cli/src/pane/list.rs:4-16`, `:170-184`):

  ```rust
  //! The shared items are `pub` and live here because the frozen `pane/mod.rs` admits no new
  //! module. [`PaneRow`] is one pane in summary: a `list` row, and the `pane` of a `watch`
  //! put. [`SessionSync`] is SYNC, read from a record through [`shown_differs`], the one
  //! SHOWN/DRIVEN rule (`pane doctor` uses it too). [`text_value`] and [`json_text`] make a
  //! stored string safe to print on a terminal, and [`observed_at`] prints a record
  //! timestamp. `get.rs` and `watch.rs` use them through `super::list`, and a later consumer
  //! (the roster, #648) can use them through `crate::pane::list`.
  //!
  //! The read verbs read the record and nothing else. They call `PaneStore::{get, list,
  //! watch}`, `ProfileStore::get` and `ProfileScope::resolve`, never an adapter or a probe,
  //! and they write nothing. SHOWN and health are what reconcile last recorded. DRIVEN is
  //! printed as the record holds it and never compared: nothing records it before the hub
  //! wiring (#649).
  ```

  ```rust
  impl From<&Pane> for PaneRow {
      fn from(pane: &Pane) -> Self {
          Self {
              name: pane.name.clone(),
              pos: pane.herdr.grid,
              profile: pane.profile.clone(),
              project: pane.host.cwd.clone(),
              health: pane.harness.health.clone(),
              shown: pane.last_observed.shown.clone(),
              driven: pane.last_observed.driven.clone(),
              sync: SessionSync::of(pane),
              hold: pane.hold.clone(),
          }
      }
  }
  ```

  The CLI's ports until #649 (`crates/holler-cli/src/pane/wiring.rs:8-13`, `:29-33`):

  ```rust
  //! **Stub (story #670).** `connect` hands out [`Unwired`], whose every method answers
  //! `not-implemented`, so a verb that runs before its wiring exists fails loudly and never acts.
  //! Story #649 replaces the body of `connect` (and the fields of `Wiring`) with the real stores and
  //! adapters, and keeps `Unwired`: it is the one not-implemented port set, and the in-process verb
  //! harness (`tests/verb_harness`) builds its `Ports` from it until the test kit's fakes (#638)
  //! replace it there.
  ```

  ```rust
  impl Wiring {
      /// Build the ports of a run. Fails when something they need cannot be reached.
      pub fn connect() -> Result<Self, PaneError> {
          Ok(Self { ports: Unwired })
      }
  ```
- **E4, `roster --profile` today.** The flag (`crates/holler-cli/src/cli.rs:464-484`) and its group (`crates/holler-cli/src/pane/args.rs:19-25`):

  ```rust
  #[derive(Parser, Debug)]
  pub struct Roster {
      /// Include `gone` rows (bodies that have dropped off) in the listing.
      /// Without this the roster shows the live-only view (connected +
      /// reconnecting).
      #[arg(long)]
      pub all: bool,
      /// List only sessions named exactly PREFIX or nested under it
      /// (`PREFIX/…`) — e.g. `--prefix io` (or `io/`) matches `io` and every
      /// `io/<session>` (ADR 0005 §4).
      #[arg(long)]
      pub prefix: Option<String>,
      /// Run against a remote hub instead of the local one (issue #508). See
      /// [`Status::server`].
      #[arg(long)]
      pub server: Option<String>,
      // `--profile NAME` (epic #633): parsed here and refused by `main.rs` until
      // story #648 (the roster story) renders it.
      #[command(flatten)]
      pub profile: ProfileOpt,
  }
  ```

  ```rust
  /// `--profile NAME`: scope the verb to a profile.
  #[derive(Args, Debug, Clone, Default, PartialEq, Eq)]
  pub struct ProfileOpt {
      /// Act on the panes of this profile; a named pane must belong to it.
      #[arg(long, value_name = "NAME")]
      pub profile: Option<String>,
  }
  ```

  In `crates/holler-cli/src/main.rs`: the story constant (`:43-44`), how a leaf's result is printed (`:80-89`, in `print_leaf_result`, `:74-90`), `dispatch` up to the roster's arm (`:214-229`), and the helper with its refusal (`:272-285`):

  ```rust
  /// The story that renders `roster` (and so `roster --profile`).
  const ROSTER_STORY: u32 = 648;
  ```

  ```rust
      if !message.is_empty() {
          if to_stderr {
              eprintln!("error: {message}");
          } else if newline_on_success {
              println!("{message}");
          } else {
              print!("{message}");
          }
      }
      exit_code
  ```

  ```rust
  /// Run the command and return its exit code. Every [`Command`] has an arm and
  /// there is no catch-all, so a new command that is not dispatched fails to
  /// compile. `--format=json` is `--json` for the verbs that predate the
  /// envelope: they keep their own `--json` shape, and this passes them the one
  /// resolved `json` (#648 takes the explicit-`--format=json` bit for `roster`
  /// from `choice.json_explicit`).
  fn dispatch(cli: &Cli, choice: FormatChoice) -> i32 {
      let json = choice.format == Format::Json;
      match &cli.command {
          Command::Hub(hub) => run_hub(hub, json),
          Command::Body(body) => run_body(body, json),
          Command::Pane(cmd) => run_with_stdio(choice.format, |ctx| holler_cli::pane::run(cmd, ctx)),
          Command::Profile(cmd) => {
              run_with_stdio(choice.format, |ctx| holler_cli::profile::run(cmd, ctx))
          }
          Command::Roster(roster) => run_roster(roster, json),
  ```

  ```rust
  /// `roster`. `--profile` (epic #633) parses but is refused until story #648
  /// renders it, so `roster_cmd.rs` stays untouched for that story.
  fn run_roster(roster: &Roster, json: bool) -> i32 {
      if roster.profile.profile.is_some() {
          return print_leaf_result(
              &output::not_implemented_message(ROSTER_STORY),
              true,
              1,
              false,
          );
      }
      let result = holler_cli::roster_cmd::run(roster, json);
      print_leaf_result(&result.message, result.to_stderr, result.exit_code, false)
  }
  ```

  The message (`crates/holler-cli/src/output.rs:251-255`):

  ```rust
  /// What a verb says until its story lands: `not implemented (story #N)`. The one wording of the
  /// stubs and of the `--pane` and `--profile` refusals of the legacy verbs.
  pub fn not_implemented_message(story: u32) -> String {
      format!("not implemented (story #{story})")
  }
  ```

  The workspace lint (`Cargo.toml:32-33`):

  ```toml
  [workspace.lints.rust]
  dead_code = "deny"
  ```

  The existing process test that pins the refusal (`crates/holler-cli/tests/pane_verbs/process/legacy_verbs.rs:95-105`) and its constant (`crates/holler-cli/tests/pane_verbs/process/stub.rs:35-38`):

  ```rust
  #[test]
  fn roster_profile_is_refused_with_the_roster_story() {
      for format in [&[][..], &["--json"], &["--format=json"]] {
          let mut argv = vec!["roster", "--profile", "demo"];
          argv.extend_from_slice(format);
          let out = holler(&argv);
          assert_eq!(out.code, 1, "{argv:?}: {out:?}");
          assert!(out.stdout.is_empty(), "{argv:?}: {out:?}");
          assert!(out.stderr_has_line(ROSTER_REFUSAL), "{argv:?}: {out:?}");
      }
  }
  ```

  ```rust
  /// The refusal of `roster --profile`, until story #648.
  // #646
  // #648
  pub const ROSTER_PROFILE_REFUSAL: &str = "error: not implemented (story #648)";
  ```
- **E5, `--format=json` told from `--json`.** The global flags (`crates/holler-cli/src/cli.rs:63-81`):

  ```rust
      /// Print machine-readable output. The same as `--format=json`.
      ///
      /// Global (issue #147/#155): reachable after any leaf, including
      /// `hub token mint --label x --json` — which the per-struct copies this
      /// replaced could not parse (the flag sat on `Token`, before the leaf).
      #[arg(long, global = true)]
      pub json: bool,

      /// Output format: `text` (the default) or `json`.
      ///
      /// `--format=json` is the same as `--json`, and `--json` with
      /// `--format=text` is a usage error. Under `holler pane` and `holler
      /// profile` JSON mode prints one envelope; the other verbs keep their own
      /// `--json` shape (ADR 0003).
      // Global (epic #633, story #670). Captured as an `Option` so that an
      // explicit `--format=json` can be told from a bare `--json` (`roster` keeps
      // its legacy `--json` shape and prints the envelope only for the former).
      #[arg(long, global = true, value_enum)]
      pub format: Option<Format>,
  ```

  The resolver and its "explicit json" bit (`crates/holler-cli/src/output.rs:49-82`):

  ```rust
  /// How the global `--format` and `--json` resolved.
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub struct FormatChoice {
      /// The format every verb prints in.
      pub format: Format,
      /// `--format=json` was given itself (`--json` alone does not set it). `roster` prints the
      /// envelope only for this, so `roster --json` keeps its legacy shape (ADR 0003: the `--json`
      /// shape is part of the surface); the `pane` and `profile` verbs treat either flag the same.
      pub json_explicit: bool,
  }

  /// Resolve the global `--json` and `--format` once, after parsing.
  ///
  /// `--format=json` is `--json`; `--json` with `--format=text` contradict each other and are a
  /// usage error (exit 2).
  pub fn resolve_format(json: bool, format: Option<Format>) -> Result<FormatChoice, ErrorBody> {
      match (json, format) {
          (true, Some(Format::Text)) => {
              Err(usage_body("--json cannot be combined with --format=text"))
          }
          (_, Some(format)) => Ok(FormatChoice {
              format,
              json_explicit: format == Format::Json,
          }),
          (true, None) => Ok(FormatChoice {
              format: Format::Json,
              json_explicit: false,
          }),
          (false, None) => Ok(FormatChoice {
              format: Format::Text,
              json_explicit: false,
          }),
      }
  }
  ```
- **E6, the envelope and its exit codes.** In `crates/holler-cli/src/output.rs`: the module's rule (`:10-16`), the version and the namespaces whose usage errors are envelopes (`:33-38`), the envelope (`:150-171`), `emit` (`:201-217`) and the exit code of an error (`:336-340`):

  ```rust
  //! The exit code is the same in both modes: `0` ok, `1` runtime failure, `2` usage, `3` refusal
  //! (ADR-0021 section 9). The class of the error's code decides it, through
  //! `holler_pane::error::class_of`, and this module keeps no table of its own. An error coded
  //! `usage` exits 2 however it got here, so a run-time guard of `holler-pane` (a bad pane name,
  //! `--command-json` that is not JSON) is a usage error and not a runtime failure; a request
  //! understood and declined (`pane-in-other-profile`, `grid-ambiguous`) exits 3; something that
  //! went wrong while doing the work (`timeout`, `unavailable`) exits 1.
  ```

  ```rust
  /// The envelope's `schema_version` (epic #633, "Output"). A change to the envelope's shape bumps it.
  pub const SCHEMA_VERSION: u32 = 1;

  /// The two subcommands whose usage errors are envelopes in JSON mode (decision 1(d) of #670).
  /// Every other verb keeps its own usage-error output, as ADR 0003 requires.
  const ENVELOPE_NAMESPACES: [&str; 2] = ["pane", "profile"];
  ```

  ```rust
  /// The one envelope of every `holler pane` and `holler profile` result in JSON mode.
  ///
  /// Keys are written in this order: `schema_version`, `ok`, `data`, `error`. `data` is `null` when
  /// `ok` is false and `error` is `null` when it is true.
  #[derive(Debug, Serialize)]
  pub struct Envelope<T> {
      pub schema_version: u32,
      pub ok: bool,
      pub data: Option<T>,
      pub error: Option<ErrorBody>,
  }

  impl<T> Envelope<T> {
      /// A successful result.
      pub fn success(data: T) -> Self {
          Self {
              schema_version: SCHEMA_VERSION,
              ok: true,
              data: Some(data),
              error: None,
          }
      }
  ```

  ```rust
  /// Print one result and return the exit code: 0 ok, else the exit code of the error's class
  /// (1 runtime failure, 2 usage, 3 refusal).
  ///
  /// `text` renders the data for text mode (JSON mode serializes the data itself and never calls
  /// it). The text ends with one newline, which `emit` adds when it is missing; empty text writes
  /// nothing.
  pub fn emit<T: Serialize>(
      sink: &mut Sink<'_>,
      format: Format,
      result: Result<T, ErrorBody>,
      text: impl FnOnce(&T) -> String,
  ) -> i32 {
      match format {
          Format::Text => emit_text(sink, result, text),
          Format::Json => emit_json(sink, result),
      }
  }
  ```

  ```rust
  /// The exit code of an error: the exit code of its code's class, decided by
  /// `holler_pane::error::class_of` (1 runtime failure, 2 usage, 3 refusal).
  fn exit_code(error: &ErrorBody) -> i32 {
      class_of(error.code.as_str()).exit_code()
  }
  ```

  The classes and their exit codes (`crates/holler-pane/src/error.rs:235-246`, `:266-303`) and the text of `profile-not-found` (`:659`):

  ```rust
  impl ErrorClass {
      /// The exit code of a verb that ends with an error of this class, the same in
      /// text and JSON mode: 2 usage, 3 refusal, 1 runtime failure (ADR 0003's exit
      /// codes; success is 0 and has no class).
      pub const fn exit_code(self) -> i32 {
          match self {
              ErrorClass::Usage => 2,
              ErrorClass::Refusal => 3,
              ErrorClass::Failure => 1,
          }
      }
  }
  ```

  ```rust
  pub fn class_of(code: &str) -> ErrorClass {
      let Some(closed) = PaneCode::parse(code) else {
          return if is_valid_code(code) {
              ErrorClass::Refusal
          } else {
              ErrorClass::Failure
          };
      };
      match closed {
          PaneCode::Usage => ErrorClass::Usage,
          // Understood and declined: a guard, a policy or a gate said no, the name is
          // taken, or the request named something that does not exist.
          PaneCode::GridAmbiguous
          | PaneCode::GridOutOfRange
          | PaneCode::CommandNotArgv
          | PaneCode::EnvNameInvalid
          | PaneCode::ProfileSecretRefused
          | PaneCode::ProfileExists
          | PaneCode::ProfileHasLivePanes
          | PaneCode::PaneNotInProfile
          | PaneCode::PaneInOtherProfile
          | PaneCode::ProbeFailed
          | PaneCode::HerdrVersionUnsupported
          | PaneCode::ProfileNotFound
          | PaneCode::PaneNotFound
          | PaneCode::SessionNotFound => ErrorClass::Refusal,
          // Went wrong while doing the work: a race between writers, a bound that ran
          // out, something unreachable or unreadable, live state that disagrees with
          // its spec, or work the verb cannot do yet.
          PaneCode::GenerationConflict
          | PaneCode::ProfileConflict
          | PaneCode::Timeout
          | PaneCode::Unavailable
          | PaneCode::StoreCorrupt
          | PaneCode::NotImplemented
          | PaneCode::ProfileDrift => ErrorClass::Failure,
      }
  }
  ```

  ```rust
              PaneError::ProfileNotFound { what } => write!(f, "profile not found: {what}"),
  ```

  The usage-error paths in `crates/holler-cli/src/main.rs` (`:115-120`, the doc of `parse_or_exit`; `:160-184`):

  ```rust
  /// Parse the command line. A usage error exits 2 with clap's own message, and
  /// `--help`/`--version` print and exit 0, exactly as `Cli::parse()` did — with
  /// one addition (story #670, decision 1(d)): under the `pane` and `profile`
  /// subcommands, in JSON mode, a usage error is an envelope with code `usage`
  /// on stdout instead. Every other verb's usage error is untouched (ADR 0003:
  /// the `--json` shape is part of the surface).
  ```

  ```rust
  /// Resolve `--json` and `--format` once. `--json` with `--format=text` is a
  /// usage error (exit 2): an envelope under `pane` and `profile`, plain text
  /// elsewhere.
  fn format_or_exit(cli: &Cli) -> FormatChoice {
      match output::resolve_format(cli.json, cli.format) {
          Ok(choice) => choice,
          Err(error) => {
              let code = if matches!(cli.command, Command::Pane(_) | Command::Profile(_)) {
                  let (mut out, mut err) = (std::io::stdout(), std::io::stderr());
                  output::emit_error(
                      &mut Sink {
                          out: &mut out,
                          err: &mut err,
                      },
                      Format::Json,
                      error,
                  )
              } else {
                  eprintln!("error: {}", error.message);
                  2
              };
              std::process::exit(code)
          }
      }
  }
  ```
- **E7, `check_envelope`**, in the test-kit module #681 added (`crates/holler-pane-testkit/src/lib.rs:22`):

  ```rust
  //! - [`envelope`] — the JSON-envelope checker (slice b, #681).
  ```

  Its rules (`crates/holler-pane-testkit/src/envelope.rs:9-33`) and its signature (`:197-202`):

  ```rust
  //! [`check_envelope`] checks these rules in this order. The first rule broken is the
  //! fault, so output that breaks two rules always gets the same fault.
  //!
  //! 0. The exit code is 0, 1, 2 or 3. This is checked before stdout is read.
  //! 1. Stdout is one JSON value, followed by at most one `\n`. A value that starts at the
  //!    first byte and is followed by anything else (a `\r`, a blank line, a second
  //!    envelope) is `TextAfter`. Otherwise, output that does not start with `{` but has a
  //!    later `{` that begins valid JSON (a log line or a space before the envelope) is
  //!    `TextBefore`. Anything else (empty, blank, truncated, not JSON) is `NotJson`.
  //! 2. The value is an object.
  //! 3. It has the keys `schema_version`, `ok`, `data` and `error`. The first missing one,
  //!    in that order, is named.
  //! 4. It has no other key. The smallest extra key is named. Map order is not used: it
  //!    differs between builds, because `serde_json/preserve_order` (on in a `--workspace`
  //!    build) keeps document order.
  //! 5. `schema_version` is the JSON integer 1.
  //! 6. `ok` is the boolean `true` when the exit code is 0, and `false` otherwise.
  //! 7. On success, `error` is `null`. `data` may be any value, `null` included.
  //! 8. On failure, `error` is an object.
  //! 9. On failure, `data` is `null`.
  //! 10. `error` has exactly the keys `code` and `message`. A missing one is named first
  //!     (`error.code`, then `error.message`), then the smallest extra one (`error.<key>`).
  //! 11. `code` is a string that [`is_valid_code`] accepts.
  //! 12. `message` is a string that is not blank and has no `\n` or `\r`.
  //! 13. The exit code matches the class of the code, `class_of(code).exit_code()`.
  ```

  ```rust
  /// Checks that `stdout` is exactly one envelope that agrees with `exit_code`, the exit
  /// code of the verb that wrote it.
  ///
  /// Returns the envelope, or the fault of the first rule it breaks, in the order the
  /// module doc lists.
  pub fn check_envelope(stdout: &str, exit_code: i32) -> Result<Envelope, EnvelopeFault> {
  ```

  `holler-cli` already has the test kit as a dev-dependency (`crates/holler-cli/Cargo.toml:431-435`):

  ```toml
  [dev-dependencies]
  # #670: the fakes and conformance suite of holler-pane (empty until #638). Its
  # consumer test is `testkit_links` in tests/pane_verbs/list.rs; #638 and
  # #643-#647 use it from their own pane_verbs/<verb>.rs files.
  holler-pane-testkit = { path = "../holler-pane-testkit" }
  ```
- **E8, `ProfileScope`** (`crates/holler-pane/src/profile.rs:272-277`, `:377-408`):

  ```rust
  /// What [`ProfileScope::resolve`] returns: the profile and the panes in scope.
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct ResolvedScope {
      pub profile: Profile,
      pub panes: Vec<Pane>,
  }
  ```

  ```rust
  /// The helper every `--profile` verb uses to scope itself to a profile and to edit a
  /// spec in one transaction with the live change. Implemented in
  /// `holler-cli/src/pane/profile_scope.rs` (#663); frozen by #637.
  ///
  /// **Blocking.** Every method is synchronous. Call from `spawn_blocking` (or a
  /// thread) in async code. Every method returns within I5's bound (default 10 s) or
  /// with [`PaneError::Timeout`]. An implementation is `Send + Sync`.
  pub trait ProfileScope: Send + Sync {
      /// The profile and the panes of it a verb acts on. With no `pane`, every pane
      /// of the profile; with a named pane, just that one, which must belong to the
      /// profile (`pane-not-in-profile` otherwise). A missing profile is
      /// `profile-not-found`.
      fn resolve(
          &self,
          profile: &ProfileName,
          pane: Option<&PaneName>,
      ) -> Result<ResolvedScope, PaneError>;

      /// Edit the spec of `pane` in `profile` and make the live change (`act`) as one
      /// transaction (I8): the profile is written with a compare-and-swap on its
      /// generation first, then `act` runs, then the result is recorded; if `act`
      /// fails nothing is recorded, and a conflict after `act` is `profile-conflict`.
      /// Returns the edited profile. With `profile: None` it runs only `act` and
      /// touches no profile (and returns `None`).
      fn edit_spec(
          &self,
          profile: Option<&ProfileName>,
          pane: &PaneName,
          edit: &SpecEdit,
          act: &mut dyn FnMut() -> Result<(), PaneError>,
      ) -> Result<Option<Profile>, PaneError>;
  }
  ```

  The CLI's implementation (`crates/holler-cli/src/pane/profile_scope.rs:76-90`, `:159-174`):

  ```rust
      /// The profile `profile` as stored: `profile-not-found` when there is none.
      fn stored(&self, profile: &ProfileName) -> Result<Profile, PaneError> {
          let stored = self.profiles.get(profile)?;
          stored.ok_or_else(|| PaneError::ProfileNotFound {
              what: profile.to_string(),
          })
      }

      /// Every pane whose record names `profile` (by slug), in name order.
      fn members(&self, profile: &ProfileName) -> Result<Vec<Pane>, PaneError> {
          let mut members = self.panes.list()?;
          members.retain(|pane| belongs(pane, profile));
          members.sort_by(|x, y| x.name.cmp(&y.name));
          Ok(members)
      }
  ```

  ```rust
  impl ProfileScope for StoreScope {
      fn resolve(
          &self,
          profile: &ProfileName,
          pane: Option<&PaneName>,
      ) -> Result<ResolvedScope, PaneError> {
          let stored = self.stored(profile)?;
          let panes = match pane {
              None => self.members(&stored.name)?,
              Some(name) => vec![self.member(&stored.name, name)?],
          };
          Ok(ResolvedScope {
              profile: stored,
              panes,
          })
      }
  ```

  The test kit's (`crates/holler-pane-testkit/src/profile_scope.rs:1-4`, `:190-200`):

  ```rust
  //! `FakeProfileScope`, the in-memory `ProfileScope` that verb tests run against instead
  //! of the CLI's (#663): membership, the every-pane scope and the spec-edit transaction,
  //! in ADR-0021's I8 write order, over any `ProfileStore` and `PaneStore` (slice c of
  //! #638, #688).
  ```

  ```rust
  impl ProfileScope for FakeProfileScope {
      fn resolve(
          &self,
          profile: &ProfileName,
          pane: Option<&PaneName>,
      ) -> Result<ResolvedScope, PaneError> {
          let stored = self.stored(profile)?;
          let panes = match pane {
              None => self.members(&stored.name)?,
              Some(name) => vec![self.member(&stored.name, name)?],
          };
  ```

  How a verb holds it (`crates/holler-pane/src/ports.rs:269-280`) and how `pane list` uses it (`crates/holler-cli/src/pane/list.rs:93-107`):

  ```rust
  /// One `&dyn` of each port: what a verb holds. `Ports` is `Copy`, so it is passed
  /// by value or by reference freely, and `Ports<'static>` is `Send + Sync`.
  #[derive(Clone, Copy)]
  pub struct Ports<'a> {
      pub pane_store: &'a dyn PaneStore,
      pub profile_store: &'a dyn ProfileStore,
      pub herdr: &'a dyn HerdrPort,
      pub host: &'a dyn HostPort,
      pub harness: &'a dyn HarnessPort,
      pub scope: &'a dyn ProfileScope,
      pub prober: &'a dyn Prober,
  }
  ```

  ```rust
  /// The panes in scope, as rows sorted by name. The names are typed here, so a bad one is
  /// `usage` (exit 2) in both formats. With `--profile` the scope decides (`profile-not-found`,
  /// `pane-not-in-profile`); without it a named pane that has no record is an empty table.
  fn rows_in_scope(args: &PaneList, ports: Ports<'_>) -> Result<PaneTable, PaneError> {
      let pane = args.pane.as_deref().map(PaneName::parse).transpose()?;
      let mut panes = match (profile_name(&args.profile)?, pane) {
          (Some(profile), pane) => ports.scope.resolve(&profile, pane.as_ref())?.panes,
          (None, Some(pane)) => ports.pane_store.get(&pane)?.into_iter().collect(),
          (None, None) => ports.pane_store.list()?,
      };
      panes.sort_by(|a, b| a.name.cmp(&b.name));
      Ok(PaneTable {
          panes: panes.iter().map(PaneRow::from).collect(),
      })
  }
  ```
- **E9, ADR 0003 lines 105-106** (`docs/adr/ADR-0003.md`, section "Versioning"), verbatim:

  ```text
  - **SemVer from 0.1.0.** Standard pre-1.0 behaviour — **breaking changes bump MINOR while MAJOR is 0**. This **deliberately reverses ADR-0014's opt-out**, because there is no external library consumer.
  - **"Breaking"** = the CLI surface, the `--json` shape, the on-disk formats (token store, credential file, session config), and the wire protocol version. **Additive = MINOR. Fixes = PATCH.**
  ```
- **E10, the grid position.** The pane's name and where it sits in Herdr (`crates/holler-pane/src/pane.rs:25-32`, `:91-100`):

  ```rust
  /// The name of a pane, e.g. `hj-c1r1` (the tmux session name). It is a
  /// `holler_proto::vocab::SessionName`: the ADR 0005 name grammar is reused, not
  /// copied, so a pane name is exactly a valid session name. Serde goes through
  /// [`PaneName::parse`]; an invalid name is `usage`.
  ///
  /// Pane names such as `hj-c1r2` are names, not positions, and are not renamed.
  #[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
  pub struct PaneName(SessionName);
  ```

  ```rust
  /// Where a pane sits in Herdr: its session, workspace, Herdr's id for it and its
  /// grid cell. Also what `HerdrPort::ensure_pane` returns for a pane that exists.
  #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  #[serde(deny_unknown_fields)]
  pub struct HerdrPane {
      pub session: String,
      pub workspace: String,
      pub pane_id: PaneId,
      pub grid: GridPos,
  }
  ```

  `GridPos` (`crates/holler-pane/src/grid.rs:68-73`, `crates/holler-pane/src/grid.rs:129-138`):

  ```rust
  impl fmt::Display for GridPos {
      /// The `rRcC` form, row first: `r2c1`.
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
          write!(f, "r{}c{}", self.row, self.col)
      }
  }
  ```

  ```rust
  impl Serialize for GridPos {
      /// `{"row":R,"col":C,"pos":"rRcC"}`, in that key order.
      fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
          let mut out = serializer.serialize_struct("GridPos", 3)?;
          out.serialize_field("row", &self.row)?;
          out.serialize_field("col", &self.col)?;
          out.serialize_field("pos", &self.to_string())?;
          out.end()
      }
  }
  ```

  `docs/adr/ADR-0021.md` section 10, lines 610-615:

  ```text
  - **Output** is always row first: text `r2c1`; JSON `{"row": 2, "col": 1, "pos": "r2c1"}`, in that key order. On input `pos`
    is optional.
  - The Herdr adapter (#640) is the only code that converts a `GridPos` to Herdr's own order and base. `HerdrPort::ensure_pane`
    reaches a position by right and down splits (B4, `plan_splits`, #640) or fails loudly, and never relocates a healthy pane.
    `relaunch` moves a pane only when `--grid` is given. A pane's position comes from Herdr's snapshot, never from its name:
    `hj-c1r2` sits at `r2c1`.
  ```

  `pane list` takes `pos: pane.herdr.grid` (`crates/holler-cli/src/pane/list.rs:174`, in E3) and prints it through the `Display` above, as the second text cell of a row (`crates/holler-cli/src/pane/list.rs:190-193`):

  ```rust
      pub fn cells(&self) -> [String; COLUMNS.len()] {
          [
              text_value(self.name.as_str()),
              self.pos.to_string(),
  ```
- **E11, the SHOWN/DRIVEN rule already in the code.** The CLI's check for `--pane` routing (`crates/holler-cli/src/say_cmd.rs:341-364`, called by `resolve_pane_target` at `:314`), and its twin, the hub's prompt gate (`crates/holler-hub/src/circuit/dispatch.rs:77-103`):

  ```rust
  /// What the record says against prompting the pane, in the order parked, unhealthy,
  /// SHOWN differing from DRIVEN (one side absent is not a mismatch); each message names
  /// the pane, the code and the remedy on one line (stored text is `quoted`).
  fn pane_state_refusal(record: &Pane) -> Option<PaneError> {
      let name = &record.name;
      let doctor = doctor_command(Some(name), false);
      let (code, message) = if let Hold::Parked { reason, .. } = &record.hold {
          let reason = quoted(reason);
          let message = format!("{name} is parked ({PANE_PARKED}, reason {reason}); run holler pane unpark {name} first");
          (PANE_PARKED, message)
      } else if let Health::Unhealthy(reason) = &record.harness.health {
          let reason = quoted(reason);
          (
              PANE_UNHEALTHY,
              format!("{name} is unhealthy ({PANE_UNHEALTHY}): {reason}; run {doctor}"),
          )
      } else {
          let seen = &record.last_observed;
          let (Some(shown), Some(driven)) = (seen.shown.as_deref(), seen.driven.as_deref()) else {
              return None;
          };
          if shown == driven {
              return None;
          }
  ```

  ```rust
  /// What a pane's record says against prompting its session of record: its code and
  /// a few words of why, or `None` when nothing does. The record is the source (the
  /// hold `park` set, the health and the last observation reconcile wrote), never a
  /// live probe: parked; unhealthy; or SHOWN and DRIVEN both observed and differing
  /// (one side absent is not a mismatch). Checked in that order.
  ///
  /// Its twin is the CLI's routing engine, `holler_cli::say_cmd::resolve_pane_target`,
  /// which refuses the same three conditions before contacting the hub. The predicate
  /// exists twice only because `holler-pane` is frozen and the hub cannot depend on the
  /// CLI crate; keep the two equal.
  fn pane_state_refusal(pane: &holler_pane::Pane) -> Option<(&'static str, &'static str)> {
      use holler_pane::pane::{Health, Hold};
      if matches!(pane.hold, Hold::Parked { .. }) {
          return Some((PANE_PARKED, "is parked"));
      }
      if matches!(pane.harness.health, Health::Unhealthy(_)) {
          return Some((PANE_UNHEALTHY, "is unhealthy"));
      }
      let seen = &pane.last_observed;
      match (seen.shown.as_deref(), seen.driven.as_deref()) {
          (Some(shown), Some(driven)) if shown != driven => Some((
              PANE_SHOWN_DRIVEN_MISMATCH,
              "shows a session other than the one it drives",
          )),
          _ => None,
      }
  }
  ```

  *Added at the brief gate (round 2):* a third form, `shown_differs`, is the rule that reconcile, `pane list`'s SYNC, launch and switch use (`crates/holler-pane/src/reconcile.rs:171-181`; `list.rs` names it in E3). Unlike the pair rule above, it compares SHOWN with `session_of_record`, not with `last_observed.driven`, and it counts `shown: None` as a mismatch. Its doc names the roster among the readers that call it; the gate at `send_prompt` that it also names is the hub's prompt gate above, which applies the pair rule instead. Scope item 3 states the summary line's own rule ("both known and not equal"); which session DRIVEN is remains the question scope item 7 leaves open.

  ```rust
  /// Whether a pane whose session of record is `session_of_record` shows something else: the
  /// shown/driven mismatch, since I2 makes the session of record the session the hub drives.
  /// `shown: None` is the home screen, a mismatch too. A pane with no session of record has
  /// no mismatch (it is `no-session-of-record`).
  ///
  /// The one form of this comparison: a pass calls it with what the TUI shows now, and a
  /// reader of a record (the roster, `pane get`, the gate at `send_prompt`) with
  /// `last_observed.shown`, provided `last_observed.at > 0` (`0` is never observed).
  pub fn shown_differs(session_of_record: Option<&str>, shown: Option<&str>) -> bool {
      session_of_record.is_some_and(|record| shown != Some(record))
  }
  ```
- **E12, who writes `last_observed` (added at the brief gate, round 2).** Three production writers set it, and none gives `driven` a new value: reconcile (#647) writes `last_observed.shown` and `last_observed.at` (and `harness.health`); `pane switch` and `pane reset` (#645) write `session_of_record`, `last_observed.shown`, `last_observed.at` and `harness.health`; `pane launch` and `pane relaunch` (#644) write a whole record whose `driven` is the old record's, or `None` for a new pane. A search of the source at `92fbc6e` for `last_observed` finds no other write: its other uses read it, declare it or are comments, except the two test helpers quoted below. A test is not bound by this: it builds a record from the test kit's `sample_pane`, whose `last_observed` is empty, and sets the pair it needs, as the hub's prompt-gate tests do. Reconcile's rule (`crates/holler-pane/src/reconcile.rs:31-35`) and its writer (`crates/holler-pane/src/reconcile/observe.rs:350-375`):

  ```rust
  //! - `last_observed.shown` is the session the TUI showed (after a fix, the one it showed
  //!   then). `None` with `at > 0` is an observed home screen, or a pane with no TUI.
  //! - `last_observed.driven` is left as stored. No port observes DRIVEN before #649, so the
  //!   mismatch compares SHOWN with `session_of_record` ([`shown_differs`]), which I2 makes
  //!   the session the hub drives.
  ```

  ```rust
  /// Record what the chain observed (`harness.health`, `last_observed.shown`, stamped
  /// `request.now_ms`) when it differs from the record or is the record's first observation
  /// (`at == 0`). One compare-and-swap at the generation the pass read, never retried; a
  /// failed write is an `observe-failed` finding.
  fn record(
      ports: Ports<'_>,
      request: &ReconcileRequest<'_>,
      pane: &Pane,
      server: &Server,
      screen: &Screen,
  ) -> Option<Finding> {
      let health = server.health().recorded();
      if health.is_none() && matches!(screen, Screen::Unseen) {
          return None;
      }
      let mut next = pane.clone();
      if let Some(health) = health {
          next.harness.health = health;
      }
      if let Screen::Seen(shown) = screen {
          next.last_observed.shown.clone_from(shown);
      }
      if next == *pane && pane.last_observed.at != 0 {
          return None;
      }
      next.last_observed.at = request.now_ms;
  ```

  Switch and reset (`crates/holler-pane/src/tx_switch.rs:391-400`):

  ```rust
  /// The record a run writes: `record` with `target` as its session of record and as what
  /// its TUI was seen showing at `now_ms`, and its server healthy. Every other field, and
  /// `last_observed.driven`, is `record`'s.
  fn recorded(record: &Pane, target: &str, now_ms: i64) -> Pane {
      let mut next = record.clone();
      next.session_of_record = Some(target.to_owned());
      next.last_observed.shown = Some(target.to_owned());
      next.last_observed.at = now_ms;
      next.harness.health = Health::Healthy;
      next
  ```

  Launch and relaunch (`crates/holler-pane/src/tx_launch.rs:311-322`, `:343-347`):

  ```rust
      /// The record R writes: what the act brought up and observed, and the spec's values. A
      /// relaunch keeps the record's hold, its stored DRIVEN (never inferred) and, without a
      /// profile, its profile.
      fn record(&self, seen: &Seen, live: Live, at: i64) -> Pane {
          let (hold, driven, profile) = match self.old {
              Some(old) => (
                  old.hold.clone(),
                  old.last_observed.driven.clone(),
                  self.profile.or(old.profile.as_ref()).cloned(),
              ),
              None => (Hold::None, None, self.profile.cloned()),
          };
  ```

  ```rust
              last_observed: LastObserved {
                  shown: live.shown,
                  driven,
                  at,
              },
  ```

  The test kit's sample record (`crates/holler-pane-testkit/src/fixture.rs:67-71`) and the hub's prompt-gate tests seeding a pair on it (`crates/holler-hub/src/circuit/dispatch.rs`, lines 548-556, in the module's `#[cfg(test)]` part, which starts at line 456):

  ```rust
          last_observed: LastObserved {
              shown: None,
              driven: None,
              at: 0,
          },
  ```

  ```rust
          let mut pane = holler_pane_testkit::fixture::sample_pane(name).unwrap();
          pane.session_of_record = session_of_record.map(str::to_owned);
          pane.hold = hold;
          pane.harness.health = health;
          pane.last_observed = LastObserved {
              shown: shown.map(str::to_owned),
              driven: driven.map(str::to_owned),
              at: 1,
          };
  ```
- **E13, how a verb gets its ports today (added at the brief gate, round 2).** `--profile` resolves through `ProfileScope` (scope item 4), which a verb holds as `Ports.scope` (E8). `main.rs` builds `Ports` in one place, `run_with_stdio`, which the `pane` and `profile` verbs run through: it takes them from `Wiring::connect` and hands them to the verb in a `VerbCtx` (`crates/holler-cli/src/main.rs:253-270`, `crates/holler-cli/src/output.rs:191-199`). `run_roster` builds no `VerbCtx`, and `roster_cmd::run(roster, json)` takes no `Ports` (E1, E4). Until #649 those ports are `Unwired` (E3), so in the real binary `ports.scope.resolve` answers `not-implemented` (`crates/holler-cli/src/pane/wiring.rs:203-210`); the acceptance tests run on the test kit's fakes (scope item 7). The issue does not fix `roster_cmd::run`'s new signature; `main.rs` and `roster_cmd.rs` are both in the blast radius. `crates/holler-cli/src/roster.rs`, which the blast radius also names, does not exist at `92fbc6e`.

  ```rust
  /// Run a `pane` or `profile` verb over the real wiring, printing to stdout and
  /// stderr. A wiring that cannot be built is reported the way a verb reports an
  /// error (exit 1, with the error's code).
  fn run_with_stdio(format: Format, verb: impl FnOnce(&mut VerbCtx<'_>) -> i32) -> i32 {
      let (mut out, mut err) = (std::io::stdout(), std::io::stderr());
      let mut sink = Sink {
          out: &mut out,
          err: &mut err,
      };
      match Wiring::connect() {
          Ok(wiring) => verb(&mut VerbCtx {
              format,
              ports: wiring.ports(),
              sink,
          }),
          Err(error) => output::emit_error(&mut sink, format, ErrorBody::from(&error)),
      }
  }
  ```

  ```rust
  /// What a `holler pane` or `holler profile` verb runs with.
  ///
  /// `ports` is held by value: [`Ports`] is `Copy`. A verb takes `&mut VerbCtx` because writing to
  /// the sink needs a mutable borrow.
  pub struct VerbCtx<'a> {
      pub format: Format,
      pub ports: Ports<'a>,
      pub sink: Sink<'a>,
  }
  ```
- **E14, what decides a roster envelope and a roster usage error (added at the brief gate, round 3).** `emit_error`, after `emit` (E6) in `crates/holler-cli/src/output.rs` (`:240-243`):

  ```rust
  /// Print an error (a failed `emit` with no data) and return its exit code.
  pub fn emit_error(sink: &mut Sink<'_>, format: Format, error: ErrorBody) -> i32 {
      emit(sink, format, Err::<(), _>(error), |()| String::new())
  }
  ```

  The one reader of `ENVELOPE_NAMESPACES` (`output.rs:352-369`). A search of `crates/holler-cli/src` finds the constant only at its declaration (`:38`) and here, and `envelope_namespace` called only at `crates/holler-cli/src/main.rs:148`:

  ```rust
  /// What the raw command line says, for a parse that failed: a failed parse has no parsed flags, so
  /// the choice between an envelope and clap's own message is made from the arguments themselves.
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct RawArgs<'a> {
      /// The first token that is neither a flag nor a flag's value: the subcommand the user meant.
      pub namespace: Option<&'a str>,
      /// `--json`, `--format=json` or `--format json` is on the line.
      pub json: bool,
  }

  impl<'a> RawArgs<'a> {
      /// The namespace whose usage error is an envelope: `pane` or `profile`, in JSON mode.
      /// Every other verb keeps its own usage-error output (decision 1(d) of #670).
      pub fn envelope_namespace(&self) -> Option<&'a str> {
          self.namespace
              .filter(|ns| self.json && ENVELOPE_NAMESPACES.contains(ns))
      }
  }
  ```

  How a failed parse is printed (`crates/holler-cli/src/main.rs:126-158`; the first paragraph of `parse_or_exit`'s doc, `:115-120`, is in E6). Under `roster`, `envelope_namespace` is `None`, so `usage_envelope` returns `None` and `error.exit()` prints clap's own message:

  ```rust
  fn parse_or_exit() -> Cli {
      match Cli::try_parse() {
          Ok(cli) => cli,
          Err(error) => {
              if error.use_stderr() {
                  if let Some(code) = usage_envelope(&error) {
                      std::process::exit(code);
                  }
              }
              error.exit()
          }
      }
  }

  /// The exit code after printing `error` as a usage envelope, if the command
  /// line asked for JSON under `pane` or `profile`. A failed parse has no parsed
  /// flags, so this reads the raw arguments.
  fn usage_envelope(error: &clap::Error) -> Option<i32> {
      let argv: Vec<String> = std::env::args_os()
          .map(|arg| arg.to_string_lossy().into_owned())
          .collect();
      let raw = output::scan_args(&argv, &Cli::global_value_flags());
      let message = output::usage_message(error, raw.envelope_namespace()?);
      let (mut out, mut err) = (std::io::stdout(), std::io::stderr());
      Some(output::emit_usage_error(
          &mut Sink {
              out: &mut out,
              err: &mut err,
          },
          Format::Json,
          &message,
      ))
  }
  ```

  The text of a usage error (`crates/holler-pane/src/error.rs:643`), which `format_or_exit` prints after `error: ` (E6), so `--json --format=text` under `roster` prints `error: --json cannot be combined with --format=text`:

  ```rust
              PaneError::Usage { message } => f.write_str(message),
  ```
- **E15, making a fake port fail, and reading its calls (added at the brief gate, round 3).** The fault switch every fake of the test kit shares (`crates/holler-pane-testkit/src/fault.rs:25-35`, `:67-77`, `:85-88`):

  ```rust
  /// A standing fault: it applies to every call until it is cleared.
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub enum Fault {
      /// The port does not answer: every call fails with `PaneError::Timeout { op }`,
      /// where `op` is the method's [`PortOp::as_str`]. The fake answers at once, without
      /// waiting out I5's bound. Add [`FaultSwitch::set_delay`] to make a caller's own
      /// timer fire.
      Wedged,
      /// Every call fails with this error (e.g. `store-corrupt` or `unavailable`).
      Fail(PaneError),
  }
  ```

  ```rust
      /// Turn a standing fault on (`Some`) or off (`None`).
      pub fn set(&self, fault: Option<Fault>) {
          self.lock().standing = fault;
      }

      /// Fail the next call of `op` with `error`, once. The errors queued for one method
      /// come out in the order they were queued, and a call of another method leaves them
      /// queued. A standing fault answers first, also leaving them queued.
      pub fn fail_next(&self, op: Op, error: PaneError) {
          self.lock().queued.push((op, error));
      }
  ```

  ```rust
      /// Every call made through the port, oldest first, the failed ones included.
      pub fn calls(&self) -> Vec<Op> {
          self.lock().calls.clone()
      }
  ```

  `FakePaneStore` reaches it through `faults()` (`crates/holler-pane-testkit/src/pane_store.rs:107-111`, below), and its `list` is the op `PaneStoreOp::List` (`:25`, in the enum at `:23-31`; `as_str` names it `"pane_store.list"`, `:37`). `FakeHarness`, `FakeHerdr` and `FakeHost` have the same accessor over their own ops (`crates/holler-pane-testkit/src/harness.rs:179`, `herdr.rs:211`, `host.rs:106`).

  ```rust
      /// The fault switch of every port method and the log of the calls made through
      /// the port.
      pub fn faults(&self) -> &FaultSwitch<PaneStoreOp> {
          &self.faults
      }
  ```

  The stable codes of the two store errors the tests use are `PaneCode::NotImplemented => "not-implemented",` (`crates/holler-pane/src/error.rs:100`) and `PaneCode::Unavailable => "unavailable",` (`:120`), both in `PaneCode::as_str` (`:97-123`).
- **E16, the order of the hub's rows (added at the brief gate, round 3).** `crates/holler-hub/src/roster.rs:521-526`, and `:553` in `rows_matching` (`:536-555`), which builds the `control/roster` reply (E2):

  ```rust
      /// The rows, sorted by name. `all` of `None` excludes `gone` rows (the
      /// default roster listing); `Some(true)` includes them. Each returned row
      /// has `stalled` applied to its `state` display where applicable.
      pub fn rows(&self, all: Option<bool>) -> Vec<Row> {
          self.rows_matching(all, None)
      }
  ```

  ```rust
          out.sort_by(|a, b| a.name.cmp(&b.name));
  ```
- **E17, what `pane list` prints for an absent value (added at the brief gate, round 3).** The SHOWN and DRIVEN cells (`crates/holler-cli/src/pane/list.rs:197-198`, in `PaneRow::cells`, `:190-202`), `optional_text` (`:293-296`) and `NO_VALUE` (`:34-35`):

  ```rust
              optional_text(self.shown.as_deref()),
              optional_text(self.driven.as_deref()),
  ```

  ```rust
  /// [`text_value`] of `value`, or `-` when there is none.
  pub fn optional_text(value: Option<&str>) -> String {
      value.map_or_else(|| NO_VALUE.to_owned(), text_value)
  }
  ```

  ```rust
  /// What a text cell or field holds when the record has no value.
  pub const NO_VALUE: &str = "-";
  ```
- **E18, a hub row's name beside a pane's name (added at the brief gate, round 4).** A pane's name is a `SessionName` (`crates/holler-pane/src/pane.rs:25-32`, in E10). The three name kinds (`crates/holler-proto/src/vocab.rs:3-11`), and the check that keeps a `/` out of a `SessionName` (`:79-88`):

  ```rust
  //! Three **distinct** name kinds, each a newtype around a `String`, sharing
  //! one segment grammar but differing in length and structure:
  //!
  //! - [`Label`] — the hub-unique hostname / label (the prefix a session is
  //!   routed under). 1–32 chars.
  //! - [`SessionName`] — a session **within** a single body. 1–32 chars, **no
  //!   `/`**.
  //! - [`RoutableName`] — the cross-hub router: `<label>/<session>`, or a bare
  //!   name. Each `/`-separated segment is 1–32 chars.
  ```

  ```rust
      /// Succeeds iff the string is a single 1–32-char segment matching the
      /// grammar. A `SessionName` never contains a `/` (that is the
      /// [`RoutableName`] shape).
      pub fn parse(s: &str) -> Result<Self, NameError> {
          if s.is_empty() {
              return Err(NameError::Empty);
          }
          if s.contains('/') {
              return Err(NameError::Slash);
          }
  ```

  How the hub names a row (`crates/holler-hub/src/roster.rs:845-857`). Rows are inserted only at `:727` and `:734`, both under the `name` that `apply_ad` takes from this function (`:709-710`), and `build_row` copies it into the row's `name` field (E2's `Row.name`):

  ```rust
  /// The roster row's key: `<label>/<bare-name>` (ADR 0005 §2) once the roster
  /// knows the advertising token's label (bound at `circuit/authenticate` time
  /// — see [`Roster::set_label`]); the bare name alone when it doesn't (a unit
  /// test that builds rows without binding a label, or — defensively — a
  /// presence that somehow outraces the auth-time bind). Bodies only ever send
  /// the bare name (ADR 0005 §2: "Bodies never send the label"), so this is
  /// entirely the hub's own attribution, not anything read off the wire.
  fn qualified_name(info: &TokenInfo, s: &SessionAd) -> String {
      match info.label.as_deref() {
          Some(label) if !label.is_empty() => format!("{label}/{}", s.name),
          _ => s.name.clone(),
      }
  }
  ```

  Where the hub binds the label (`crates/holler-hub/src/circuit.rs:300-307`), and the label a token is minted with (`crates/holler-hub/src/token.rs:413-417`; `mint` refuses a label that fails it, `:524-528`):

  ```rust
      // Issue #236 (ADR 0005 §2): the label travels with the authenticated
      // token, never on the wire, so this is the one place the hub can read it
      // — `record` is the verified token record from just above. Binding it
      // (and the client id) here, before any `session/presence` arrives, means
      // every roster row this body advertises is qualified `<label>/<session>`
      // from its very first row, not just from the second presence beat.
      roster.set_token(&params.token_id, &client_id);
      roster.set_label(&params.token_id, &record.label);
  ```

  ```rust
  /// The label grammar (ADR 0005 §1): `^[a-z0-9][a-z0-9-]{0,31}$`.
  fn valid_label(label: &str) -> bool {
      if !(1..=32).contains(&label.len()) {
          return false;
      }
  ```

  The code that relates a pane record, or a bare session name, to a hub session today. None of it compares a row's `name` with a pane's name. The hub's prompt gate matches a pane's `session_of_record` with the bare session name being prompted (`crates/holler-hub/src/circuit/dispatch.rs:105-111`, `:130-133`):

  ```rust
  /// The pane-state gate: refuse a prompt to `session` (the bare session name being
  /// sent) while any pane whose session of record it is has a refusal
  /// ([`pane_state_refusal`]). Fail-closed across panes on different bodies that
  /// record the same session name: any bad one refuses, and the first bad one in name
  /// order (`list` is name-sorted) is the one named. One synchronous read with no
  /// `await`, so it is totally ordered against a pane write, as `holds.admit` is
  /// against `hold`/`release`.
  ```

  ```rust
      let refused = records
          .iter()
          .filter(|pane| pane.session_of_record.as_deref() == Some(session))
          .find_map(|pane| pane_state_refusal(pane).map(|refusal| (pane, refusal)));
  ```

  `say --pane` sends to the pane's `session_of_record` (`crates/holler-cli/src/say_cmd.rs:309-316`, in `resolve_pane_target`, `:301-318`):

  ```rust
      let Some(session) = record.session_of_record.clone() else {
          return Err(PaneError::SessionNotFound {
              what: format!("{name} has no session of record"),
          });
      };
      match pane_state_refusal(&record) {
          Some(refusal) => Err(refusal),
          None => Ok(session),
  ```

  The roster's own match of a row with a bare session name (`crates/holler-hub/src/roster.rs:859-868`):

  ```rust
  /// True iff a (possibly qualified) row `name` is the advertisement for the
  /// *bare* session name `bare` — either the row carries no label (`name ==
  /// bare`) or it is qualified and `bare` is its final segment (`name ==
  /// "<label>/<bare>"`). Used by [`Roster::set_turn_id`]/[`Roster::
  /// set_last_turn`], whose callers (`talk::say`) know a connection's token id
  /// and the bare session name it prompted, not the label the roster qualified
  /// the row with.
  fn row_matches_bare(name: &str, bare: &str) -> bool {
      name == bare || name.ends_with(&format!("/{bare}"))
  }
  ```

## Pipeline

`rigor: second-opinion`.

## Depends on

#639, #643, #647 (part 1, merged), #670.

## Rules for this story

See the epic (#633): the contract section there is fixed. Issue first, tests first, Holler's pipeline and hooks apply (branch `issue-<N>-implementation`, fresh worktree, Conventional Commits, clippy clean, no new `unsafe`). Edit only the files in the Blast radius. No story touches a live fleet, a running pane or a real Herdr session; verification is the fakes from the test kit and temporary directories. Never print or commit a credential.

## Comments from the issue thread

### aangelinsf, 2026-10-11 02:48:59 MDT (2026-10-11T08:48:59Z)

A real instance to build a roster fixture from (observed 2026-10-11): a roster row can be internally consistent with `doctor` and still show another session's figures.

A pane was relaunched onto a new session S2 (176 messages, with an active sub-agent session under it, last message minutes ago). `doctor` reported the pane ok and the hub drove S2. The roster row for that pane kept showing the previous session's figures: `idle 10h`, context `63k 31%`, last turn `end_turn 841m ago`, and a stale work label. It was wrong for hours, and an orchestrator reading the roster acted on it.

Suggested tests:
- After a pane's session changes (relaunch, `switch`, `reset`), the next roster read shows the new session's context, idle time and last turn, never the previous session's. Assert on all three columns.
- A pane whose top-level session is waiting on an active sub-agent session shows as working (or names the sub-agent), not idle for the hours since the top-level session's own last message.
- The loud first line fires when the row's figures come from a session that is not the one driven.
