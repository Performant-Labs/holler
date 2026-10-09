# Handoff-T-red: Phase 4 - #637 slice a, the `holler-pane` crate (AUTHOR / RED)

**Date:** 2026-10-09
**Branch:** issue-637-implementation (at 868de88)
**Brief / wireframe reviewed:** docs/handoffs/637-brief.md (revision 6); epic #633 contract section; issue #637 (amended 2026-10-09). No wireframe (no UI surface).

## A precondition

Confirmed: A returned PASS on the plan (docs/handoffs/637/handoff-A.md, revision 5, 0 blocks and 13 warns). The operator's decisions on the three warns that change what T writes (env guard codes, the profile `actor`, `PaneStore::delete`) and the folded reviewer warns (`RefusalCode`, methods test home, `Ports` seam test) are in revision 6, and the tests follow revision 6.

## Tests authored

All new tests are Rust, `cargo test`, no browser. Each sits at the cheapest tier: unit-style tests in the `holler-pane` crate's `tests/` (pure types and traits, no process, no socket, no clock) and one in-file module in `holler-proto`.

| File | Test | Pins | Tier |
|---|---|---|---|
| `crates/holler-pane/tests/grid_test.rs` | `grid_parse_table` | AC 2: `r2c1`/`c1r2`/`2,1`; case and ASCII whitespace; bare pair is row,col; the `grid-ambiguous` list (incl. repeated label); the `grid-out-of-range` list incl. zero and `> u16::MAX` (also a 34-digit number); `u16::MAX` accepted | unit |
| | `grid_refuses_interior_whitespace_and_junk` | AC 2: interior whitespace and malformed input are refused | unit |
| | `grid_display_is_row_first_and_round_trips_every_cell_of_a_12_by_12_grid` | AC 2: `parse(format(x)) == x` over 12 x 12; `Display` is `rRcC` | unit |
| | `grid_serde_writes_row_col_pos_in_that_order_and_reads_it_back` | AC 2: exact bytes `{"row":2,"col":1,"pos":"r2c1"}` | unit |
| | `grid_serde_refuses_a_pos_that_disagrees_and_a_zero_cell` | AC 2: a `pos` that disagrees with `row`/`col` is refused; a zero cell is refused through serde too | unit |
| `.../argv_env_test.rs` | `argv_round_trips_as_a_json_array_of_strings` | AC 3: round trip; shell metacharacters stay data | unit |
| | `argv_bare_string_refused` | AC 3: a bare JSON string is refused with `command-not-argv` (in the serde message); other shapes refused | unit |
| | `argv_bare_string_is_refused_where_a_record_stores_a_command` | AC 3: the same refusal at `Pane.command`, `Pane.probe.check`, `ProfileSpec.command`, `ProfileSpec.check` | unit |
| | `env_var_name_accepts_names_and_refuses_values_and_blanks` | AC 3 / decision 2: `=` is `profile-secret-refused`; empty or whitespace is `env-name-invalid` | unit |
| | `env_var_name_serde_is_a_plain_string_with_the_same_refusals` | AC 3: the same codes through serde | unit |
| | `no_profile_spec_or_pane_field_can_hold_an_environment_value` | AC 3: `NAME=value` in `ProfileSpec.env`/`Pane.env` refused, the refusal does not echo the value (secret-absence), a name/value map refused, stored form has no `=` | unit |
| `.../records_test.rs` | `pane_full_round_trips`, `pane_without_profile_loads_as_none`, `pane_variants_round_trip`, `pane_refuses_malformed_fields` | AC 4: `Pane` full / without `profile` / every enum arm and optional / malformed fields | unit |
| | `profile_round_trips`, `profile_has_no_log_field_so_cas_put_cannot_rewrite_history` | AC 4 / decision 7: `Profile`, and it has no `log` field | unit |
| | `profile_spec_round_trips`, `profile_spec_variants_round_trip`, `a_profile_spec_may_name_a_pane_that_does_not_exist` | AC 4: `ProfileSpec`; `pane` is a plain `String` (detached specs) | unit |
| | `cas_put_with_a_stale_generation_is_a_generation_conflict`, `delete_with_a_stale_generation_is_a_generation_conflict_and_a_current_one_removes` | AC 4 / decision 7: stale `cas_put` -> `PaneError::Conflict`, `code()` `generation-conflict`, record unchanged; the same for `PaneStore::delete` (against the in-test `MemPaneStore`) | unit |
| `.../error_test.rs` | `error_codes_unique_and_kebab` | AC 5: `ALL_CODES` unique, each valid, and exactly the epic's 22 closed codes | unit |
| | `is_valid_code_is_the_kebab_case_grammar`, plus a `const _` assertion that it is a `const fn` | AC 5: `^[a-z]+(-[a-z]+)*$` | unit |
| | `every_closed_code_has_a_variant_whose_code_and_display_are_set` | AC 5: every closed code maps to its own variant (never `Refused`), `code()` right, `Display` non-empty | unit |
| | `the_two_not_found_codes_stay_distinct`, `pane_error_is_a_std_error`, `directly_built_variants_report_their_closed_code` | AC 5: `pane-not-found` vs `session-not-found`; `std::error::Error + Send + Sync`; `Timeout{op}`, `Unavailable{what}`, `Conflict`, `NotImplemented` | unit |
| | `refusal_code_validates_what_it_holds` | AC 5: `RefusalCode::parse` rejects an invalid wire code and every closed code; `from_static` usable in a `const` | unit |
| | `a_refused_error_reports_its_own_code_and_survives_the_wire`, `an_unknown_code_parses_to_refused_and_an_invalid_one_to_unavailable` | AC 5: `Refused` not in `ALL_CODES`; unknown valid code -> `Refused`; invalid wire code -> `Unavailable` | unit |
| | `a_pane_error_round_trips_through_a_pane_reply_without_losing_fields`, `a_failure_reply_is_ok_false_with_a_code_and_a_message`, `a_success_reply_carries_data_and_no_error` | AC 5 / decision 8: `PaneError -> PaneReply -> JSON text -> PaneReply -> PaneError` keeps `code()`, `Display` and (via `Debug`) every structured field; reply JSON shape, no `schema_version` | unit |
| `.../names_test.rs` | `pane_name_grammar`, `pane_name_serde_goes_through_the_grammar`, `pane_name_is_a_usable_map_key` | AC 6: `PaneName` agrees with `SessionName::parse` on 25 inputs (reuse, not a copy); serde via the grammar; `Hash`/`Ord` | unit |
| | `profile_name_slug`, `profile_names_that_differ_but_share_a_slug_have_the_same_slug`, `profile_name_refusals`, `profile_name_serde_is_a_plain_string_through_the_same_checks` | AC 6: slug table; same slug for different spellings; refusals (empty, no alphanumerics, control chars, 65 chars); 64 accepted | unit |
| | `actor_is_a_non_empty_name_of_at_most_64_chars` | decision 7: `Actor` | unit |
| `.../ports_test.rs` | `ports_are_send_sync_trait_objects`, `ports_compile_in_test_impl` | AC 6: every port implemented by a double (`PaneStore`, `ProfileStore`, `ProfileScope`, `HerdrPort` incl. `version()`, `HostPort`, `HarnessPort`, `Prober`), each `Send + Sync` | unit |
| | `scope_resolve_with_and_without_a_pane`, `scope_edit_spec_with_and_without_a_profile` | AC 6: `resolve` with and without a pane; `edit_spec` with and without a profile (trait-object `act`), a failing `act` records nothing | unit |
| | `ports_bundle_drives_a_verb_shaped_function` | AC 6 / A row 10: a `Ports` built from the doubles, driven by a function holding `&Ports` and two `&mut dyn Write` whose `edit_spec` `act` calls other ports (the shape #670's verbs use) | unit |
| | `run_probe_stub_never_reports_success` | AC 6: the `run_probe` stub and `SystemProber` must not return `ProbeResult::Ok` | unit |
| `crates/holler-proto/src/methods.rs` (`#[cfg(test)] mod tests`, no production lines touched) | `pane_methods_not_in_wire_catalog` | AC 7: `find("pane/list")`/`find("profile/list")` are `None`, `CATALOG.len() == 22`, `PANE_METHODS` (5, incl. `pane/delete`) and `PROFILE_METHODS` (7) exactly, no name shadows a wire method | unit |
| | `is_pane_method_and_is_profile_method_classify_by_list` | AC 7: the two classifiers | unit |

Totals: 48 tests in `holler-pane` (six files, 1521 lines in all, largest `ports_test.rs`, all under 900), 2 in `holler-proto`.

AC 1 (build, clippy, machete, lint, changelog, rustfmt) and AC 8 (blast radius) are gates, not tests; T checks them at GREEN. Already checked now: every new `.rs` passes `rustfmt --check --edition 2021`, `bash scripts/lint.sh` passes (the allow lines carry `// #637`), the methods.rs edit is rustfmt-clean.

## RED confirmation

The tests can only fail to compile at the crate level, because the crate does not exist. Two facts about that:

1. **A bare `tests/` directory breaks the whole workspace.** `[workspace] members = ["crates/*"]` globs `crates/holler-pane`, which has no `Cargo.toml` yet, so every `cargo` command in the worktree fails with `failed to load manifest for workspace member .../crates/holler-pane`. That is expected until F creates the crate; it is not a defect of the tests. To observe a RED that is about the missing API and not about the missing manifest, T used a throwaway scaffold (an empty-lib `Cargo.toml` + `src/lib.rs`, removed again, `Cargo.lock` restored), and a throwaway stub for the three `methods.rs` items.
2. **Crate-level RED (scaffold: empty `holler-pane` lib).** `cargo test -p holler-pane --no-fail-fast`:
   ```
   error[E0432]: unresolved import `holler_pane::GridPos`                    (grid_test)
   error[E0432]: unresolved imports `holler_pane::Argv`, `EnvVarName`, `Pane`, `ProfileSpec`   (argv_env_test)
   error[E0432]: unresolved imports `holler_pane::PaneError`, `holler_pane::PaneReply`   (error_test)
   error[E0432]: unresolved imports `holler_pane::Actor`, `PaneName`, `ProfileName`   (names_test)
   error[E0432]: unresolved imports `holler_pane::run_probe`, `Actor`, `Argv`, `Cursor`, `HarnessPort`, ... `Watch`   (ports_test)
   error[E0432]: unresolved imports ... `holler_pane::Pane`, `PaneStore`, `Profile`, `ProfileSpec`   (records_test)
   error: could not compile `holler-pane` (test "<each of the six>")
   ```
   Every error is E0432 on an item the feature must define. There is no typo, no missing `[[test]]` (holler-pane keeps default autotests), and no setup error. This is the RED the brief names ("the `holler-pane` tests cannot compile until the crate exists").
3. **Assertion-level RED for `holler-proto`** (stub `PANE_METHODS = &[]`, `PROFILE_METHODS = &[]`, `is_*_method -> false`, the minimum that compiles): `cargo test -p holler-proto --lib methods`
   ```
   test methods::tests::pane_methods_not_in_wire_catalog ... FAILED
     left: []
     right: ["pane/get", "pane/list", "pane/cas_put", "pane/delete", "pane/watch"]
   test methods::tests::is_pane_method_and_is_profile_method_classify_by_list ... FAILED
   test result: FAILED. 0 passed; 2 failed
   ```
   The first draft of the classifier test passed vacuously against empty lists; it now asserts positives (`is_pane_method("pane/cas_put")` etc.), so the stub fails it. Without the stub the committed test does not compile (`PANE_METHODS` not found), which is the true RED.

**The tests are satisfiable and not vacuous.** T wrote a throwaway reference implementation outside the repo (scratchpad, never committed, not handed to F) and ran:
- `cargo test`: 48 of 48 pass; `cargo clippy --all-targets -- -D warnings` is clean for the tests under the workspace lint set (this found and fixed `type_complexity`, `assertions_on_constants`, `iter_cloned_collect`, and `cognitive_complexity` 20/15 in the tests themselves).
- 17 single-line mutations of that implementation (swapped row/col, `pos` ignored, zero accepted, `=` mapped to the wrong code, no trim, no lower-case in the slug, `PaneName` grammar widened, `Argv` accepting a string, `RefusalCode::parse` accepting a closed code, invalid wire code kept as `Refused`, structured fields dropped from the wire, `profile-drift` missing from `ALL_CODES`, validator allowing digits, probe stub returning `Ok`, empty reply message, `Profile.log` added): every mutation is killed by at least one named test.

## API the tests pin (F must match, or amend the test with a note in decisions.md)

Everything is imported from the **crate root** (`holler_pane::X`), a flat re-export in the style of `holler-proto/src/lib.rs`, except the three items the brief names under `error::`: `holler_pane::error::{is_valid_code, ALL_CODES, RefusalCode}`. Where the brief left a shape open, T chose the simplest one; each is cheap to change here and expensive after the freeze, so it is listed.

- **Records.** `Pane`, `Profile`, `ProfileSpec`: `Debug + Clone + PartialEq + Serialize + Deserialize`; pub fields used by tests: `Pane.name: PaneName`, `Pane.generation: u64`, `Pane.hold`, `Pane.profile: Option<ProfileName>`, `Profile.name: ProfileName`, `.slug: String`, `.generation: u64`, `.panes: Vec<ProfileSpec>`, `ProfileSpec.pane: String`. The JSON is the contract section's names; the arms the tests pin: `role` `"agent"|"orchestrator"`; `hold` `"none"`, `{"parked":{"reason","release_when" (string),"since" (i64)}}`, `"drained"`; `health` `"healthy"`, `{"unhealthy":"<reason>"}`, `"unknown"`; `ProbeResult` `"ok"`, `{"failed":{"missing":[..]}}`, `{"error":"<reason>"}`; timestamps are `i64` ms; `session_of_record`/`shown`/`driven` are strings; `harness.kind` `"opencode"`, `pid` and `herdr_api_version` optional, an absent `profile` loads as `None`. Fixtures: `tests/common/mod.rs`.
- **Names.** `PaneName`/`ProfileName`/`Actor`/`EnvVarName`: `parse(&str) -> Result<_, PaneError>`-like (tests use only `is_ok`/`is_err` and, for `EnvVarName`/`GridPos`, `.code()` on the error, so `Result<_, PaneError>` is required there), `as_str()`; `PaneName`: `Clone + Eq + Hash + Ord + Display`; `ProfileName::parse` trims surrounding whitespace and keeps the display form otherwise; `ProfileName::slug() -> String`; serde as a plain string through `parse`; serde errors carry the code text (`command-not-argv`, `profile-secret-refused`, `env-name-invalid`) in their message and never echo a rejected env value.
- **`GridPos`**: pub `row`/`col: u16` and nothing else (`GridPos { row, col }` is built directly); `parse(&str) -> Result<GridPos, PaneError>`.
- **`Argv`**: serde only (no constructor is pinned); **`ProbeResult`**: `Ok`, `Failed { missing: Vec<String> }`, `Error(String)`, `Debug + Clone + PartialEq`; free `run_probe(&Argv, &[String], Duration) -> ProbeResult`; `SystemProber` is a unit struct.
- **`PaneError`** (`Debug + Display + std::error::Error + Send + Sync`): unit variants `NotImplemented`, `Conflict`; struct variants `Timeout { op: String }`, `Unavailable { what: String }`, `Refused { code: RefusalCode, message: String }`; `code(&self) -> &str`. No other variant's shape is pinned; the tests reach every other code through the wire parse-back.
- **`RefusalCode`**: `const fn from_static(&'static str)`, `parse(String) -> Result<_, _>`, `as_str(&self) -> &str`. `is_valid_code` is a `const fn`.
- **`PaneReply`** (T's choice of names; brief fixes only the fields): `PaneReply::success(serde_json::Value)`, `PaneReply::failure(&PaneError)`, `into_result(self) -> Result<Option<serde_json::Value>, PaneError>`; JSON `{ok, data, error:{code, message}}` with `message == err.to_string()`; structured fields travel in an optional extra field so the round trip keeps them; an invalid code on the wire gives `Unavailable`, an unknown valid one `Refused`.
- **Ports.** Method signatures as in the brief's decision 7. Where the brief says only "minimal data types": `HostPort { ensure_session(&PaneName, cwd: &str) -> Result<()>; run(&PaneName, &Argv) -> Result<()>; stop_owned(&PaneName) -> Result<()>; ps(&PaneName) -> Result<Vec<u32>> }`; `HarnessPort { serve(&PaneName, port: u16) -> Result<u32>; health(port) -> Result<bool>; create_session(port) -> Result<String>; list_sessions(port) -> Result<Vec<String>>; abort(port, &str) -> Result<()>; attach_tui(&PaneId, port, &str) -> Result<()>; select_session(&PaneId, &str) -> Result<()>; shown_session(&PaneId) -> Result<Option<String>> }`; `HerdrPort` returns `Result<(), PaneError>` for `send_text`/`send_keys`/`close`, `String` for `read` and `version`; `PaneId` deserializes from a string; `HerdrSpec`, `HerdrPane`, `HerdrSnapshot`, `Key` are only named, never built. `Cursor(pub u64)`. `Watch<T>` as in the brief. Item types `PaneEvent`, `ProfileEvent`, `ProfileLogEntry` are only named. `ResolvedScope { pub profile, pub panes }`. **`SpecEdit::Set(Box<ProfileSpec>)` and `SpecEdit::Remove`**: the `Box` is forced, not a preference: `large_enum_variant` is fatal under `-D warnings` and `ProfileSpec` is far over the 200-byte threshold (the reference crate hit it). `Ports<'a> { pane_store, profile_store, herdr, host, harness, scope, prober }`, pub fields, one `&'a dyn` each, and `Ports<'static>: Send + Sync`.
- All `HostPort`/`HarnessPort` signatures above are T's proposal (the brief gives only the method names); they stay provisional until spikes #636/#635 report, as the brief says. Change them here, in the test, and record it.

## Not covered at RED; owner and how it is checked

- **The `compile_fail` doctest of AC 5** cannot live in `tests/` (doctests run only on library sources) and T may not write production files. It must be added to the doc comment of `PaneError::Refused` / `RefusalCode` in `src/error.rs` as a comment on the type, not a test; T verifies it at GREEN with `cargo test -p holler-pane --doc` and by removing the privacy to see it fail. Text (use the error-code forms; a bare `compile_fail` can pass for the wrong reason):
  ````
  /// ```compile_fail,E0277
  /// use holler_pane::PaneError;
  /// let _ = PaneError::Refused { code: "Not A Code".into(), message: String::new() };
  /// ```
  ///
  /// ```compile_fail,E0080
  /// use holler_pane::error::RefusalCode;
  /// const BAD: RefusalCode = RefusalCode::from_static("Not A Code");
  /// ```
  ````
  This requires `RefusalCode` to have no `From<&str>`/`From<String>`/`Default` and no public field.
- **AC 7 "no golden file changes"**: `bash scripts/golden-diff-summary.sh` at GREEN.
- **Not asserted because the brief leaves them to F** (A rows 2, 4, 6-9): `deny_unknown_fields`, `Profile.slug == name.slug()` on read, a non-ASCII slug, `Argv::from_json`, a shared `generation::cas` helper, the event types' fields, the exact `PaneError` variant list beyond the codes, the `Display` wording. If F adopts any of them, T adds a test at GREEN; none is silently relied on.

## Ready for F

Confirmed: RED is valid. Every holler-pane test target fails to compile on `E0432` for the API the brief defines, and the two `holler-proto` tests fail on assertions against the minimal stub. The tests are satisfiable together (reference implementation: 48 of 48 green, clippy-clean) and every mutation tried is caught. F may implement against them.

Note for F: create `crates/holler-pane/Cargo.toml` first, or no `cargo` command runs in the worktree (see RED confirmation, fact 1). Use `[lints] workspace = true`, no `[dev-dependencies]` are needed (the tests use `serde_json`, already a dependency). `src/lib.rs` must declare all modules named in the brief and re-export per the pinned API above.
