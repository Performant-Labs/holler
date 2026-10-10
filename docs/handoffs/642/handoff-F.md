# Handoff-F: Phase 6 - #642a the OpenCode adapter, server side (`serve`, `health`, `create_session`, `list_sessions`, `abort`, `http::request`)

**Date:** 2026-10-09
**Branch:** issue-642-implementation (round 2 on 4c9b710, T's round-2 commit; round 1 was on 0ce98a4)
**Issue:** #642, part 1 of 2 (the PR says `Part of #642`). Brief: `docs/handoffs/642-brief.md`, its 642a AC list only.

## Round 2: rework after the outside diff gate's round 1

### What this round answers

- **The outside diff gate's round 1** (`docs/handoffs/642-diff-result-r1.md`, untracked): 1 BLOCK (B-1), NV-1 to NV-5, W-1 to
  W-3, NIT-1 and NIT-2. On this route the driver passes F no note, so these come from that file.
- **A's re-review** (handoff-A.md at c052d02, PASS):
  - N-1: the agent part waits on #700. Change no code for it, and reword `tui.rs:3-4` if docs are edited.
  - N-2: belongs to 642b.
  - N-3: B-1's premise does not hold as measured. Any F change stays in `http.rs`, with no test-only knob.
- **T's round 2** (handoff-T-red.md at 4c9b710): 30 tests, all passing on the existing code. F had nothing to turn GREEN.

### What was done

- **`crates/holler-adapter-opencode/src/http.rs`** (348 to 360 lines):
  - **B-1, the code change.** The 64 MiB bound is now exact, and it counts every byte the reply takes off the connection.
    `Reader` gains one private field, `taken`. `fill` adds each read's length to it. If the total passes `MAX_REPLY`,
    `fill` answers `Garbled` before it keeps that read's bytes. The old check before the read (`buf.len() >= MAX_REPLY`) is
    gone. The load-bearing lines:
    ```rust
    Ok(n) => {
        self.taken += n;
        if self.taken > MAX_REPLY {
            return Err(too_long());
        }
        self.buf.extend_from_slice(&chunk[..n]);
        return Ok(n);
    }
    ```
  - **The docs now match the code.** `MAX_REPLY`'s doc and the module doc say the bound covers the heads (an interim `1xx`
    one too), any chunk framing and the body, all counted as read off the connection. That closes A's note that "head and
    body together" was looser than the code, which drained the head before it counted the body.
  - **NV-2, wording only.** The module doc says a trailer after the `0` chunk is never read and goes with the connection.
    The old wording, "whose trailers are discarded", was read by the gate as "read and then discarded".
  - **W-2.** `encode`'s doc lists the head's headers and says no other header is sent: no `User-Agent`, and no `Expect`.
- **`src/server.rs`** (comments only, 186 to 189 lines). For W-1, the docs of `BOOT_INTERVAL` and `wait_until_up` now say
  what the loop does. Tries start at least 150 ms apart, and a try that takes longer is followed at once. A try can take up
  to `boot_try` when its GET hangs in the boot race. The gate's "~650 ms" period does not happen: `next` is measured from the
  start of the try, so after a 500 ms try the sleep is zero.
- **`src/tui.rs`** (docs only). For A's N-1(e), the module doc no longer calls `OpenCodeConfig` final, because it will gain
  the agent field. It now says the module holds the type of that struct's `tmux` field.
- **`docs/handoffs/642/evidence.md`**: two entries for NV-3 and NV-4, quoting the Rust 1.98.1 standard library's
  `CommandExt::process_group` and `Child::id` (see the Evidence appendix below).

No test was edited. `lib.rs`, `exec.rs`, `Cargo.toml`, `CHANGELOG.md` and `Cargo.lock` are unchanged in this round.

### How each gate finding stands

| Finding | Answer | Where |
|---|---|---|
| B-1 (the 64 MiB bound on a read to the close) | The premise did not hold. The old check let at most one 8 KiB read past the cap, and A measured that (N-3). Applied anyway, in the form the gate suggested: the check comes before a read's bytes are kept, so the read buffer never holds more than the cap. The count now covers heads and framing too. T's over and under cases pin it. | `http.rs` `fill`; T's `ac1_an_unframed_reply_past_64_mib_is_garbled_and_one_under_it_is_read` |
| NV-1 (the index `parse_chunk_size` returns) | Settled by `httparse`'s own doc example. | T's evidence entry |
| NV-2 (trailers) | Trailers are never read, and the client sends `Connection: close`. The doc now says so. T's new test reads a chunked body that has an extension and a trailer. | `http.rs` module doc; T's `ac1_a_chunked_reply_with_an_extension_and_a_trailer_reads_its_body` |
| NV-3 (`process_group(0)` without `unsafe` on both CI targets) | It is a safe trait method, stable since 1.64.0. `rustc --print cfg --target aarch64-apple-darwin` prints `unix`, so `std::os::unix` is there on macOS. | F's evidence entry; T's `serve_kills_its_process_group_when_the_deadline_passes` |
| NV-4 (`Child::id()` is the group id) | std: "A process group ID of 0 will use the process ID as the PGID", and `Child::id` is the child's process id. | F's evidence entries; the same T test |
| NV-5 (`workdir` is resolved only after the health check) | T's counting resolver asserts zero calls when the port is held or frozen. | T's two extended `ac8_serve_*` tests |
| W-1 (boot-poll period) | Comment corrected (above). No behaviour change. | `server.rs` |
| W-2 (no extra headers) | Comment added (above). | `http.rs` `encode` |
| W-3 (`Garbled` maps to `unavailable`) | No change. This is the brief's mapping (Behaviour: "`Garbled`, or a status the step does not expect -> `unavailable` naming the route and status"), and the gate itself says "No change required". | — |
| NIT-1 (zero budget) | No change. With nothing left, `healthy` answers `false` and `refuse_a_held_port` answers `timeout`, both right for a call that is out of time. | — |
| NIT-2 | No change; the gate says none is needed. | — |
| The B-2 the gate withdrew itself | No change. When less than one `SETTLE_POLL` is left, `settled` answers `timeout` without a last poll, because that poll's request budget would be about zero and would time out anyway. | — |

### Design decisions

1. **Count every byte taken, not just the buffer.** Three options were weighed:
   - **(a) Change the docs only** ("the head, then the body, each within one 8 KiB read of the cap"). This is A's minimum.
     I rejected it because the buffer could still pass the stated cap by up to 8,191 bytes, which is exactly what the gate
     flagged. It also left the edges inconsistent: a read-to-the-close body of exactly `MAX_REPLY` bytes was `Garbled`, while
     a `Content-Length` body of the same size was read.
   - **(b) Check `buf.len() + n > MAX_REPLY`** (the gate's second suggestion). This bounds the buffer. But the head is drained
     before the body is counted, so "head and body together" stays untrue, and a chunked reply's framing is never counted.
   - **(c) Count every byte read** (chosen). One counter and one comparison make the doc's sentence literally true for every
     framing, and the read buffer never holds more than `MAX_REPLY`.

   **The cost:** a reply's head and chunk framing now count toward the 64 MiB, so the largest body that can be read is a
   little under 64 MiB. It is smaller by the head's length, and for a chunked reply by its framing (about 0.1% with 8,000-byte
   chunks). No OpenCode reply comes near that size: a list of thousands of sessions is a few MiB.
2. **The early refusals stay.** `sized` refuses a declared `Content-Length` over the cap before any read. `chunked` does the
   same for a declared chunk size that cannot fit. Each is a necessary condition, so it saves the read without changing the
   bound, and the counter does the rest.

### Reuse / extend-vs-new

No new object. The change extends `http.rs`'s private `Reader` with one field.

### Architecture notes for A

None. No module, public type, signature or dependency changed. `MAX_REPLY`, `Reader` and `taken` are private, and the public
surface is still exactly the brief's API. archChanged: false.

### Deviations from spec

None. The brief's reading rules (brief:558-563) set no size bound. The bound is F's round-1 decision 9, and this round only
makes it exact. A's N-3 asks O to record the bound in the brief's `http.rs` section at its next amendment.

### Tier 1 self-check (incl. tests GREEN)

```
$ cargo test -p holler-adapter-opencode
running 30 tests ... test result: ok. 30 passed; 0 failed; 0 ignored; finished in 1.05s   (30/30 before the change too)
$ cargo clippy --workspace --all-targets -- -D warnings      -> Finished, no warnings
$ rustfmt --check --edition 2021 crates/holler-adapter-opencode/{src,tests,tests/support}/*.rs   -> exit 0
$ RUSTDOCFLAGS="-D warnings" cargo doc -p holler-adapter-opencode --no-deps   -> Finished, no warnings
$ bash scripts/lint.sh          -> exit 0 (the crate's only note: T's hermetic_test.rs at 775 lines)
$ bash scripts/changelog-check.sh   -> changelog-check: ok
$ cargo machete                 -> didn't find any unused dependencies
$ grep -rn "4700[0-9]\|--continue" crates/holler-adapter-opencode   -> nothing (exit 1)
$ git diff origin/main -- crates/ | grep '^+' | grep -c unsafe     -> 0
$ rustc --print cfg --target aarch64-apple-darwin | grep -E 'unix|target_family'   -> target_family="unix", unix
```
The whole-diff `git diff origin/main | grep -c unsafe` is 19, and every one is prose under `docs/handoffs/`: the brief, the
handoffs, decisions.md and this round's evidence entry. The pipeline removes those files before merge. Outside them the
count is 0 (`git diff origin/main -- . ':(exclude)docs/handoffs' | grep -c unsafe`), so no code line contains `unsafe`.

AC 20, `cargo test --workspace` (CI's form, `-- --skip roster_stays_accurate_under_concurrent_body_load`, with
`HOLLER_STATE_DIR` set to a scratch dir as in round 1): exit 0, 126 suites, **1413 passed, 0 failed, 5 ignored**. That is
round 1's 1410 plus the crate's three new tests (27 to 30). The run started before the last two doc-comment edits to
`http.rs`. The crate's tests, clippy, rustfmt, rustdoc, `lint.sh` and `cargo machete` were run again after those edits, and
all are clean (30/30).

**The bound, measured.** A throwaway program (in the session scratchpad, outside the repo, now deleted) called the public
`http::request` against one-shot loopback servers. Each server streams its reply in 8 KiB writes, so it holds no large buffer
of its own. M is 64 MiB (67,108,864 bytes), and the unframed head is 38 bytes.
```
unframed, head + body = M - 1: Ok((200, 67108825))
unframed, head + body = M:     Ok((200, 67108826))
unframed, head + body = M + 1: Err(Garbled("the reply is longer than 67108864 bytes"))
unframed, body = M - 1024 (T's under case): Ok((200, 67107840))
unframed, body = M + 1   (T's over case):   Err(Garbled(...))
Content-Length, body = M - 1 KiB: Ok((200, 67107840))
Content-Length, body = M:         Err(Garbled(...))                 (head + body > M)
Content-Length, body = M + 1:     Err(Garbled(...)) in 84 us        (refused before reading)
chunked, body = M - 64 KiB:       Err(Garbled(...))                 (framing of 8,000-byte chunks counts)
chunked, body = 2 KiB:            Ok((200, 2048))
peak:  VmHWM 2,448 kB before; a 200 MiB unframed reply -> Garbled in 29 ms; VmHWM 67,540 kB after
```

### Evidence appendix

`docs/handoffs/642/evidence.md` gains two entries this round: std's `process_group` (`library/std/src/os/unix/process.rs:
174-204`) and `Child::id` (`library/std/src/process.rs:2351-2370`), from Rust 1.98.1. They are quoted from the rendered source
that rustup's `rust-docs` component installs, outside the repo, so the gate cannot attach them; they stand as verbatim quotes.

**For T-green:** the file is now 9,528 bytes, and the gate's appendix cap is 12,000 bytes
(`DUAL_REVIEW_EVIDENCE_MAX_BYTES`, `dual-review.sh:948`). Any text past the cap is cut from the end, so T-green has about
2.4 KB left for new entries.

### Tests that look wrong (for T)

None. T's two cap cases keep their margins under the new counting. The unframed head is 38 bytes, so the under case takes
64 MiB - 986 bytes in all, and the over case takes 64 MiB + 39.

**For T-green's mutation check:** round 2's M1 (delete `fill`'s check before the read) no longer applies, because that line is
gone. The equivalent mutation now is deleting the `self.taken > MAX_REPLY` check.

### Known issues

None new. Carried from round 1, with their status:
- **`serve`'s success path** still has no hermetic test. It needs a child that answers HTTP, so it belongs to 642b's
  real-OpenCode rig. The deadline-kill path is now pinned by T-green's test.
- **`kill -s KILL -- -<pgid>` on macOS** is unverified. It is checked on Linux only (procps-ng 4.0.4), and CI's macOS leg
  will show it when it runs `serve_kills_its_process_group_when_the_deadline_passes`. Any fix belongs in `exec.rs`.
- **The port race in `serve`** is inherent to choosing a port. The registry gives each pane its own port (#644).
- **The issue's 2026-10-09 amendment** (the pane's OpenCode agent, which depends on #700) is A's N-1. It is out of 642a, and
  no code was changed for it. The 642a PR body should name it as #642's open remainder, waiting on #700.
- **Before the PR (for O and the run's agent):**
  - A's W-4(c): file the `FakeHarness` parity follow-up.
  - A's W-3(2): the PR body lists divergence 15(a), cites #695, and says the ADR-0021 note lands with 642b.
  - The AI disclosure required by `CONTRIBUTING.md`.

### Files changed (round 2)

- `crates/holler-adapter-opencode/src/http.rs`
- `crates/holler-adapter-opencode/src/server.rs` (comments only)
- `crates/holler-adapter-opencode/src/tui.rs` (docs only)

Handoff files: `docs/handoffs/642/handoff-F.md` (this section), `docs/handoffs/642/evidence.md` (two entries), and the F
round-2 entry in `docs/handoffs/642/decisions.md`.

---

## Round 1 record (999d7e3), unchanged below

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
   body. (Round 2 makes the 64 MiB exact: every byte read off the connection counts.)
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
