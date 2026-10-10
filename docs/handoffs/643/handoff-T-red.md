# Handoff-T-red: Phase 4 - #643 the read verbs (`holler pane list`, `get` and `watch`, with SHOWN and DRIVEN)  (round 2)

**Date:** 2026-10-09
**Branch:** issue-643-implementation (on 04e6af2, A's round-2 PASS; F's code at 06320ad, T-green at 50bcc83)
**Brief / wireframe reviewed:** docs/handoffs/643-brief.md (unchanged, sha256 62fce781...); docs/handoffs/643/handoff-A.md
(round 2); docs/handoffs/643-diff-result-r1.md (the outside diff gate, round 1, gitignored). Wireframe: N/A (no UI surface).

Round 1 of this handoff (the 28-test RED, on fafd138) is in git: `git show 837718b:docs/handoffs/643/handoff-T-red.md`.
This file replaces it.

## A precondition

Confirmed: A returned PASS on round 2 (handoff-A.md: 0 blocks, 9 open warns). The run came back here because the outside
diff gate BLOCKed round 1 and F had reported `archChanged`. A triaged the gate's B-1 to B-3 as not defects and said they
need no production change. It routed the gate's W-4, W-5 and NITs to T. Its note 4 says these findings add no behaviour,
so nothing can be RED this cycle: a test that pins current behaviour passes at once, and that is expected.

## What this cycle changes, finding by finding

| Gate finding | T's call | Change |
|---|---|---|
| B-3, the predicate is "heuristic" and "the test does not pin the exact predicate" | Pin one character per class the gate named | New `text_output_escapes_each_hidden_class_and_keeps_plain_unicode` |
| NV-6 and W-1, "over-escapes `é`" | Pin the opposite: precomposed accents and CJK text print as themselves | The same test |
| B-2 and W-3, a stored `-` is "indistinguishable from the sentinel" | Pin that the two print differently | New `a_stored_dash_prints_apart_from_the_empty_value` |
| W-5, the no-write check covers only the pane store | Agreed. Extend it to the profile store | `read_verbs_call_no_adapter_or_probe` also asserts no `ProfileStoreOp::{CasPut, Delete, Rename}` call, and at least one profile-store read |
| W-4, AC 14 may see `WatchNext` before the verb blocks | No change | The fake logs `WatchNext` on entry to `step()`, before `poll()` waits (feed.rs:263-269, evidence.md). The write lands just before the poll or during the wait. Either way that `next()` returns it once, and once is what the test asserts. The tester overlay requires races to be asserted as invariants, not timings, and the brief allows either order. |
| B-1, `{:?}` in `get`'s `pane-not-in-profile` | No test | `get`'s own construction runs only when the scope answers success without the pane. The fake scope never does that, because it raises the same text itself (profile_scope.rs:122-123). A test through the fake would pin the fake's string, not the verb's. |
| NIT-6, the `data.get(..).is_some()` check is "redundant" | No change; it is not redundant | `serde_json`'s `Index` returns `Null` for a missing key, so `data["profile"] == Null` also holds when the key is absent. The `get().is_some()` check is the one that pins "always present". |
| NIT-1 to NIT-5, NIT-7 | No change | The gate itself calls them not bugs. |

## Tests authored (round 2)

All tests run in-process over the `Rig`, as in round 1. This is the cheapest tier that runs the real verb through clap,
dispatch and `output.rs`, and `text_value` is reached only through it.

| Test | File | Pins |
|---|---|---|
| `text_output_escapes_each_hidden_class_and_keeps_plain_unicode` | `tests/pane_verbs/get.rs:289` | AC 18 and F's deviations 1 and 2, one character per class: DEL, NEL, NBSP, the soft hyphen, ALM, ZWSP, ZWJ, LRM, LRI, PDI, U+2028, the BOM and a combining acute. Each prints as `"/srv/a\u{..}b"`. Precomposed `é` and `ï` and CJK text print unchanged and unquoted, both in a stored string and in an argv printed as JSON. |
| `a_stored_dash_prints_apart_from_the_empty_value` | `tests/pane_verbs/get.rs:355` | Decision 11 as F built it: a stored `-` (`session_of_record`, `last_observed.shown`) prints as `"-"`, and an absent value prints as `-`, both in `get` and in `list`'s SHOWN and DRIVEN cells. |
| `read_verbs_call_no_adapter_or_probe` (extended) | `tests/pane_verbs/list.rs:511-519` | AC 17 and Decision 9: no profile-store write either. |

Expected strings are literals written by T, not values computed with `{:?}`, so the test does not just mirror the code.
rustc 1.98.1 was used to confirm each class's `escape_debug` and `{:?}` form before the literals were written. The probe
was a scratch program outside the repo, and its output matches A's round-2 check.

## RED confirmation

**There is no RED this cycle, by design** (A's note 4). The cycle adds no behaviour, so a valid test passes on F's code.
Validity is shown the other way round: each new test fails when the behaviour it pins is removed. T applied each mutation
to `crates/holler-cli/src/pane/list.rs` with `perl`, ran `cargo test -p holler-cli --test pane_verbs -- list:: get:: watch::`,
and restored the file with `git checkout`:

| Mutation | Result |
|---|---|
| Ma: `acts_on_terminal` becomes `c.is_control() \|\| !c.is_ascii()`, the over-escaping the gate's remediation would cause | 34 passed, 1 failed: `text_output_escapes_each_hidden_class_and_keeps_plain_unicode` panicked at get.rs:341 (the plain-text assertion) |
| Mb: `acts_on_terminal` becomes `c.is_control()` only | 33 passed, 2 failed: the new test (get.rs:332, the hidden-class assertion) and the existing `text_output_escapes_c1_and_bidi_characters` |
| Mc: `text_value` drops `&& value != NO_VALUE` | 34 passed, 1 failed: `a_stored_dash_prints_apart_from_the_empty_value` panicked at get.rs:362 |

The W-5 extension is a guard: no verb writes, so no mutation of the current code can turn it red. Its read half
(`!calls.is_empty()`) proves it is not vacuous.

Unmutated, on the current tree:

```
cargo test -p holler-cli --test pane_verbs -- list:: get:: watch::
test result: ok. 35 passed; 0 failed; 0 ignored; 0 measured; 60 filtered out

cargo test -p holler-cli --test pane_verbs
test result: ok. 95 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

After the mutations, `git status --short` lists only the two test files, so no production file changed.

Static checks on the touched files:

| Check | Result |
|---|---|
| `rustfmt --check --edition 2021` on `tests/pane_verbs/{get,list,watch}.rs` | exit 0 |
| `cargo clippy -p holler-cli --all-targets -- -D warnings` | exit 0 |
| `bash scripts/lint.sh` | exit 0 |
| Test file sizes | get.rs 370, list.rs 545, watch.rs 311, all under 900 |
| Non-ASCII bytes in the test files | none (`grep -P '[^\x00-\x7f]'` finds nothing); every non-ASCII character is a `\u{..}` escape |
| New `#[allow]` | none |
| Secrets | none: neutral `demo-*` and `/srv/...` only |

## Ready for F

The round-2 tests are valid. They pass on F's code and fail when the behaviour they pin is removed. A's note 3 still
applies to F: make no production change for B-1 to B-3. Append the triage facts and the four W-9 deviation entries to
`evidence.md`. T added three evidence entries of its own (fault.rs:90-98, profile_store.rs:27-37 and :250-251,
feed.rs:263-269), so the round-2 gate can check W-4 and W-5 from the excerpts.
