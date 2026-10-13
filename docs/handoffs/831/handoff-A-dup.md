# Handoff-A-dup: #831 adopt - adopting the fleet's existing Claude Code panes  (anti-duplication gate)

**Date:** 2026-10-12
**Branch:** `issue-831-implementation`
**Diff base:** `23d7ff6` (the third A pass; the cycle's code is `af8709a`..`1f962c0`)   **Diff head:** `1f962c0`
**Reuse map:** the brief's "Reuse map" paragraph (`docs/handoffs/831-adopt-brief.md`)
**Verdict:** PASS

## Summary

PASS: F extended the analogous objects the map named and built no parallel path. The one new module, `import/adopt.rs`, is the sibling the brief asks for (file size) and shares the join with the OpenCode path instead of copying it. Zero `block` findings, 2 `warn`.

## Reuse checked, object by object

| Map item | What the diff does | Verdict |
|---|---|---|
| One `Pane` literal | `assess.rs::record()` is still the only place a `Pane` is built; the OpenCode and Claude paths differ only in a `HarnessPart`. | extended |
| Join start/tail | `assess` split into `begin`/`finish`; `adopt::assess_adopted` calls both; `labelled` made `pub(super)` and reused for the label match; `files.problems` reused. | extended |
| Chain / unreadable finding | `observe.rs::unread_finding` factored out of `tmux_finding`; `adopt::Miss::Unread` reuses it. The chain's `Unread` gains `pub(super)` fields, no second error type. | extended |
| `HarnessPort::shown_session` | Used as is, with `HarnessEndpoint::Process`; no new harness method. | reused |
| Specs | `extend` appends with `spec_from_pane`, the one mapping `profile_from_panes` and `relaunch` use. | reused |
| The undo of `writes.rs` | `Failed`/`undone`/`undo`/`reconcile` generalised (`Write::{Record, Fleet}`, `created: Option<(&Profile, &Actor)>`); `write_records` shared by Create and Extend. No second undo. | extended |
| Remedy text helpers | `single_quoted`, `pane_list` from `profile/delete.rs`, `insert_profile` unchanged for Create. | reused |
| Login variables | `HarnessKind::login_variables()` iterated, not retyped. | reused |
| Name rule | `is_claude_code_command` unchanged, the one rule. | reused |
| Bounded runner / `ps` seam | `exec::ps_group` goes through `exec::run` and the shared `holler_pane::bounded` runner, beside `kill_group`, with `with_ps_binary` mirroring `with_kill_binary`; `tmux::decimal` made `pub(crate)` instead of copied. | extended |
| Listing parse | `tty_listing`/`parse_ttys` follow `foreground_listing`/`parse_foreground` (strict, all or nothing). | extended |
| Wrappers | `Unwired`, `BindingHost`, `Wired<dyn HostPort>` forward the one method, as for every other method; the trait has no default, per the standing rule. | extended |
| `Outcome::claude_code` | Unchanged for `Adoption::NotAsked`; only its words changed. | unchanged |
| Launcher table | `checked` keeps a port-less entry; its checks and texts are otherwise the same; the OpenCode path re-emits `source-missing` in the table's words from `LauncherEntry.about`. | extended |
| Summary / dry-run text | `push_profile` and the summary extend the existing "to create" lines; no second renderer. | extended |

Drift dimensions, as far as the diff touches them: the choke point (`send_prompt`) is untouched; the new `ps` subprocess has its own seam and the shared deadline; the five new finding codes are additive with the closed `ALL`/`code()`/`level()` lists, ADR-0021 and the guide updated in the same change; nothing persists beyond what `write` already did; no secret or value is named in a finding (variable names only). `ports.rs` is 785 lines (under 800); the other touched files are under it.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-pane/src/import.rs:291` | `extend` looks a spec up by pane name and appends, which is the shape of `with_edit` (`crates/holler-cli/src/pane/profile_scope.rs:251`, `SpecEdit::Set`) and its test-kit twin. The semantics differ (the import keeps an existing spec and reports a kind mismatch; `with_edit` replaces), they live in different crates, and `with_edit` is private to the CLI, so this is not a parallel path. | None needed. If a third caller appears, lift a shared "spec for pane" lookup into `holler_pane::profile_snapshot`. |
| 2 | warn | `crates/holler-pane-testkit/src/harnesses.rs:172` | `Lent<'a, T>` is a 12-method forwarding impl of `HarnessPort`, the same shape as `Wired`/`BindingHarness` in the CLI wiring. It is the only borrowing forwarder in the test kit and exists so one router can reach a rig's own `FakeHarness`. A blanket `impl HarnessPort for &T` was not considered in the handoff. | Optional: a blanket impl for `&T` in `holler-pane` would remove the wrapper but widens the public trait surface; leave unless a second borrowing forwarder is needed. |

No duplication; the extension is clean.

## Notes for F

None (PASS).
