# Handoff-F: Phase 6 - #642a the OpenCode adapter, server side (`serve`, `health`, `create_session`, `list_sessions`, `abort`, `http::request`)

**Date:** 2026-10-09
**Branch:** issue-642-implementation (on 0ce98a4, T's RED commit)
**Issue:** #642, part 1 of 2 (the PR says `Part of #642`). Brief: `docs/handoffs/642-brief.md`, its 642a AC list only.

## What was done

- `crates/holler-adapter-opencode/src/lib.rs` (483 lines). The crate docs carry what A's W-3 asks for: decisions 1, 2, 13, 14
  (the `workdir` half) and 15(a), and the three TUI methods answering `not-implemented` until part 2. They also carry W-8 (`http`
  is not an interface) and W-2 (no per-pane environment). The file holds `Resolver`, `ProcessEnv`, `Timeouts` (default
  10 s / 5 s / 2 s / 500 ms / 5 s), `OpenCodeConfig` and `OpenCodeHarness`, plus the `HarnessPort` impl:
  - `health`;
  - `create_session` (POST, then the title PATCH; a failed PATCH is followed by a DELETE);
  - `list_sessions` (top-level sessions only);
  - `abort` (the existence check, the abort, then the settle poll of `GET /session/status`);
  - `attach_tui`, `select_session` and `shown_session` answer `PaneError::NotImplemented`.
  It also holds the private `Call` (the per-method deadline, the `HttpError` to `PaneError` mapping and the one-line
  reply-shape message), the percent-encoded id path, `json_of`, `excerpt`, `one_line`, `deadline_after` and `budget`.
- `src/server.rs` (new, 186 lines): `serve` and the shared `healthy` check. `serve` makes one health GET first and never adopts
  a server. It then resolves `workdir`, spawns the server in a process group of its own and polls health only while it boots.
  On the deadline it kills the group and reaps the child; on success a detached thread reaps the child.
- `src/http.rs` (348 lines): the blocking loopback HTTP/1.1 client, which follows the brief's reading rules (see Design
  decisions 3 and 9).
- `src/exec.rs` (new, private, 77 lines): the bounded runner and `kill -s KILL -- -<pgid>`. It holds only what the kill path
  reads (A's W-6).
- `src/tui.rs` (33 lines): docs only. T's stub markers are replaced and the types are unchanged.
- `Cargo.toml`: adds `httparse = { workspace = true }`, with a consumer comment. The description no longer says "empty in the
  workspace skeleton".
- `CHANGELOG.md`: one `[Unreleased]` Enhancements entry linking #642 and epic #633.
- `Cargo.lock`: mechanical. `httparse` is added to `holler-adapter-opencode`'s dependency list and nothing else changes.

## Design decisions

1. **`serve` lives in `src/server.rs` from the start.** The brief names this split itself ("if `lib.rs` nears 600 lines, move
   `serve` and its boot poll to `src/server.rs`"). With it, `lib.rs` is 483 lines and leaves room for part 2's three TUI
   methods. The alternative, one `lib.rs`, would have started near 650 lines, past the 600-line warning.
2. **A message names an id route as `/session/:id`, never the concrete path.** `abort` takes any caller string as the id. A
   concrete, percent-encoded path would echo caller input and could push a message past AC 11d's 200 bytes. With the route
   template and an excerpt of at most 60 bytes, the longest reply-shape message is about 170 bytes; the HTML case measures
   about 140. Rejected: the concrete path, or a truncated id.
3. **`HttpError::Garbled` also covers a connect failure other than a refusal or a timeout**, such as `EMFILE` or
   `EADDRNOTAVAIL`. Its text says which ("cannot connect: ..."). Folding these into `Refused` was rejected. `serve` treats
   `Refused` as "the port is free", so on a full file table it would start a server it could not poll and answer `timeout`
   only after `call`. As built, `serve` answers at once with "cannot serve port N: ...". I widened the variant's doc comment to
   match (see Deviations).
4. **Any answer to `serve`'s first health GET means the port is in use**, not only a healthy one. Something listens there,
   so a new server could not bind. A healthy answer gives "port N is in use: a harness server already answers there";
   another answer names the status and quotes the body.
5. **A healthy boot answer counts only while the child still runs.** `try_wait` runs after each health try. If the child has
   exited, `serve` answers `unavailable` with its status, even when something else answered healthy, so `serve` never returns
   a pid for a server that is not its own.
6. **The kill on the deadline** runs `kill -s KILL -- -<pgid>` (bound 1 s), then `Child::kill` as a fallback for the direct
   child, then `wait`. A blocking `wait` can therefore not hang if the group kill failed. `kill_group` refuses a pgid of 0 or 1,
   because `kill` reads `-0` as its caller's own group and `-1` as every process it may signal.
7. **The DELETE after a failed title PATCH runs within what is left of the call.** I5 holds. When nothing is left, the
   untitled session stays and reconcile (#647) sees it as a stray. Rejected: a grace period past the deadline, which would
   break the port's bound.
8. **Request heads** carry `Host: 127.0.0.1:P`, `Accept: application/json` and `Connection: close`. A body adds
   `Content-Type: application/json`. A `POST`, `PUT` or `PATCH` without a body (the abort) sends `Content-Length: 0`, as
   RFC 9110 section 8.6 asks of a method that defines content.
9. **Bounds on what is read.** A head may carry at most 64 headers, and a reply may be at most 64 MiB (head and body);
   `TooManyHeaders` or a longer reply is `Garbled`. A declared `Content-Length` over the cap fails before any read. A
   `Content-Length` must be all digits (`+5` is refused), and two different values are `Garbled`. A `204` or `304` has no
   body.
10. **The excerpt:** at most 60 bytes of the body, decoded lossily, control characters as spaces, cut at a character boundary,
    then `...` when the body was longer. `one_line` also cleans every other external text a message quotes (an `io::Error`,
    a path, a `Garbled` reason).
11. **`parentID`:** only a string marks a child session, as the brief says. A missing or `null` value, or any other type,
    is top-level. An element without a string `id` makes the whole list `unavailable`.
12. **`deadline_after` caps a bound at one year**, so a `Timeouts` value of `Duration::MAX` cannot overflow an `Instant`
    (which panics).
13. **`http` is not `#[doc(hidden)]`.** A's W-8 made that optional. The crate docs and the module doc say instead that it is
    not an interface, so its rustdoc stays readable for part 2.
14. **Polling intervals:** the boot poll asks every 150 ms, inside the brief's "100-200 ms". The abort settle poll asks every
    100 ms. The kill runner checks every 5 ms.

## Reuse / extend-vs-new

- **Reused unchanged:** `HarnessPort`, `PaneError` and its closed codes, `PaneName` and `PaneId`, and the workspace's
  `httparse` and `serde_json`. No file outside this crate's directory changes, apart from `CHANGELOG.md` and `Cargo.lock`.
- **New, as the brief's Reuse map justifies in writing:**
  - `OpenCodeHarness` and `OpenCodeConfig`: the port's one designated implementation.
  - `http.rs`: the existing clients are async in `holler-body`, and `reqwest`'s `blocking` feature would unify features
    across the workspace.
  - `exec.rs`: private, shaped like #641's runner so that #696 is a move.
  - `tui.rs`: the types only.
- **`server.rs` is the brief's own named split of `lib.rs`** (Size check), not a new object.
- **Mirrored, not imported** (production code cannot depend on the test kit): `FakeHarness`'s text for an unreachable server
  and the `HarnessOp` op strings. evidence.md has both excerpts.
- **Not duplicated:**
  - `holler_pane::run_probe` is #663's stub, and its contract (a `ProbeResult`) does not fit.
  - #641's `exec.rs` is unmerged (its branch holds only the brief). #696 consolidates the two.

## Architecture notes for A

- **Layers.** One crate. It depends on `holler-pane`, `serde_json` and `httparse`, and on `holler-pane-testkit` as a
  dev-dependency only. It depends on no other adapter, and not on the hub or the body (ADR-0021 section 5). Nothing depends
  on it yet (`cargo tree -i` lists only the crate).
- **Public surface.** The brief's API, with nothing added: `MAX_REPLY` and every helper are private.
- **New private modules:** `exec` and `server`.
- **Platform and `unsafe`.** No `unsafe`: the process group comes from `std::os::unix::process::CommandExt::process_group(0)`
  and the kill from the `kill` binary. The crate is Unix-only, which matches the CI matrix (ubuntu, macos).
- **Child ownership** (the brief's W-4 rule, restated in `server.rs`'s module doc). `serve` alone owns the `Child` until it
  returns. An exit seen at boot is reaped by `try_wait`. On the deadline the group is killed and the child waited on. On
  success the child moves to the reaper thread, its one owner. So every child is reaped exactly once.
- **The one deadline rule.** Every method makes a `Call` at entry. Each request takes `min(own bound, what is left)`, and the
  settle poll is also ended by `settle`.

## Deviations from spec

- **`HttpError::Garbled`'s doc comment is widened** (Design decision 3). It now reads "No reply the client can read: not an
  HTTP/1.x response, a reply cut short, or a connection that failed other than by a refusal or a timeout". The brief's comment
  was "not an HTTP/1.1 response we can read". The variants are unchanged.
- **The CHANGELOG entry follows A's W-5 for a part-1 entry**: part 1, the TUI half follows, and nothing a user runs changes
  yet (#649). AC 23 asks the entry to say "the real-OpenCode tests are opt-in", but 642a adds none, so the entry says that
  part 2 brings the opt-in tests against a real OpenCode.
- There are no other deviations. `tui.rs` stays types only, `exec.rs` holds only the kill path, and `ADR-0021.md` and the
  two `holler-pane` files are untouched (642b's).

## Tier 1 self-check (incl. tests now GREEN)

T's suite, now GREEN (the RED was 1 passed and 26 failed):
```
$ cargo test -p holler-adapter-opencode
     Running tests/hermetic_test.rs
running 27 tests
... (all 27 `ok`)
test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.04s
```
Repeated runs, as a timing-flake check: 8 parallel runs gave 27/27 every time (1.03-1.04 s). One `--test-threads=1` run
gave 27/27 (2.67 s).

Gates:
```
$ cargo clippy --workspace --all-targets -- -D warnings      -> Finished, no warnings
$ rustfmt --check --edition 2021 crates/holler-adapter-opencode/src/*.rs (and the test files)   -> exit 0
$ RUSTDOCFLAGS="-D warnings" cargo doc -p holler-adapter-opencode --no-deps   -> Finished, no warnings
$ bash scripts/lint.sh        -> exit 0. The crate's only warning is T's hermetic_test.rs at 647 lines; the src files are 33-483 lines.
$ bash scripts/changelog-check.sh   -> changelog-check: ok
$ cargo machete               -> didn't find any unused dependencies
$ grep -rn "4700[0-9]\|--continue" crates/holler-adapter-opencode   -> nothing (exit 1)
$ git diff origin/main -- crates/ | grep '^+' | grep -c unsafe     -> 0
```
The whole-diff `git diff origin/main | grep -c unsafe` is 5, and none of the five comes from this phase: 4 are in the brief's
prose and 1 is in `handoff-A.md` ("with no `unsafe`").

AC 20, `cargo test --workspace` (CI's form, `-- --skip roster_stays_accurate_under_concurrent_body_load`):
- **First run (exit 101):** 4 failures in `holler-cli/tests/logging_test.rs`, all "Unexpected success" of `holler roster`.
  This machine runs a live hub on the default state dir, and those tests expect no hub. They are environmental and unrelated:
  nothing depends on this crate.
- **The same 11 tests with `HOLLER_STATE_DIR` set to a scratch dir:** 11 passed.
- **The whole workspace with `HOLLER_STATE_DIR` set to a scratch dir:** exit 0, 126 suites, **1410 passed, 0 failed, 5
  ignored**.

Behaviour no hermetic test reaches. I checked it once, from a throwaway program in the session scratchpad (outside the repo,
deleted with it), against fake servers on loopback:
```
A serve -> pid 3254302 after 751.55267ms          (python fake that boots in 0.7 s)
A pgid of pid = 3254302 (want 3254302)            (its own process group)
A health = Ok(true)
A create_session = ses_00000000000000000000000001  (POST then PATCH, against a different HTTP server: python http.server)
A list_sessions = Ok(["ses_00000000000000000000000001"])
A abort(id) = Ok(())
A abort(unknown) = Err(SessionNotFound { what: "ses_zzzzzzzzzzzzzzzzzzzzzzzzzz" })
A serve again = Err(Unavailable { what: "port 43881 is in use: a harness server already answers there" })
A alive after the adapter is dropped: true        (it outlives the adapter; the check then killed its group)
B serve = Err(Timeout { op: "harness.serve" }) after 1.956592915s   (a server that never answers, call = 2 s)
exact leftovers: []                               (its `sleep 3007 &` child in the same group was killed too)
```
Reply reading, against raw canned replies:
```
100 Continue, then 200: Ok((200, "true"))
chunked with trailers: Ok((200, "true"))
HTTP/1.0, no length (to EOF): Ok((200, "{\"healthy\":true}"))
no headers at all (to EOF): Ok((200, "[1,2]"))
cut short: Err(Garbled("the connection closed before the declared length (6 of 40 bytes)"))
oversized Content-Length: Err(Garbled("the reply is longer than 67108864 bytes"))
HTTP/2 preface: Err(Garbled("not an HTTP/1.x reply (invalid HTTP version)"))
closed with no reply: Err(Garbled("the server closed the connection without a reply"))
```
How the messages read (the rendered `Display`, against a server that answers every route with OpenCode's HTML page):
```
141 bytes | unavailable: GET /session/:id on port 44555 answered 200, not that session: "<!doctype html> <html lang="en"> <head><meta charset="utf-8"..."
143 bytes | unavailable: GET /session on port 44555 answered 200, not a list of sessions: "<!doctype html> ..."
163 bytes | unavailable: port 44555 is in use: GET /global/health answered 200, not a healthy harness server: "<!doctype html> ..."
130 bytes | unavailable: cannot start the harness server /nonexistent/opencode in /nonexistent/project: No such file or directory (os error 2)
```
`kill -s KILL -- -<pgid>` through procps-ng 4.0.4's `/usr/bin/kill` killed a scratch `setsid` group (exit 0, group gone).

## Evidence appendix

`docs/handoffs/642/evidence.md`: 12 entries. They cover:
- the fake's unreachable text and op strings;
- `NotImplemented`'s documented meaning;
- the suite's binding rules and the port's I5 contract;
- `Unavailable`'s `Display`;
- the one-line message rule (ADR-0021:329);
- the 200-HTML catch-all (`holler-body`);
- frozen versus killed, and the boot race (the spike);
- the workspace `httparse` entry, and `httparse`'s HTTP/1.x-only version parse.

The last entry is a dependency's source, outside the repo.

## Tests that look wrong (for T)

None is wrong. One fragility note for T to weigh:

- **The "42" check of `ac8_serve_of_a_program_that_exits_at_once_is_unavailable_with_its_status`.** It removes the port from
  the message and then looks for `"42"`. The scratch directory is named `hlr642-<pid>-<n>`, which contains `42`, and a pid
  can contain it too. Today's message, `the harness server for demo-c1r1 on port P exited before it answered (exit status:
  42)`, holds neither the directory nor a pid, so the assertion is meaningful as built. A later change that put the
  directory in that message would pass it vacuously, though. Checking for `"exit status: 42"`, or using an exit code with no
  such collision, would close that.

## Known issues

- **The deadline kill has no hermetic test.** No test in 642a reaches `serve`'s kill or success path, because that needs a
  server that boots slowly or never answers. I checked both by hand (above). T may pin them now or with 642b's rig.
- **The issue's 2026-10-09 amendment is in neither the brief nor 642a.** It covers the pane's OpenCode agent
  (`Pane.opencode_agent`, applied as the server's `default_agent` or per prompt) and depends on the open #700. O needs to
  route it to 642b or a follow-up.
- **A port race in `serve` remains.** Between `serve`'s refused check and its child's bind, another process can take the port.
  If that process answers healthy while our child is still booting, `serve` returns our child's pid (Design decision 5 covers
  only a child that has already exited). The race is inherent to choosing a port. The registry gives each pane its own port
  (#644).
- **For O and the PR:**
  - A's W-4(c): the `FakeHarness` parity follow-up issue still needs filing before the PR.
  - A's W-3(2): the 642a PR body should list divergence 15(a), cite #695, and say that the ADR-0021 note lands with 642b.
  - CLAUDE.md asks for the AI disclosure in the PR body.

## Files changed

- `crates/holler-adapter-opencode/Cargo.toml`
- `crates/holler-adapter-opencode/src/lib.rs`
- `crates/holler-adapter-opencode/src/http.rs`
- `crates/holler-adapter-opencode/src/server.rs` (new)
- `crates/holler-adapter-opencode/src/exec.rs` (new)
- `crates/holler-adapter-opencode/src/tui.rs` (docs only)
- `CHANGELOG.md`
- `Cargo.lock` (mechanical)

Handoff files: `docs/handoffs/642/handoff-F.md`, `docs/handoffs/642/evidence.md` (new), and the F entry in
`docs/handoffs/642/decisions.md`.
