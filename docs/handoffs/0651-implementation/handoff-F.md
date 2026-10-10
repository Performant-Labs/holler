# Handoff-F: Phase 5 — Implement (attempt 3): implemented, locally GREEN

**Date:** 2026-10-10
**Branch / worktree:** `issue-0651-implementation` (run worktree `<run-worktree>`, repo-relative
`.claude/worktrees/0651-implementation`)
**Contract:** handoff-T-red.md (10 tests in `plugins/herdr-holler/tests/`, 5 RED)

The operator's role-doc unblock worked. This attempt could write `plugins/herdr-holler/**`,
so it implemented the plan from attempts 1–2, with the small corrections listed below. Every
change is inside `plugins/herdr-holler/` (the `src/**` files and the crate `Cargo.toml`). No
test file, nothing in `crates/**`, no root `Cargo.toml`, no `Cargo.lock`, no manifest change.

## What I changed

### `plugins/herdr-holler/Cargo.toml`
- **`serde_json` moved from `[dev-dependencies]` to `[dependencies]`** (comment updated). The
  library builds the request JSON and decodes `PaneReply.data` into `Vec<Pane>`, so it is a real
  dependency now. The tests keep using it through the normal dependency.
- **`[[bin]] name = "herdr-holler", path = "src/main.rs"`.** This is the binary the
  manifest's `[[events]]` hooks run (`["herdr-holler","refresh"]`).
- `holler-hub` is now used (`control::run`, `control::sock_path`, `state::*`):
  `cargo machete` passes.

### `src/herdr.rs` (new): the plugin's own minimal Herdr client (A-warn 1)
Built to satisfy `per_pane_display_facts` and `unknown_never_stale` (the fake Herdr socket),
and to follow the binding A-warn 1 that it mirror the adapter's transport discipline
(`crates/holler-adapter-herdr/src/transport.rs`):
- `enum Method { PaneReport, WorkspaceReport }` is closed. The code can't build any other
  Herdr request, which backs up the allowlist scan.
- One request per connection: `{"id":"holler:<method>","method":…,"params":…}` plus `\n`, then
  read to the first newline or EOF.
- One 2 s deadline covers connect, write and read. They run on a worker thread, and the caller
  waits with `recv_timeout`. Each socket timeout is set to the time left. The macOS
  `InvalidInput` from `set_*_timeout` on a peer-closed socket is tolerated (the adapter's rule;
  the fake Herdr closes right after replying, so the macOS CI runner would hit it).
- Replies are capped at 64 KiB. The reply must be a JSON object that echoes the request id.
  `result.type == "ok"` means it landed.
- `Failure::Refused`: Herdr answered with an error. The other reports still go out, and the
  error code is quoted and cut to 64 chars. `Failure::Fault`: no socket, the deadline ran out,
  or the reply was garbled. The refresh stops, so a wedged Herdr costs one deadline in total,
  not one per pane. No message carries request or reply bytes.

### `src/report.rs` (new): report params
Satisfies `per_pane_display_facts`, `unknown_never_stale` and
`sync_comes_from_shown_differs_and_cites_the_cli_cells`.
- **Pane report:** `{pane_id, source:"holler", state_labels:[], tokens:{pos, project, shown,
  driven, sync, hold}, ttl_ms:120000}`.
  - `pos` is the `GridPos` Display. `project` is `host.cwd`.
  - `shown`/`driven` come from `last_observed`. A missing or empty value renders `-`.
  - `hold` is `none`, `parked` or `drained`.
- **Unknown report:** the same six tokens, all `"unknown"`, plus `state_labels:["unknown"]`
  and `ttl_ms`.
- **Workspace report:** `{workspace_id, source:"holler", tokens:{profile}}`, with **no**
  `ttl_ms` (A-warn 4). There is one per `herdr.workspace` whose panes name a profile. If a
  workspace's panes name several profiles, the token holds the sorted distinct names joined
  with `,`.
- **`sync_word`** mirrors `SessionSync::of` case for case and doc-cites it (A-warn 2). The
  word is `-` unless `last_observed.at > 0` and there is a session of record. After that,
  `shown_differs` decides between `mismatch` and `ok`. This is the only file that names
  `shown_differs`, and no source compares `shown` itself.

### `src/lib.rs` (stubs replaced; the public API is unchanged)
- **`Endpoints::from_env`** (`endpoints_come_from_the_environment`):
  - `HERDR_SOCKET_PATH` is required and must be non-empty, or the call returns `Env`.
  - The hub socket is `control_sock_path(&HubState::from_root(resolve_state_dir()?))`, the
    same resolution `holler_hub::control::run` makes. If the state dir can't be resolved, the
    call returns `Env`.
- **`Reporter::refresh`** (`per_pane_display_facts`, `unknown_never_stale`):
  1. Read the registry with `ControlCall { method: "pane/list", params: None, timeout: 5 s }`
     through `holler_hub::control::run`, then `PaneReply::into_result`, then `Vec<Pane>`.
  2. **Live path:** remember the read's Herdr pane ids, send one pane report per record and
     then the workspace reports, and return `Refreshed{panes, workspaces, false}`.
  3. **Degraded path (any read failure):** record the cause, send one unknown report per
     remembered id and nothing else, and return `Refreshed{n, 0, true}`. The remembered set is
     kept, so repeated failures keep re-sending `unknown`.
  4. **Herdr errors:** return `Err(PlugError::Herdr)` straight away on a fault. If Herdr
     refused some reports, the rest are still sent, and then the call fails, naming the count
     and the first refusal.
- **Hub-socket guard:** `control::run` dials whatever the ambient `HOLLER_STATE_DIR` names,
  and `crates/**` is read-only, so there is no `run_at`. `read_registry` therefore checks
  `control::sock_path() == Endpoints::hub_control_socket` first and takes the degraded path
  on a mismatch. The plugin never shows one hub's panes while naming another.
- **`Reporter::hub_error()`** says why the last read failed, for the binary's stderr line.
  `Refreshed` couldn't gain a field because the tests build it as a struct literal.
- **`resolve_action_pane`** (`resolves_the_registry_pane_by_herdr_pane_id`) returns the name of
  the first record whose `herdr.pane_id` matches, else `None`.
- **`PlugError`** gets `Display` and `std::error::Error`.

### `src/main.rs` (new): `herdr-holler refresh`
- Returns `std::process::ExitCode`; there is no `process::exit`.
- Args are read with `args_os`, so a non-UTF-8 argument can't panic.
- Exit codes:
  - **0** when the reports landed. A degraded refresh also exits 0 and prints one stderr line
    with the hub-read cause and how many panes were reported unknown.
  - **1** on an `Env` or `Herdr` error.
  - **2** on usage.

### Traps the contract's scans set (all respected)
- No `<namespace>.<word>` outside the allowlist in `src/**`. Bindings are named `record`, not
  `pane`, and the manifest file's name never appears in `src/**`.
- No `shown ==` or `!= shown` anywhere, doc comments included.
- `shown_differs` and `SessionSync::of` appear in the same file (`report.rs`).

## Self-check

Every result here is from a run in the run worktree:

- `cargo test -p herdr-holler --no-fail-fast`: **10 passed, 0 failed.**
  - `action_resolution`: 2/2.
  - `display_only_allowlist`: 2/2.
  - `manifest`: 3/3.
  - `report_contract`: 3/3.
  - The 5 tests that were RED at T-red are GREEN. The 5 green-on-contact guards are still
    green.
- `cargo fmt -p herdr-holler -- --check`: clean. rustfmt touched only my new `src` files.
- `cargo clippy -p herdr-holler --all-targets -- -D warnings`: clean. There is no `#[allow]`
  in `src/**`, and no `unsafe`.
- `cargo machete plugins/herdr-holler`: no unused dependencies.
- **Binary smoke test**, temp dirs only, no live hub or Herdr session:
  - no args or an unknown verb: usage, exit 2.
  - `HERDR_SOCKET_PATH` unset: exit 1, naming the variable.
  - Hub absent: exit 0, plus one stderr line ("no live holler hub reachable … 0 pane(s)
    reported unknown").
- **Full workspace:** `cargo test --workspace --no-fail-fast -- --skip
  roster_stays_accurate_under_concurrent_body_load` gave **EXIT 0: 147 test targets, 1762
  passed, 0 failed, 25 ignored, 1 filtered (the configured skip).** That's on a clean rebuild
  of every workspace member; see the next point. clippy and fmt were re-run on that fresh
  build: still clean.
- **The shared build cache was poisoned. This is an environment fault, not a code fault; it
  matters for T-green.**
  - This worktree's `target/` is a symlink to the primary's shared `target/` (config
    `linkDirs`). The first full run here exited **101**: 44 `holler-cli` targets,
    `holler-adapter-host`'s `fake_tmux_test` and `holler-adapter-opencode`'s `attach_test`
    failed.
  - The test binaries embed `env!("CARGO_MANIFEST_DIR")`/fixture paths from the
    **since-deleted `.claude/worktrees/0700-implementation`** (checked with `strings`). The
    shared `holler-pane` rlib was built from later sources too: it has `ProfileSpec.
    opencode_agent`, which this branch doesn't have. Cargo decides freshness by mtime, so a
    sibling run's newer artifacts count as fresh here.
  - Cleaning only the three failing packages made it worse: `holler-cli` then **didn't
    compile** (`missing field opencode_agent`) against the stale `holler-pane`.
  - Fix: `cargo clean -p` for **every workspace member** (the 10 crates under `crates/`, plus
    `herdr-holler`) in the shared cache, then the run above, which is green. Registry
    dependencies were left alone, and nothing tracked was touched. It freed about 37 GiB of
    build cache in total, all of it rebuildable.
  - **Risk for T-green:** if another worktree (e.g. the live `0646-implementation`) builds
    into the shared `target/` before T-green runs, the same false RED can come back. If T-green
    fails in `holler-cli`/adapter crates with paths from another worktree, or with
    `opencode_agent`, it's this cause, not #651's code. Re-run after a `cargo clean -p` of the
    workspace members, or with a private `CARGO_TARGET_DIR`. The lasting fix is for O (and
    possibly Aftersight): a shared `target/` across worktrees on different commits isn't safe
    with mtime freshness.

The authoritative GREEN is still the plugin's own T-green run, not this self-check.

## Tests I think are wrong (if any)

None of the assertions are wrong. These findings carry forward; the operator already
journalled 1–3 as follow-ups:

1. **The actions can't run as pinned.** The manifest actions are bare `["holler","pane",
   <verb>]`. Nothing carries `HERDR_PANE_ID` through `resolve_action_pane` to the CLI.
   `resolve_action_pane` is implemented and tested, but no production code calls it yet. The
   wiring belongs CLI-side (`crates/**`, #649 territory).
2. **A-warn 2 asks for four SYNC cases; the suite covers three.** There's no case with `at > 0`
   and no session of record. The code handles it (`-`), but T may want to pin it.
3. **`unknown_never_stale` holds only in-process.** Each hook runs a fresh process with nothing
   remembered, so a degraded refresh there reports nothing. The 120 s TTL is the only backstop
   (the smoke test shows "0 pane(s) reported unknown"). Closing that gap needs a watch loop or
   a persisted last-reported set.
4. Workspace reports carry no TTL, and a degraded refresh sends none. So a workspace's
   `profile` token stays up while the hub is down. It's a fairly static fact, but it is the one
   token that "unknown, never stale" doesn't cover.

**(new, attempt 3)** Two more, not built because no test asks for them:

5. **A pane that leaves the registry** keeps its last report until the TTL expires, because the
   live path only reports the panes in the current read. Re-reporting dropped ids as `unknown`
   would be a small change if wanted.
6. **Token values are sent verbatim** (`host.cwd`, session names). The CLI makes stored strings
   terminal-safe (`text_value`) before printing them. Whether Herdr's sidebar escapes token
   values is unverified, so a follow-up could apply the same rule here.

## Ready for T(green)

Yes. It is implemented against the RED contract and locally green. The plugin should now run
the suite: GREEN means the loop advances, RED means I rework.
