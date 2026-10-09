# Handoff-F: Phase 5 - #643 the read verbs (`holler pane list`, `get` and `watch`, with SHOWN and DRIVEN)

**Date:** 2026-10-09
**Branch:** issue-643-implementation (on 837718b, T-red's PASS commit)
**Issue:** #643 (epic #633). Brief: `docs/handoffs/643-brief.md`; prior handoffs: `handoff-A.md` (PASS, 8 warns),
`handoff-T-red.md` (RED valid, 28 failing).

## What was done

- `crates/holler-cli/src/pane/list.rs` (rewrite, 316 lines): `PaneList` (T's field kept, docs written: the long help
  of Decision 15), `run`, and the view code the three verbs share, all `pub` (Decision 2): `PaneRow` (`From<&Pane>`,
  `cells()`), `SessionSync` (`of`, `text`), `text_value`, `json_text`, `observed_at`, and the small shared pieces
  `COLUMNS`, `NO_VALUE`, `profile_name`, `optional_text`, `health_word`, `hold_word`. Private: the `{"panes": [...]}`
  data and the aligned-table renderer.
- `crates/holler-cli/src/pane/get.rs` (rewrite, 275 lines): `PaneGet` (field kept, docs written), `run`, the
  `PaneDetail` data (`pane`, `profile`, `spec`, `sync`), the `--profile` and no-profile lookups of Decision 5, and the
  27-line `key: value` renderer.
- `crates/holler-cli/src/pane/watch.rs` (rewrite, 227 lines): `PaneWatch` (fields kept, docs written), `run`, the
  `PaneChange` data (`cursor`, `name`, `change`, `pane`), the `Changes` iterator over the port's `Watch` (filters,
  idle, error), the `--profile` membership of Decision 7 (`Members`), and the line renderer.
- `CHANGELOG.md`: one `## [Unreleased]` / `### Enhancements` entry for the three verbs, linking #643 (AC 23).
- `docs/handoffs/643/evidence.md` (new), `docs/handoffs/643/decisions.md` (F entry appended), this file.

## Design decisions

- **One table definition.** `COLUMNS` is the column set, and `PaneRow::cells()` yields the cells in that order. The
  `list` table is `COLUMNS` plus the cells of each row. A `watch` put line is `cursor=N put NAME` plus
  `COLUMNS[1..]` lower-cased as `key=value` over the same cells. So the table and the stream cannot drift apart, and a
  later consumer (the roster, #648) gets the same words by calling `cells()`.
- **SHOWN/DRIVEN rule exactly as Decision 3 / AC 3.** `SessionSync::of(&LastObserved)` is the one copy:
  `ok` when both are set and equal, `mismatch` when both are set and differ, `unobserved` otherwise. Its text words
  are `ok` / `MISMATCH` / `-`. A's W-1 (which pair the epic compares) is still the MO's call. If the MO picks SHOWN
  vs `session_of_record`, this function is the one place the code changes, plus AC 3's tests and the help sentence.
- **`text_value` is a superset of Decision 11 (adopts A's W-8).** A value prints unchanged only when it is not
  empty, is not the literal `-`, and has no whitespace, no `"`, `\` or `=`, and no character that could act on a
  terminal. That last class is a control character, or any character Rust's `{:?}` writes as a `\u{..}` escape
  (bidi overrides, zero-width and combining characters, line separators). Anything else prints as `{:?}`. The
  predicate is `acts_on_terminal(c) = c.is_control() || c.escape_debug().nth(1) == Some('u')`, with no table of my
  own. Quoting a literal `-` keeps the help's "a `-` is an empty value" true. Checked with rustc 1.98: `é` and CJK
  pass unchanged; U+202E, U+200B, U+FEFF, U+00A0, U+2028, U+009B and DEL are escaped.
- **`json_text` for the JSON-rendered text fields (Decision 6).** `command`, `probe-check`, `probe-expect`,
  `probe-last`'s `missing` and `spec` are still compact JSON arrays or objects, never joined. `serde_json` escapes
  only the C0 controls, so DEL, the C1 controls (U+009B is a one-character CSI on some terminals) and format
  characters would otherwise reach the terminal raw. `json_text` rewrites exactly the characters `acts_on_terminal`
  flags as JSON `\u` escapes (UTF-16 units). The line stays valid JSON for the same value, and every value in the
  tests is byte-identical to plain `serde_json::to_string`. It is not `text_value`, as Decision 11 requires.
  `serde_json::to_string` cannot fail for these types (no non-string map key, no erroring `Serialize`). The
  `unwrap_or_default()` that would print an empty value is documented as unreachable.
- **`get`'s `profile` is the record's.** `data.profile` and the `profile:` line come from `pane.profile`, not from
  the `--profile` argument, so a pane prints the same value with and without `--profile` (in scope they agree by
  slug). With `--profile`, the pane is the one `resolve` returned, with no second read, and `find` by name raises
  `pane-not-in-profile` for an empty or wrong scope answer (Decision 5's empty case, and the wrong-name case with
  it). Without `--profile`, a `profile_store.get` error fails the verb. A gone profile, or one with no spec for the
  pane, gives `spec: null`.
- **`watch` is an iterator adapter over the port's stream** (`Changes`), handed to `emit_stream`. `Ok(Some)` that
  passes the filters becomes one line. `Ok(None)` ends the stream with `--until-idle`; without it the verb polls
  again, which is not a busy loop because the port's `next()` blocks up to its bound (evidence: `ports.rs:43-52`).
  `Err` becomes the stream's error item, and `emit_stream` ends there. Every refusal (`usage` for names or for
  `--since` ahead of the head, `profile-not-found`, `pane-not-in-profile`) happens in `open`, before the first line.
- **`watch --profile` membership** (`Members`: the profile's slug and a `BTreeSet<PaneName>`). It starts as the names
  `resolve` returned and follows every event: an event prints when its record names the profile (by slug) or when
  the pane was a member as of the last record seen. A pane leaving the profile, or deleted while in it, prints that
  one event. The `--since` replay gap of the brief's Risks is in `--help`.
- **Words by exhaustive `match`, not by serde names**, for role, harness kind, health and hold. A new variant breaks
  the build here instead of printing a stale word.
- **`observed_at`** uses `u64::try_from(ms)`, so there is no sign cast, and prints `never` for `ms <= 0`, else
  `format_epoch(ms / 1000) + " UTC"` (the CLI's only formatter; Decision 12). A parked `since` goes through
  `text_value`, so a date prints quoted (`since="2026-09-21 14:13:20 UTC"`).
- **Help text** is each struct's multi-paragraph doc comment. clap (derive only, no markdown feature) merges each
  paragraph into one line, as the rest of the CLI's help does. JSON shapes are in backticks, so rustdoc does not read
  `[ROW, ...]` as a link, and no line is indented, so no doc comment becomes a doctest.

## Reuse / extend-vs-new

Extended or reused, per the brief's Reuse map: `output::{emit, emit_stream, emit_error, ErrorBody}` (every result,
error and exit code; no printing or exit table of my own), `ProfileOpt` (flattened), `PaneStore::{get, list, watch}`,
`ProfileStore::get`, `ProfileScope::resolve`, `PaneName::parse` / `ProfileName::parse` (typed in `run`, so a refusal is
`usage`), `GridPos` Display and Serialize, the record serde forms (`Pane` verbatim in `get`, `Health` and `Hold` in a
row, `ProfileSpec`, `Cursor`), the closed `PaneError::{PaneNotFound, PaneNotInProfile}`, `time_fmt::format_epoch`, and
`serde_json`.

New, as the brief's Reuse map says, in `list.rs` with one copy each: `PaneRow`, `SessionSync`, `text_value` and
`observed_at`. The other new `pub` items there are the shared pieces the three verbs need, kept with them for the
same reason (the frozen `pane/mod.rs` admits no new module): `json_text`, `profile_name`, `optional_text`,
`health_word`, `hold_word`, `COLUMNS` and `NO_VALUE`. The existing escaping helpers A's W-2 lists were not copied,
and none of them fits: `hub_cmd::printable` is private and replaces with `?`, `holler_proto::log::escape_field_value`
is private and does no quoting, `holds::sanitize_reason` drops characters, `LockoutKey`'s `Display` replaces with
`?`, and `holler_pane::error::excerpt` is `pub(crate)`, always quotes and cuts to 64.

## Architecture notes for A

- Layer: `holler-cli` only, the three verb files. No new module (`pane/mod.rs` untouched), no new dependency, no
  `holler-pane`, hub, proto or golden-file change, no frozen file edited.
- New public surface of the `holler_cli` library: `holler_cli::pane::list::{PaneRow, SessionSync, COLUMNS, NO_VALUE,
  text_value, json_text, observed_at, profile_name, optional_text, health_word, hold_word}`. `get.rs` and `watch.rs`
  depend on `list.rs` (the sibling dependency of Decision 2); nothing depends on `get.rs` or `watch.rs`.
- New public JSON (stable from merge, ADR-0021 section 9). `list`: `{"panes": [PaneRow]}`, with the keys `name`,
  `pos`, `profile`, `project`, `health`, `shown`, `driven`, `sync`, `hold`, and `null` for an empty value. `get`:
  `{"pane": Pane, "profile", "spec", "sync"}`. `watch`: `{"cursor", "name", "change", "pane": PaneRow | null}`. A's
  W-4 (the key `pane` with two shapes) is implemented as the tests pin it.
- Port calls: `list` makes 1 (`resolve`, `get` or `list`); `get` at most 2 (`resolve`, or `get` plus
  `profile_store.get`); `watch` makes `resolve` (with `--profile`), then `watch`, then one `next()` per poll. No
  adapter or probe call (AC 17, both the grep and the call logs).

## Deviations from spec / wireframe

1. **`text_value`'s trigger set is wider than Decision 11's.** It adds the characters Rust's `{:?}` escapes as `\u{..}`
   (A's W-8, "optional and cheap") and the literal `-`. Every AC string is unchanged by it.
2. **The JSON-rendered text fields go through `json_text`** (`serde_json` plus `\u` escapes for DEL, C1 and format
   characters) rather than bare `serde_json::to_string`. They still do not go through `text_value`, as Decision 11
   says. Same JSON value; every AC string is byte-identical.
3. **More `pub` helpers in `list.rs`** than the four Decision 2 names (listed above), all in the same file and used by
   `get.rs` or `watch.rs`.
4. **`get --profile`** raises `pane-not-in-profile` when `resolve` returns no pane *of that name*. That covers
   Decision 5's empty-`panes` case and also a wrong-name answer.

The wireframe does not apply: there is no UI surface.

## Tier 1 self-check (incl. tests now GREEN)

```
$ cargo test -p holler-cli --test pane_verbs -- list:: get:: watch::
test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 60 filtered out; finished in 10.03s

$ cargo test -p holler-cli --test pane_verbs --test pane_cli_process --test cli_surface_test --test docs_cli_test
cli_surface_test:  test result: ok. 3 passed; 0 failed
docs_cli_test:     test result: ok. 3 passed; 0 failed
pane_cli_process:  test result: ok. 34 passed; 0 failed
pane_verbs:        test result: ok. 90 passed; 0 failed

$ rustfmt --check --edition 2021 <the 7 touched .rs files>      -> exit 0
$ cargo clippy -p holler-cli --all-targets -- -D warnings        -> exit 0
$ cargo clippy --workspace --all-targets -- -D warnings          -> exit 0
$ bash scripts/lint.sh                                           -> exit 0 (warnings only, on files this story does not touch)
$ bash scripts/changelog-check.sh                                -> changelog-check: ok
$ cargo machete                                                  -> didn't find any unused dependencies
$ grep -nE 'ports\.(herdr|host|harness|prober)' src/pane/{list,get,watch}.rs     -> nothing (AC 17)
$ grep -nE 'not_implemented|const STORY' src/pane/{list,get,watch}.rs            -> nothing (AC 21)
$ grep -n unsafe <the 7 files>                                                   -> nothing (AC 22)
$ git diff --stat origin/main -- Cargo.toml Cargo.lock 'crates/*/Cargo.toml'     -> nothing (AC 22)
$ wc -l src/pane/{list,get,watch}.rs                                             -> 316, 275, 227
```

`cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load` is CI's form. On this host, its
first run stopped at `holler-cli`'s `logging_test`: 4 cases failed because the host's live hub answered `roster`
(see "Known issues"). The rerun was isolated from that hub:

```
$ HOLLER_STATE_DIR=<empty scratch dir> cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load
125 test-result lines: 1409 passed, 0 failed, 5 ignored (doc-tests included); exit 0
logging_test: test result: ok. 11 passed; 0 failed
```

I also eyeballed the real text output over the fakes in a throwaway scratch program outside the repo. The table is
aligned, with no trailing space. MISMATCH is flagged. A cwd holding ESC, a newline, U+009B and U+202E prints as one
escaped cell, field or `key=value`. An argv holding U+009B and U+202E prints as `["opencode","a\u009bb","x‮y"]`.
A missing pane gives `error: pane not found: demo-c9r9` (exit 3), and `list` of a missing pane prints the header alone.

## Evidence appendix

`docs/handoffs/643/evidence.md`: 11 facts in unchanged code that the diff relies on. They cover envelope
serialization order, the `GridPos` key order, how `emit_stream` ends, the newline added by `write_line`, the `Watch`
idle and error rules, a delete as `pane: None`, the `resolve` contract, the exit class of the two codes the verbs
raise, `format_epoch` being UTC, profile slugs, and the `Health` serde form.

## Tests that look wrong (for T)

None.

## Known issues

- **Workspace suite on a host with a live hub.** `holler-cli`'s `logging_test` runs `holler roster` without
  isolating the state dir and expects exit 1 (no hub). On this host the operator's live hub answers, so 4 of its
  cases fail with "Unexpected success" (`banner_names_resolved_level_and_format`, `debug_flag_beats_env`,
  `env_none_loses_to_flag_noisy`, `log_output_stays_off_stdout`). This is unrelated to this diff, which touches no
  roster or logging code. CI has no hub. `cargo test` also stops at the first failing target, so I re-ran the whole
  workspace with `HOLLER_STATE_DIR` set to an empty scratch dir and `--no-fail-fast` (result above). T-green should
  do the same on this host.
- **JSON mode passes DEL, C1 and format characters raw** (observed in the scratch run: `\u{9b}` and `\u{202e}` in a
  cwd reach stdout as raw bytes under `--format=json`). The envelope is written by `output.rs` with `serde_json`,
  outside this blast radius (#660 owns `output.rs`), and the brief treats JSON as the machine interface. A
  terminal-safe JSON encoding at that one choke point would close it. Raised for the MO; not fixed here.
- **Open warns for the MO** (A's handoff), unchanged by this phase. W-1: the SHOWN/DRIVEN reading; the code keeps
  Decision 3, and one function changes if the MO decides otherwise. W-2: designate `pane::list::text_value` /
  `json_text` as the one terminal-safe helper, or move it to `output.rs` under #660. W-3: one `pane_verbs` rig.
  W-4: `pane` with two shapes. W-5: an empty `watch --until-idle` stream fails `check_ndjson`. W-6: no ADR line for
  I5 per port call.
- **Until #649 wires the hub client**, the installed binary's `pane list`, `get` and `watch` answer
  `error: not implemented` (exit 1) over `Unwired` (C7). The `STUBS` rows were already removed by T, and the CHANGELOG
  entry says so.
- **Decision 7's leave-the-profile event** is implemented (`Members`) but has no AC and no test (T's hedge in
  `decisions.md`).

## Files changed

- `crates/holler-cli/src/pane/list.rs`
- `crates/holler-cli/src/pane/get.rs`
- `crates/holler-cli/src/pane/watch.rs`
- `CHANGELOG.md`
