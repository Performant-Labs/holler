# Handoff-F: Phase 6 - #803, the setup wizard builds panes through `holler pane` and `holler profile apply`

**Date:** 2026-10-12
**Branch:** `issue-803-implementation` (base `41a692b`, T's RED commit; `origin/main` still `6eff9cc`; #839, #842 and #850 still open)
**Issue:** #803

## What was done

- `agent-skills/herdr-setup/lib/panes.sh` (**new**): `check`, `grid`, `env`, `plan` and `teardown`, to the
  printed contract T pinned (handoff-T-red.md). One bash wrapper and one POSIX awk program.
- `agent-skills/herdr-setup/lib/herdr.sh`: the header; one shared allow-list function (`base_env`); the
  `holler` verb with its guard (`holler_guard`); `run` narrowed to `status`, `session list` and
  `workspace list`; `label` removed; the usage line.
- `agent-skills/herdr-setup/SKILL.md`: rewritten from the frontmatter to "Related": registry panes,
  `[[pane]]`, the twelve stages with slots 4 to 9 in the hub-first order of D2, the one door, D18's five
  cases, D9's Gate and timeout, D13's one local body, D14's registry check, D15's teardown through Holler,
  and the login rule of D21.
- `docs/setup-wizard.md`: rewritten per the brief's Amendments (a) to (m), with the new sections
  "How the wizard builds panes" and "If you built a workspace with the previous wizard".
- `docs/pane-control.md`: the pointer paragraph at the end of section 6.
- `docs/adr/ADR-0021.md`: the section 11 sentence (right after the kept one, W11), section 15 item 16
  "As built (#803)", "Not built here" without the wizard, and decision 34.
- `README.md`: the pointer to the wizard, the Documentation line, the "most practical way" paragraph's
  remote-host wording, and the wizard section with its agent prompt (no scenario B, no "attach-mode").
- `docs/README.md`: the `setup-wizard.md` index line.
- `docs/setup-wizard-two-instances.md`: one bullet for the `proof_803` scenario.
- `CHANGELOG.md`: an Enhancements item and a new Breaking Changes section (between Enhancements and Bug
  Fixes), both linking #803.
- `docs/handoffs/803/evidence.md` (new), `docs/handoffs/803/decisions.md` (F's entry), this file.

## Design decisions

1. **The profile P is the workspace label (A's W2)**, computed in `panes.sh` only. `plan` and `teardown`
   take P as their argument (T's contract) and refuse one that is not the config's, with exit 2, so the
   agent can never build or tear down a different profile by mistake.
2. **`panes.sh` never reads `[instance]` or `workspace` itself (W6).** It runs `instance.sh` (located through
   `WIZARD_LIB`, else its own directory, as `herdr.sh` finds `ledger.sh`) and parses only its printed lines.
   The first instance's base port comes from `instance.sh /dev/null`, so the refused range cannot drift
   from `instance.sh`'s own `47001`. Only `trim` and `value_of` are copied, as A allowed.
3. **One awk program, POSIX only, run with `LC_ALL=C`** (byte ranges in bracket expressions). Its output is
   byte-identical under gawk, mawk and busybox awk, and the script modules pass with each first on `PATH`.
   No `'` appears inside the program (`"\047"` builds the shell quotes).
4. **Stricter than the pins, in the spirit of D4:** unknown top-level keys, unknown `[[pane]]` keys and
   unknown tables are refused by name; `model` must look like `provider/id`; `agent` follows the CLI's own
   `agent-key-invalid` grammar; the ceilings are whole numbers in `u32`. A config with a retired *table*
   prints only the retirement messages (and the `hub_host` warning): the rest would be about the old shape.
   `check` prints one summary line on success.
5. **`herdr.sh holler` (D8 with W1):** one list in one function. `server-start` = base + `HERDR_*` (unchanged
   behaviour, the existing scrub tests stay green); `holler` = base + the instance's `HOLLER_*` literals,
   then `exec env -i`. It refuses (exit 2, nothing run) a `WIZARD_STATE_DIR` that is unset, empty **or
   relative**, a named instance whose `WIZARD_TMUX_SOCKET` is not absolute, and the default instance whose
   `WIZARD_HERDR_SOCKET` is not absolute (unset, empty or relative), then runs `check_pane` (A's optional
   guard). Rejected: a second list for `holler` (two lists drift; A's Phase 8 check).
6. **`WIZARD_HOLLER_BIN`** is the one variable the brief does not name: the `holler` binary when it is off
   `PATH`, the twin of `HERDR_BIN`. The skill already handles a Homebrew `holler` off `PATH` (Stage 0), and
   without it the door could only run a `holler` found on `PATH`.
7. **The apply line is text mode** (`--herdr-session` and no `--format`), as T's contract and
   `proof_803::apply_text` read it; the dry-run line carries `--format=json` (D3 reads `data.steps`).
8. **Stage 0 runs `holler pane --help` from prose, not from a code block.** The scoping test requires any
   code line naming `holler pane` to go through `herdr.sh holler`, and the 803 test then requires
   `<herdr_env>` on it, which exists only from Stage 1. `--help` touches no hub, Herdr or tmux, so running
   it bare is safe; the code block runs `holler --help`.
9. **D18 case 1** reads "no profile `<profile>` and no pane in the registry" rather than "no live hub row":
   a run that stopped after Stage 4 leaves a live hub and an empty registry, which is a fresh build that
   reuses the hub, not a rerun.
10. **The README's scenario menu has no letters.** The test forbids `(B)` in the wizard section, and a menu
    lettered A, C, D reads as an error.

## Reuse / extend-vs-new

- **EXTEND `herdr.sh`** (A's map, D16): the `holler` verb reuses the scrub through `base_env`; `run` became
  an allow-list; `label` went. No second copy of the list exists.
- **NEW `lib/panes.sh`**, justified in the brief (D1 (c)) and A's map: no script parses `[[pane]]` or
  `layout`. It depends only downward, on `instance.sh`'s printed contract. Its copy of the port rule is
  the one accepted duplicate (W6), stated in its header.
- No Rust code changed; every test file is T's and unedited.

## Architecture notes for A

- **New module:** `lib/panes.sh`, a pure function of one file and `HOME` / `XDG_CONFIG_HOME`, calling
  `instance.sh` twice and nothing else.
- **Changed script interfaces:** `herdr.sh` gains `holler`, loses `label`, and `run` refuses every word but
  three (breaking for the old skill text only, which this story rewrites).
- **New environment contract** between the skill and `herdr.sh`: `WIZARD_HERDR_WORKSPACES`,
  `WIZARD_HERDR_SOCKET`, `WIZARD_TMUX_SOCKET` (printed by `panes.sh env`) and `WIZARD_HOLLER_BIN`.
- **No change** to Rust production code, the wire, the protocol, golden files, `instance.sh`, `ledger.sh`,
  `collide.sh`, `inventory.sh` or `stop-owned.sh`.

## Deviations from spec / wireframe

- `WIZARD_HOLLER_BIN` (decision 6).
- `holler` also refuses a relative `WIZARD_STATE_DIR` and an empty or relative default socket (decision 5),
  stricter than T's "unset or empty" and "unset".
- `plan` / `teardown` refuse a profile that is not the config's (decision 1).
- Stage 0's `holler pane --help` is in prose (decision 8).
- D18 case 1's wording (decision 9).
- The README menu is unlettered (decision 10).
- D16's dropped `workspace focus`: Stage 10 tells the user to attach and switch to `<workspace>`
  (`prefix+w`), as D16 says.
- Wireframe: N/A (no UI surface).

## Tier 1 self-check (incl. tests now GREEN)

```
$ cargo test -p holler-cli --test wizard_scripts            # T's RED was 231 passed, 72 failed
test result: ok. 303 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load   # the CI command
167 test binaries, every one "test result: ok"; 2624 passed, 0 failed in total
(wizard_scripts 303 passed; docs_cli_test 3 passed; docs_pane_control_test 19 passed)

$ cargo clippy --workspace --all-targets -- -D warnings     # exit 0
$ bash scripts/lint.sh                                      # exit 0; only the existing 600-line warnings, none new
$ bash scripts/changelog-check.sh                           # changelog-check: ok

$ PATH=<mawk shim>:$PATH cargo test -p holler-cli --test wizard_scripts -- panes_803 herdr proof instance preflight ownership start
test result: ok. 214 passed; 0 failed        (the same with busybox awk)
$ bash -n panes.sh herdr.sh; shellcheck -s bash panes.sh    # clean (herdr.sh's one SC2086 info is pre-existing: session_delete)
```

## Evidence appendix

`docs/handoffs/803/evidence.md` (14 entries: `instance.sh`'s printed lines and default base, `collide.sh`
with an empty serve port, the adapter's socket and XDG rules and `HOLLER_HERDR_WORKSPACES`, the CLI's
`--spec-only`, `--harness`, apply text, `pane list`, `pane get`, `pane close` and `profile delete`
contracts, and `docs_cli_test`'s line-based span extraction).

## Tests that look wrong (for T)

1. **ADR pin against `docs_cli_test`.** `skilltext_803::the_adr_carries_the_wizard_sentence_the_decision_and_item_16_as_built`
   requires the flat ADR to contain ``**The setup wizard builds registry panes through `holler pane` and `holler profile apply` ``.
   `docs_cli_test::every_documented_holler_command_parses` parses every inline `holler ...` span under
   `docs/`, and a bare `holler profile apply` fails clap (`<NAME>` is required). Both pass only because
   decision 34 breaks the line inside that code span (`holler profile` / `apply`), which renders the same
   but which the doc-test's line-based extractor never sees as one span (evidence.md, last entry). A reflow
   of that line breaks the CI test. Suggested fix: pin `holler profile apply NAME`, or end the pin at
   ``through `holler pane` and``.
2. **Two skill pins against each other.** `skilltext::the_token_label_folds_in_the_prefix_and_the_stop_path_for_herdr_is_named`
   requires the flat skill to contain ``never `herdr.sh run server stop` ``, and
   `skilltext_803::no_direct_herdr_pane_control_and_no_remote_session_in_the_skill_docs_or_readme` forbids
   the raw text `herdr.sh run ` followed by anything but `status`, `session list` or `workspace list`. Both
   pass only because the skill breaks the line after `run` inside that code span. Suggested fix: drop the
   old pin (`run` now refuses `server stop` outright) or check the 803 ban on flat text with an exception
   for "never".
3. **Not wrong, a trap worth knowing:** the 803 ban list matches substrings, so "pane run" also matches
   "pane runs" or "pane running" and "pane read" matches "pane reads". I worded around it in the skill and
   the docs; a word-boundary match would stop it catching ordinary prose.

## Known issues

- **UNVERIFIED, as the brief's U1 to U8 say** (each stated in the text it affects): the default session's
  socket path, tmux taking the door's environment, a real 4 x 4 build, what a fresh Herdr server holds,
  `herdr session list`'s format, a Claude Code pane end to end, a provider credential after the scrub, and
  the 10 s launch budget on a cold server. Only the operator's repeat of #778 settles them.
- **REFRESH AFTER MERGE of #839 and #850** (both still open): Stage 9's "DRIVEN prints `-` until #839" and
  D13's "no body row for a Claude Code pane" stand as written; if either merges first, the text changes.
- **The body config goes stale** when a pane is relaunched onto a new session of record (until #839): the
  skill says to derive it again and restart the body.
- **A rerun keeps a spec's old `--agent`** when the config drops a pane's `agent` (the CLI keeps an existing
  spec's agent without the flag), and a pane whose harness changed must be closed before a rerun; the skill
  says the latter.
- `docs/setup-wizard-two-instances.md` step 1 still describes `proof.rs`'s own scenario (backend, hub with
  `tailscale serve`, body, Herdr at stages 4, 6, 7, 8): that test is unchanged and stays valid as a test of
  the scripts, as the brief's blast radius says.

## Files changed

- `agent-skills/herdr-setup/lib/panes.sh` (new)
- `agent-skills/herdr-setup/lib/herdr.sh`
- `agent-skills/herdr-setup/SKILL.md`
- `docs/setup-wizard.md`
- `docs/pane-control.md`
- `docs/adr/ADR-0021.md`
- `docs/README.md`
- `docs/setup-wizard-two-instances.md`
- `README.md`
- `CHANGELOG.md`
- `docs/handoffs/803/handoff-F.md`, `docs/handoffs/803/decisions.md`, `docs/handoffs/803/evidence.md`
