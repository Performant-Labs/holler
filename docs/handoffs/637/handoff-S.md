# Handoff-S: Phase 9 - #637 slice a, the `holler-pane` crate (SPEC AUDIT)

**Date:** 2026-10-09, 04:45 MDT
**Branch:** issue-637-implementation (at af72b6d; base origin/main 27da2da)
**Issue:** #637 (last amended 2026-10-09 03:30 MDT) and the contract section of epic #633 (last edited 03:30 MDT)
**Brief:** docs/handoffs/637-brief.md, revision 6
**Handoffs reviewed:** handoff-A.md (PASS), handoff-T-red.md, handoff-F.md, handoff-T-green.md (PASS), handoff-A-dup.md (PASS), decisions.md, evidence.md
**Verdict:** **REWORK (production).** Two changes are required, and two operator decisions should be asked now.

## A precondition

Met. handoff-A.md reviews brief revision 5 and returns **PASS** with 0 blocks and 13 warns. Revision 6 folds in the warns the operator decided. The Phase 7 anti-duplication gate (handoff-A-dup.md) also returns **PASS**, with 0 blocks and 6 warns.

## T precondition

Met. handoff-T-green.md lists **no blocking issues**.

- **RED is confirmed in handoff-T-red.md:**
  - all six `holler-pane` test targets fail with E0432 on the API the brief defines;
  - the two `methods.rs` tests fail on assertions against a minimal stub.
- **GREEN is confirmed in handoff-T-green.md:**
  - `holler-pane` passes 57 tests and 2 doctests;
  - `holler-proto` passes its 2 `methods` tests;
  - every Tier 1 gate passes.

## Acceptance criteria

| # | Criterion | Proving test / evidence | Status |
|---|---|---|---|
| 1 | build, clippy `-D warnings`, test, machete, lint.sh, changelog-check pass; no file over 900 lines; `[Unreleased]` entry links #637; new `.rs` files rustfmt-clean, no existing file reformatted | T-green's Tier 1 table. `wc -l`: the largest touched file is `error.rs` at 608 lines. The CHANGELOG `### Enhancements` entry links #637. The `methods.rs`, `CHANGELOG.md` and `Cargo.lock` diffs only add lines. | PASS |
| 2 | Grid parsing table, case and whitespace rules, bare pair is row,col, ambiguous and out-of-range lists, 12 x 12 round trip, serde key order, disagreeing `pos` refused | `grid_test.rs`: `grid_parse_table`, `grid_refuses_interior_whitespace_and_junk`, `grid_display_is_row_first_and_round_trips_every_cell_of_a_12_by_12_grid`, `grid_serde_writes_row_col_pos_in_that_order_and_reads_it_back` (exact bytes), `grid_serde_refuses_a_pos_that_disagrees_and_a_zero_cell` | PASS |
| 3 | `Argv` is a JSON array and a bare string is `command-not-argv`; `EnvVarName` is the one guard for env entries (`=` gives `profile-secret-refused`; empty or whitespace gives `env-name-invalid`); no `ProfileSpec` field can hold an env value | `argv_env_test.rs`: `argv_round_trips_as_a_json_array_of_strings`, `argv_bare_string_refused`, `argv_bare_string_is_refused_where_a_record_stores_a_command`, `env_var_name_accepts_names_and_refuses_values_and_blanks`, `env_var_name_serde_is_a_plain_string_with_the_same_refusals`, `no_profile_spec_or_pane_field_can_hold_an_environment_value` | **PARTIAL.** Entries behave correctly. A bare-string `env` (`"env": "TOKEN=hunter2"`) is refused as `usage`, and the reply repeats the value word for word. See REWORK 2. |
| 4 | Round trips of `Pane` (full and without `profile`), `Profile` and `ProfileSpec`; a stale `cas_put` returns `Conflict` with code `generation-conflict` | `records_test.rs`: `pane_full_round_trips`, `pane_without_profile_loads_as_none`, `profile_round_trips`, `profile_spec_round_trips`, `cas_put_with_a_stale_generation_is_a_generation_conflict` (the stale write also leaves the record unchanged), and the matching `delete_with_a_stale_generation_...` test | PASS |
| 5 | `ALL_CODES` unique and kebab-case through `is_valid_code`; every variant's `code()` and `Display` covered; `RefusalCode::parse` rejects invalid and closed codes; `compile_fail` doctest; reply round trip; unknown code parses to `Refused` | `error_test.rs`: `error_codes_unique_and_kebab` (exactly the 22 codes), `every_closed_code_has_a_variant_whose_code_and_display_are_set`, `refusal_code_validates_what_it_holds`, `a_pane_error_round_trips_through_a_pane_reply_without_losing_fields`, `an_unknown_code_parses_to_refused_and_an_invalid_one_to_unavailable`. Doctests at `error.rs:231` (E0080) and `error.rs:385` (E0277). | PASS. See advisory 1 on the doctests. |
| 6 | Every port implemented in-test (`Send + Sync`); `resolve` and `edit_spec` with and without a profile or pane; `run_probe`, `ProbeResult` and `version()` used; `Ports` driven by a verb-shaped function; `pane_name_grammar` and `profile_name_slug` tables | `ports_test.rs`: `ports_are_send_sync_trait_objects`, `ports_compile_in_test_impl`, `scope_resolve_with_and_without_a_pane`, `scope_edit_spec_with_and_without_a_profile`, `ports_bundle_drives_a_verb_shaped_function`, `run_probe_stub_never_reports_success`. `names_test.rs`: `pane_name_grammar` (agrees with `SessionName::parse` on 25 inputs), `profile_name_slug`, `profile_names_that_differ_but_share_a_slug_have_the_same_slug`, `profile_name_refusals`. | PASS |
| 7 | `find("pane/list")` and `find("profile/list")` are `None`; `CATALOG.len()` is 22; the two lists exactly; test inside `methods.rs`; no golden change | `methods.rs` `mod tests`: `pane_methods_not_in_wire_catalog`, `is_pane_method_and_is_profile_method_classify_by_list`. T-green ran `golden-diff-summary.sh` with no drift, and the diff touches no golden file. | PASS |
| 8 | `git diff --name-only origin/main...HEAD` stays inside the Blast radius | All 47 paths are under `Cargo.lock`, `CHANGELOG.md`, `crates/holler-pane/**`, the four skeleton crates, `crates/holler-proto/src/methods.rs` or `docs/handoffs/637*`. | PASS |

## Spec compliance

Each "decision already made" in the brief, checked against the code:

- **Decision 2 (error taxonomy): implemented.**
  - The codes are in `error.rs`, with exactly the closed set of 22.
  - `ALL_CODES`, `code()` and the wire parse-back all come from one private `PaneCode` table.
  - `is_valid_code` is a `const fn` and the only code validator in the workspace (confirmed with grep).
  - `Refused { code: RefusalCode, message }` has a private field. `from_static` const-asserts the grammar and also refuses closed codes. `parse` refuses invalid and closed codes. An invalid code read off the wire becomes `Unavailable`.
  - Each variant's doc comment names its owning story.
  - The brief's `NotFound { what }` is split into `PaneNotFound` and `SessionNotFound`. This follows the issue's "one variant per closed code", which wins over the brief, and the change is recorded.
  - `code()` returns `&str` rather than `&'static str`. Only `&str` can work, because a `RefusalCode` read off the wire is owned (advisory 2).
- **Decision 3: implemented.**
  - The crate depends only on `serde`, `serde_json` and `holler-proto`.
  - `PaneName` is a newtype over `SessionName`, with serde through `parse`.
  - `ProfileName` has the one `slug()`. Slugs are ASCII-only, as A's row 13a recommended, and this is recorded.
  - Every timestamp is `i64` milliseconds.
  - The `GridPos` rules match.
- **Decision 7 (ports): signatures implemented, one silent deviation.**
  - Every signature matches the brief, as do `Cursor(u64)`, `Watch<T>`, `Actor`, `ProfileLogEntry { at, generation, actor, change }`, "no `log` field on `Profile`", `ResolvedScope`, `SpecEdit`, `Prober` with `SystemProber`, and `Ports` with one `&dyn` per port.
  - **Deviation:** the brief says "each trait carries the doc rule 'call from `spawn_blocking` (or a thread) in async code; return within I5's bound (default 10 s) or with `PaneError::Timeout`'". The issue says the ports are "each documented" with it.
    - `ProfileStore` (`profile.rs:321`) and `ProfileScope` (`profile.rs:374`) carry the rule.
    - The five traits in `ports.rs` do not: `PaneStore` (:52), `HerdrPort` (:110), `HostPort` (:136), `HarnessPort` (:152) and `Prober` (:179). Their only copy is the module doc (`ports.rs:8-10`), which an implementer viewing the trait never sees.
    - decisions.md does not record this. → **REWORK 1.**
- **Decision 8 (methods and replies): implemented.**
  - The five pane and seven profile method names sit outside `CATALOG`.
  - `PaneReply { ok, data, error: {code, message, detail?} }` has no `schema_version`. The optional `detail` is the brief's own mechanism for structured fields.
  - On parse-back, a closed code becomes its variant, any other valid code becomes `Refused`, and an invalid code becomes `Unavailable`.
  - Each `*/watch` request gets one `WatchReply { events, cursor }`.
- **Decision 9: implemented.** The `Hold` type's doc says it is a pane-record state, not a prompt hold, and puts any prompt refusal at `send_prompt`.
- **Files and Reuse map: implemented.**
  - Every listed module exists, and `lib.rs` declares all of them.
  - Each of the four skeleton crates has a manifest with `[lints] workspace = true`, no dependencies, and a `lib.rs` with a doc comment only.
  - The `methods.rs` module doc is extended as specified. `vocab.rs`, `holler-proto`'s `lib.rs` and the workspace `Cargo.toml` are untouched.
  - No root re-export collides with a `holler_proto` root name.
- **Epic ruling 7 and AC 3 (one env guard, no pre-scan by #661/#665): only partly met.** See REWORK 2.

## Quality audit

- **Correctness and failure handling**
  - **Fail-closed paths hold:**
    - every CAS goes through `next_generation`: a stale generation is `Conflict`, and an overflow is `store-corrupt`, never a wrap;
    - `deny_unknown_fields` is on every stored record, nested record, params struct and event;
    - a profile is refused on read when its slug does not match its name;
    - a malformed reply is `unavailable`.
  - **Defect: a bare-string `env` repeats its value back.** I checked this in a scratch crate with a path dependency on the worktree; no worktree file was touched.
    - `ProfileSpec` or `Pane` with `"env": "TOKEN=hunter2"` fails with serde's own type error: `invalid type: string "TOKEN=hunter2", expected a sequence`.
    - `decode_params::<ProfileCasPutParams>` maps that error to `usage`.
    - `PaneReply::failure` then sends `{"code":"usage","message":"invalid type: string \"TOKEN=hunter2\", ...","detail":"invalid type: string \"TOKEN=hunter2\", ..."}`.
    - So the value appears twice on the wire, and in any log or terminal that prints the reply.
    - This breaks the crate's own guarantees: `error.rs:19-21` says "none echoes a secret", and `argv.rs:7-12` says the hub and the importer "need no second scan". Ruling 7 forbids #661 and #665 from pre-scanning, so only this crate can close the gap. `Argv` already avoids it by reading a `serde_json::Value` first (`argv.rs:76-81`). → **REWORK 2.**
- **Build guards**
  - Nothing in `src/` calls `unwrap`, `expect`, `panic!` or `unreachable!`.
  - The one `assert!` is the const assertion in `RefusalCode::from_static` (`error.rs:239`). The brief requires it, and the function's doc says that a run-time call with a bad literal panics (advisory 3).
  - All 8 `#[allow]` attributes are in test files, and each has a `// #637` link.
  - No file is at or above 900 lines; `error.rs` is at 608, which only triggers lint.sh's warning.
  - There is no dead code: rustc denies it, and T's clippy run is clean.
- **Protocol**
  - No v2 wire change: `CATALOG` stays at 22 rows, and only control-socket names are added.
  - No golden file changes.
  - `docs/protocol/v2.md` is not touched, by design: ruling 6 keeps these names out of it, and ADR-0021 (#634) documents them.
- **Tests**
  - All tests are unit tests. None needs a cross-process harness, and none uses a process, clock, thread or sleep.
  - T recorded RED first, then GREEN, plus mutation spot-checks: 17 mutations at RED and 5 at GREEN.
- **Documentation**
  - The CHANGELOG `[Unreleased]` entry links #637 and #633.
  - The change adds no log event, CLI surface or v2 field, so README and `docs/` need no update.
- **Public-repository privacy: clean.**
  - I grepped the added lines for host names, tailnet names, IPs, emails, home paths and token shapes, and found nothing.
  - The test host name `kiwi` is the repo's existing placeholder (`holler-body/src/x25519_identity.rs:282`, `holler-cli/tests/attach_mode_test.rs`).
  - The only address is `127.0.0.1`.
  - `gitleaks stdin` over the whole branch diff found no leaks.
- **Commit and PR hygiene**
  - Every subject is a Conventional Commit.
  - Every author is the GitHub no-reply address.
  - Every commit has a `Co-Authored-By` trailer. None has a session link, but neither do any of the last 200 commits on main, and `.githooks/prepare-commit-msg` adds only the trailer. This is the repo's practice, not a defect of this run.
  - The PR is not open yet. After the script opens it, the run's agent must add the AI-disclosure line that `CONTRIBUTING.md` requires.

## Scope check

- **Delivered exactly the brief's scope.** Everything beyond the literal file list is one of two kinds:
  - **Required by the brief:** the params structs, the minimal port data types, `ProfileChange` (the brief's `change` type), and the event types that `Watch<PaneEvent>` needs.
  - **Asked for by A's PASS warns (rows 2, 4 and 6-9, 13) and recorded in decisions.md:** `next_generation`, `decode_params`, `Argv::from_json`, `WatchReply`, `WatchParams`, the unknown-field policy, and the slug check on read.
- **No unrelated refactor.** T-green's `adopted_test.rs` is inside the radius.
- **Under-delivery** is limited to the two REWORK items.

## Verdict

**REWORK (production).** Both changes need edits in `crates/holler-pane/src/`.

1. **`crates/holler-pane/src/ports.rs`: `PaneStore` :52, `HerdrPort` :110, `HostPort` :136, `HarnessPort` :152, `Prober` :179.**
   - **Change:** add the decision-7 doc rule to each trait's own doc comment, in the form `ProfileStore` and `ProfileScope` already use (`profile.rs:321-323`): "**Blocking.** Every method is synchronous. Call from `spawn_blocking` (or a thread) in async code. Every method returns within I5's bound (default 10 s) or with `PaneError::Timeout`. An implementation is `Send + Sync`."
   - **`Prober` wording:** `Prober` returns a `ProbeResult`, not a `Result`, so for it say "returns within the `timeout` it is given (a timeout is `ProbeResult::Error`)".
   - Doc-only.
2. **`crates/holler-pane/src/pane.rs:240-242` (`Pane.env`) and `crates/holler-pane/src/profile.rs:187-189` (`ProfileSpec.env`).**
   - **Change:** refuse an `env` value that is not an array of strings without repeating its content anywhere: not in the message, and not in `detail`.
   - **Code:** a JSON string containing `=` in the `env` position is `profile-secret-refused` (AC 3, ruling 7, I7).
   - **Suggested shape:** a `deserialize_with` helper in `argv.rs`, kept beside `#[serde(default)]`. It reads a `serde_json::Value` first, as `Argv` does at `argv.rs:76-81`.
     - An array of strings runs each element through `EnvVarName::parse`, with the codes unchanged.
     - A string is `profile-secret-refused` if it contains `=`, else `env-name-invalid`.
     - Any other shape is `env-name-invalid`, or `usage` with a fixed message.
     - The field type stays `Vec<EnvVarName>`, so the frozen contract does not change.
   - **Test:** T pins this at GREEN. The test puts `"env": "TOKEN=hunter2"` in a `ProfileSpec` and in a `Pane`, both directly and through `decode_params`. It expects the code `profile-secret-refused`, and expects `serde_json::to_string(&PaneReply::failure(&err))` not to contain `hunter2`.

## Operator decisions (ask now; both freeze when #637 merges; neither blocks REWORK 1 or 2)

The relayed operator request for this run is "Ask for decisions now before I go back to sleep". Both questions come from handoff-A-dup.md. Each has a default that F applies if no answer arrives.

- **D1. How `Watch` signals "nothing happened" (A-dup row 1).** Today `next()` yields `Err(PaneError::Timeout)` when the I5 window passes with no change (`ports.rs:36-46`). A wedged store gives the same error, so a consumer cannot tell idle from hung. The hub's own long-poll, `control/wait`, treats an idle window as a success, not an error. The options:
  - **(a) A-dup's recommendation:** the item becomes `Result<Option<T>, PaneError>`, where `Ok(None)` means idle. This amends the brief's fixed `Watch<T>` alias.
  - **(b) Keep today's behaviour and document it.** Consumers treat `timeout` as "keep polling".
  - **(c) No amendment:** the iterator ends (`None`) when the window passes idle, and the caller calls `watch(last_cursor)` again. `Err(Timeout)` then means only "the store did not answer". This matches `control/wait`, and the empty-iterator test doubles already behave this way.
  - **Default if there is no answer:** (b), which leaves the current code and recorded docs unchanged.
- **D2. `HarnessKind` (A-dup row 5).**
  - **What exists:** the closed enum `HarnessKind { Opencode }` (`pane.rs:117-123`) is a second vocabulary beside `holler_proto::vocab::HARNESS_IDS`, and nothing links the two lists.
  - **(a) Keep the enum (recommended).** It matches the epic's non-goal of no new harness, and it fails closed on a stored record. F records the choice, and T adds one test that every `HarnessKind` serde name is in `HARNESS_IDS`.
  - **(b) Use a `String` checked against `HARNESS_IDS`.**
  - **Default if there is no answer:** (a).

**Recommended in the same pass (no decision needed, not blocking).** Each is cheap now and costly after the freeze:

- **A-dup row 2:** add one doc line to `PaneStore::delete` (`ports.rs:63-66`): a missing record is `pane-not-found`, as #639's amended text decides. Add the same line to `ProfileStore::delete` (`profile.rs:344-350`) with `profile-not-found`. T then makes `MemPaneStore::delete` (`tests/common/mod.rs:148-156`) follow the rule.
- **A-dup row 3:** the doc of `decode_params` (`reply.rs:105-112`) should say that a request without `params` decodes from `{}`, as `holler_proto::typed_params` does.
- **T-green advisory 1:** add a third doctest on `RefusalCode` that `RefusalCode(std::borrow::Cow::Borrowed("Not A Code"))` does not compile. Field privacy is the one frozen invariant that no gate checks; making the field `pub` leaves today's suite green.
- **A-dup row 4 (test-only, T):** route both test doubles' CAS through `holler_pane::next_generation`.

## Advisory notes

1. **Stable rustdoc does not check the error code in `compile_fail,E0xxx`.**
   - **Evidence:** in a scratch crate on 1.98.1 (the pinned stable channel), a block annotated `E0599` that actually fails with E0425 passes.
   - **What the doctests prove:** both doctests in `error.rs` prove only that the code does not compile. F and T confirmed by hand, once, that today they fail for the right reasons (E0277 and E0080). A later edit could leave them passing for a different reason.
   - **What the handoffs say:** T-red and T-green suggest the codes pin the reason; on this toolchain they do not.
   - The privacy doctest suggested above stays meaningful, because privacy is the only thing that would make it compile.
2. **The brief contradicts itself on `code()`.** Decision 2 says `code() -> &'static str` in one place and `code(&self) -> &str` for `Refused` in another. F's `&str` is the only workable reading. This is not a hold.
3. **`RefusalCode::from_static` panics if called at run time** with an invalid literal. The const assertion is required by the brief, and the panic is documented. A `refusal_code!` macro that expands to `const { RefusalCode::from_static(..) }` would force build-time checking at every call site. Optional.
4. **Two codes for an out-of-range grid cell.** A JSON grid cell above `u16::MAX` read through serde is `usage` (serde's `u16` error). A zero cell is `grid-out-of-range`. AC 2 pins only the text parser, so this is optional.
5. **T-green's workspace test count predates its own test file.** The count (911) was taken before `adopted_test.rs` added 9 tests. The `holler-pane` suite and clippy were re-run with that file, and the REWORK pass re-runs everything anyway.
6. **Smaller items.**
   - `HarnessPort::health` returns `bool`, while the record's `Health` can be `Unhealthy(reason)`. This stays provisional until #635.
   - `HerdrSpec` and `SpecHerdr` differ only in word order (A-dup row 6, optional rename).
