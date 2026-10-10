# Handoff-F: Phase 5 - Implement — BLOCKED (write permission), not implemented

**Date:** 2026-10-10
**Branch / worktree:** `issue-0651-implementation` (run worktree `<run-worktree>`, repo-relative
`.claude/worktrees/0651-implementation`)
**Contract:** handoff-T-red.md (10 tests in `plugins/herdr-holler/tests/`, 5 RED)

## Status: BLOCKED — no production code was written

**F could not write any file under `plugins/herdr-holler/**`.** Both `Write` (new
`src/herdr.rs`) and `Edit` (existing `src/lib.rs`) were refused by the stage's permission
mode. The suite is therefore **unchanged and still RED** (the same 5 failures T recorded).
T-green will see RED, and a rework will hit the same wall unless the cause below is fixed.

**Cause.** `.aftersight/pipeline.config.json` → `paths.production` is
`["crates/*/src/**", "crates/*/Cargo.toml", "Cargo.toml", "Cargo.lock", "scripts/**"]`.
The implement stage's write allowlist follows that list. This story's crate is
`plugins/herdr-holler/` (new directory, A-approved), so its `src/**`, `Cargo.toml` and
`herdr-plugin.toml` are outside every production glob. handoff-A spotted the same mismatch
for `paths.test` (`crates/*/tests/**`) but not the effect on F's write permission.

**Why F did not work around it.** Writing the source through `Bash` (a heredoc or `cp`)
would get around the permission boundary rather than respect it. The brief also rules
Aftersight config out of scope ("no Aftersight/opencode config"), so F cannot widen the
globs either. This needs the operator.

**What unblocks it (operator, pick one):**
1. Add `"plugins/*/src/**"`, `"plugins/*/Cargo.toml"`, `"plugins/*/herdr-plugin.toml"` to
   `paths.production` (and `"plugins/*/tests/**"` to `paths.test`) in
   `.aftersight/pipeline.config.json`. This is an out-of-run config change, so land it on
   `main` or with the operator's explicit sign-off. Then re-run the implement stage.
2. Or grant the implement stage write access to `plugins/herdr-holler/**` for this run only,
   and re-run.

Possible Aftersight improvement (file in Aftersight per the operator's standing rule): when the
brief declares a blast-radius exception, the stage's write allowlist could follow it instead of
only the static `paths.production` globs.

## What I would have changed (the plan, ready for the re-run)

I read the full contract (all 4 test files, the stub `lib.rs`, the manifest) and the seams it
reaches into (`holler_hub::control::{run, ControlCall}`, `holler_hub::state::{resolve_state_dir,
control_sock_path, HubState}`, `holler_pane::{Pane, PaneReply, reconcile::shown_differs}`,
`holler-cli`'s `SessionSync::of`/`hold_word`, the adapter's `transport.rs`/`protocol.rs`
discipline). The implementation is fully designed; only the writes are missing.

- **`Cargo.toml` (crate):** move `serde_json` from `[dev-dependencies]` to
  `[dependencies]`, because the library builds the Herdr request JSON and decodes
  `PaneReply`/`Vec<Pane>`. Add `[[bin]] name = "herdr-holler", path = "src/main.rs"`.
  `holler-hub` becomes consumed (`cargo machete` clean).
- **`src/herdr.rs` (new):** the plugin's minimal Herdr report client (A-warn 1). It mirrors
  the adapter's rules: one request per connection; one deadline (2 s) bounding connect,
  write and read, which run on a worker thread the caller waits for with `recv_timeout`;
  a reply cap (64 KiB); id `holler:<method>`, and the reply must carry that id; success is
  `result.type == "ok"`. Errors name the method, the socket and Herdr's code (quoted, cut to
  64 characters), never request or reply bytes. It returns `Refused` (Herdr answered an
  error: keep sending the rest) or `Failed` (no socket, timeout or garbled reply: abort the
  refresh).
- **`src/report.rs` (new):** report params. Pane: `{pane_id, source:"holler",
  state_labels:[], tokens:{pos,project,shown,driven,sync,hold}, ttl_ms}`. The explicit
  empty `state_labels` clears a previous `unknown` label, since Herdr's merge semantics are
  unverified (the same reasoning as T's re-send of every token). Unknown: the same six
  tokens all `"unknown"`, `state_labels:["unknown"]`, `ttl_ms`. Workspace:
  `{workspace_id, source, tokens:{profile}}` with **no** `ttl_ms`, one report per
  `herdr.workspace` whose panes name a profile. If the panes name more than one, the sorted
  distinct names are joined with `,`, so the token never shows one profile as the
  workspace's when it isn't. `sync_word` mirrors `SessionSync::of` (doc-cited, A-warn 2):
  `-` unless `last_observed.at > 0` and there is a session of record, then
  `shown_differs` → `mismatch`/`ok`. `hold_word` gives `none`/`parked`/`drained`.
  `PANE_TTL_MS = 120_000` (see the decision below).
- **`src/lib.rs`:**
  - `Endpoints::from_env`: `HERDR_SOCKET_PATH` (required, non-empty). The hub socket uses
    the hub's own resolver, `control_sock_path(&HubState::from_root(resolve_state_dir()?))`,
    so it never drifts from what `control::run` dials.
  - `Reporter { endpoints, shown: Vec<PaneId> }`. `refresh` does one
    `ControlCall { method: "pane/list", params: None, timeout: 5 s }` through
    `holler_hub::control::run`, then `PaneReply::into_result`, then `Vec<Pane>`. Any failure
    is the degraded path. Live path: set `shown`, send one pane report per record and one
    workspace report per mapped workspace. Degraded path: one unknown report per remembered
    id. `Err(PlugError::Herdr)` if any report was refused or the exchange failed.
  - `resolve_action_pane` returns the first record whose `herdr.pane_id` matches, as its
    `name`.
  - `Display` and `Error` for `PlugError`.
- **`src/main.rs` (new):** `herdr-holler refresh`. Exit 0 when live. When degraded, exit 0
  with one stderr line naming the hub socket. Exit 1 on an env or Herdr error. Exit 2 on
  usage.

**Things the contract's own scans enforce (traps for whoever writes this):**
- The allowlist scan (`display_only_allowlist.rs`) flags **any** `<namespace>.<word>` in
  `src/**`, Rust field access included. So a binding named `pane` with `pane.herdr`,
  `pane.hold`, etc. fails the test, and so would `session.x`, `workspace.x`, `events.x` or
  `tab.x`. Name the binding `record`.
- The no-second-comparison scan rejects `shown ==`, `shown !=`, `== shown` and `!= shown`
  anywhere in `src/**`, doc comments included. `observed.shown.as_deref()` passed into
  `shown_differs` is fine.
- Any file that contains `shown_differs` must also contain the text `SessionSync::of`.

## Decisions I would have recorded

- **TTL 120 s.** The only triggers are `pane.created`/`layout.updated`, and each one runs a
  one-shot process. Registry facts (SHOWN, DRIVEN, hold) change without any Herdr event, so
  the TTL is what bounds how long a report reads as current. Two minutes trades a sometimes
  empty sidebar for never showing stale facts as current for long. It is a named constant.
- **Refusal vs fault.** A per-report Herdr error (for example a registry pane whose Herdr id
  is gone) does not stop the other panes' reports. A transport fault aborts, so a wedged
  Herdr costs one deadline, not one per pane.

## Self-check

Nothing to run: no production code changed. The suite is in T-red's state:
`cargo test -p herdr-holler`, 10 tests, 5 failing (the stub `Err`/`None`, no `shown_differs`
in `src/`). I did not run cargo, since there is no change to measure.

## Tests I think are wrong (if any)

None are wrong as assertions. Three things for T and the operator:

1. **The actions can't run as pinned.** `every_action_is_a_holler_pane_verb` fixes the
   argv to bare `["holler","pane",<verb>]`. `holler pane switch` needs the `PANE SESSION`
   positionals (`crates/holler-cli/src/pane/switch.rs`), and nothing passes
   `HERDR_PANE_ID` through `resolve_action_pane` to the CLI. Because command[0] must be
   `holler`, a plugin-side wrapper can't do it, so the operand has to be wired CLI-side
   (`crates/**`, outside this run; #649 territory). T's journal already hedges this. It
   needs a follow-up issue.
2. **A-warn 2 asks for four SYNC cases; the suite covers three.** It has ok, mismatch, and
   never observed (`at: 0` together with no session of record). There is no case with
   `at > 0` and no session of record. T may want to add one.
3. **`unknown_never_stale` holds only in-process.** The binary runs once per hook, so a
   fresh process has no "panes shown last time" and a degraded refresh reports nothing.
   The TTL is then the only backstop. The test pins the library, correctly, but the
   production binary doesn't honour the acceptance criterion across invocations. A
   long-lived `herdr-holler watch` loop (the survey's escalation) or a persisted
   last-shown set would close the gap. That's the operator's call; neither is in the
   current contract.

## Ready for T(green)

**No.** Nothing was implemented, because the stage can't write to `plugins/herdr-holler/**`
(see Status). T-green will report the same RED. Fix the write permission first, then re-run
the implement stage; the plan above is complete.
