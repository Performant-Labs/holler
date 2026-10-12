# [feat(cli): the roster shows pane, SHOWN and DRIVEN, and a loud line when they differ](https://github.com/Performant-Labs/holler/issues/648)

Part of epic #633. This is the complete statement of the story (consolidated 2026-10-11 from earlier amendments; where an older note disagreed, this text is the decision).

## Refresh at main `bda1069` (2026-10-11): what changed since the brief gate

*Facts only. No decision of rounds 1 to 6 is withdrawn. #649, #832 and #652 merged after round 6 and changed facts the brief rested on; a passage below marked "(refresh)" had one of its facts corrected in place. Where an older passage and this section disagree on a fact, this section is the fact. On a decision the older passage stands, except where R1 to R9 below name it. These nine are new decisions of the refresh, taken under the standing rule to decide and report; the operator can overrule any of them. No outside model has reviewed the refresh, and nothing here claims that a gate passed.*

**The cut.** The excerpts of rounds 3 to 6 were cut at `3c139dd`. The first cut, `92fbc6e`, is not an object of this clone, so it could not be checked; instead every fenced excerpt was matched against the file at `bda1069`, and every one that is not listed below still matches. Every file this story edits is unchanged since `3c139dd`: `crates/holler-cli/src/roster_cmd.rs` (138 lines), `main.rs` (346), `cli.rs` (780), `output.rs` (467), and so are `crates/holler-hub/src/roster.rs`, `ProfileScope` in `crates/holler-pane/src/profile.rs`, the test kit's `check_envelope`, `FakePaneStore` and `FakeProfileScope`. E1, E2, E4 to E9, E11, E14, E16, E18 to E20 and the line numbers cited for those files stand as cut. These files changed and their excerpts were re-cut: `crates/holler-cli/src/pane/wiring.rs` and the new `wiring/store.rs` (#649; E3, E13), `crates/holler-cli/src/pane/list.rs` (#832; E3, E10, E17, and the pane-list citations), `crates/holler-pane/src/pane.rs` and `ports.rs` (E3, E10, E15, E19), the test kit's `harness.rs`, `herdr.rs`, `host.rs`, `fixture.rs` (E12, E15), `tx_launch.rs`, `tx_switch.rs`, `reconcile.rs` (E12, line shifts only), the process test's `stub.rs` and `pane_verbs/relaunch.rs` (E4, E20, line shifts only), `crates/holler-cli/Cargo.toml` (E13) and `docs/adr/ADR-0021.md` (E3, E10; 510 lines changed).

**What merged since the cut.**

- **#649** (`2d217dd`, 2026-10-11 17:35 MDT). The binary's wiring is real. `Wiring::connect` makes one `control/status` exchange with the hub (its own 5 s bound) and fails `unavailable`, naming the hub, when there is none; otherwise it builds the pane and profile registries' clients (`HubPaneStore`, `HubProfileStore`: each call is one control-socket request, 10 s bound, always to the hub of the local state directory), the Herdr adapter (built on first use), the tmux host, the OpenCode harness, the prober and a `StoreScope`, with no adapter I/O. `Unwired` stays, as the port set tests build from. E3 and E13 quote it. The decision "What the roster shows before #649 wires the real store" and every note that says the real binary answers `not-implemented` rested on a fact that is history now; they are corrected where they stand.
- **#832** (`652841b`, `bda1069`). A whole-registry `pane list` also reads Herdr and prints the panes no record names as `unmanaged`: a second table in text, `data.unmanaged` in JSON. `PaneStore::list` and the record are unchanged, and `rows_in_scope` still reads the registry through `ports.pane_store.list()` first (E8, refreshed). R1 settles whether the roster shows them.
- **#652** (`0e62b51`, 2026-10-11 18:13 MDT). `docs/pane-control.md` (the operator guide) and `crates/holler-cli/tests/docs_pane_control_test.rs` exist. Section 4 says the roster extension is "later" and a test pins that wording. R9 and the Blast radius list what #648 changes there.
- **#802, #808, #650, #664** (merged after `3c139dd`). `HarnessPort::serve` became `serve_command` plus `verify_serving`; `HerdrPort` gained `observe_pane` and `observe_panes`; `HerdrPane` gained `label`; `pane import` and `profile apply` are real verbs, so the stub table lost two rows. Only the test kit's call-log counts of E15 (acceptance 7, round 6) are touched by this.

**R1. The roster does not show unmanaged panes.** The roster is the hub's rows joined to the registry's records (scope item 1; acceptance 10). An unmanaged pane has no record and no hub row, so it has no PANE, PROFILE, HEALTH, SHOWN or DRIVEN to show, and showing it would need a Herdr call, which scope item 7 and acceptance 7 forbid (the roster makes no adapter call; "a roster is cheap and bounded", I5). `pane list` already shows them. ADR-0021 line 996 says the roster's "columns can read `holler_pane::unmanaged::observe`": that is an option #832 left open, not a requirement, and this decision declines it. Consequences: a Herdr pane labelled like a hub row's label but without a record joins to nothing (the join is to registry records only, acceptance 10); `--profile P` never lists an unmanaged pane (it belongs to no profile); acceptance 7's three empty call logs now also cover `FakeHerdr`'s `snapshot` and `observe_pane`, the two calls #832 added to `pane list`; and acceptance 7's comparison with `pane list` is with its registry rows (`data.panes`, the first table), not its `unmanaged` array.

**R2. The roster reads the registry through the real wiring, after the hub.** `roster_cmd::run` keeps the hub call it makes today, first and unchanged. Only after a successful reply does it call `Wiring::connect()` and then `ports.pane_store.list()` (and, under `--profile`, `ports.scope.resolve(P, None)`), and it makes no other port call. So a hub that is down gives today's exit 1 and today's one stderr line whatever the registry would answer (acceptance 11, round 5), and the decision "Exit codes" holds. A `connect` that fails after the hub answered (it makes its own `control/status` exchange) is a registry error: `note: pane registry unavailable (<code>)`, `<code>` the error's stable code (`unavailable`), exit 0, as acceptance 8 says for a failing `list`. The cost of a roster call grows by those two bounded exchanges (5 s and 10 s). `connect` builds the three adapters without I/O, so "the roster makes no adapter call" holds and acceptance 7's three empty call logs still hold. The tests inject `Ports` from the test kit and never call `connect`; `main.rs`'s `run_roster` is the one place that does.

**R3. With `--server` the roster does not read the registry.** `HubPaneStore` and `StoreScope` always talk to the hub of the local state directory (`transport::call(None, ..)`, E3), whereas `roster --server URL` reads the rows of another hub (`crates/holler-cli/src/roster_cmd.rs:39-49`, E1). Joining one hub's rows to another hub's registry would print a pane's figures against a session that is not the pane's, the very fault this story exists to make loud. So with `--server`: the hub call is unchanged; the registry is not read; the six new columns are `-` (JSON: `null`); there is no summary line and `data.mismatches` is `[]`; stderr has exactly one line, `note: pane registry not read (--server names another hub)`; exit 0. `--profile P` together with `--server` is a usage error, since it cannot be answered: `error: --profile cannot be combined with --server`, exit 2, nothing on stdout, in both formats, before any dial, by the same check that refuses `--json --format=text` today (`main.rs:160-184`, E6). Acceptance 13 tests it.

**R4. `--profile P` with a registry that cannot answer is an error, not an empty list.** The `-` columns and the `note:` line of acceptance 8 are for the columns. `--profile P` is a filter that needs the answer: if `ports.scope.resolve(P, None)` fails, the roster prints nothing else and returns that error through `output::emit_error`, with the exit code of its class (`profile-not-found` and the other refusals 3; `unavailable`, `timeout`, `store-corrupt`, `not-implemented` 1). Listing no row would pass as "P has no pane", which is false. This is the one exit-1 path the roster gets of its own, only under `--profile`; the decision "Exit codes" ("no exit-1 path of its own") is about the columns and stands for them. Acceptance 14 tests it.

**R5. HEALTH and the other cells are `pane list`'s, in text and in JSON.** The roster takes PANE, POS, PROFILE, HEALTH, SHOWN and DRIVEN from `crate::pane::list::PaneRow` (`PaneRow::from(&Pane)` and its `cells`, E3 and E17), which `list.rs`'s own module doc offers to "a later consumer (the roster, #648)", so a change in how `pane list` words a value (a new `Health`, a quoting rule for a stored string) reaches the roster with no roster edit. The brief left one thing open: the JSON form of `health`. It is `pane list`'s: `"healthy"`, `"unknown"` or `{"unhealthy": REASON}` (the module doc of `list.rs`, E3), where the text cell is `healthy`, `unhealthy` or `unknown` (`health_word`, `list.rs:331-337`). Acceptance 3's fixture pane with the `unhealthy` HEALTH value pins the object form.

**R6. DRIVEN is the registry's `last_observed.driven` whatever #839 decides.** The roster's six columns are the registry's values, read as `pane list` reads them, so they stay correct whatever #839 (the hub's DRIVEN session follows each pane's session of record; it absorbs #828) decides about the mechanism, as long as it records the session in `last_observed.driven` (its acceptance says it does). Until then no production writer gives `driven` a value (E12), so in production the DRIVEN column is `-`, no pane has both values, and the summary line never fires; that is accurate, not a defect, and the guide says so (R9). If #839 records DRIVEN elsewhere than `last_observed.driven`, this story's DRIVEN source is the one line to change. **Order:** #648 may merge before or after #839 with no code conflict (#839 touches the hub's `circuit/dispatch.rs`, `say_cmd.rs`, `tx_switch.rs`, reconcile and ADR-0021 sections 8 and 11; #648 touches none of those code files). The shared text is in the next section.

**R7. The issue comment (a roster row that kept the previous session's figures) is out of scope here; a follow-up is named.** The comment's figures are `idle 10h`, context `63k 31%`, last turn `end_turn 841m ago` and a work label. This roster has no idle, context or work-label column: its header is SESSION, HARNESS, MODE, STATE, CONN, HOSTNAME, PENDING, LAST TURN, HOLD (E1). Its `LAST TURN` is the hub row's `last_turn`, filled from what the body that advertises the session reported (E2), and the hub's row is keyed on the body's advertised session, which this story neither changes nor can see per pane: nothing joins a body's `harness_session_id` to a pane's `session_of_record` until #839 decides the address. So the roster cannot honestly fix the stale figures inside this story's blast radius, and the comment's three suggested tests are not this story's: (1) and (2) assert on columns the roster does not have and on the hub's `state`; (3) is met in part, because the loud line fires when a pane's SHOWN and DRIVEN differ (acceptance 2), but not for a row whose own session differs from the pane's. The follow-up to file, once #839 has decided the join, is: `feat(cli): the roster says when a row's figures come from a session other than the pane's session of record` (compare the row's `harness_session_id` with the joined pane's `session_of_record` and `driven`; add idle and context only if the hub's row carries them). It is not filed by this refresh.

**R8. How a test supplies the hub's rows (a gap in rounds 1 to 6, closed here).** No earlier round said how acceptance 1 to 10's fixture rosters reach `roster_cmd`, whose hub call is the free function `crate::transport::call` (E1). Decision: `roster_cmd` splits into the hub call and one `pub` pure function from the hub's reply document (`serde_json::Value`), the registry's answer (records, or the error to turn into `-` columns and a note, or "not read") and the flags, to the text, the JSON and the stderr note. `run(roster, choice, ports)` calls the hub, then that function. The golden, JSON, envelope, join, mismatch and profile tests (acceptance 1 to 10) call the function with a literal `{"rows": [...]}` and the test kit's fakes. The exit-code and wiring tests (acceptance 11 to 14) run the binary against a live hub (`support::Hub`, as `roster_cli_test.rs` does). `roster_cmd.rs` is 138 lines, so the merge fits under the 900-line lint with room; `roster_panes.rs` stays permitted only if clippy's `too_many_lines` forces a split.

**R9. Docs, ADR-0021 and CHANGELOG, the exact edits.** They are listed in the Blast radius, under "Added at the refresh". In one line each: `docs/pane-control.md` section 4 loses the sentence that says the roster extension is later and gains the roster paragraph and examples; section 12 loses its roster bullet; `docs_pane_control_test.rs` loses the guard "the guide documents `holler roster --profile`, which #648 has not built" and swaps `section_4_marks_the_roster_extension_later` for a test that section 4 names the roster's columns; ADR-0021 changes its two "Deferred to #648" lines; CHANGELOG gains one entry.

**R10. ADR-0003 line 94 loses the stale refusal sentence.** "Until #648, `roster --profile` is refused with exit 1 and `error: not implemented (story #648)`." is removed from `docs/adr/ADR-0003.md` line 94 in this story's change; nothing else in that file changes. Decided by the main session at the launch, 2026-10-11 (R3, R4 and R8 of the refresh are confirmed by the same decision: the operator can overrule any of them at the plan check).


## Scope

Extend `holler roster` with the pane's place in the registry, so a mismatch between what a pane **shows** and what the hub **drives** is loud.

1. **New columns**, appended after the existing ones, in this order: `PANE` (the pane's name, `-` for an agent with no pane: the io body, the alpha and beta agents), `POS` (the pane's grid position), `PROFILE` (the pane's profile, `-` for none), `HEALTH` (the pane's harness health: `healthy`, `unhealthy`, `unknown`, `-` when no pane), `SHOWN` (the session the pane's TUI shows, `-` when unknown or no pane) and `DRIVEN` (the session the hub drives, `-` when no pane).
   - *Noted at the brief gate (round 4; facts only, E18 quotes them; the decision "Where every new column comes from" below is unchanged):* a pane's name is a `SessionName`, which never contains a `/` (E10, E18). The hub names every row through `qualified_name`: `<label>/<session>` once the advertising token's label is bound, and the hub binds it when the body authenticates, before its first `session/presence`; a label minted by the hub is 1 to 32 characters. A row's name is the bare session name only when no label is bound, which the hub's own doc gives as a unit test or a presence that outraces the bind. So a row's `name` equals a pane's name only for a row with no label. No code today matches a row's `name` with a pane's name. What relates a pane record to a hub session today is its `session_of_record`, which the hub's prompt gate matches with the bare session name being prompted and to which `say --pane` sends.
   - *Clarified at the brief gate (round 5), DRIVEN of a pane whose record holds none (the column list above is unchanged):* DRIVEN is `-` in two cases, as SHOWN is: for an agent with no pane, and for a pane whose record's `last_observed.driven` is `None` (in production every record's is `None` until #839 builds the writer: #649 merged without one, E12). The cell is the one `pane list` prints for the same record (scope item 7, acceptance 7), `optional_text(self.driven.as_deref())` (`crates/holler-cli/src/pane/list.rs:276`), and `optional_text` turns `None` into `NO_VALUE`, the string `-` (`:371-374` and `:38-39`, each quoted in full in E17). Such a pane is never counted or named in the summary line, since the pair rule needs both values (scope item 3, E11), and its JSON `driven` is `null` (acceptance 3, round 5). Acceptance 1 (round 5) and acceptance 7 (round 3) pin its `-`.
   - *Clarified at the brief gate (round 5):* the note above compares a row's whole `name` with a pane's name. The join that decides which pane fills a row's six new columns compares the part of `Row.name` before its first `/` with the pane's name (acceptance 10, added with the round 4 decisions); acceptance 10 (round 5) states how that join and the decision "Where every new column comes from" read together.
2. **`POS` is rowcol**: text `r2c1`; JSON `{"row":2,"col":1,"pos":"r2c1"}` with `pos` equal to `r{row}c{col}`. A pane named `hj-c1r2` is a name, not a position: it sits at `r2c1` and is not renamed. The roster's help and legend say so.
   - *Clarified at the brief gate:* POS is read from the pane's registry record ("the pane's place in the registry" above), never from its name. The record holds where the pane sits in Herdr as `herdr.grid`, a `GridPos` (`crates/holler-pane/src/pane.rs:91-108`, `:240`), the field `pane list` (#643) prints (`crates/holler-cli/src/pane/list.rs:252`, `crates/holler-cli/src/pane/list.rs:271`). `GridPos` prints as `r{row}c{col}` and serializes as `{"row":R,"col":C,"pos":"rRcC"}`, row first (`crates/holler-pane/src/grid.rs:68-73`, `crates/holler-pane/src/grid.rs:129-138`). ADR-0021 section 10 (lines 848-849): "A pane's position comes from Herdr's snapshot, never from its name: `hj-c1r2` sits at `r2c1`." So the fixture's pane is named `hj-c1r2` and its record's `herdr.grid` is row 2, column 1. E10 quotes them.
   - *Clarified at the brief gate (round 3):* today's help does not say this, so the story adds it. The roster's help is the doc comment of `Command::Roster`, `/// List the roster. (hub-only, top-level daily verb)` (`crates/holler-cli/src/cli.rs:107`), and the doc comments of the `Roster` struct's fields (`cli.rs:464-484`, quoted in full in E4); none of them says that a pane's name is not its position. The roster prints no legend today: `render_table` writes a header line and the rows and nothing else (E1), and a search of `cli.rs` and `roster_cmd.rs` at `92fbc6e` finds no `legend`, `long_about` or `after_help` (the one `about` in `cli.rs`, at line 26, is the root command's). So the legend is new text as well.
3. **The mismatch summary line** (text output only): when at least one pane's `SHOWN` differs from its `DRIVEN` (both known and not equal), the first line of the output is `N pane(s) show a different session than the hub drives: <names>`, names in roster order, separated by `, `. When every pane matches, or no pane has both values, no such line is printed.
   - *Clarified at the brief gate:* "both known and not equal" means that a pane whose SHOWN is `-` is never counted or named, whatever its DRIVEN, and neither is an agent with no pane. That is the rule the hub's pane-state prompt gate and `say --pane` already apply to a pane's recorded pair: "one side absent is not a mismatch" (`crates/holler-cli/src/say_cmd.rs:341-342`, `:358-364`; the hub's twin, `crates/holler-hub/src/circuit/dispatch.rs:80-81`, `:95-101`). Acceptance 2 tests it; E11 quotes the rule.
   - *Clarified at the brief gate (round 3):* the line is the byte format of the decision "Summary line, byte format" below for every N, one included: only N and the names vary, and `pane(s)` and `show` are written literally, so one mismatched pane gives `1 pane(s) show a different session than the hub drives: <name>`. Acceptance 2 pins N = 1 as well as N = 2.
   - *Clarified at the brief gate (round 3):* "roster order" is the order in which the roster prints its rows, a name taking the place of the row it is printed on. The roster prints the rows of the hub's `control/roster` reply in the reply's order (`render_table`'s `for row in &rows`, `crates/holler-cli/src/roster_cmd.rs:75`, in E1), and the hub sorts them by row name (`crates/holler-hub/src/roster.rs:553`, E16). Acceptance 1 holds each of those lines byte-identical to today's up to its `HOLD` cell, so the pane store's name order (E8) is not roster order and does not reorder the rows.
4. **`--profile P`** lists only P's panes, through the `ProfileScope` trait (#637). A profile that does not exist is a **refusal** (`profile-not-found`, exit 3, one line, nothing else printed).
   - *Clarified at the brief gate:* with no pane named, `ProfileScope::resolve(P, None)` answers P and every pane of P, and a P that does not exist is `profile-not-found` (`crates/holler-pane/src/profile.rs:384-393`). The CLI's implementation, `StoreScope`, takes every pane whose record names P, in name order (`crates/holler-cli/src/pane/profile_scope.rs:84-90`, `:159-174`); the tests use the test kit's `FakeProfileScope` (`crates/holler-pane-testkit/src/profile_scope.rs:1-4`, `:190-200`). A verb holds the scope as `Ports.scope` (`crates/holler-pane/src/ports.rs:421`), and `pane list --profile` calls it that way (`crates/holler-cli/src/pane/list.rs:133-134`). `profile-not-found` is a refusal with exit 3, the same in text and JSON (`class_of`, `crates/holler-pane/src/error.rs:289-291`; `ErrorClass::exit_code`, `:235-246`). E6 and E8 quote them.
   - *Clarified at the brief gate (round 4), the order under `--profile`:* `--profile P` changes which rows are printed, never their order. The rows it prints come in the order of the hub's `control/roster` reply, as every roster's rows do (scope item 3, round 3), and not in the name order in which `ProfileScope::resolve` returns P's panes (`StoreScope`'s `members`, E8). The names of the summary line and of `data.mismatches` (scope item 5) follow the same order.
5. **`--format=text|json`** (default text). `--format=json` prints the shared envelope (`output::emit()`, #670) with the same new fields in `data`; in JSON the mismatch is `data.mismatches` (an array of pane names) and there is no summary line.
   - *Clarified at the brief gate:* `--format=json` is told from a bare `--json` by `FormatChoice.json_explicit`, which only `--format=json` sets (`crates/holler-cli/src/output.rs:49-82`); `main.rs` already reserves the bit for this story (`crates/holler-cli/src/main.rs:216-219`), and today `dispatch` hands `roster` only the resolved `json` (`:221`, `:229`). `output::emit` writes one envelope (`schema_version` 1, `ok`, `data`, `error`) in JSON mode and the text otherwise, and in both returns 0 or the exit code of the error's class (`output.rs:150-171`, `:201-217`, `:336-340`; `SCHEMA_VERSION`, `:33-34`). A usage error that clap reports under `roster` is clap's own text on stderr with exit 2 in every format, since only `pane` and `profile` get usage envelopes (`output.rs:36-38`; `main.rs:115-120`), and `--json --format=text` is `error: ...` with exit 2 for `roster` (`main.rs:160-184`). `check_envelope(stdout, exit_code)` is in the test kit, which `holler-cli` already has as a dev-dependency. E5, E6 and E7 quote them.
   - *Clarified at the brief gate (round 3; E14 quotes the lines not quoted elsewhere):* with this story `roster` prints the envelope, and only for an explicit `--format=json`, the case #670 reserved for it: "`roster` prints the envelope only for this, so `roster --json` keeps its legacy shape" (`FormatChoice.json_explicit`, `output.rs:54-57`, in E5). Under `--format=json` the roster's success (`ok` true, `data` as the decision "`--format=json` `data` shape" below says) and its `profile-not-found` refusal (`ok` false, `error.code` `profile-not-found`, exit 3) are each one envelope on stdout, written by `output::emit` (`output.rs:207-217`, in E6) and `output::emit_error` (`:240-243`). Both are `pub` and take a `Sink`; `emit` takes any `Serialize` data and `emit_error` an `ErrorBody`, and neither names a verb, so the roster calls them as the `pane` and `profile` verbs do, with no change to `output.rs` (not in the blast radius). In text mode the roster prints its table, and a bare `--json` prints no envelope (scope item 6). `ENVELOPE_NAMESPACES` stays `["pane", "profile"]` (`output.rs:36-38`): its one reader is `RawArgs::envelope_namespace` (`:362-369`), which only `usage_envelope` calls, for a command line clap could not parse (`main.rs:140-158`), so it decides usage errors and nothing else. A usage error under `roster` is therefore what it is today in every format, `--format=json` included: clap's own message on stderr with exit 2 (`--format=xml` is one, since `--format` is a `value_enum`, `cli.rs:80-81` in E5), or `error: --json cannot be combined with --format=text` on stderr with exit 2 (`main.rs:160-184`, in E6), and nothing on stdout. Acceptance 4 says what its test checks for each class.
   - *Clarified at the brief gate (round 4), the one shape of `data` (the decision "`--format=json` `data` shape" below is unchanged; this spells out what it gives):* on success, `data` is one object with exactly two keys, `rows` and `mismatches`: `{"rows": [ROW, ...], "mismatches": [NAME, ...]}`. `data.rows` is the array that `roster --json` prints as its own top-level `rows` for the same hub reply, pane registry and flags: the same rows in the same order, each with today's keys (E2) and the six new keys of scope item 6. `data.mismatches` is an array of pane names: the names the text summary line gives for the same input (scope item 3's rule), in roster order. The decision makes `mismatches` a key of the object, so it is in every success envelope, and it is `[]` when no pane is a mismatch, which is when the text output has no summary line. `data` has no other key: `rows` is not moved out of `data` or nested deeper, and `roster --json` stays `{"rows": [...]}`, with no `mismatches` (scope item 6). So with two mismatched panes `a` and `b`, in that roster order, the text output's first line is `2 pane(s) show a different session than the hub drives: a, b`, and the `data` of `--format=json` is `{"rows": [...], "mismatches": ["a", "b"]}`. This object is "the roster payload" of acceptance 4.
6. **`roster --json` keeps its shape and gains the new fields additively** (`pane`, `pos`, `profile`, `health`, `shown`, `driven` as extra keys). ADR 0003 lines 105-106: a changed `--json` shape is breaking, but "Additive = MINOR", so there is **no ADR 0003 amendment and no deprecation notice**. Every existing key and its value is unchanged. (Operator decision, 2026-10-11.)
   - *Evidence added at the brief gate (the operator decision above is unchanged):* ADR 0003 lines 105-106 are quoted verbatim in E9. Today `roster --json` prints the hub's `control/roster` reply as it arrives, `{"rows": [ROW, ...]}` (`crates/holler-cli/src/roster_cmd.rs:44-45`; the hub builds it at `crates/holler-hub/src/control_server.rs:465-474`), each ROW being the serde form of the hub's `Row` with its `hold` flattened (`crates/holler-hub/src/roster.rs:131-180`). Those are the keys acceptance 3 keeps. E1 and E2 quote them.
7. Tests use only #638's fakes and envelope helper; no new dependency. `SHOWN` and `DRIVEN` are the pane record's `last_observed.shown` and `last_observed.driven`, the same values `pane list` prints (see "Decisions on the review's blocking points"); the roster makes no adapter call.
   - *Clarified at the brief gate (facts only; E2 and E3 quote them):* today a roster call reads one thing, the hub's reply to `control/roster` (`crates/holler-cli/src/roster_cmd.rs:39-49`), which the hub builds on each call from its in-process roster, filling `hold` from hub state at read time (`crates/holler-hub/src/control_server.rs:461-475`); the hub's `Row` has no pane field (`crates/holler-hub/src/roster.rs:131-180`). A pane's sessions are in three places. (a) The port `HarnessPort::shown_session`, "The session the TUI of `pane` shows, if it can tell" (`crates/holler-pane/src/ports.rs:341-342`), which the OpenCode adapter answers with `None` whenever it cannot tell (ADR-0021 lines 211-212); only the CLI process can make that call, because the hub runs no adapter (ADR-0021 line 892; `crates/holler-hub/src/panes/mod.rs:8-10`). (b) The record's `last_observed { shown, driven, at }`, "What reconcile last observed" (`crates/holler-pane/src/pane.rs:188-199`): `pane list` prints these two as SHOWN and DRIVEN (`crates/holler-cli/src/pane/list.rs:16-18`, `:256-257`), and nothing writes `driven` yet (ADR-0021 lines 885-888: the cutover wires it; #839 builds it). (c) The record's `session_of_record`, which I2 makes "the only session a pane's TUI shows and the hub drives" (ADR-0021 line 306; `pane.rs:243-245`); how the hub's own DRIVEN session is pointed at it is deferred (ADR-0021 lines 883-884, "Deferred to #654 (cutover)"; #839 builds it). Of the three, only (a) is an observation made at the moment of the call. In the CLI, since #649 merged (refresh, 2026-10-11), `Wiring::connect` builds the real ports, the pane store among them: `HubPaneStore`, which asks the hub's `pane/list` over the local control socket (`crates/holler-cli/src/pane/wiring.rs:1-27`, `crates/holler-cli/src/pane/wiring/store.rs:125-132`, both in E3); `Unwired`, whose every port answers `not-implemented`, stays as the test-only port set (`wiring.rs:58`). The issue does not say which of these is "the hub's current drive state" for DRIVEN, nor what the roster prints when a port call for a pane fails rather than answering `None`.

## Acceptance (each is a test; "golden" means a committed expected-output file)

*Cross-reference added at the brief gate (the numbering is the issue's):* scope item 1 is tested by acceptance 1 and 6, item 2 by 5, item 3 by 2 and 7, item 4 by 6, item 5 by 4, item 6 by 3, and item 7 by 7.

1. **Old columns unchanged.** A golden test over a fixture roster (including an agent with no pane) shows that every pre-existing column of every row is byte-identical to today's output; the only differences are the new trailing columns, which show `-` for an agent with no pane.
   - *Clarified at the brief gate:* the fixture also has a pane with a profile and a pane without one, and one pane for each HEALTH value of scope item 1 (`healthy`, `unhealthy`, `unknown`), so the golden file pins every PROFILE and HEALTH value as well as the `-` cells of the agent with no pane. None of its panes shows a session other than the one driven, so no summary line comes before the table.
   - *Clarified at the brief gate:* today's table has nine columns, `SESSION` to `HOLD` (E1). Each but `HOLD` is left-aligned to a fixed width and followed by one space; `HOLD` is the last and is not padded, so each line ends right after its `HOLD` cell (`crates/holler-cli/src/roster_cmd.rs:71-74`, `:81-92`). The new columns therefore follow the `HOLD` cell, and "byte-identical" covers each line from its start through the end of that cell. An empty roster prints `(no sessions on the hub)` and no header (`:67-69`).
   - *Clarified at the brief gate (round 5):* one of the fixture's panes has a record whose `last_observed.shown` is set and whose `last_observed.driven` is `None`, so the golden file also pins the DRIVEN `-` of a pane that has a record (scope item 1, round 5). That pane is not a mismatch, since the pair rule needs both values (scope item 3), so the output still has no summary line.
2. **Text mismatch.** A fixture with two panes whose SHOWN differs from DRIVEN prints the summary line first with both names in roster order (golden); a fixture with none prints no such line (golden).
   - *Clarified at the brief gate:* both fixtures also hold a pane whose SHOWN is `-` and whose DRIVEN is known. It is never named, and N counts only the panes with both values known and different (scope item 3), so in the fixture with none the line stays absent although that pane's two cells differ.
   - *Clarified at the brief gate (round 3):* a third golden fixture has exactly one pane whose SHOWN and DRIVEN are both known and differ. Its output starts with the line `1 pane(s) show a different session than the hub drives: <name>`, `<name>` being that pane's name, and then the table (scope item 3: `pane(s)` is written literally for every N).
3. **`--json` additive.** The `--json` output of the fixture parses; every key present in today's output is present with an equal value; the new keys are present.
   - *Clarified at the brief gate (round 5), the values of the new keys:* every row of `roster --json`, and so every row of `data.rows` under `--format=json` (scope item 5, round 4), has all six new keys. A row joined to no pane has `null` in each of them, `pos` included (acceptance 10): never a missing key, and never the text cell `-`. That is every row while the pane store's `list` fails (acceptance 8), since no row is then joined to a pane. A row joined to a pane holds that pane's record: `pane` its name; `pos` its `herdr.grid` in `GridPos`'s serde form, `{"row":R,"col":C,"pos":"rRcC"}` (scope item 2), which every record has, since a record's `herdr` is a `HerdrPane` and its `grid` a `GridPos`, neither an `Option` (`crates/holler-pane/src/pane.rs:240`, `:100`, E10); `profile` its profile; `health` its recorded `harness.health`; `shown` and `driven` its `last_observed.shown` and `last_observed.driven`. A value the record does not hold (`profile`, `shown` or `driven` `None`) is `null`, as in `pane list`'s JSON: "An empty value is `null`, never left out" (`PaneRow`, `crates/holler-cli/src/pane/list.rs:224-226`, E17). Besides the keys' presence, the test of this criterion checks the six `null`s of the agent with no pane, and for each pane of acceptance 1's fixture the values of `pane`, `pos`, `profile`, `shown` and `driven`.
   - *Added at the refresh (R5):* `health` in JSON is `pane list`'s form: `"healthy"`, `"unknown"` or `{"unhealthy": REASON}`, and the text cell is `healthy`, `unhealthy` or `unknown`. The test checks the fixture's `unhealthy` pane against the object form in JSON and against the word in text.
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
   - *Clarified at the brief gate (round 6), how the test changes a record between the two roster calls:* through the test kit's own `FakePaneStore`, as scope item 7 requires. The test seeds one store with the fixture's records (`FakePaneStore::seeded`), runs the roster, rewrites one record's `last_observed` with `FakePaneStore::concurrent_put`, and runs the roster again over the same store. `concurrent_put` is another writer's put: it stores the record unconditionally, at the stored generation + 1, and bypasses the fault switch and the call log. It takes `&self`, so the test can call it while the roster's `Ports` borrows the store. The fake's `list` reads the records the store holds when it is called, so the second call sees the rewrite. E20 quotes the three (`crates/holler-pane-testkit/src/pane_store.rs:90-99`, `:113-118`, `:182-185`) and a verb test that already sets a record's `last_observed.driven` this way before it runs the verb (`crates/holler-cli/tests/pane_verbs/relaunch.rs:474-487`).
   - *Clarified at the brief gate (round 6), what the three call logs record (the round 3 note above is unchanged):* `FakeHarness`, `FakeHerdr` and `FakeHost` each call their fault switch's `enter` first in every port method they implement, and `enter` records the call before it answers any fault, so `faults().calls()` lists every call of those methods, failed ones included. They implement eight of the nine methods of `HerdrPort` (refresh: #832 added `observe_pane`, which `FakeHerdr` implements, and `observe_panes`, a default that calls `observe_pane` for each pane and so reaches the log through it), all four of `HostPort` and nine of the eleven of `HarnessPort` (refresh: #802 replaced `serve` by `serve_command` and added `verify_serving`). The other two, `send_prompt` and `session_activity`, have default bodies in the trait, which `FakeHarness` keeps (no file of the test kit defines either): `send_prompt`'s answers the refusal `prompt-unsupported`, `session_activity`'s answers `Ok(Activity::Idle)`, and neither reaches a fault switch. So the three empty logs show that the roster called none of the other twenty-one methods, and show nothing about those two. How the test also shows that the roster calls neither is not decided here: a `HarnessPort` the test writes itself would not be one of #638's fakes (scope item 7). E15 (round 6) quotes the ops, the accessors, `enter` and the two defaults.
   - *Added at the refresh (R1, R2):* the three empty call logs also show that the roster never calls `FakeHerdr`'s `snapshot` or `observe_pane`, the two calls #832 added to `pane list`, so a roster that borrowed `pane list`'s unmanaged reading fails this test. The comparison with `pane list` is with its registry rows (`data.panes`, the first table), never with its `unmanaged` array.
8. **Pane registry unavailable.** With a `PaneStore` whose `list` answers `not-implemented` (`Unwired` or a fake; the real binary until #649, which has merged, see the refresh section), the roster prints the hub's rows exactly as today with `-` in every new column, no summary line, nothing on stderr, exit 0, in text and in `--json`; with a store whose `list` answers `unavailable` it prints the same stdout and exactly one stderr line `note: pane registry unavailable (unavailable)`, exit 0. (`(code)` is the `PaneError`'s stable code string; a `timeout` gives `(timeout)`.)
   - *Clarified at the brief gate (round 5), every other store error (this criterion and the decision "What the roster shows before #649 wires the real store" are unchanged):* `PaneStore::list` has no closed set of errors. Its doc is "Every pane.", and the trait's says only that every method returns within I5's bound or with `PaneError::Timeout`, and that a stale write is `generation-conflict`, which `list`, a read, does not make (E19). The decision sorts a failed `list` into two kinds: `not-implemented`, and "a second, different store error", which it illustrates with `timeout` and `unavailable`. Read as written, the second kind is every store error other than `not-implemented`, and the decision "Exit codes" agrees: it gives the roster no exit-1 path of its own, and a store error that ended the roster would be one, since `timeout`, `unavailable` and `store-corrupt` are each `ErrorClass::Failure`, exit 1 (`class_of`, E6). So every code `list` answers other than `not-implemented` (`timeout`, `unavailable`, `store-corrupt` or any other) gives what this criterion gives for `unavailable`, with its own code in the note: the hub's rows with `-` in each of the six new columns and `null` in each new JSON key (acceptance 10), no summary line and `data.mismatches` `[]` (no row is joined to a pane), exactly one stderr line `note: pane registry unavailable (<code>)`, `<code>` being that error's stable code (E19), and exit 0. One more test pins it, besides the decision's two (acceptance 7, round 3): test (ii) run again with `list` answering `timeout` and then `store-corrupt`, set through the same fault switch (E15), whose stderr lines are `note: pane registry unavailable (timeout)` and `note: pane registry unavailable (store-corrupt)`; each exits 0 in text mode, and under `--json` and `--format=json` as well.
   - *Added at the refresh (R2):* a `Wiring::connect` that fails after the hub has answered (it makes its own `control/status` exchange) is a registry error like the ones above: the `-` columns, `null` keys, one stderr line `note: pane registry unavailable (unavailable)` and exit 0. The binary's real path, with the real clients, is acceptance 12.
9. **One mismatch, and the summary line is unconditional `pane(s)`.** A fixture with exactly one mismatched pane prints `1 pane(s) show a different session than the hub drives: <name>` as its first line (golden); the line never uses another plural form.
10. **Join and `--profile` over hub rows.** A hub row is joined to a pane record by its label: the part of `Row.name` before the first `/` equals the pane's name; every row of that label carries the pane's columns, and the pane is named once in the summary. A row with no matching pane keeps `-` in the new columns (text) and `null` in the new JSON keys. With `--profile P`, only rows joined to a pane of P are listed; rows with no pane record are dropped (a test with both).
   - *Clarified at the brief gate (round 5), the join key (this criterion and the decision "Where every new column comes from" are both unchanged; this states how they read together):* the join compares two names and nothing else: a pane's name, and the part of `Row.name` before its first `/`. That part is the label of a row named `<label>/<session>` (the label grammar has no `/`, E18) and the whole name of a row with no label (`qualified_name`, E18). This criterion, added with the round 4 decisions, states the join key, so the agent's name that the decision matches with the pane's name is that part of the row's name, not the whole `Row.name`. A labelled row is therefore joined to the pane named like its label, never to a pane named like the session after its `/`; a row with no label is joined to the pane named like its whole name; and a pane's `session_of_record` plays no part in the join. The round 4 note under scope item 1 says that a row's whole `name` equals a pane's name only for a row with no label: that is a fact about the whole name, not the join, and both stand. For example, with the hub rows `alpha`, `hj-c1r2/build` and `hj-c1r2/review` and one pane record, `hj-c1r2`: the rows `hj-c1r2/build` and `hj-c1r2/review` both carry that pane's six cells, `alpha` carries `-` in each (and `null` in each new JSON key), and when the pane's SHOWN and DRIVEN are both known and differ, the text output's first line is `1 pane(s) show a different session than the hub drives: hj-c1r2` and `data.mismatches` is `["hj-c1r2"]`. The pane is counted and named once, however many rows carry it.
   - *Added at the brief gate (round 6), a test of the join for a row with no label (this criterion and the decision "Where every new column comes from" are unchanged):* the example above joins two labelled rows to a pane and leaves the one row with no label, `alpha`, with none, so no test yet fails a join that never joins a row with no label. The test of this criterion therefore also has a fixture with a hub row with no label, `hj-c3r1`, and a pane record of the same name, `hj-c3r1`: that row carries the pane's six cells in text and its values in the six new JSON keys (acceptance 3). For a name with no `/`, the part before its first `/` is the whole name, so this is the case the round 5 note above states as "a row with no label is joined to the pane named like its whole name".
11. **Exit codes, one test per class and format.** 0 for a success fixture; 1 for an unreachable hub or a control error, exactly as today (text: the same stderr line as today; `--format=json`: the failure envelope); 2 for a usage error such as `--format=xml` (clap's own text and exit 2 in both formats: usage errors of `roster` are not enveloped, unchanged); 3 for an unknown `--profile` (text: the one-line `error:` form on stderr; `--format=json`: the failure envelope with code `profile-not-found`).
   - *Clarified at the brief gate (round 5), a failed hub call beside the pane store (this criterion is unchanged):* when the hub call fails (no live hub, or another control error), the result is this criterion's exit 1 "exactly as today", whatever the pane store answers: in text, today's one stderr line and nothing else, so no `note:` line of acceptance 8 is added; under `--format=json`, the failure envelope. No store error can change that exit code, since none is a roster failure (acceptance 8, round 5). The order in which the roster reads the hub and the pane store is the implementation's, provided this holds.
   - *Added at the refresh (R3, R4):* exit 2 is also the `--profile` with `--server` usage error (acceptance 13), and exit 1 is also what `--profile P` gives when the scope cannot be resolved because the registry is unavailable (acceptance 14). Neither changes the exit codes the criterion lists for the other cases.
   - *Added at the brief gate (round 6), the test of the note above (the order stays the implementation's):* with no live hub, the roster runs twice in text and twice under `--format=json`: once over a `FakePaneStore` seeded with acceptance 1's records, and once over one whose `list` fails with `unavailable`, the store error that adds acceptance 8's `note:` line. All four runs exit 1, and in each format the two runs print the same stdout and the same stderr: in text, an empty stdout and today's one stderr line, with no `note:` line; under `--format=json`, the failure envelope on stdout. Whichever the roster reads first, a result that depends on the store fails this test.
12. **The real wiring, end to end (added at the refresh, R2).** In `crates/holler-cli/tests/roster_cli_test.rs` (a real hub and real bodies through `support::Hub`), over a live hub whose pane registry is empty: `holler roster` exits 0; its header line is today's header, through `HOLD`, followed by `PANE`, `POS`, `PROFILE`, `HEALTH`, `SHOWN` and `DRIVEN`; every row has `-` in the six new cells; there is no summary line and stderr has no `note:` line. `holler roster --json` keeps every existing key of every row and has the six new keys `null`. `holler roster --format=json` passes `check_envelope` with `data.mismatches` `[]`. `holler roster --profile nosuch` exits 3 with the `profile-not-found` one-line `error:` form on stderr (text) and the failure envelope (`--format=json`). This pins that the binary's roster goes through `Wiring::connect` and the real registry clients, which the in-process tests with fakes cannot show.
13. **`--server` (added at the refresh, R3).** (a) With no hub running, `holler roster --profile demo --server ws://127.0.0.1:1` exits 2, its stderr has the line `error: --profile cannot be combined with --server` and its stdout is empty, in text and under `--format=json`. (b) Over the remote-admin rig (`crates/holler-cli/tests/support/remote_admin_rig.rs`, which `remote_admin_liveness_test.rs:241` already uses for `roster --server ... --json`), `holler roster --server URL` exits 0, the six new cells are `-`, stderr is exactly the one line `note: pane registry not read (--server names another hub)`, and `--json` has the six keys `null`. Test (b) goes in `roster_cli_test.rs` if the rig links there, else in `remote_admin_liveness_test.rs`.
14. **`--profile` with a registry that cannot answer (added at the refresh, R4).** In-process, over a `FakeProfileScope` built on a `FakeProfileStore` that holds the profile and a `FakePaneStore` whose standing fault is `Fault::Fail(unavailable)` (E15; the scope has no fault switch of its own, its stores do): `roster --profile P` prints nothing on stdout in text and its stderr is the error's one `error:` line, exit 1; under `--format=json` stdout is the failure envelope with code `unavailable` (it passes `check_envelope`), exit 1. With the same fault and no `--profile`, the roster is acceptance 8's: columns `-`, the note, exit 0.

## Blast radius

- `crates/holler-cli/src/roster_cmd.rs` (`crates/holler-cli/src/roster.rs` does not exist and is not created; if the pane-side merge does not fit `roster_cmd.rs` under the 900-line lint, put it in one new sibling file `crates/holler-cli/src/roster_panes.rs`)
- `crates/holler-cli/src/main.rs`: pass #670's "explicit `--format=json`" bit to `roster_cmd::run` and remove #670's `roster --profile P` refusal ("not implemented (story #648)") from the extracted roster helper; if that needs more than the roster helper's lines, say so in the brief and keep the edit inside that helper
  - *Clarified at the brief gate (as this bullet asks; E4 quotes each line):* the refusal is `crates/holler-cli/src/main.rs:275-282`, inside `run_roster` (`:272-285`); it prints `error: not implemented (story #648)` on stderr and exits 1. Removing it and passing the bit needs more than the helper's lines. `ROSTER_STORY` (`main.rs:43-44`) has no use but the refusal (`main.rs:277`), so it becomes dead code, which `dead_code = "deny"` (`Cargo.toml:32-33`) makes a build error. The bit is in `choice`, which `dispatch` holds, and `dispatch` hands `run_roster` only `json` (`main.rs:221`, `:229`), so its `Command::Roster` arm changes too. Outside `main.rs`, the existing test `roster_profile_is_refused_with_the_roster_story` (`crates/holler-cli/tests/pane_verbs/process/legacy_verbs.rs:95-105`, its constant at `crates/holler-cli/tests/pane_verbs/process/stub.rs:33-36`) asserts that refusal with no format flag, with `--json` and with `--format=json`, so it no longer passes once the refusal is gone; it is not one of this story's new tests. The comment on `Roster`'s `--profile` field (`crates/holler-cli/src/cli.rs:480-481`) also describes the refusal.
- `crates/holler-hub/src/roster.rs`: **no change** (the hub's `Row` is untouched; every new column comes from the pane registry, see "Decisions on the review's blocking points")
- the new tests under `crates/holler-cli/tests/` and their golden files; `docs/` and `README.md` only where the roster's documented output appears; `CHANGELOG.md`
- *Added at the brief gate (round 2; each file follows from a change listed above, and E4 quotes the lines):*
  - `crates/holler-cli/tests/pane_verbs/process/legacy_verbs.rs` and `crates/holler-cli/tests/pane_verbs/process/stub.rs`: the story deletes the test `roster_profile_is_refused_with_the_roster_story` (`legacy_verbs.rs:95-105`), the import of its constant (`legacy_verbs.rs:13`, `use crate::stub::ROSTER_PROFILE_REFUSAL as ROSTER_REFUSAL;`) and that constant (`stub.rs:33-36`, documented as "The refusal of `roster --profile`, until story #648."). Both pin the refusal this story removes, and the test is the constant's only use. Acceptance 6 tests what `roster --profile` does instead. A search of the tree at `92fbc6e` for `ROSTER_PROFILE_REFUSAL` and `story #648` finds no other test that pins the refusal.
  - `crates/holler-cli/src/cli.rs`: the comment on `Roster`'s `--profile` field (`:480-481`), which says `main.rs` refuses the flag, and the roster's help, which scope item 2 requires to say that a pane's name is not its position. That help is the doc comment of `Command::Roster` (`:107`, `/// List the roster. (hub-only, top-level daily verb)`) and those of the `Roster` struct's fields (`:464-484`, in E4).
  - ADR 0003 line 94 also states the refusal, in words that end it at this story: "Until #648, `roster --profile` is refused with exit 1 and `error: not implemented (story #648)`." **Decided at the launch (R10, superseding the sentence this replaces):** the sentence becomes untrue when this story merges, so `docs/adr/ADR-0003.md` line 94 loses it (a one-sentence factual edit, no decision changes; scope item 6's "no ADR 0003 amendment" concerns the `--json` shape and stands) and `docs/adr/ADR-0003.md` is added to the Blast radius.
- *Added at the refresh (R1 to R9; each file follows from a change listed there; sizes are today's, and the 900-line lint is for `.rs` files):*
  - `crates/holler-cli/src/roster_cmd.rs` (138 lines) also exports `pub const PANE_COLUMNS: [&str; 6]` (`PANE`, `POS`, `PROFILE`, `HEALTH`, `SHOWN`, `DRIVEN`), the one table the docs test reads, as `pane::list::COLUMNS` is for `section_4_names_every_column`. `main.rs` (346 lines), `cli.rs` (780 lines) and the other files above are unchanged in size by this refresh and stay well under 900 after the story.
  - `crates/holler-cli/Cargo.toml` (534 lines): one `[[test]]` entry, `roster_panes_test` with `path = "tests/roster_panes_test.rs"`, next to `roster_cli_test` (line 203), because the manifest sets `autotests = false` (line 14). A shared hot spot (see "Clashes with stories in flight").
  - `crates/holler-cli/tests/roster_panes_test.rs` (new) and its golden files under `crates/holler-cli/tests/fixtures/roster_panes/` (new): acceptance 1 to 10 and 14, in-process (R8). `crates/holler-cli/tests/roster_cli_test.rs` (253 lines): acceptance 11's process cases, 12 and 13 (b); or `crates/holler-cli/tests/remote_admin_liveness_test.rs` for 13 (b).
  - `docs/pane-control.md` (794 lines) and `crates/holler-cli/tests/docs_pane_control_test.rs` (523 lines), the exact edits (R9):
    - Guide section 4, line 432, the sentence "The roster extension (the PANE, POS, PROFILE, HEALTH, SHOWN and DRIVEN columns on `holler roster`, #648) is later: ...": replace it by the paragraph below. Section 12, line 787, the bullet "the roster extension: the pane columns on `holler roster` (#648);": delete it. Nothing else of the guide changes: the DRIVEN row and the "SHOWN against DRIVEN" paragraph already say DRIVEN is `-` until #839, and section 10's "SYNC is read from `pane list` and `doctor`, not from the roster" stays true, since the roster has no SYNC column.
    - Test `every_verb_is_named_and_proposed_lines_say_so` (lines 155-184): delete the guard `if doc.contains("holler roster --profile") { ... }` (lines 181-183) and the words "and no `holler roster --profile`" in its doc comment (line 156). It pinned the absence of the flag this story builds, so it fails the moment the guide documents it.
    - Test `section_4_marks_the_roster_extension_later` (lines 295-303; acceptance 7 of #652): replace it by `section_4_documents_the_roster_columns`. Section 4 names each of `holler_cli::roster_cmd::PANE_COLUMNS` as a whole word and has a line that names `holler roster` and `--profile`; no line of the guide names `holler roster`, `#648` and `later` together; no line of section 12 names `#648`. #652's own text anticipates this: its section 12 is the list of what is later, and its roster bullet exists to be removed when #648 lands.
    - `docs_cli_test` parses every `holler ...` command the guide shows, so the examples in the paragraph (`holler roster --profile fleet`, `holler roster --format=json`) must parse; they do.
    - The paragraph for section 4:

      > **The roster.** `holler roster` appends six columns after HOLD: PANE (the pane's name), POS (`r2c1`, row first; a pane named `hj-c1r2` is a name, and it sits at `r2c1`), PROFILE, HEALTH, SHOWN and DRIVEN. They are the registry's values, the same cells `pane list` prints for the same record, and an empty cell is `-`; a session with no pane (the io body, an alpha or beta agent) shows `-` in all six. A roster row is joined to a pane when the part of its name before the first `/` is the pane's name. The roster calls no adapter and shows no unmanaged pane (`pane list` does), and it reads the registry of the local hub, so with `--server` the six columns are `-`. When at least one pane's SHOWN and DRIVEN are both known and differ, the first line of the output says so: `2 pane(s) show a different session than the hub drives: demo-alpha, demo-beta`. `holler roster --profile fleet` lists only that profile's panes, and a profile that does not exist is refused `profile-not-found` (exit 3). `holler roster --format=json` prints the envelope of section 9 with `data` = `{"rows": [...], "mismatches": ["demo-alpha", "demo-beta"]}`; `holler roster --json` keeps its shape and gains the keys `pane`, `pos`, `profile`, `health`, `shown` and `driven` (`null` for a row with no pane). DRIVEN is `-` for every pane until the hub's DRIVEN session is recorded ([#839](https://github.com/Performant-Labs/holler/issues/839)), so the summary line does not appear before then; read SYNC from `holler pane list` and `holler pane doctor`.
  - `docs/adr/ADR-0021.md`: two hunks, both of which resolve a "Deferred to #648": line 825-826 (section 9, "**Deferred to #648:** how `roster`'s existing `--json` output and the envelope coexist (#670's format resolver reports whether `--format=json` was given explicitly).") becomes "**Decided (#648):** `roster --json` keeps its shape and gains the six pane keys additively (ADR 0003: additive is MINOR); `roster --format=json`, and only it, prints the envelope, with `data` = `{"rows": [...], "mismatches": [...]}`.", and line 1039 ("The roster's `--json` shape beside the envelope: #648.", in "Deferred to named stories") is deleted. Line 996 ("The roster (#648) is not changed here ...") is #832's history and stays.
  - `CHANGELOG.md` (982 lines): one entry under `## [Unreleased]`, `### Enhancements`: "The roster shows the pane (epic [#633](https://github.com/Performant-Labs/holler/issues/633), [#648](https://github.com/Performant-Labs/holler/issues/648)): `holler roster` appends the columns PANE, POS, PROFILE, HEALTH, SHOWN and DRIVEN after HOLD, read from the pane registry as `holler pane list` prints them (`-` for a session with no pane), and prints `N pane(s) show a different session than the hub drives: <names>` as its first line when a pane's SHOWN and DRIVEN are both known and differ. `roster --profile NAME` lists only that profile's panes (it was refused with `not implemented (story #648)`; an unknown profile is refused `profile-not-found`, exit 3), `roster --format=json` prints the JSON envelope with `data.rows` and `data.mismatches`, and `roster --json` keeps its shape and gains the keys `pane`, `pos`, `profile`, `health`, `shown` and `driven` (additive, no ADR 0003 change). The roster makes no call to Herdr, tmux or OpenCode and shows no unmanaged pane; with `--server` the new columns are `-`, since the registry is the local hub's. DRIVEN is `-` until the hub's DRIVEN session is recorded (#839)."
  - Not changed, checked: `README.md` (lines 99, 107, 138, 171, 370 and 481 name the command and show no roster output), `docs/protocol/v2.md` (line 535 describes hub fields; the six keys are the CLI's, not wire fields), `crates/holler-cli/tests/fixtures/cli-surface.txt` (`roster | --profile demo` is already a line and no flag is added), ADR 0003 line 94 (scope item 6 records no ADR 0003 amendment; the sentence is dated by "Until #648").

## Evidence the brief must quote

The reviewers cannot see the repo. The brief quotes, with file and line: #670's roster helper and the refusal in `crates/holler-cli/src/main.rs`; the format resolver's "explicit json" bit in `crates/holler-cli/src/output.rs`; `ProfileScope` in `crates/holler-pane/src/profile.rs`; `check_envelope` from #681 in `holler-pane-testkit`; the existing `render_table` header in `roster_cmd.rs`; and the hub's roster row type in `crates/holler-hub/src/roster.rs`.

## Decisions on the review's blocking points (final, 2026-10-11, taken under the operator's standing rules; they override any earlier wording above, including the "current drive state" and "live observation" phrases)

- **Where every new column comes from: the pane registry, read through `Ports.pane_store.list()`.** One source of truth, the one `pane list` already uses (E-list in the evidence). `PANE` = the record's name, matched to an agent row by the agent's name equalling the pane's name; `POS` = the record's `herdr.grid`, printed rowcol; `PROFILE` = the record's `profile`; `HEALTH` = the record's recorded `harness.health`; `SHOWN` = `last_observed.shown`; `DRIVEN` = `last_observed.driven`. **The roster makes no live adapter call**: a roster is cheap and bounded (I5), and "observe, don't infer" (I6) is reconcile's job, which writes `last_observed`. The hub's `Row` and `holler-hub` are not touched.
- **What the roster shows before #649 wires the real store:** *(Refresh, 2026-10-11: #649 has merged, so the first sentence's premise is history. This is what the decision said, with its facts corrected.)* when the pane store's `list` answers `not-implemented` (it did in the real binary, because `Wiring::connect` handed out `Unwired`; since #649 merged the binary's store is the real `HubPaneStore`, and `Unwired` and the fakes are what still answer it), the roster still prints the hub's rows exactly as today, with `-` in every new column, no summary line, and exit 0 (a `not-implemented` from the pane store is not a roster failure). The new columns are live in the binary now; the title of this decision is kept because the notes of acceptance 7 and 8 cite it by that title. A second, different store error (`timeout`, `unavailable`) is also not a roster failure: it prints `-` columns and one line on stderr `note: pane registry unavailable (<code>)`, exit 0. Tests cover `not-implemented` and `unavailable`.
- **The mismatch rule is the existing pair rule:** a pane is a mismatch only when both `last_observed.shown` and `last_observed.driven` are `Some` and unequal; one side absent is never a mismatch (`say_cmd.rs` and the hub's `dispatch.rs` apply the same rule to a recorded pair, E11). `reconcile::shown_differs` answers a different question (shown against `session_of_record`) and is not used here.
- **Summary line, byte format:** `N pane(s) show a different session than the hub drives: a, b` (colon, one space, names in roster order separated by `, `), followed by a newline, then the table. Pinned in the golden files.
- **`--format=json` `data` shape:** the same object `roster --json` prints (`{"rows": [...]}`, each row with the new keys added) plus one more key, `"mismatches": [pane names]`, next to `rows`. `roster --json` itself does not get `mismatches`.
- **Exit codes:** `roster` has exit 0 (success, in both formats), 2 (a usage error such as `--format=xml`) and 3 (the `profile-not-found` refusal); it has no exit-1 path of its own. Acceptance 4 is tested once per class in both formats (a success fixture, `--format=xml`, an unknown `--profile`).
- **The `--profile` refusal output** goes through the same shared helper every refusing verb uses (`output.rs`): text mode prints the one-line `error:` form on stderr, `--format=json` prints the failure envelope; nothing else is printed.
- **`roster_cmd::run`'s new signature** is chosen by the story and stated in the brief: it receives the resolved format choice (with #670's `json_explicit` bit) and a `Ports`; `main.rs`'s `run_roster` builds the `Ports` the way `run_with_stdio` does for the pane verbs.

## Evidence (brief gate, rounds 1 to 6)

Added at the brief gate, with every line above whose italic label says "at the brief gate", to answer the outside review; the rest of this brief is the issue's text. Round 2 added E12, E13 and the lines labelled "round 2"; round 3 added E14 to E17 and the lines labelled "round 3"; round 4 added E18 and the lines labelled "round 4"; round 5 added E19 and the lines labelled "round 5"; round 6 added E20 and the lines labelled "round 6". Every excerpt is verbatim from this worktree at its base, `92fbc6e` (`origin/main`); once an item names a file, a bare `:N` is a line of that file. (The worktree's base is now `3c139dd`, which differs from `92fbc6e` only in files no excerpt comes from; rounds 3 to 6 read their excerpts there. Refresh, 2026-10-11: the worktree's base is now `bda1069`; the excerpts of the files that changed since `3c139dd` were re-cut there, as the refresh section lists, and say "refresh" or carry new line numbers.) The "E-list in the evidence" that the first decision above cites is this list: `pane list` reads the registry through `ports.pane_store.list()` in E8 (`rows_in_scope`) and turns a record into its cells in E3 (`PaneRow::from`) and E17. The six quotes the section above asks for are E4 (#670's roster helper and its refusal), E5 (the format resolver's "explicit json" bit), E8 (`ProfileScope`), E7 (`check_envelope`), E1 (the `render_table` header) and E2 (the hub's roster row type).

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
- **E3, where a pane's SHOWN and DRIVEN are.** The record (`crates/holler-pane/src/pane.rs:188-199`, and `:243-251`, fields of `Pane`, which is `:232-267`):

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

  The port that observes SHOWN (`crates/holler-pane/src/ports.rs:341-342`):

  ```rust
      /// The session the TUI of `pane` shows, if it can tell.
      fn shown_session(&self, pane: &PaneId) -> Result<Option<String>, PaneError>;
  ```

  `docs/adr/ADR-0021.md`: lines 211-212 (section 2, "`HarnessPort` as built (#642)"), line 306 (I2), lines 883-888 (section 11; refresh: the paragraph now says "Deferred to #654 (cutover)" and no longer "#649 (wiring) and #654") and lines 892-894 (section 12):

  ```text
  3. `shown_session` is `None` whenever it cannot tell: the home screen, a deleted session, a title that is not a whole id,
     no TUI; it never guesses.
  ```

  ```text
  | I2 | `session_of_record` is the only session a pane's TUI shows and the hub drives. | After each of launch, relaunch, switch and reset on the fakes, the fake TUI's shown session, the driven session and `session_of_record` are equal. |
  ```

  ```text
  **Deferred to #654 (cutover):** how the hub's DRIVEN session is pointed at `session_of_record` for each registered pane,
  and when it moves after a switch or reset. The wiring (#649) wires the ports and leaves DRIVEN to the cutover.
  Until the cutover wires DRIVEN, reconcile (#647) compares SHOWN with `session_of_record`, which I2 makes the session the
  hub drives, and leaves `last_observed.driven` as stored.
  A switch or reset (#645) likewise leaves `last_observed.driven` as stored; the hub's DRIVEN session follows the new
  `session_of_record` once the cutover wires it.
  ```

  ```text
  **Decided:** the hub runs no adapters (ruling 1), so it executes no long work. The CLI process that runs a verb executes
  every step, and every port call is bounded by I5 (default 10 s) or ends in `timeout`. A verb that times out stops,
  ```

  How `pane list` (#643) reads them, and its offer to the roster (`crates/holler-cli/src/pane/list.rs:4-18`, `:248-262`):

  ```rust
  //! The shared items are `pub` and live here because the frozen `pane/mod.rs` admits no new
  //! module. [`PaneRow`] is one pane in summary: a `list` row, and the `pane` of a `watch`
  //! put. [`SessionSync`] is SYNC, read from a record through [`shown_differs`], the one
  //! SHOWN/DRIVEN rule (`pane doctor` uses it too). [`text_value`] and [`json_text`] make a
  //! stored string safe to print on a terminal, and [`observed_at`] prints a record
  //! timestamp. `get.rs` and `watch.rs` use them through `super::list`, and a later consumer
  //! (the roster, #648) can use them through `crate::pane::list`.
  //!
  //! The read verbs read the record and nothing else, with one exception (story #832): `list`
  //! over the whole registry, and `get` when no record answers, also read Herdr
  //! (`HerdrPort::{snapshot, observe_pane}`) for the panes no record names, which they show as
  //! `unmanaged`. They call `PaneStore::{get, list, watch}`, `ProfileStore::get` and
  //! `ProfileScope::resolve` otherwise, never a probe, and they write nothing. SHOWN and health are what reconcile last recorded. DRIVEN is
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

  The CLI's ports since #649 merged (refresh, 2026-10-11; `crates/holler-cli/src/pane/wiring.rs:1-27`, `:101-128` and `:205-207`, `crates/holler-cli/src/pane/wiring/store.rs:115-132`). The stub these lines replace said that `connect` hands out `Unwired`; now `connect` builds the two registries' clients, the three adapters (without I/O) and the scope, and needs only the hub:

  ```rust
  //! The ports a `holler pane` or `holler profile` verb runs against (epic #633).
  //!
  //! [`Wiring`] owns them: [`Wiring::connect`] builds them, and [`Wiring::ports`] lends them as the
  //! [`Ports`] bundle a verb holds. `main.rs` calls `connect` once per run, and `say_cmd.rs` once for
  //! `say`, `interrupt` and `answer` with `--pane`; each reports its error the way a verb reports one,
  //! so `pane::run` and `profile::run` never see it. The in-process verb harness and the verb tests
  //! build their `Ports` from fakes, or from [`Unwired`], and never call `connect`.
  //!
  //! **What `connect` builds (#649, D1).** The verbs run here, CLI-side; the hub is the store only
  //! and constructs no adapter (ADR-0021 section 5). So `connect` builds:
  //!
  //! - the two registries' clients ([`store`]), over the hub's control socket;
  //! - the Herdr adapter, built on its first use ([`bind::LazyHerdr`]): it pings Herdr when it is
  //!   built;
  //! - the tmux host (`TmuxHost`) and the OpenCode harness (`OpenCodeHarness`), on one tmux server;
  //!   building either does no I/O;
  //! - `SystemProber`, and `StoreScope` over the two clients as the `ProfileScope`. The scope logs
  //!   every profile write as `holler pane`: what a `--profile` edit of a pane verb writes, and
  //!   through it what `profile apply`'s creates and relaunches write (one actor; per-verb actors are
  //!   #825).
  //!
  //! **What `connect` needs: the hub, and nothing else (D2).** It makes one `control/status`
  //! exchange, and any failure of it (no hub, an error, no answer within 5 s) is `unavailable`
  //! naming the hub, so with no hub every verb exits 1 and touches no adapter. It touches no Herdr,
  //! tmux or OpenCode, and a configuration variable that is missing or malformed never fails it
  //! (D26): the error is kept, and the port it configures answers it at its first use. So `pane list`,
  //! `pane get`, `pane watch`, `profile list` and `profile show` work with nothing but the hub.
  ```

  ```rust
  impl Wiring {
      /// Build the ports of a run from this process's environment, once the hub has answered.
      /// Fails `unavailable`, naming the hub, only when the hub cannot be reached (D2).
      ///
      /// A variable whose value is not UTF-8 is read as the empty value, never as unset
      /// ([`env::Lookup`]): unset, `HOLLER_TMUX_SOCKET` would be tmux's default server, the
      /// operator's live one (D6).
      pub fn connect() -> Result<Self, PaneError> {
          store::reach_hub()?;
          let actor = Actor::parse(ACTOR)?;
          let lookup = |name: &str| {
              std::env::var_os(name).map(|value| value.into_string().unwrap_or_default())
          };
          Ok(Self::from_env(&lookup, actor))
      }

      /// The ports, lent for one verb.
      pub fn ports(&self) -> Ports<'_> {
          Ports {
              pane_store: self.panes.as_ref(),
              profile_store: self.profiles.as_ref(),
              herdr: &self.herdr,
              host: &self.host,
              harness: &self.harness,
              scope: &self.scope,
              prober: &self.prober,
          }
      }
  ```

  ```rust
  /// A port set whose every method answers [`PaneError::NotImplemented`]. Its probe never passes.
  #[derive(Debug, Clone, Copy, Default)]
  pub struct Unwired;
  ```

  The pane store `connect` hands out reads the hub on every call (`wiring/store.rs`), through the CLI's one dispatch point with no server argument, so it never follows a roster's `--server` (R3 of the refresh section):

  ```rust
  /// `connect`'s one exchange (D2): `control/status`, within the control client's own 5 s (E60).
  /// Every failure, a hub that does not answer in time included, is `unavailable` naming the hub:
  /// a verb can do nothing without it.
  pub(crate) fn reach_hub() -> Result<(), PaneError> {
      let status = ControlCall::status();
      crate::transport::call(None, &status)
          .map(drop)
          .map_err(|error| hub_unavailable(status.method, &error))
  }

  impl PaneStore for HubPaneStore {
      fn get(&self, name: &PaneName) -> Result<Option<Pane>, PaneError> {
          PANE_GET.request(&PaneGetParams { name: name.clone() })
      }

      fn list(&self) -> Result<Vec<Pane>, PaneError> {
          PANE_LIST.request(&json!({}))
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

  The existing process test that pins the refusal (`crates/holler-cli/tests/pane_verbs/process/legacy_verbs.rs:95-105`) and its constant (`crates/holler-cli/tests/pane_verbs/process/stub.rs:33-36`):

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

  *Added at the brief gate (round 6), the flag's values:* the flag's type, `Format` (`crates/holler-cli/src/output.rs:40-47`), has two variants and derives clap's `ValueEnum`, so `text` and `json` are the only values `--format` takes. `--format=xml` is therefore refused by clap's parse itself (`Cli::try_parse()` in `parse_or_exit`, E14). `format_or_exit` (E6), which takes the parsed `Cli`, never runs, and under `roster` the failed parse ends in `error.exit()`: clap's own message on stderr, with exit 2 (E14).

  ```rust
  /// The output format: the global `--format`, and what `--json` means.
  #[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
  pub enum Format {
      /// Human-readable text (the default).
      Text,
      /// One JSON envelope per result.
      Json,
  }
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

  `holler-cli` already has the test kit as a dev-dependency (`crates/holler-cli/Cargo.toml:443-447`):

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

  How a verb holds it (`crates/holler-pane/src/ports.rs:412-423`) and how `pane list` uses it (`crates/holler-cli/src/pane/list.rs:127-155`):

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
      let unscoped = pane.is_none() && args.profile.profile.is_none();
      let mut panes = match (profile_name(&args.profile)?, pane) {
          (Some(profile), pane) => ports.scope.resolve(&profile, pane.as_ref())?.panes,
          (None, Some(pane)) => ports.pane_store.get(&pane)?.into_iter().collect(),
          (None, None) => ports.pane_store.list()?,
      };
      panes.sort_by(|a, b| a.name.cmp(&b.name));
      // Only the whole registry also shows what the registry does not hold: a pane scoped by
      // name or profile is a question about records.
      let (unmanaged, gap) = match unscoped.then(|| unmanaged::observe(ports, now_millis())) {
          Some(Ok(found)) => (found, None),
          Some(Err(error)) => (Vec::new(), Some(error)),
          None => (Vec::new(), None),
      };
      Ok(PaneTable {
          panes: panes.iter().map(PaneRow::from).collect(),
          unmanaged,
          unmanaged_unavailable: gap.as_ref().map(|e| e.code().to_string()),
          unmanaged_gap: gap.map(|e| {
              let body = ErrorBody::from(&e);
              format!("{}: {}", body.code.as_str(), body.message)
          }),
      })
  }
  ```
- **E9, ADR 0003 lines 105-106** (`docs/adr/ADR-0003.md`, section "Versioning"), verbatim:

  ```text
  - **SemVer from 0.1.0.** Standard pre-1.0 behaviour — **breaking changes bump MINOR while MAJOR is 0**. This **deliberately reverses ADR-0014's opt-out**, because there is no external library consumer.
  - **"Breaking"** = the CLI surface, the `--json` shape, the on-disk formats (token store, credential file, session config), and the wire protocol version. **Additive = MINOR. Fixes = PATCH.**
  ```
- **E10, the grid position.** The pane's name and where it sits in Herdr (`crates/holler-pane/src/pane.rs:25-32`, `:91-108`):

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
  /// grid cell. Also what `HerdrPort::ensure_pane` returns for a pane that exists, and
  /// what `HerdrPort::snapshot` lists; a pane Herdr reports also carries its label.
  #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  #[serde(deny_unknown_fields)]
  pub struct HerdrPane {
      pub session: String,
      pub workspace: String,
      pub pane_id: PaneId,
      pub grid: GridPos,
      /// The pane's label as Herdr reports it, read by the Herdr adapter through its
      /// `label_of`: an observation (I6), and the join key from a pane name to a live Herdr
      /// pane (#808). `launch` and `relaunch` never write one, so a record's `herdr.label`
      /// is `None` and its JSON has no `label` key. `None` stands both for a pane Herdr
      /// sent no label for and for one it sent an empty label for.
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub label: Option<String>,
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

  `docs/adr/ADR-0021.md` section 10, lines 844-849:

  ```text
  - **Output** is always row first: text `r2c1`; JSON `{"row": 2, "col": 1, "pos": "r2c1"}`, in that key order. On input `pos`
    is optional.
  - The Herdr adapter (#640) is the only code that converts a `GridPos` to Herdr's own order and base. `HerdrPort::ensure_pane`
    reaches a position by right and down splits (B4, `plan_splits`, #640) or fails loudly, and never relocates a healthy pane.
    `relaunch` moves a pane only when `--grid` is given. A pane's position comes from Herdr's snapshot, never from its name:
    `hj-c1r2` sits at `r2c1`.
  ```

  `pane list` takes `pos: pane.herdr.grid` (`crates/holler-cli/src/pane/list.rs:252`, in E3) and prints it through the `Display` above, as the second text cell of a row (`crates/holler-cli/src/pane/list.rs:268-271`):

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

  *Added at the brief gate (round 2):* a third form, `shown_differs`, is the rule that reconcile, `pane list`'s SYNC, launch and switch use (`crates/holler-pane/src/reconcile.rs:176-186`; `list.rs` names it in E3). Unlike the pair rule above, it compares SHOWN with `session_of_record`, not with `last_observed.driven`, and it counts `shown: None` as a mismatch. Its doc names the roster among the readers that call it; the gate at `send_prompt` that it also names is the hub's prompt gate above, which applies the pair rule instead. Scope item 3 states the summary line's own rule ("both known and not equal"); which session DRIVEN is remains the question scope item 7 leaves open.

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
- **E12, who writes `last_observed` (added at the brief gate, round 2).** Three production writers set it, and none gives `driven` a new value: reconcile (#647) writes `last_observed.shown` and `last_observed.at` (and `harness.health`); `pane switch` and `pane reset` (#645) write `session_of_record`, `last_observed.shown`, `last_observed.at` and `harness.health`; `pane launch` and `pane relaunch` (#644) write a whole record whose `driven` is the old record's, or `None` for a new pane. A search of the source at `92fbc6e` for `last_observed` finds no other write: its other uses read it, declare it or are comments, except the two test helpers quoted below. (Refresh: at `bda1069` the same search finds one more writer, `pane import` (#650), which stores a new record whose `last_observed` is empty, `shown` and `driven` `None` and `at` 0 (`crates/holler-pane/src/import/assess.rs:394-398`); it gives `driven` no value either.) A test is not bound by this: it builds a record from the test kit's `sample_pane`, whose `last_observed` is empty, and sets the pair it needs, as the hub's prompt-gate tests do. Reconcile's rule (`crates/holler-pane/src/reconcile.rs:31-35`) and its writer (`crates/holler-pane/src/reconcile/observe.rs:350-375`):

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

  Switch and reset (`crates/holler-pane/src/tx_switch.rs:389-398`):

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

  Launch and relaunch (`crates/holler-pane/src/tx_launch.rs:361-372`, `:397-401`):

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

  The test kit's sample record (`crates/holler-pane-testkit/src/fixture.rs:72-76`) and the hub's prompt-gate tests seeding a pair on it (`crates/holler-hub/src/circuit/dispatch.rs`, lines 548-556, in the module's `#[cfg(test)]` part, which starts at line 456):

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
- **E13, how a verb gets its ports today (added at the brief gate, round 2).** `--profile` resolves through `ProfileScope` (scope item 4), which a verb holds as `Ports.scope` (E8). `main.rs` builds `Ports` in one place, `run_with_stdio`, which the `pane` and `profile` verbs run through: it takes them from `Wiring::connect` and hands them to the verb in a `VerbCtx` (`crates/holler-cli/src/main.rs:253-270`, `crates/holler-cli/src/output.rs:191-199`). `run_roster` builds no `VerbCtx`, and `roster_cmd::run(roster, json)` takes no `Ports` (E1, E4). *(Refreshed at `bda1069`.)* Since #649 merged those ports are the real ones (E3, refreshed): `Wiring::ports` lends the registries' clients and a `StoreScope` over them (`crates/holler-cli/src/pane/wiring.rs:117-128`), so in the binary `ports.scope.resolve` and `ports.pane_store.list` answer from the hub, where this paragraph said they answered `not-implemented` (`Unwired`, now test-only, `wiring.rs:205-207`). `Wiring::connect` itself makes one `control/status` exchange and fails `unavailable`, naming the hub, when there is none (`wiring.rs:101-115`, `wiring/store.rs:115-123`). The acceptance tests run on the test kit's fakes (scope item 7). The issue does not fix `roster_cmd::run`'s new signature; `main.rs` and `roster_cmd.rs` are both in the blast radius. `crates/holler-cli/src/roster.rs`, which the blast radius also names, does not exist at `bda1069` either.

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

  *Added at the brief gate (round 6), the rest of what `emit` and `emit_error` run (`crates/holler-cli/src/output.rs`):* `Sink` (`:184-189`); the two functions `emit` calls, `emit_text` and `emit_json`; and what those call: `write_envelope`, `write_line` and `settle` (`:266-334`, whole), `exit_code` (`:336-340`, in E6) and `one_line` (`:342-350`). Each takes only a `Sink` or one of its writers, the result (any `T: Serialize` in `emit` and `emit_json`, or an `ErrorBody`), the text closure, an exit code or a string. None takes a `VerbCtx` (E13) or a `Ports`, and none reads which verb is running: the exit code comes from the error's code alone, through `holler_pane::error::class_of` (E6). The doc of `Envelope` (`:150`, in E6) names the `pane` and `profile` verbs, but the type is generic over its data.

  ```rust
  /// The two writers a verb prints to: the real `main` builds it over stdout and stderr, a test
  /// over buffers.
  pub struct Sink<'a> {
      pub out: &'a mut dyn Write,
      pub err: &'a mut dyn Write,
  }
  ```

  ```rust
  fn emit_text<T>(
      sink: &mut Sink<'_>,
      result: Result<T, ErrorBody>,
      text: impl FnOnce(&T) -> String,
  ) -> i32 {
      match result {
          Ok(data) => {
              let rendered = text(&data);
              let written = if rendered.is_empty() {
                  Ok(())
              } else {
                  write_line(sink.out, &rendered)
              };
              settle(written, 0)
          }
          Err(error) => {
              let written = write_line(sink.err, &format!("error: {}", error.message));
              settle(written, exit_code(&error))
          }
      }
  }

  fn emit_json<T: Serialize>(sink: &mut Sink<'_>, result: Result<T, ErrorBody>) -> i32 {
      match result {
          Ok(data) => settle(write_envelope(sink, &Envelope::success(data)), 0),
          Err(error) => {
              let code = exit_code(&error);
              let error = ErrorBody {
                  message: one_line(&error.message),
                  ..error
              };
              settle(write_envelope(sink, &Envelope::<()>::failure(error)), code)
          }
      }
  }

  /// Write one envelope as one compact line. Data that cannot be encoded is reported on `err` and
  /// fails the write, so no half-written envelope reaches `out`.
  fn write_envelope<T: Serialize>(sink: &mut Sink<'_>, envelope: &Envelope<T>) -> io::Result<()> {
      match serde_json::to_string(envelope) {
          Ok(line) => write_line(sink.out, &line),
          Err(cause) => {
              write_line(
                  sink.err,
                  &format!("error: cannot encode the result: {cause}"),
              )?;
              Err(io::Error::other(cause))
          }
      }
  }

  /// Write `line` and end it with a newline (unless it has one), then flush.
  fn write_line(to: &mut dyn Write, line: &str) -> io::Result<()> {
      to.write_all(line.as_bytes())?;
      if !line.ends_with('\n') {
          to.write_all(b"\n")?;
      }
      to.flush()
  }

  /// The exit code of a verb that wrote `written`: `code`, except that output nobody received
  /// cannot be a success.
  fn settle(written: io::Result<()>, code: i32) -> i32 {
      if written.is_err() && code == 0 {
          1
      } else {
          code
      }
  }
  ```

  ```rust
  /// `message` on one line, as the envelope requires: each line break becomes a space.
  fn one_line(message: &str) -> String {
      message
          .lines()
          .map(str::trim)
          .filter(|line| !line.is_empty())
          .collect::<Vec<_>>()
          .join(" ")
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

  `FakePaneStore` reaches it through `faults()` (`crates/holler-pane-testkit/src/pane_store.rs:107-111`, below), and its `list` is the op `PaneStoreOp::List` (`:25`, in the enum at `:23-31`; `as_str` names it `"pane_store.list"`, `:37`). `FakeHarness`, `FakeHerdr` and `FakeHost` have the same accessor over their own ops (`crates/holler-pane-testkit/src/harness.rs:202`, `herdr.rs:222`, `host.rs:112`).

  ```rust
      /// The fault switch of every port method and the log of the calls made through
      /// the port.
      pub fn faults(&self) -> &FaultSwitch<PaneStoreOp> {
          &self.faults
      }
  ```

  The stable codes of the two store errors the tests use are `PaneCode::NotImplemented => "not-implemented",` (`crates/holler-pane/src/error.rs:100`) and `PaneCode::Unavailable => "unavailable",` (`:120`), both in `PaneCode::as_str` (`:97-123`).

  *Added at the brief gate (round 6), what the call logs of `FakeHarness`, `FakeHerdr` and `FakeHost` record (acceptance 7, round 6):* the switch's `enter` (`crates/holler-pane-testkit/src/fault.rs:90-103`) records the call before it sleeps or answers a fault, and each of the three fakes calls it first in every port method it implements. `impl HarnessPort for FakeHarness` (`crates/holler-pane-testkit/src/harness.rs:320-414`, refreshed at `bda1069`) begins each of its nine methods with `self.faults.enter(HarnessOp::…)?;` (`:327`, `:333`, `:347`, `:352`, `:359`, `:368`, `:379`, `:396`, `:411`), `impl HerdrPort for FakeHerdr` (`herdr.rs:278`) each of its eight (`:280`, `:290`, `:301`, `:319`, `:324`, `:333`, `:339`, `:344`), and `impl HostPort for FakeHost` (`host.rs:176`) each of its four (`:178`, `:190`, `:195`, `:203`). `HostPort` has no other method and no default body (`crates/holler-pane/src/ports.rs:259-275`). `HerdrPort` (`ports.rs:169-221`) has nine: the eight above and `observe_panes` (`:215-220`), a default that calls `observe_pane` for each pane it is asked about, so it reaches `FakeHerdr`'s log through `observe_pane` (and makes no call for an empty list). `HarnessPort` (`ports.rs:286-366`) has eleven: the nine above, and `send_prompt` and `session_activity`, whose bodies are the trait's defaults (`ports.rs:344-365`, below). No file of the test kit defines either, so `FakeHarness` keeps the defaults, which reach no fault switch and so no call log. Below: `enter`, the three op enums (`harness.rs:44-57`, `herdr.rs:54-66`, `host.rs:29-36`), the three accessors (`harness.rs:200-204`, `herdr.rs:220-224`, `host.rs:110-114`), the first two methods of `FakeHarness` as an example of the pattern (`harness.rs:320-344`), and the two defaults.

  ```rust
      /// What a fake calls first in every port method. It records the call, sleeps for
      /// the delay (without holding the lock, so other calls proceed), and then answers
      /// the standing fault if there is one, or else the oldest error queued for `op`.
      pub(crate) fn enter(&self, op: Op) -> Result<(), PaneError> {
          let delay = {
              let mut state = self.lock();
              state.calls.push(op);
              state.delay
          };
          if let Some(delay) = delay {
              thread::sleep(delay);
          }
          self.lock().take_fault(op)
      }
  ```

  ```rust
  /// A method of the `HarnessPort` port, as a fault targets it and the call log records
  /// it.
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
  pub enum HarnessOp {
      ServeCommand,
      VerifyServing,
      Health,
      CreateSession,
      ListSessions,
      Abort,
      AttachTui,
      SelectSession,
      ShownSession,
  }
  ```

  ```rust
  /// A method of the `HerdrPort`, as a fault targets it and the call log records it.
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
  pub enum HerdrOp {
      EnsurePane,
      SendText,
      SendKeys,
      Read,
      Close,
      Snapshot,
      Version,
      /// `observe_pane` (#832).
      ObservePane,
  }
  ```

  ```rust
  /// A method of the `HostPort` port, as a fault targets it and the call log records it.
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
  pub enum HostOp {
      EnsureSession,
      Run,
      StopOwned,
      Ps,
  }
  ```

  ```rust
      /// The fault switch of every port method and the log of the calls made through
      /// the port.
      pub fn faults(&self) -> &FaultSwitch<HarnessOp> {
          &self.faults
      }
  ```

  ```rust
      /// The fault switch of every port method and the log of the calls made through the
      /// port.
      pub fn faults(&self) -> &FaultSwitch<HerdrOp> {
          &self.faults
      }
  ```

  ```rust
      /// The fault switch of every port method and the log of the calls made through
      /// the port.
      pub fn faults(&self) -> &FaultSwitch<HostOp> {
          &self.faults
      }
  ```

  ```rust
  impl HarnessPort for FakeHarness {
      fn serve_command(
          &self,
          name: &PaneName,
          port: u16,
          spec: &ServeSpec,
      ) -> Result<Argv, PaneError> {
          self.faults.enter(HarnessOp::ServeCommand)?;
          self.lock().serve_specs.push(spec.clone());
          Ok(description(name, port, spec.agent.as_ref()))
      }

      fn verify_serving(&self, port: u16, spec: &ServeSpec) -> Result<(), PaneError> {
          self.faults.enter(HarnessOp::VerifyServing)?;
          let world = self.lock();
          world.reach(port, HarnessOp::VerifyServing)?;
          let running = world.servers.get(&port).and_then(|s| s.agent.as_ref());
          if running == spec.agent.as_ref() {
              return Ok(());
          }
          Err(PaneError::Refused {
              code: AGENT_MISMATCH,
              message: format!("the server on port {port} does not run the agent the spec asks for"),
          })
      }
  ```

  ```rust
      /// Queue `text` as a prompt to `session` on the server at `port`, through the
      /// harness API, never by typing into a TUI (I4). `pane reset --first` calls it (#645).
      ///
      /// The default refuses with [`PROMPT_UNSUPPORTED`]: an impl that does not send
      /// prompts says so as a stable refusal, which the real adapter replaces (#642).
      fn send_prompt(&self, port: u16, session: &str, text: &str) -> Result<(), PaneError> {
          let _ = (port, session, text);
          Err(PaneError::Refused {
              code: PROMPT_UNSUPPORTED,
              message: "this harness does not queue a prompt".to_owned(),
          })
      }

      /// What `session` on the server at `port` is doing now (#645: `pane reset` refuses a
      /// conversation that is not [`Activity::Idle`]).
      ///
      /// The default answers [`Activity::Idle`], so an impl that does not report activity
      /// refuses nothing.
      fn session_activity(&self, port: u16, session: &str) -> Result<Activity, PaneError> {
          let _ = (port, session);
          Ok(Activity::Idle)
      }
  ```
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
- **E17, what `pane list` prints for an absent value (added at the brief gate, round 3).** The SHOWN and DRIVEN cells (`crates/holler-cli/src/pane/list.rs:275-276`, in `PaneRow::cells`, `:268-280`), `optional_text` (`:371-374`) and `NO_VALUE` (`:38-39`):

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

  *Added at the brief gate (round 5):* the second excerpt is the whole of `optional_text` (`:374` is its closing brace). For `None`, `map_or_else` returns its first closure's value, `NO_VALUE.to_owned()`, which is the string `-`; for `Some(value)` it returns `text_value(value)`. So a record whose `last_observed.driven` is `None` has the DRIVEN cell `-` in `pane list`, and so in the roster, whose SHOWN and DRIVEN are the values `pane list` prints (scope item 7; scope item 1, round 5). In JSON, `pane list` writes such a value as `null` (`PaneRow`'s doc, `crates/holler-cli/src/pane/list.rs:224-226`):

  ```rust
  /// One pane in summary: a row of `pane list`, and the `pane` of a `pane watch` put.
  ///
  /// Its JSON keys are its fields, in this order. An empty value is `null`, never left out.
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

  *Added at the brief gate (round 6), the whole label check:* the rest of `valid_label` (`crates/holler-hub/src/token.rs:413-427`, whole, below) takes as the first character a lowercase ASCII letter or a digit, and after it only lowercase ASCII letters, digits and `-`, so no label it passes holds a `/`. The name vocabulary's own `Label::parse` (`crates/holler-proto/src/vocab.rs:32-45`) refuses a `/` outright, as `SessionName::parse` above does, and the segment grammar that the three name kinds share allows lowercase ASCII letters, digits and `-` only (`vocab.rs:13-15`).

  ```rust
  /// The label grammar (ADR 0005 §1): `^[a-z0-9][a-z0-9-]{0,31}$`.
  fn valid_label(label: &str) -> bool {
      if !(1..=32).contains(&label.len()) {
          return false;
      }
      let mut chars = label.chars();
      let Some(first) = chars.next() else {
          return false;
      };
      if !(first.is_ascii_alphanumeric() && first.is_ascii_lowercase() || first.is_ascii_digit()) {
          return false;
      }
      chars
          .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
  }
  ```

  ```rust
      /// Parse a wire string into a `Label`.
      ///
      /// Succeeds iff the string is a single 1–32-char segment matching the
      /// grammar (no `/`). A `Label` never contains a `/`.
      pub fn parse(s: &str) -> Result<Self, NameError> {
          if s.is_empty() {
              return Err(NameError::Empty);
          }
          if s.contains('/') {
              return Err(NameError::Slash);
          }
          check_segment(s.as_bytes())?;
          Ok(Self(s.to_owned()))
      }
  ```

  ```rust
  //! Segment grammar (ADR 0005): lowercase ASCII letters and digits only; `-`
  //! is allowed **between** word chars, but a segment must not start or end
  //! with one. The 32-char per-segment limit (docs §3) is enforced here.
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
- **E19, what `PaneStore::list` can answer (added at the brief gate, round 5).** The trait's doc and its `list` (`crates/holler-pane/src/ports.rs:63-76`):

  ```rust
  /// The registry of panes, kept by the hub (#639) and faked by the test kit (#638).
  ///
  /// **Blocking.** Every method is synchronous. Call from `spawn_blocking` (or a
  /// thread) in async code. Every method returns within I5's bound (default 10 s) or
  /// with [`PaneError::Timeout`]. An implementation is `Send + Sync`.
  ///
  /// Every write is a compare-and-swap on the pane's `generation` (see
  /// [`crate::generation`]); a stale one is `generation-conflict`.
  pub trait PaneStore: Send + Sync {
      /// The pane named `name`, or `None`.
      fn get(&self, name: &PaneName) -> Result<Option<Pane>, PaneError>;

      /// Every pane.
      fn list(&self) -> Result<Vec<Pane>, PaneError>;
  ```

  The stable codes of the two store errors that acceptance 8 (round 5) adds a test for, in `PaneCode::as_str` (`crates/holler-pane/src/error.rs:97-123`; E15 gives the other two): `PaneCode::Timeout => "timeout",` (`:116`) and `PaneCode::StoreCorrupt => "store-corrupt",` (`:119`). Like `not-implemented` and `unavailable`, both are `ErrorClass::Failure` in `class_of` (`:266-303`, in E6). The test kit's `Fault::Fail` takes any `PaneError`: "Every call fails with this error (e.g. `store-corrupt` or `unavailable`)" (`crates/holler-pane-testkit/src/fault.rs:33-34`, in E15).
- **E20, changing a record between two roster calls (added at the brief gate, round 6).** `FakePaneStore`'s seeding, its other writer's put and its `list` (`crates/holler-pane-testkit/src/pane_store.rs:90-99`, `:113-118`, `:182-185`). `concurrent_put` takes `&self`, so a test that has lent the store to the roster's `Ports` (whose `pane_store` is a `&dyn PaneStore`, E8) can still write to it between two calls; `list` reads the records the store holds when it is called.

  ```rust
      /// A store holding `panes`, each created at expected generation 0 and so stored
      /// at 1, in order. Seeding bypasses the faults and the call log. Two seeds with one
      /// name are `generation-conflict`, as a second create would be.
      pub fn seeded(panes: impl IntoIterator<Item = Pane>) -> Result<Self, PaneError> {
          let store = Self::new();
          for pane in panes {
              store.put(&pane, Writer::Port(0))?;
          }
          Ok(store)
      }
  ```

  ```rust
      /// Another writer stores `pane` unconditionally, at the stored generation + 1 (or
      /// at 1 for a new record), without the membership rule, and publishes its event.
      /// It bypasses the faults and the call log. Returns the stored record.
      pub fn concurrent_put(&self, pane: &Pane) -> Result<Pane, PaneError> {
          self.put(pane, Writer::Other)
      }
  ```

  ```rust
      fn list(&self) -> Result<Vec<Pane>, PaneError> {
          self.faults.enter(PaneStoreOp::List)?;
          Ok(self.feed.read(|log| log.records().cloned().collect()))
      }
  ```

  A verb test that already sets a record's `last_observed.driven` with `concurrent_put` before it runs the verb (`crates/holler-cli/tests/pane_verbs/relaunch.rs:474-487`):

  ```rust
  /// AC 20 (decision 22): the DRIVEN another writer stored is kept by a relaunch.
  #[test]
  fn relaunch_keeps_the_stored_driven() {
      let rig = Rig::new();
      let mut theirs = rig.live(false);
      theirs.last_observed.driven = Some("ses_driven_by_hub".to_owned());
      rig.fakes.panes.concurrent_put(&theirs).unwrap();
      let run = rig.run(RELAUNCH, Format::Json);
      data_of(&run);
      let driven = rig.record(PANE).unwrap().last_observed.driven;
      assert_eq!(driven.as_deref(), Some("ses_driven_by_hub"));
      rig.assert_matches(&run, PANE);
      rig.assert_no_keystroke();
  }
  ```

## Clashes with stories in flight

*Read-only look at 2026-10-11 at issues #839, #831, #835, #667 and #828 (folded into #839) and the briefs and worktrees of #831, #835 and #828. #839 had no brief yet, so its row rests on the issue text. None of these stories edits a roster code file: the files of #648 are `roster_cmd.rs`, the roster helper of `main.rs`, the `Roster` struct of `cli.rs`, the roster tests, the guide and its test, two lines of ADR-0021 and CHANGELOG.*

| Story | Text shared with #648 | Code shared with #648 |
|---|---|---|
| #839 (the hub's DRIVEN follows the session of record; absorbs #828) | ADR-0021: #839 amends sections 8 and 11 (the "Deferred to #654" paragraph, lines 883-888) and the DRIVEN bullet of "Deferred to named stories" (line 1040), and #648 deletes line 1039 of the same list: adjacent lines, a hand-resolved conflict. CHANGELOG `[Unreleased]`: both add an entry at the same place. `docs/pane-control.md`: #839 will reword the DRIVEN text of section 4 (lines 398 and 420-424, a few lines above the sentence at line 432 that #648 replaces) and the DRIVEN bullet of section 12 (line 789, two lines below the roster bullet at line 787 that #648 deletes). This brief's E3 quotes ADR-0021 lines #839 rewrites; the quotes go stale and the code does not. | None: #839 edits `circuit/dispatch.rs`, `say_cmd.rs`, `tx_switch.rs`, reconcile and the hub. |
| #831 (a Claude Code harness) | ADR-0021 (its own amendment sections), CHANGELOG `[Unreleased]`, and `docs/pane-control.md` ("if present", its brief says). | `crates/holler-cli/Cargo.toml`: it adds `[[test]] pane_claude` at line 491 and #648 adds `roster_panes_test` at line 203 (different hunks). `pane_verbs/main.rs` and `cli-surface.txt` are its only. A Claude Code pane is a record like the others, so the roster joins it with no change, and a new `Health` value reaches the roster through `PaneRow` (R5). |
| #835 (import leaves foreign panes alone) | CHANGELOG (its hunk is at line 64), ADR-0021 (sections 2, 9, 13 and the closing lists, so possibly the same "Deferred" list), ADR 0003. | `crates/holler-cli/tests/docs_pane_control_test.rs`: it appends tests at the end (line 521) and #648 edits lines 155-184 and 295-303: no overlap. `docs/pane-control.md` is in its file list, but its diff against main showed no hunk when read. |
| #667 (the real-adapter scenario for `profile apply`) | None found. | `crates/holler-cli/Cargo.toml` is named in its issue (the adapter dependencies, which #649 has since added); a test-only story. |

**Recommended merge order:** #648 first, or at any time before the others: it is the smallest, has no code overlap with the four, and its shared text (CHANGELOG, the ADR-0021 list, the guide's bullets) is where the later PR resolves a conflict by hand. Then #835, then #839, then #831 (its scenario needs #839, its own brief says), and #667 at any point. If #839 merges first, the only cost to #648 is a re-cut of the ADR-0021 quotes in E3 and a one-line change of the guide's DRIVEN sentence, and the roster's DRIVEN column starts showing values with no code change (R6). The order is a recommendation for whoever launches the runs.

## Pipeline

`rigor: second-opinion` (the pipeline's default minimum for non-trivial work: the outside model `deepseek-v4-pro` at the brief and diff gates), `uiSurface: false`. After the refresh the brief needs a new brief gate before a run; nothing here claims the brief has passed one.

## Depends on

Checked with `gh issue view` on 2026-10-11: #639 (closed), #643 (closed), #647 (open: part 1, the reconcile engine and `pane doctor`, is merged and is what this story builds on; part 2, the `agent-cannot-dispatch` and `unattached` findings, is parked until after #654 and is not a dependency), #649 (closed 2026-10-11 17:35 MDT: the real wiring and the pane store the roster reads; **new in the refresh**), #670 (closed), and, as the test kit and the scope trait the brief cites, #637, #638 and #681 (all closed). Merged since the brief gate and relevant, not dependencies: #832 (R1), #652 (R9). Open and related, with no dependency in either direction: #839 (R6), #831, #835, #667.

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
