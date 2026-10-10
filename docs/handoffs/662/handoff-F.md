# Handoff-F: Phase 5 - #662a profile verbs: the pure core and the read verbs

**Date:** 2026-10-09
**Branch:** issue-662-implementation (worktree `.claude/worktrees/0662-profile-verbs`, head `5fef0c3`; this run is 662a only.
The Workflow script calls this phase 6.)
**Issue:** #662 (run 662a: the PR says `Part of #662`)

## What was done

- `crates/holler-pane/src/profile_snapshot.rs`: the snapshot.
  - `FIXED_PORT_POLICY_PREFIX` and `fixed_port_policy` (`fixed:<port>`).
  - `spec_from_pane`: the brief's mapping table, written into its doc with the "Not copied" list.
  - `profile_from_panes`: the slug comes from the name, the generation and both stamps are 0, and there is one spec per pane,
    in order.
  - The three signatures #644's pre-flight grep pins are byte-identical.
- `crates/holler-pane/src/profile_diff.rs`: the comparison.
  - `SpecField`: one table for the 15 fields (`ALL` and `as_str`), and `Serialize` goes through `as_str`.
  - `SpecField::value`: returns a typed `FieldValue`. The text of `harness.kind` and `role` comes from serde.
  - `FieldValue` and its `Display`.
  - `FieldDiff`, `PaneStatus` and `PaneDiff`.
  - `is_member`: slugs compared on both sides.
  - `diff_spec`: compares the spec with `spec_from_pane(live)`; `env` and `expect` compare as sets.
  - `diff_profile`: spec rows in profile order, then extra rows in the order of `live`.
- `crates/holler-cli/src/profile/list.rs`: `profile list`.
  - One `profile_store.list()`, then one `pane_store.list()`.
  - Rows are sorted by slug. The live count is the profile's members, by `is_member`.
  - Text is `NAME (SLUG): N panes, L live, generation G`, or `no profiles`. JSON is an array of a derived struct.
  - Also `count`, which pluralizes a noun and which `show` shares.
- `crates/holler-cli/src/profile/show.rs`: `profile show`. `run` replaces the stub:
  - parse NAME (`usage`);
  - `profile_store.get` (`None` gives `profile-not-found`, with the name in `{:?}`);
  - one `pane_store.list()`, filtered by `is_member`;
  - `diff_profile`;
  - each row's `probe` is the member's stored `probe.last`.

  The text form follows "What each verb prints". The JSON is `{"profile", "comparison": [{"pane", "status",
  "differences", "probe"}]}`. T's `ProfileShow { name }` is kept, with a long `--help`.
- `docs/adr/ADR-0003.md`: the `show` row reads `holler profile show NAME`, with `#662` in the same column (67) as its
  neighbours.
- `docs/adr/ADR-0021.md`: Decision 14 (i) and (iii).
  - Section 3 now says `fixed:<port>` and defines "live panes".
  - The two #662 bullets of "Deferred to named stories" are deleted.
- `CHANGELOG.md`: one `### Enhancements` entry under `## [Unreleased]`, linking #662.
- `docs/handoffs/662/evidence.md`: new, with 15 facts.

## Design decisions

1. **`SpecField` serializes through `as_str()`**, with no per-variant serde renames.
   - This keeps T's stub and takes A warn 1: one table of paths.
   - AC 2e's test pins the 15 paths and checks that serde equals `as_str`.
2. **How fields compare.** `diff_spec` takes both sides' values from `SpecField::value`: the spec's, and those of
   `spec_from_pane(live)`.
   - The private `SpecField::differs` compares `env` and `expect` as `BTreeSet<&str>`s, and every other field by
     `FieldValue` equality, so an argv compares in order.
   - Set comparison is tied to those two fields, not to every `List` value. A future list field therefore stays ordered
     unless someone names it.
   - Differences carry the values in stored order.
   - The alternative was a hand-written comparison per field. It would be a second copy of the field table.
3. **Escaping.**
   - **`Text`** escapes control characters only, through `char::escape_default`. This is the brief's rule and the rule of
     `holler_proto::log::escape_field_value` (evidence). #643's `text_value` also quotes bidi and format characters; that
     is the divergence the brief's Follow-up "Shared read-verb text forms" records.
   - **`List` and `Argv`** print as `serde_json`'s compact array. Then any control character left in it is written as a JSON
     `\u` escape. `serde_json` escapes the C0 range itself but writes DEL and the C1 controls raw.
   - The result is still the compact JSON of the same strings that the brief asks for. It also carries the brief's Risks
     goal ("no stored text reaches the terminal raw") over to argv and lists. Without this pass, a C1 CSI (U+009B) in a
     stored argv would reach the terminal.
   - No test pins it: 2f pins ESC and newline in `Text`, and the argv shape.
4. **`serde_text`** gives the text of `harness.kind` and `role`: the JSON string from `serde_json::to_value`, else an empty
   text, never a panic (the brief's wording). It means `profile_diff.rs` has no `"opencode"`, `"agent"` or
   `"orchestrator"` literal, and AC 2e's grep is clean.
5. **`write_json_strings` uses `serde_json::to_string(items).unwrap_or_default()`.** Encoding a list of strings cannot fail.
   The fallback avoids both `unwrap` and returning `fmt::Error` from a `Display`, which would make `to_string()` panic.
6. **`show`'s JSON row** is `ComparisonRow { #[serde(flatten)] diff: PaneDiff, probe }`.
   - The row is flat, `{"pane","status","differences","probe"}`, as the brief fixes, and it reuses `PaneDiff` rather than
     copying its fields.
   - `probe` is looked up by name among the members. A `Missing` row has no member of its name, so it gets `null` without
     a special case. Text mode prints no probe line under a `Missing` row.
7. **The live count in `show`'s header** is `members.len()`, carried in the view with `#[serde(skip)]`.
   - The brief does not put it in JSON `data`.
   - Counting rows would count one member twice if two specs name it.
8. **Plurals.** `count(n, noun)` prints `1 pane` and `1 spec`, and `N panes` and `N specs` for every other N. T left the
   count of 1 to F.
   - It lives in `list.rs` as `pub(crate)`, because the frozen `profile/mod.rs` admits no new module. `show` calls it as
     `super::list::count`, the same pattern #643 uses for its shared view code in `pane/list.rs`.
   - 662b's `create` line ("2 specs, generation 1") can use it.
9. **A `failed` probe with an empty `missing` list** prints `failed`, not `failed (missing )`. Such a result contradicts its
   own doc; the JSON shows the stored form either way.
10. **Order of store calls.** `list` calls the profile store, then the pane store, as the brief says. `show` calls `get`,
    then `list`, so a missing profile is reported without listing any panes. Every store error passes through with its
    own code: `timeout` and `unavailable` exit 1, and are never read as `profile-not-found`.
11. **The long `--help`** on `ProfileList` and `ProfileShow` describes both forms, in the style of #643's `PaneList`. The
    first line (`about`) is unchanged.

## Reuse / extend-vs-new

Per the brief's Reuse map:

- **`output::{emit, ErrorBody, VerbCtx}`.** Every result goes through `emit`. The verbs have no output code beyond their
  text renderers, and their exit codes come from `class_of`, inside `emit`.
- **The closed `PaneError` variants.**
  - Used: `ProfileNotFound`, and `Usage` (raised by `ProfileName::parse`).
  - Store errors pass through unchanged.
  - No `Refused`, no new code, and no edit to `error.rs`.
- **`ProfileName::parse` and `slug()`** give names and membership. There is no second slug rule.
- **The record types, unchanged:**
  - `GridPos`'s `Display` and `Serialize`, through `FieldValue::Grid`;
  - `Argv` and `EnvVarName`;
  - `ProbeResult`'s serde form, which `show` prints in JSON.
- **New, one copy each (the brief's "new here"):**
  - `spec_from_pane`, `profile_from_panes` and `fixed_port_policy`;
  - `is_member`;
  - `diff_spec`, `diff_profile` and their types.
- **Reuse among the new items.**
  - `diff_spec` goes through `spec_from_pane`.
  - `show` reuses `diff_profile`, `is_member`, `PaneDiff` (flattened) and `FieldValue`'s `Display`, which also escapes its
    pane names (A warn 5).
  - `list` reuses `is_member`.
- **Beyond the brief's API:** only private helpers and the verbs' own `data` structs (`ProfileRow`, `ProfileView`,
  `ComparisonRow`).

## Architecture notes for A

- **Layers.** The holler-pane modules are pure: they call no port and do no I/O. The holler-cli verbs take their ports from
  `ctx.ports` and print only through `emit`.
  - New dependency edges: `profile_diff` → `profile_snapshot` inside holler-pane, and the verbs →
    `holler_pane::profile_diff`.
  - No new crate dependency, and no manifest or `Cargo.lock` change. Beyond what was already used, only
    `std::collections::BTreeSet`.
- **Public API.** Exactly the brief's: holler-pane gains no pub item beyond it, and `SpecField::differs` is private. In
  holler-cli, `list::count` is `pub(crate)`.
- **The JSON contract.** The shapes are the brief's table.
  - `list`'s `data` is a bare array, pinned by the brief and by AC 4a and 4c.
  - #643's `pane list` uses an object (`{"panes": [...]}`) "so that a field can be added later without a schema change".
  - ADR-0021 section 9 lets a verb's `data` gain a field without a `schema_version` bump. A bare array admits that only
    inside each row.
  - Not changed here. Recorded for the brief's Follow-up that settles the shared read-verb forms with #643.
- **Escaping.** As in decision 3; the text forms stay divergent from #643's, as the brief records.
- **What the verbs never do.**
  - They call no adapter, scope or prober; AC 3's grep is clean, and the rig's call logs are empty.
  - They read no environment and make no write.
- **Frozen files are untouched:** `profile/mod.rs`, `output.rs`, `wiring.rs`, `holler-pane/src/{lib,profile,pane,error,ports}.rs`,
  every manifest and the test kit.
- **`archChanged: true`.** The two empty holler-pane modules gained a public API, which the profile verbs, #644, #650, #664
  and #665 build on, and there is a new dependency edge between the modules. The API is the one the brief froze and T's
  stubs already declared. This errs on the side the role doc asks for.

## Deviations from spec / wireframe

None in behaviour or shape. Three refinements where the brief is silent, or literal on a form:

1. argv and list text also escapes DEL and the C1 controls (decision 3);
2. a `failed` probe with no missing strings prints `failed` (decision 9);
3. a count of 1 is singular (decision 8).

There is no wireframe: the story has no UI.

## Tier 1 self-check (incl. tests now GREEN)

Run in the worktree after the change (the bodies of T's stubs replaced, nothing of T's edited):

```text
$ cargo test -p holler-pane --test profile_snapshot_test --test profile_diff_test --no-fail-fast
profile_diff_test:     test result: ok. 9 passed; 0 failed
profile_snapshot_test: test result: ok. 5 passed; 0 failed

$ cargo test -p holler-pane
every target ok: unit 0, adopted 9, argv_env 6, error_class 4, error 12, grid 5, names 8, ports 6,
profile_diff 9, profile_snapshot 5, records 11, rework 7, doc-tests 3

$ cargo test -p holler-cli --test profile_verbs --no-fail-fast
test result: ok. 18 passed; 0 failed   (3 list, 9 show, and the 6 untouched stub cases of the other verbs)

$ cargo test -p holler-cli --test cli_surface_test --test docs_cli_test --test pane_cli_process --no-fail-fast
cli_surface_test:  ok. 3 passed
docs_cli_test:     ok. 3 passed      (failed at RED on ADR-0003:68; the NAME row fixes it, AC 9)
pane_cli_process:  ok. 34 passed

$ cargo clippy --workspace --all-targets -- -D warnings
Finished `dev` profile ... (no warning)

$ bash scripts/lint.sh                      -> exit 0 (size warnings only, all for files outside this diff)
$ bash scripts/changelog-check.sh           -> changelog-check: ok
$ rustfmt --check --edition 2021 $(git diff --name-only origin/main -- '*.rs')   -> exit 0 (10 files)
$ grep -nE '^[^/]*"(opencode|agent|orchestrator)"' crates/holler-pane/src/profile_diff.rs        -> no output (AC 2e)
$ grep -nE 'ports\.(herdr|host|harness|prober|scope)|std::env|env::var' <the six AC 3 files>    -> no output (AC 3)
$ git diff origin/main -- '*.rs' | grep -n '^+.*unsafe'                                        -> no output (AC 13)
$ git diff origin/main --stat -- '**/Cargo.toml' Cargo.lock                                     -> no output (AC 13)
$ cargo machete                              -> no unused dependencies
$ grep -n 'Deferred to #662' docs/adr/ADR-0021.md                                               -> no output (AC 10)
$ grep -n 'fixed:<port>' docs/adr/ADR-0021.md                                                   -> lines 154-155 (section 3)
$ grep -c '662),' crates/holler-cli/tests/pane_verbs/process/stub.rs                             -> 2 (AC 8; T's edit)

$ ./target/debug/holler profile list ; ./target/debug/holler profile show Demo --json
error: not implemented                                     (exit 1: the binary's ports are still Unwired, #649)
{"schema_version":1,"ok":false,"data":null,"error":{"code":"not-implemented","message":"not implemented"}}   (exit 1)
$ ./target/debug/holler profile show "   " --json
{"schema_version":1,"ok":false,"data":null,"error":{"code":"usage","message":"invalid profile name \"   \": a profile name must not be empty"}}   (exit 2)
```

```text
$ cargo test --workspace --no-fail-fast
127 test targets: 1404 passed, 4 failed. The one failing target is `-p holler-cli --test logging_test`:
banner_names_resolved_level_and_format, log_output_stays_off_stdout, env_none_loses_to_flag_noisy and
debug_flag_beats_env, each "Unexpected success" on `holler roster` (a hub is reachable from this machine).

$ HOLLER_STATE_DIR=<an empty scratch directory> cargo test -p holler-cli --test logging_test
test result: ok. 11 passed; 0 failed      (with no reachable hub the same 4 pass: environmental, not this diff)
```

## Evidence appendix

`docs/handoffs/662/evidence.md`: 15 facts. A script compared every excerpt with its cited source lines, and all of them
match verbatim. None of those source files changes in this run.

## Tests that look wrong (for T)

None. At Phase 6, T may add these optional cases. None of them is a defect:

1. an argv or env element holding DEL or a C1 control prints escaped (decision 3);
2. a spec or pane count of 1 prints `1 spec` or `1 pane` (decision 8);
3. the probe form `error ("reason")`, with a control character in the reason;
4. a profile-store or pane-store failure in `profile list` passes through with its code.

## Known issues

- **`logging_test`.** 4 cases fail with `Unexpected success` on `holler roster`, because a hub is reachable from this
  machine. The roster path is untouched by this diff, and T saw the same at RED. With `HOLLER_STATE_DIR` pointed at an
  empty directory (no hub), all 11 pass. Re-check it in CI, where no hub is running.
- **A warn 4.** #647 deletes ADR-0021's "periodic reconcile" bullet, which sits next to the port-policy bullet deleted here.
  Whichever of the two merges second resolves a one-line conflict by keeping both deletions.
- **The real binary.** Until #649 wires the hub's stores, the real `holler profile list` and `show` answer
  `not-implemented`, by design.
  - The message changes from `not implemented (story #662)` (the stub's) to `not implemented` (the `Unwired` port's).
  - The exit code (1) and the code are unchanged.

## Files changed

- `crates/holler-pane/src/profile_snapshot.rs`
- `crates/holler-pane/src/profile_diff.rs`
- `crates/holler-cli/src/profile/list.rs`
- `crates/holler-cli/src/profile/show.rs`
- `docs/adr/ADR-0003.md`
- `docs/adr/ADR-0021.md`
- `CHANGELOG.md`

Handoff files: `docs/handoffs/662/handoff-F.md`, `docs/handoffs/662/evidence.md`, and an entry appended to
`docs/handoffs/662/decisions.md`.
