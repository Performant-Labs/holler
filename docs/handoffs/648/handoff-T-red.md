# Handoff-T-red: Phase 4 - #648 the roster shows pane, SHOWN and DRIVEN, and a loud line when they differ

**Date:** 2026-10-11
**Branch:** issue-648-implementation (worktree `.claude/worktrees/0648b-roster-shown-driven`, head `e496221`, plus this phase's staged test changes)
**Brief / wireframe reviewed:** `docs/handoffs/648-brief.md` at `e496221` (refresh R1 to R10, scope, acceptance 1 to 14, blast radius, decisions, E1 to E20); `docs/handoffs/648/handoff-A.md` (W-1 to W-12). No wireframe (no UI surface).

## A precondition

Confirmed: A returned **PASS** on the plan (`handoff-A.md`, no blocks, eight binding warns). The tests follow the binding warns: W-1 (the seam), W-2 (the pair rule), W-4 (three modes and the error boundary), W-5 (cells by column name), W-6 (state-dir lock, no wall-clock cells) and W-7 (layout; the legend is help text). W-3 and W-8 (ADR edits) are F's and have no test here.

## The API the tests call (names chosen here, as W-1 leaves them to T and F)

All in `holler_cli::roster_cmd`:

```rust
pub const PANE_COLUMNS: [&str; 6] = ["PANE", "POS", "PROFILE", "HEALTH", "SHOWN", "DRIVEN"];

/// The registry as the pure half receives it (W-1's three cases).
pub enum Registry<'a> {
    Ports(holler_pane::Ports<'a>), // render makes the registry reads itself
    Failed(holler_pane::PaneError), // a failed `Wiring::connect`
    NotRead,                        // `--server`
}

/// The pure half (R8): the hub's reply, the registry, the flags, the resolved format, a sink.
pub fn render(reply: &serde_json::Value, registry: Registry<'_>, roster: &Roster,
              choice: FormatChoice, sink: &mut Sink<'_>) -> i32;

/// R3's check, then today's hub call, then (only after a reply, and never with `--server`)
/// the deferred source of ports, then `render`.
pub fn run<'a>(roster: &Roster, choice: FormatChoice,
               registry: impl FnOnce() -> Result<Ports<'a>, PaneError>,
               sink: &mut Sink<'_>) -> i32;
```

`RosterResult` goes (W-1). Nothing else in `src/` is named by a test.

## Tests authored

**New target** `roster_panes_test` (`crates/holler-cli/tests/roster_panes_test.rs` and four `#[path]` modules under `tests/roster_panes/`, declared in `Cargo.toml` next to `roster_cli_test`). In-process, over the test kit's fakes (`FakePaneStore`, `FakeProfileStore`, `FakeProfileScope`, `FakeHerdr`, `FakeHost`, `FakeHarness`, `FakeProber`) and `check_envelope`. The fixture rows are the hub's own `holler_hub::roster::Row`, serialized, so the fixture is the reply's real shape. No row has `last_turn` or a hold (W-6). Goldens are in `tests/fixtures/roster_panes/`: `base.txt`, `no_registry.txt`, `two_mismatches.txt`, `one_mismatch.txt`.

Tier: this is the cheapest tier that sees the output bytes and the port calls. A unit test in the module would need `pub(crate)` access the brief does not ask for, and a process test cannot inject fakes.

| # | Test | What it pins (AC / decision) |
|---|---|---|
| 1 | `old_columns_are_unchanged_and_the_six_new_ones_follow_hold` | AC 1, AC 2 (none), AC 5. Golden `base.txt`. Each line starts with today's line through HOLD, checked against a frozen copy of today's format. Header ends with `PANE_COLUMNS`. One line per row, so no legend and no summary. No `c1r2` outside the pane name. |
| 2 | `two_mismatches_lead_with_one_summary_line_in_roster_order` | AC 2. Golden `two_mismatches.txt`. The rows are in the reply's order, which is deliberately not the store's name order, so the names come out `hj-c1r2, demo-c1r1`. A pane with SHOWN `-` and DRIVEN known is not named. |
| 3 | `one_mismatch_is_named_once_however_many_rows_carry_it` | AC 9, AC 10's example. Golden `one_mismatch.txt`: `1 pane(s) ...: hj-c1r2`, one pane on two rows, named once. |
| 4 | `json_keeps_every_key_and_adds_the_six` | AC 3, R5. The top level is exactly `rows`. Every hub key is kept with its value. The six keys are present, and `null` for a row with no pane. The values: `pos` in `GridPos` form, `health` `{"unhealthy": REASON}`, DRIVEN `null` (never the session of record). W-10: none of the six is a key of the hub's `Row`. |
| 5 | `format_json_is_one_envelope_of_rows_and_mismatches` | AC 4. `check_envelope` passes. `data` has exactly two keys. `data.rows` equals `--json`'s rows. `mismatches` is `[]`, `["hj-c1r2","demo-c1r1"]` or `["hj-c1r2"]`. Nothing on err. |
| 6 | `help_names_the_columns_and_that_a_pane_name_is_not_its_position` | Scope item 2, W-7. `roster`'s long help names the six columns, `hj-c1r2` and `r2c1`. |
| 7 | `join::a_row_with_no_label_joins_the_pane_of_its_whole_name` | AC 10 (round 6). Row `hj-c3r1` carries pane `hj-c3r1`'s cells in text and its values in JSON (`r1c3`). |
| 8 | `join::a_labelled_row_is_never_joined_by_its_session` | AC 10 (round 5). `other/demo-c1r1` is not joined to pane `demo-c1r1`: `-` and `null`. |
| 9 | `profile::profile_lists_only_the_rows_of_its_panes_in_roster_order` | AC 6, AC 10. `--profile fleet` lists `demo-c1r1/main` and `zz-c2r2/work`, in that order, in all three modes. Rows with no pane and the pane with no profile are dropped. |
| 10 | `profile::profile_narrows_the_summary_line_to_its_panes` | Scope item 4 (round 4). Under `--profile`, the line and `mismatches` name only P's panes. |
| 11 | `profile::an_unknown_profile_is_refused_in_every_mode` | AC 4, AC 6, W-1, W-4. `profile-not-found`, exit 3, also over an empty reply. Text and `--json`: one `error:` line and nothing on out. `--format=json`: the failure envelope and nothing on err. |
| 12 | `profile::a_malformed_profile_name_is_a_usage_line_in_every_mode` | AC 4, W-4. `--profile '!!!'` gives exit 2, one `error:` line on err and nothing on out, in all three modes (no envelope). |
| 13 | `profile::profile_over_a_registry_that_cannot_answer_is_its_error` | AC 14, R4. The store fails `unavailable`, so `--profile fleet` is exit 1 with only the error (no `note:`). Without `--profile`, the same store gives the note and exit 0. |
| 14 | `profile::profile_after_a_failed_connect_is_its_error` | W-1. `Registry::Failed` under `--profile` is exit 1 and the error. |
| 15 | `registry::the_cells_are_pane_lists_cells_for_the_same_record` | AC 7, W-5. Each row's six cells equal `PaneRow::from(&pane).cells()`, taken by the column's name in `pane::list::COLUMNS`. |
| 16 | `registry::a_record_rewritten_between_calls_shows_its_new_values` | AC 7 (round 6). Between calls, `concurrent_put` rewrites `last_observed`. The summary line appears, then goes. |
| 17 | `registry::the_roster_reads_the_registry_and_calls_no_adapter` | AC 7, R1. In every mode, with and without `--profile`, the store's record fills its row. The Herdr, host, harness and prober logs stay empty. |
| 18 | `registry::a_not_implemented_store_leaves_the_columns_empty_and_says_nothing` | AC 8 (i). Golden `no_registry.txt`, six `null`s, `mismatches` `[]`, empty err, exit 0, in all three modes. |
| 19 | `registry::any_other_store_error_is_one_note_line_with_its_code` | AC 8 (ii) and round 5, for `unavailable`, `timeout` and `store-corrupt`. Err is exactly `note: pane registry unavailable (<code>)`, exit 0, in all three modes. |
| 20 | `registry::a_failed_connect_is_a_registry_error` | R2. `Registry::Failed` without `--profile` gives the `(unavailable)` note and exit 0. |
| 21 | `registry::a_registry_not_read_under_server_says_so` | R3. `Registry::NotRead` gives `-` columns, `null`s and exactly `note: pane registry not read (--server names another hub)`. |
| 22 | `registry::the_mismatch_rule_is_say_pane_targets_rule` | W-2. For pairs (a,a), (a,b), (a,-), (-,b) and (-,-), the roster names the pane exactly when `say_cmd::resolve_pane_target` refuses `pane-shown-driven-mismatch`, and only for (a,b). It pins behaviour, so either of W-2's two placements passes. |
| 23 | `hubless::with_no_hub_the_result_is_todays_whatever_the_store` | AC 11 (round 6), W-1, W-6. `run` with no live hub (empty scratch `HOLLER_STATE_DIR`, under a lock), over a seeded store and a failing one. The source is never called. The same bytes either way: exit 1, today's line in text and `--json`, and an `unavailable` envelope with today's message under `--format=json`. |
| 24 | `hubless::a_server_refused_by_policy_exits_3_in_every_mode` | W-4. `--server ws://example.com:1` exits 3 everywhere. Today's line in text and `--json`. Under `--format=json`, an envelope that `check_envelope` accepts at exit 3, so a refusal-class code, with today's message. The source is never called. |
| 25 | `hubless::profile_with_server_is_a_usage_line_before_any_dial` | R3, AC 13 (a). Exit 2 and the one line in all three modes. It is checked before the policy refusal, which a non-loopback URL would otherwise give. |

**Process tests** added to `crates/holler-cli/tests/roster_cli_test.rs`. Only these see `main.rs`, `Wiring::connect` and clap. Stderr is read without the log lines: the banner, and the JSON log lines a `--server` dial writes.

| # | Test | What it pins |
|---|---|---|
| 26 | `roster_reads_the_real_registry_of_the_local_hub` | AC 12, R2. A live hub with a body and an empty registry. Text: today's header, then the six columns; `-` cells; no summary; no `note:`. `--json`: six `null`s. `--format=json`: the envelope with `mismatches` `[]`. `--profile nosuch`: exit 3 and the one `error:` line, or the `profile-not-found` envelope. |
| 27 | `roster_profile_with_server_is_a_usage_error` | AC 13 (a). No hub, `ws://127.0.0.1:1`: exit 2, the one line, empty stdout, in text and `--format=json`. |
| 28 | `roster_server_does_not_read_the_local_registry` | AC 13 (b). The remote-admin rig, from the body's state dir (no hub there). Exit 0, `-` cells, stderr exactly the note, `--json` six `null`s. |
| 29 | `roster_usage_errors_are_clap_text_in_every_format` | AC 11, usage class: `--format=xml` and `--format=json ... --bogus` give exit 2, empty stdout, clap's `error:`. **This is a pin, not a RED: it passes today and must keep passing.** |

**Existing tests changed** (all in the Blast radius; T owns the tests):

- `docs_pane_control_test.rs`:
  - `section_4_marks_the_roster_extension_later` is replaced by `section_4_documents_the_roster_columns` (R9). Section 4 must name each `PANE_COLUMNS` entry as a whole word and have a line with `holler roster` and `--profile`. No line may name `holler roster`, `#648` and `later` together, and no line of section 12 may name `#648`.
  - The guard "no `holler roster --profile`" is deleted from `every_verb_is_named_and_proposed_lines_say_so`, with its doc words.
- `pane_verbs/process/legacy_verbs.rs` and `stub.rs`:
  - `roster_profile_is_refused_with_the_roster_story` is deleted, with its import and the `ROSTER_PROFILE_REFUSAL` constant. It pinned the refusal this story removes.
  - The module doc no longer lists `roster`.
- `Cargo.toml`: one `[[test]]` entry, `roster_panes_test`.

**Sizes.** `roster_panes_test.rs` is 588 lines and `roster_cli_test.rs` is 408. The modules are 148, 66, 155 and 238. All are under lint's 600-line warning.

## RED confirmation

**1. `cargo test -p holler-cli --test roster_cli_test`:** 6 passed, 3 failed. The three new REDs fail on their assertions:

```text
---- roster_profile_with_server_is_a_usage_error ----
assertion `left == right` failed: ["roster","--profile","demo","--server","ws://127.0.0.1:1"]: ... error: not implemented (story #648)
  left: (1, "")
 right: (2, "")
---- roster_reads_the_real_registry_of_the_local_hub ----
assertion `left == right` failed: the new columns after HOLD:
SESSION              HARNESS    MODE     STATE      CONN       HOSTNAME PENDING  LAST TURN            HOLD
  left: ["CONN", "HOSTNAME", "PENDING", "LAST", "TURN", "HOLD"]
 right: ["PANE", "POS", "PROFILE", "HEALTH", "SHOWN", "DRIVEN"]
---- roster_server_does_not_read_the_local_registry ----   (the same header assertion)
```

`roster_usage_errors_are_clap_text_in_every_format` passes. It is a pin.

**2. `cargo test -p holler-cli --test roster_panes_test` and `--test docs_pane_control_test`:** the build fails on exactly the items F adds, and on nothing else. A RED for an API that does not exist can only be a compile error (the #650 precedent):

```text
error[E0432]: unresolved imports `holler_cli::roster_cmd::Registry`, `holler_cli::roster_cmd::PANE_COLUMNS`
error[E0425]: cannot find function `render` in module `roster_cmd`
error[E0061]: this function takes 2 arguments but 4 arguments were supplied      # run's new signature
error[E0308]: mismatched types                                                   # run returns i32, not RosterResult
error[E0432]: unresolved import `holler_cli::roster_cmd::PANE_COLUMNS`           # docs_pane_control_test
```

**3. A scratch stub (reverted): RED at the assertion level.** I added `PANE_COLUMNS`, `Registry`, `render` and the new `run`, all doing exactly today's behaviour: today's table, `--json`'s raw document for either JSON mode, and today's error lines. With that stub, `roster_panes_test` gives **0 passed, 25 failed**. Every failure is an assertion about the missing behaviour. None is a rig, fixture or harness error. Excerpts:

```text
old_columns_...: the golden table          left: "...LAST TURN            HOLD\nalpha ..."  right: "...HOLD                 PANE ... DRIVEN\n..."
two_mismatches_...: the golden table       right starts "2 pane(s) show a different session than the hub drives: hj-c1r2, demo-c1r1\n"
json_keeps_every_key_...: `pane` is present (never left out): {"name":"alpha",...}
format_json_is_...: MissingKey("schema_version")            # no envelope
join::a_row_with_no_label_...:  left: ["idle","connected","host-a","-","-","-"]  right: ["hj-c3r1","r1c3","fleet","healthy","s1","s1"]
profile::profile_lists_only_...: Text  left: ["alpha","demo-c1r1/main","hj-c1r2/build","io/alpha","zz-c2r2/work"]  right: ["demo-c1r1/main","zz-c2r2/work"]
profile::an_unknown_profile_...: left: 0  right: 3
profile::a_malformed_profile_name_...: left: (0, "SESSION ...")  right: (2, "")
registry::any_other_store_error_...: left: (0, "")  right: (0, "note: pane registry unavailable (unavailable)\n")
registry::a_registry_not_read_...: right: (0, "note: pane registry not read (--server names another hub)\n")
registry::a_record_rewritten_...: left: "SESSION ..."  right: "1 pane(s) show a different session than the hub drives: demo-c1r1"
hubless::with_no_hub_...: NotJson: Run { code: 1, out: "", err: "error: no live holler hub reachable at ..." }   # text and --json passed (today's), --format=json fails
hubless::a_server_refused_by_policy_...: NotJson: Run { code: 3, out: "", err: "error: a plaintext ws:// ..." }  # likewise
hubless::profile_with_server_...: Text  left: (3, ("", "error: a plaintext ws:// ...\n"))  right: (2, ("", "error: --profile cannot be combined with --server\n"))
help_names_...: `holler roster --help` names `PANE`
```

**4. The suite can be satisfied (trial, reverted).** I wrote a rough trial implementation in scratch: `render`, `run`, `run_roster` over a deferred `Wiring::connect`, and an `after_long_help`. With it, `roster_panes_test` gave **25 passed** and `roster_cli_test` **9 passed**. `docs_pane_control_test` then fails only `section_4_documents_the_roster_columns`, on the three guide facts R9 changes (the section 4 "later" sentence, the section 12 `#648` bullet, no `holler roster --profile` line). That is its intended RED until F edits the guide.

Then I restored `src/roster_cmd.rs`, `src/main.rs` and `src/cli.rs` from scratchpad copies. `git status` shows no change under `src/`.

The trial exposed one discriminator worth knowing. The `io/alpha` row's CONN cell, `reconnecting`, is wider than its 10 columns, and today's table lets it overflow. A trial that padded HOLD at a fixed byte offset failed `base.txt` on that row. HOLD must be padded **as a cell**.

**Other checks at this phase's tree:**
- `cargo test -p holler-cli --test pane_cli_process`: 35 passed (after the deletion).
- `cargo clippy -p holler-cli --tests -- -D warnings`: clean (run under the trial, the only state in which every target compiles), and clean again on the compiling targets after the revert.
- `bash scripts/lint.sh`: exit 0.
- `bash scripts/changelog-check.sh`: exit 0.
- The new files are formatted with `rustfmt --edition 2021` on those files alone. `roster_cli_test.rs` keeps its existing style.

## Notes for F (decisions the tests fix)

- **Text layout (W-7).**
  - The format is `{:<20} {:<10} {:<8} {:<10} {:<10} {:<8} {:<8} {:<20} {:<20} {:<20} {:<6} {:<12} {:<10} {:<20} {}`, ending in a newline.
  - The columns are today's eight, then HOLD (now padded to 20), PANE, POS, PROFILE, HEALTH, SHOWN, and DRIVEN unpadded.
  - Pad HOLD as a cell: `{:<20}` of `hold_display(row)`. The header uses the same widths.
  - An empty reply still prints `(no sessions on the hub)`.
- **Cells.** Take the cells from `PaneRow::from(&pane).cells()` by each `PANE_COLUMNS` name's position in `pane::list::COLUMNS` (W-5).
- **JSON values.**
  - The values come from `PaneRow`'s fields: `pane` = `name`, `pos`, `profile`, `health`, `shown`, `driven`.
  - A row with no pane gets six `null`s.
  - `--json` stays `{"rows": [...]}`. Its trailing newline is not pinned.
- **Summary line.**
  - Byte format: `{N} pane(s) show a different session than the hub drives: {names joined ", "}`, then a newline.
  - It is text mode only.
  - Names follow printed-row order, with each pane once.
- **Join.**
  - A row joins a pane when the part of `Row.name` before the first `/` equals the pane's name.
  - Under `--profile`, the rows not joined to a pane of P are dropped. The join set is `scope.resolve(P, None)`'s panes, with no `pane_store.list()` beside it.
- **Stderr notes** are printed in all three modes, and stdout is unchanged by them:
  - `note: pane registry unavailable (<code>)` for any store or connect error except `not-implemented`, which prints nothing;
  - `note: pane registry not read (--server names another hub)` for `NotRead`.
- **Errors (W-4).**
  - Exit-2 usage in every mode is `error: <message>` on err with nothing on out. This covers R3's `--profile cannot be combined with --server` (checked first in `run`, before the policy check) and a malformed `--profile` (the `PaneError::Usage` message).
  - Every other error goes through `emit_error`: `Format::Json` under `--format=json`, and `Format::Text` in text and bare `--json`.
  - Hub errors keep today's messages. `NoLiveHub` and the other control errors are `unavailable`.
  - `RemotePolicyRefused` needs an open refusal code that the envelope checker classes as exit 3. Its name is F's; a trial used `roster-server-refused`.
- **The deferred source in `main.rs` (W-1).** This shape compiled in the trial. The `let slot = slot;` line makes the closure `FnOnce`; without it the closure infers `FnMut` and the borrow cannot escape.

  ```rust
  let mut slot: Option<Wiring> = None;
  let slot_ref = &mut slot;
  let source = move || {
      let slot = slot_ref;
      let wiring: &Wiring = slot.insert(Wiring::connect()?);
      Ok(wiring.ports())
  };
  holler_cli::roster_cmd::run(roster, choice, source, &mut sink)
  ```

- **Help.** `holler roster --help` must contain `PANE`, `POS`, `PROFILE`, `HEALTH`, `SHOWN`, `DRIVEN`, `hj-c1r2` and `r2c1`. The trial used `after_long_help` on `Command::Roster`; W-7 also allows the doc comment, or a const in `roster_cmd.rs`.
- **Docs.** `section_4_documents_the_roster_columns` passes once R9's guide edits are made: the section 4 paragraph replaces the "later" sentence, and the section 12 bullet goes. `docs_cli_test` will parse the paragraph's commands.

## Not pinned (left open, as the brief leaves them)

- **W-12, an empty `--profile` result.** What `--profile P` prints when P joins no row is not asked of any test.
- **W-9, the prefix.** `note:` is pinned, as the brief decides.
- **ADR edits (W-3, W-8) and the CHANGELOG entry.** These are F's, with no test beyond `changelog-check.sh`.
- **Evidence appendix.** Phase 6 checks `evidence.md` for the unchanged-source facts these tests rely on (today's widths, `roster_cmd.rs:71-92`; the hub `Row`; `check_envelope`'s rules; the pair rule, `say_cmd.rs:358-364`; `FakeProfileScope`'s members; the policy message).

## Ready for F

Confirmed: RED is valid. Every new test fails on its assertion against today's behaviour (proved with the reverted stub). The only compile errors are the four API items F adds. A trial showed the suite can be satisfied. **T-red complete, RED is valid. F may implement against the authored tests.**
