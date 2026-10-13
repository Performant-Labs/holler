# Handoff-F: Phase 5 - #831 adopt, adopting the fleet's existing Claude Code panes

**Date:** 2026-10-12
**Branch:** `issue-831-implementation` (worktree `.claude/worktrees/0831-adopt`, head `af8709a` before this phase)
**Issue:** #831

## What was done

- `crates/holler-pane/src/ports.rs`: `HostPort::foreground_pids(name) -> Result<Vec<u32>, PaneError>`, required, with its doc;
  the two `compile_fail,E0046` doctests get the method, and a third (`NoForegroundPids`) omits it.
- `crates/holler-adapter-host/src/lib.rs`, `tmux.rs`, `exec.rs`: `TmuxHost::foreground_pids` (one `list-panes -s` of
  `#{pane_dead} #{pane_tty}`, strict tty parse; then `ps -t <tty> -o pid= -o pgid= -o tpgid=` per live pane, `LC_ALL=C`,
  one shared deadline, all or nothing, sorted and de-duplicated), the `ps: PathBuf` field and `with_ps_binary`,
  `OP_FOREGROUND_PIDS`, `tmux::tty_listing`/`parse_ttys`, `exec::ps_group` beside `kill_group`; the crate docs' errors
  table gains the column.
- `crates/holler-cli/src/pane/wiring.rs`, `wiring/bind.rs`: `Unwired`, `BindingHost` and `Wired<dyn HostPort>` forward it.
- `crates/holler-pane/src/import/findings.rs`: the five codes in `ALL`, `code()`, `level()` (23 kinds); the
  `HarnessClaudeCode` doc reworded.
- `crates/holler-pane/src/import/files.rs`: `LauncherEntry.port: Option<u16>`, new `about: String`.
- `crates/holler-pane/src/import/observe.rs`: `Target.adopt: Adopt` (`Off`, `On(PaneId)`, `OnUnlabelled`),
  `Observed::ClaudeCode(Adoption)`, the hook into `adopt::read`; `unread_finding` factored out of `tmux_finding`; the
  call-count doc rewritten.
- `crates/holler-pane/src/import/adopt.rs` (new, 225 lines): `Adoption`, `Process`, `Miss`, `target` (the label match,
  through `assess::labelled`), `read` (the AC9 chain and stop rule), `assess_adopted` (the D3 rule, the login check,
  `claude-code-adopted`).
- `crates/holler-pane/src/import/assess.rs`: `assess` split into `begin` / `finish` and a `HarnessPart` passed to the one
  `record()`; the port-less `source-missing` on the OpenCode path; the reworded `harness-claude-code` message; `assess_all`
  routes `Adoption::NotAsked` to `Outcome::claude_code` and the other two to `assess_adopted`.
- `crates/holler-pane/src/import.rs`: `ImportRequest.adopt`, `Plan.fleet: FleetPlan` (`None`, `Create`, `Extend { profile,
  generation_read, added }`), the one `ProfileStore::get(fleet)` (flag on and a record to write), `extend` with
  `spec-kind-differs`.
- `crates/holler-cli/src/pane/import.rs`: the flag accepted (`CLAUDE_CODE_STORY` and the refusal gone), `adopt` threaded;
  help and docs; "to extend" text with the added specs; the summary counts adopted panes and says "extended".
- `crates/holler-cli/src/pane/import/writes.rs`: the write matches on `FleetPlan`; Extend writes records first and `fleet`
  last; the undo generalised (`Write::{Record(&Pane), Fleet}`, `created: Option<(&Profile, &Actor)>`, the planned-record
  equality check); the extension's reconcile text.
- `crates/holler-cli/src/pane/import/launcher.rs`: `checked` keeps a port-less entry with a cwd (every other check and
  text unchanged; the relative-cwd check still runs for it).
- `docs/adr/ADR-0021.md`, `docs/pane-control.md`, `CHANGELOG.md`: the amendment (AC1).

## Design decisions

- **`FleetPlan::Extend` keeps `generation_read`** beside `profile.generation` (they are equal at plan time), as the brief
  names the field; the write uses `generation_read`.
- **Specs are appended to the read profile in record order**; existing specs keep their order. Nothing pins an order.
- **`HarnessPart.keeps_agent: bool`** rather than an `Option<AgentKey>` from the caller: the OpenCode caller would have to
  reach into `files.env` for the agent before `record()` checks `files.env` is there. One bool keeps that check in one place.
- **`plan` reads `fleet` when the flag is on and any outcome has a record**, blocked plan or not, so a dry run of a blocked
  run still shows "to extend"; the real run is refused before any write anyway.
- **An extension that adds no spec reports the profile as read** (`Written.profile`), since nothing wrote it.
- **The model warning and the login check run whenever `pane.env` was read**, even if the pane is blocked for another
  reason, so the report lists every reason at once.
- **`ps` parsing:** fields by `split_whitespace` (procps pads them), exactly three, pid and pgid decimal (`tmux::decimal`,
  made `pub(crate)` rather than copied), `tpgid` decimal or `-1`; an empty line fails the answer (T pins it).
- **The tty check rejects a leading `/`** as well (a "relative" name), besides the brief's rules.

## Reuse / extend-vs-new

Extended, per the brief's Reuse map and A's warns 1 and 2: one `record()` (no second `Pane` literal), the join's start and
tail shared (`begin`/`finish`), `labelled` reused for the label match, `join` and `files.problems` reused by the adopt join;
`HarnessPort::shown_session` (no new harness method); `spec_from_pane`/`profile_from_panes`; the one undo of `writes.rs`
generalised (`Failed`/`undone`/`undo`/`reconcile`), no second copy; `single_quoted`/`pane_list`;
`HarnessKind::login_variables()`; `exec::run` and the `kill_group` pattern for `ps_group`; `parse_foreground`'s strictness;
`Outcome::claude_code` unchanged for `NotAsked`. New: `import/adopt.rs`, the sibling module the brief asks for (file size).

## Architecture notes for A

- Layers: `holler-pane` (port trait, import engine), `holler-adapter-host` (a third subprocess, `ps`, behind its own seam and
  the shared bounded runner), `holler-cli` (verb, writes, wiring). No new dependency.
- Public interface changes: `HostPort::foreground_pids` (required: every implementer adds it); `ImportRequest.adopt`;
  `Plan.profile` replaced by `Plan.fleet: FleetPlan` (only `holler-cli` reads it); `LauncherEntry.port` optional and
  `about` added; five `ImportFinding` variants; `TmuxHost::with_ps_binary`.
- No wire/protocol change beyond the five additive finding codes; JSON keys of the verb unchanged.
- The `send_prompt` choke point is untouched.

## Deviations from spec / wireframe

- The brief says "do not write a number" for the two new ADR decisions: they are listed after decision 49 as two bullets
  under a sentence saying each takes the next free number at merge (a numbered item needs a number).
- ADR section 2 has no host-adapter errors table; the errors table with the new `foreground_pids` column and "tmux, `kill`
  or `ps`" is in the crate docs (`crates/holler-adapter-host/src/lib.rs`), and the ADR's section 2 prose and section 5's
  runner paragraph name the third subprocess.
- The guide has no port list; `foreground_pids` is named in section 10 (the fact about the Claude Code panes) and section 3.

## Tier 1 self-check (incl. tests now GREEN)

```
cargo build --workspace --all-targets                         -> ok, no warning
cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load
                                                              -> every binary ok after the docs_cli fix below
  (first run: holler-body rebind_test flaked once, passed in the no-fail-fast run, as T recorded)
cargo test -p holler-cli --test pane_verbs                    -> 455 passed (43 adopt*, harness.rs's 6 #831 cases)
cargo test -p holler-cli --test docs_pane_control_test        -> 23 passed
cargo test -p holler-cli --test docs_cli_test                 -> 3 passed
cargo test -p holler-pane --test import_test                  -> 6 passed
cargo test -p holler-pane --doc                               -> 12 passed (3 HostPort compile_fail)
cargo test -p holler-adapter-host                             -> all ok (foreground_pids 13, bound 6)
cargo test -p holler-adapter-host --test real_tmux_test -- --ignored -> 10 passed (typed-in case on tmux 3.7c, private socket)
cargo clippy -p holler-pane -p holler-adapter-host -p holler-pane-testkit --all-targets -- -D warnings -> clean
cargo clippy -p holler-cli --lib --bins --test pane_verbs --test docs_pane_control_test --test pane_integration
  --test pane_claude --test docs_cli_test -- -D warnings      -> clean
  (holler-cli's token_load_test and roster_cli_test fail clippy on toolchain 1.93 at main; untouched)
cargo fmt --check on the touched files                        -> clean (only those files formatted)
bash scripts/lint.sh                                          -> exit 0 (adapter lib.rs 592, assess.rs 479, ports.rs 785)
bash scripts/changelog-check.sh                               -> ok
grep -rn "CLAUDE_CODE_STORY" crates/holler-cli/src            -> nothing
```

`docs_cli_test::every_documented_holler_command_parses` failed once on a guide line of mine that quoted
`holler pane import --adopt-claude-code` without its required arguments; the line was reworded.

## Evidence appendix

`docs/handoffs/831/evidence.md` (the Claude adapter's pid-only `shown_session`, the router, `is_member`, `Pane`'s
`PartialEq`, the bounded runner's no-spawn-after-deadline).

## Tests that look wrong (for T)

None.

## Known issues

- `cargo fmt -p holler-cli` with the local rustfmt reformats ~100 files the story does not touch (the crate is not clean
  under the local toolchain); those changes were reverted, and only the touched files are formatted.
- The real-store side of the undo's equality check (the hub store keeping a record verbatim apart from `generation`) is
  stated in the guide, as A's warn 5 asked; no test exercises the hub store here.

## Files changed

- `crates/holler-pane/src/ports.rs`
- `crates/holler-pane/src/import.rs`
- `crates/holler-pane/src/import/adopt.rs` (new)
- `crates/holler-pane/src/import/assess.rs`
- `crates/holler-pane/src/import/files.rs`
- `crates/holler-pane/src/import/findings.rs`
- `crates/holler-pane/src/import/observe.rs`
- `crates/holler-adapter-host/src/lib.rs`
- `crates/holler-adapter-host/src/tmux.rs`
- `crates/holler-adapter-host/src/exec.rs`
- `crates/holler-cli/src/pane/import.rs`
- `crates/holler-cli/src/pane/import/launcher.rs`
- `crates/holler-cli/src/pane/import/writes.rs`
- `crates/holler-cli/src/pane/wiring.rs`
- `crates/holler-cli/src/pane/wiring/bind.rs`
- `docs/adr/ADR-0021.md`
- `docs/pane-control.md`
- `CHANGELOG.md`
