# Handoff-F: Phase 5 - #643 the read verbs (`holler pane list`, `get` and `watch`, with SHOWN and DRIVEN)  (round 2)

**Date:** 2026-10-09
**Branch:** issue-643-implementation (on 3140f35, T-red's round-2 PASS; F's code unchanged since 06320ad)
**Issue:** #643 (epic #633). Brief: `docs/handoffs/643-brief.md` (unchanged, sha256 62fce781...). Prior handoffs:
`handoff-A.md` (round 2, PASS, 9 warns), `handoff-T-red.md` (round 2, valid, no RED by design), and the outside diff
gate's round 1, `docs/handoffs/643-diff-result-r1.md` (gitignored, BLOCK, 4 BLOCKs).

Round 1 of this handoff is in git: `git show 06320ad:docs/handoffs/643/handoff-F.md`. This file replaces it. This
cycle is the Workflow script's phase 6 (F), after A's round-2 PASS and T's round-2 tests.

## What was done

- **No production change.** A's note 3 says B-1 to B-3 need none, and T-red's round-2 tests pass on round 1's code.
  `git diff 06320ad..HEAD -- crates CHANGELOG.md` lists only T's three test files.
- **`docs/handoffs/643/evidence.md`, rebuilt for the round-2 gate.** It now opens with an
  "F (Phase 5, implement, round 2)" section:
  - "Deviations from the brief": one entry for each of A's four W-9 points (eleven shared `pub` items, `json_text`
    in place of bare `serde_json`, the wider `text_value` trigger set, and bidi characters now escaped in text mode).
  - "The round-1 gate's findings, settled from source": B-1, B-2/W-3, B-3/NV-6/W-1, NV-1/W-2 and NV-5. Each has
    `file:line` and a verbatim excerpt, as A's triage table asks.
- `docs/handoffs/643/decisions.md`: the F round-2 entry. This file.

### Why evidence.md had to be rebuilt, not appended to

The gate does not read the whole file. `dual-review.sh` keeps the evidence appendix up to `DUAL_REVIEW_EVIDENCE_MAX_BYTES`
(default 12,000, not set in `.env`) and drops every line past it. It drops line by line, so a later short line can
still get in. The gate also resolves `file:line` references only in the text it kept. Round 1's gate saw 10,280 bytes,
all of it. T-red's round-2 entries took the file to 12,744, so their last entry (`feed.rs:263-269`) would already have
been partly cut. A round-2 section appended
at the end would never have reached the gate. Also, the prompt's "Feature Handoff" is `(no handoff file)` (round 1's
prompt, line 1290), so this handoff and its deviation list never reach the gate either. That is why A asked for
the facts in `evidence.md`.

The file is now 11,696 bytes, all of it inside the window. To fit it:

1. The round-2 section comes first, so later appends cannot push it out.
2. Facts the brief's Evidence section already quotes verbatim are pointers (`file:line` plus the fact, in one
   paragraph). These were seven of F's round-1 entries (`GridPos`, `emit_stream`, the `Watch` idle rule, the
   `PaneEvent` delete, `resolve`, `format_epoch`, `Health`). Also F's `write_envelope`, `write_line` and exit-class
   entries, which the brief's `output.rs:201-243` quote covers ("JSON mode serializes the data itself", the added
   newline, "1 runtime failure, 2 usage, 3 refusal"). F's slug entry is cut to its two decisive lines.
3. T-green's three entries whose excerpts the brief quotes verbatim (`feed.rs:22-26` in the brief's `feed.rs:18-31`,
   `pane_store.rs:54` in `pane_store.rs:46-60`, and `envelope.rs:243-249`) are one pointer bullet that keeps T's
   wording. Every other T entry is byte-identical. No fact was dropped.

## Design decisions

- **No production change for the gate's BLOCKs**, per A, and the code matches its triage:
  - **B-1:** a profile name embedded in a sentence-shaped `what` is `{:?}`-quoted in five places in merged code: the
    fake scope (the same text as `get.rs:105`), the fake and hub pane stores' `pane-in-other-profile`, and the fake
    and hub profile stores' slug clash. A bare-name `what` is not quoted. Changing `get.rs` alone would make one code
    read two ways.
  - **B-2:** the quoted `-` is the feature. It is what tells a stored `-` apart from the empty value, and no pane or
    profile name can be `-`.
  - **B-3:** in rustc 1.98.1, `escape_debug_ext` names its classes explicitly: control, private use, whitespace,
    grapheme extender, default-ignorable, format control and unassigned. So `acts_on_terminal` already is the
    "explicit and testable predicate" the gate asked for. It is just not spelled out a second time.
- **B-3's evidence is library source**, because `rust-src` is not installed on this host. I fetched
  `library/core/src/char/{methods,mod}.rs` and `library/core/src/fmt/mod.rs` from rust-lang/rust at tag `1.98.1`
  (this host's `rustc 1.98.1 (48a229cea)`) into the scratchpad, read them, and quoted them. Then I checked them against
  the installed compiler. A probe ran the predicate exactly as written in `list.rs:305-307`, and `{:?}`, on 39
  characters: C0, DEL, C1, NBSP, EM SPACE, the soft hyphen, combining marks, ZWSP/ZWNJ/ZWJ, LRM, RLO, LRI, PDI,
  ALM, U+2028/9, BOM, VS16, the Hangul fillers, U+180E, private use and unassigned characters, plus `é`, `ï`, CJK,
  emoji, Greek, Cyrillic and U+FFFC. The two agree on every character. `é`, CJK, emoji and U+FFFC are unchanged. The
  gate's excerpt resolver cannot attach a file outside the repo, but its discipline names "a library source excerpt"
  as evidence that settles a claim.
- **Deviation 2's claim about `serde_json`** is quoted from the locked serde_json 1.0.151 in the local cargo registry
  (`src/ser.rs:2142`, the `UU` row of its `ESCAPE` table). It is one inline line, to save bytes.
- **The four deviation entries carry no excerpt.** They point at lines in the diff, and the evidence rules say
  "nothing the diff already shows".
- **`archChanged: false`.** No production file, module boundary, public interface or dependency changed this
  cycle.

## Reuse / extend-vs-new

No code was written this cycle. Round 1's reuse map stands (`git show 06320ad:docs/handoffs/643/handoff-F.md`,
"Reuse / extend-vs-new"). A's round-2 review checked it again and passed it.

## Architecture notes for A

None. No production change: the layers, the `holler_cli::pane::list` public surface, the JSON shapes and the port
calls are as in 06320ad.

## Deviations from spec / wireframe

Unchanged since round 1. `evidence.md` now carries A's four W-9 points for the gate. Round 1's deviation 4 is not one
of them: `get --profile` also raises `pane-not-in-profile` when `resolve` answers with a pane of another name, as well
as when it answers with an empty `panes`. It is the code path of the B-1 entry, and the round-1 gate did not question
it. The wireframe does not apply: there is no UI surface.

## Tier 1 self-check (incl. tests now GREEN)

```
$ cargo test -p holler-cli --test pane_verbs -- list:: get:: watch::
test result: ok. 35 passed; 0 failed; 0 ignored; 0 measured; 60 filtered out; finished in 10.03s

$ cargo test -p holler-cli --test pane_verbs --test pane_cli_process --test cli_surface_test --test docs_cli_test
cli_surface_test:  test result: ok. 3 passed; 0 failed
docs_cli_test:     test result: ok. 3 passed; 0 failed
pane_cli_process:  test result: ok. 34 passed; 0 failed
pane_verbs:        test result: ok. 95 passed; 0 failed

$ cargo clippy --workspace --all-targets -- -D warnings          -> exit 0
$ bash scripts/lint.sh                                           -> exit 0
$ bash scripts/changelog-check.sh                                -> changelog-check: ok
$ cargo machete                                                  -> didn't find any unused dependencies
$ rustfmt --check --edition 2021 <the 7 touched .rs files>      -> exit 0
$ grep -nE 'ports\.(herdr|host|harness|prober)' src/pane/{list,get,watch}.rs     -> nothing (AC 17)
$ grep -nE 'not_implemented|const STORY' src/pane/{list,get,watch}.rs            -> nothing (AC 21)
$ grep -n unsafe <the 7 files>                                                   -> nothing (AC 22)
$ git diff --stat origin/main -- Cargo.toml Cargo.lock 'crates/*/Cargo.toml'     -> nothing (AC 22)
$ wc -l src/pane/{list,get,watch}.rs                                             -> 316, 275, 227

$ HOLLER_STATE_DIR=<empty scratch dir> cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load
125 test-result lines: 1413 passed, 1 failed, 5 ignored
  failed: body_run_test::fresh_hello_and_presence_on_every_reconnect
          ("hub did not report listening within 10s: Disconnected", after 0.73 s)
$ HOLLER_STATE_DIR=<same> cargo test -p holler-cli --test body_run_test      (3 reruns)
test result: ok. 10 passed; 0 failed   (x3)
```

The one workspace failure is a flake in a target this branch does not touch (`git diff origin/main...HEAD` has no
`body_run_test.rs`, hub or body change). It passes 10 of 10 on each of three reruns. Round 1 ran the same production
code through the same suite with 0 failures. The count went from 1409 to 1414 because T added five `pane_verbs` tests.

## Evidence appendix

`docs/handoffs/643/evidence.md`, 11,696 bytes. The round-2 section is first. It holds four deviation entries and five
gate-finding entries with excerpts: `profile_scope.rs:122-123`, `panes/store.rs:348-350` and `:296-297`, `:311`,
`profile.rs:30` and `:50-51`, `argv.rs:25-27`, `feed.rs:278-282`, and the rustc 1.98.1 library lines. Below it come
the pointers and the entries kept from F round 1, T-green and T-red round 2.

## Tests that look wrong (for T)

None. T's round-2 tests pin the behaviour as built: `text_output_escapes_each_hidden_class_and_keeps_plain_unicode`,
`a_stored_dash_prints_apart_from_the_empty_value`, and the profile-store half of `read_verbs_call_no_adapter_or_probe`.

## Known issues

- **For T-green: evidence.md has 304 bytes of headroom.** The gate reads only 12,000 bytes. If T adds an entry, keep
  `wc -c docs/handoffs/643/evidence.md` at 12,000 or less, for example by turning more brief-quoted facts into
  pointers. Otherwise the tail past the cap is dropped line by line, with only a "[truncated]" marker for the gate and
  a stderr line for the run.
- **For the MO (pipeline): the evidence cap is silent for the author.** Nothing warns F or T when `evidence.md` passes
  `DUAL_REVIEW_EVIDENCE_MAX_BYTES`. In this run, T-red's round-2 entries took it from 10,280 bytes (what round 1's gate
  saw, whole) to 12,744, with no signal. The round-1 prompt shows the
  "Feature Handoff" as `(no handoff file)`, so the handoff's deviation list never reaches the gate either. Both belong
  to the pipeline, and I have filed nothing.
- **JSON mode writes DEL, C1 and bidi characters raw** (`write_envelope`, #660), unchanged. This is A's W-11.
- **Open warns for the MO** (handoff-A.md), unchanged by this cycle: W-1 (the SHOWN/DRIVEN reading, deadline #649), W-2
  and W-10 (one terminal-safe helper and one `--profile` guard, before the second sibling merges), W-3, W-4 (the last
  cheap moment to rename `PaneChange.pane`), W-5, W-6 and W-11.
- **Until #649 wires the hub client,** the installed binary's `pane list`, `get` and `watch` answer
  `error: not implemented` (exit 1) over `Unwired` (C7), as in round 1.

## Files changed

Production: none this cycle. Round 1's four files (`crates/holler-cli/src/pane/{list,get,watch}.rs`, `CHANGELOG.md`)
are unchanged since 06320ad.

Handoff documents (this cycle): `docs/handoffs/643/evidence.md`, `docs/handoffs/643/handoff-F.md`,
`docs/handoffs/643/decisions.md`.
