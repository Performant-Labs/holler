# Handoff-T-red: Phase 4 - #642b the OpenCode adapter, part 2: the TUI side of `HarnessPort`

**Date:** 2026-10-09
**Branch:** issue-642-implementation at cb4d5da (base `dc300ab`)
**Brief / wireframe reviewed:** docs/handoffs/642-brief.md at 6c3809c (the brief A reviewed; not amended since); no wireframe (no UI surface)

## A precondition

Confirmed: A returned PASS on the plan (Phase 3), with 0 blocks and 9 warns (docs/handoffs/642/handoff-A.md).

A asked for two warns to be settled **before T-red**: W-1 (target `=<session>:^` instead of `=<session>:`) and W-8(a)
(name the escapes `escape`/`escape_cwd`). The brief was not amended after A (the last brief commit is 6c3809c; cb4d5da is
A's verdict). So these tests pin the brief **as written**: every `-t` is `=<session>:` (AC 11a, 27-30), and the escapes
are `escape_arg`/`escape_dir` (the brief's API and Test plan). If O amends either, only these strings change: the
`=demo-c1r1:`/`=demo:` literals in `tui_test.rs` and `attach_test.rs` (the `query()` helper and the AC 30(a) prefix), and
the two imported names. W-1 then stays a follow-up that must land before #644's relaunch runs against real tmux.

## Tests authored

All staged by explicit path. The first T run (interrupted) wrote the bulk of this; this run reviewed every file against
the brief, added one test and one stub helper, and re-ran everything.

**`tests/tui_test.rs` (new, 335 lines; pure, unit tier: no process, so CI's two legs run it).**

| Test | Pins |
|---|---|
| `ac9_parse_title_reads_a_whole_id_or_home_and_never_guesses` | AC 9: `Session` for a whole id, `Home` for `OpenCode`, `Unrecognised` for the cut, default, person's, host and empty titles |
| `ac10_attach_port_reads_a_loopback_attach_line_and_nothing_else` | AC 10, including tmux's re-quoted form, no flags and flags in another order (NIT-2), `serve` and a non-loopback URL |
| `ac11a_every_builder_targets_the_exact_session` | AC 11a's exact vectors for `respawn_args`, `remain_on_exit_args` and the head of `query_args` |
| `ac11a_no_builder_passes_a_bare_session_name_as_a_target` | AC 11a: for `demo`, every `-t` is `=demo:` and no element is a bare `demo` |
| `ac11b_escape_arg_guards_only_a_final_semicolon`, `ac11b_escape_dir_doubles_every_hash_then_escapes`, `ac11b_respawn_escapes_the_directory_and_every_tui_element` | AC 11b, raw values (`r"..."`), and the `-c`/`K=v\;`/`--dir` placement in a respawn |
| `ac11c_tmux_command_names_the_socket_and_drops_tmux_and_tmux_pane` | AC 11c for `Path`, `Name` and `Default`: program, args and `("TMUX", None)`, `("TMUX_PANE", None)` |
| `ac11f_the_tui_argv_under_inherit_and_isolated` | AC 11f's two argvs exactly, and that `attach_port` reads each back once space-joined |
| `ac27_the_query_is_display_message_of_the_five_fields` | AC 27: `QUERY_FORMAT` is Decision 23's string and `query_args` is the exact vector |
| `ac27_a_live_pane_gives_its_start_command_and_its_whole_title`, `ac27_a_dead_pane_gives_its_exit_status`, `ac27_a_reply_that_is_not_this_sessions_pane_is_no_pane` | AC 27: `Live`, a title holding a TAB kept whole, `Dead(Some(7))`/`Dead(None)`, and `NoPane` for the four-TAB reply, another name, three fields, a bad `pane_dead` and an empty reply. The last test asserts its own contrast first, so a stub that always answers `NoPane` fails it |

These sit at unit tier because every item is a pure function. The process-level tests below use their output only as
expected values, never re-deriving the rules.

**`tests/attach_test.rs` (new, 658 lines; integration tier: the real adapter, the 642a HTTP stub, and the committed fake
tmux).** This is the cheapest tier that can see the order of the tmux and HTTP calls and the resolver's use. Helpers are in
`tests/support/fake_tmux.rs` (new, 84 lines) per the brief's 700-line guidance.

| Test | Pins |
|---|---|
| `ac33_the_fixture_is_an_executable_posix_sh_script` | AC 33: `#!/bin/sh`, and (through `fixture()`) the mode check that names `git update-index --chmod=+x` |
| `ac28a_shown_session_reads_a_live_attach_titled_with_an_id_in_one_query` | AC 28(a): `Ok(Some(id))`, exactly one call equal to the query's token list, no HTTP |
| `ac28b_shown_session_is_none_whenever_it_cannot_tell` | AC 28(b): all nine cases (three titles, dead, `sleep 3600`, four TABs, three "missing" stderrs) give `Ok(None)` |
| `ac28c_another_tmux_failure_is_unavailable_with_its_first_line_only` | AC 28(c): `unavailable` with the first stderr line only, never the format or `=demo-c1r1:` |
| `ac28d_a_tmux_that_cannot_run_is_unavailable_naming_it` | AC 28(d) |
| `ac28e_an_unknown_pane_is_the_resolvers_error_and_runs_no_tmux` | AC 28(e): `pane-not-found` as is, and `<sock>.calls` does not exist |
| `ac29a_select_checks_the_session_switches_and_confirms_by_the_title` | AC 29(a): `Ok`, the two request lines in order, and the body `{"sessionID":"ses_B"}` |
| `ac29b_select_of_an_unknown_session_is_session_not_found` | AC 29(b), both the existence check's 404 (no `POST`) and the switch's own 404 |
| `ac29c_select_without_a_tui_is_unavailable_after_one_query_and_no_request` | AC 29(c) for `sleep 3600`, four TABs and a dead pane: names the pane, echoes nothing it did not author, the stub saw nothing, and exactly one call (the query) |
| `ac29d_a_switch_the_title_never_shows_times_out_within_settle` | AC 29(d): `timeout { op: "harness.select_session" }` within `settle` + 500 ms |
| `ac29e_a_switch_answered_by_the_web_app_is_unavailable` | AC 29(e) |
| `ac30a_attach_keeps_the_pane_respawns_the_tui_escaped_and_confirms_by_the_title` | AC 30(a): call 1 is `set-option`, call 2 equals `respawn_args(...)` with `-c /p##S\;` and `--dir /p#S\;`, and every later call is the query |
| `ac30b_a_missing_tmux_session_is_unavailable_and_nothing_is_respawned` | AC 30(b) |
| `ac30c_a_tui_that_exits_is_unavailable_with_its_status` | AC 30(c): the status `3` as a whole number in the message, with no argv, directory or start command echoed |
| `ac30c_a_tui_that_exits_because_its_session_went_away_is_session_not_found` | **Added by this run.** Behaviour, `attach_tui`: "If the pane's process dies first: re-`GET` the session; 404 -> `session-not-found`". The existence check answers 200 once (new `Stub::json_once`), then OpenCode's 404. With the test above it pins the re-GET by its outcome; no AC named it, and #644 needs it to report a session deleted mid-attach |
| `ac30d_a_session_reply_without_a_directory_is_unavailable_and_touches_nothing` | AC 30(d): `unavailable`, the resolver is never called, no `<sock>.calls` |
| `ac30_attach_of_an_unknown_session_is_session_not_found_and_touches_nothing` | Behaviour, `attach_tui` (cases 11), for 404 and for 400 (malformed id), touching nothing |
| `ac30_an_attach_the_title_never_confirms_times_out_within_settle` | Behaviour: `timeout { op: "harness.attach_tui" }` within `settle` + 500 ms |
| `ac30e_each_method_past_its_deadline_times_out_with_its_own_op` | AC 30(e): with `timeouts.call = 0`, each method answers `timeout` with `HarnessOp::{AttachTui, SelectSession, ShownSession}.as_str()` |
| `ac30f_no_method_answers_not_implemented_for_any_input` | AC 30(f): the table-driven guard over five scenarios, two panes and three methods |
| `ac6_attach_to_an_unbound_port_is_unavailable_before_the_resolver` | AC 6's 642b clause, through AC 31's helper |
| `ac11d_attach_to_a_session_reply_that_is_not_that_session_is_unavailable` | AC 11d's 642b clause, for 200 HTML and another id: one line, at most 200 bytes, names the route and the status, resolver never called, no tmux call |

AC 30(f)'s other half: every scenario above asserts one exact answer (`assert_eq!` on the value, or a `matches!` on one
named variant through `unavailable()`), never `is_err()` alone.

**`tests/fixtures/fake-tmux` (new, committed, git mode `100755`, 33 lines).** It implements the brief's contract byte for
byte, using POSIX `sh` only, with `printf`, `[`, `read`, `shift`, `exit`, `:` and `cat` (AC 33). Tests write only data
files (AC 32).

**`tests/real_opencode_test.rs` (new, 367 lines) + `tests/real_opencode/rig.rs` (new, 439 lines).** These are the
opt-in, real-process tier: AC 12 (the conformance suite), 13, 14, 15, 16, 17, 18 (with B-3's three ordered steps), 19 and
19a. Each test is `#[ignore = "..."]` and returns at once without `HOLLER_TEST_OPENCODE=1`. Opted in, a missing `opencode`
or `tmux` panics.

The rig follows the brief's rules:
- a `/tmp/hlr642r-*` scratch dir with a socket path under 100 bytes;
- `Isolated` env as in `opencode-lib.sh:98-111`, with the dead-end config verbatim;
- the 127.0.0.1:9 check before setup, and the `/config/providers` guard after **every** `serve`;
- ports from 48100-48199 only, each refused and not yet handed out;
- a private `-S` tmux server with `demo-c1r1`/`demo-c2r1` on `sleep 3600`, and `tui_session` mapping `w9:p1`/`w9:p2`, with
  anything else `pane-not-found`;
- a `Guard` built before tmux starts. It SIGKILLs the recorded process groups, runs `kill-server` by socket and removes the
  dir, and it never signals or looks up anything it did not start;
- `Rig::serve` retries on "is in use" (AC 31).

**`tests/hermetic_test.rs` (changed, 798 lines; no new case) and `tests/support/stub.rs` (helpers, 311 lines).**
- AC 31: `closed_port()` is private now. `on_refused_port(call, expected)` checks for a refusal before the call and retries
  (at most 3 attempts in all) only when the answer is wrong **and** the port now accepts. All seven earlier uses in
  `hermetic_test.rs` go through it.
- AC 26: the outside-gate ids at the old lines 243, 265 and 510 (B-1, NV-2, NV-4) are now plain statements of the rule.
- The brief's D-1 follow-up: the `stub.rs` header comment is fixed.
- This run added `Stub::json_once` (a reply for the next request of a route only) for the test above.

## RED confirmation

Run on this machine (Linux, `CARGO_BUILD_JOBS=4`), with T's minimum public stubs staged in `src/tui.rs` (each pure item
returns empty, `None`, `NoPane` or `Unrecognised`; `QUERY_FORMAT` is `""`). The three methods keep `NotImplemented`. Every
target compiles; `cargo clippy -p holler-adapter-opencode --all-targets -- -D warnings` is clean.

**`cargo test -p holler-adapter-opencode --test tui_test`**: `0 passed; 13 failed`. Each one fails on its feature
assertion, for example:
```
ac9_...      left: Unrecognised          right: Session("ses_0123456789abcdefABCDEFghij")
ac10_...     left: None                  right: Some(48123)
ac11a_every_ left: ""                    right: "=demo-c1r1:"
ac11a_no_... left: []                    right: ["=demo:"]
ac11b_escape_arg   left: ""              right: "x\\;"
ac11b_escape_dir   left: ""              right: "/p##S\\;"
ac11b_respawn_...  panicked at tui_test.rs:174: a -c flag   (the builder returned no vector)
ac11c_...    left: []                    right: ["-S", "/x/sock", "display-message", "-p"]
ac11f_...    left: []                    right: ["env", "-u", "OPENCODE_DISABLE_TERMINAL_TITLE", "/bin/oc", "attach", ...]
ac27_the_query...  left: ""              right: "#{session_name}\t#{pane_dead}\t#{pane_dead_status}\t#{pane_start_command}\t#{pane_title}"
ac27_a_live_...    left: NoPane          right: Live { start_command: "env -u ... --session ses_1", title: "OC | ses_1" }
ac27_a_dead_...    left: NoPane          right: Dead(Some(7))
ac27_a_reply_that_is_not_...  panicked at tui_test.rs:310: this session's own reply is a live pane  (the contrast)
```

**`cargo test -p holler-adapter-opencode --test attach_test`**: `1 passed; 21 failed`. The 21 fail because the methods
still answer `Err(NotImplemented)`, the skeleton's answer the feature replaces, for example:
```
ac28a_...  left: Err(NotImplemented)  right: Ok(Some("ses_0123456789abcdefABCDEFghij"))
ac28e_...  left: Err(NotImplemented)  right: Err(PaneNotFound { what: "w9:p2" })
ac29b_...  left: Err(NotImplemented)  right: Err(SessionNotFound { what: "ses_B" })
ac29d_...  left: Err(NotImplemented)  right: Err(Timeout { op: "harness.select_session" })
ac30a_...  left: Err(NotImplemented)  right: Ok(())
ac30c_..._went_away_...  left: Err(NotImplemented)  right: Err(SessionNotFound { what: "ses_A" })
ac30e_...  left: Err(NotImplemented)  right: Err(Timeout { op: "harness.attach_tui" })
ac6_..., ac11d_..., ac28c/d_..., ac29c/e_..., ac30b/c/d_...:  "<case>: expected unavailable, got Err(NotImplemented)"
ac30f_...  panicked at attach_test.rs:576: attach_tui on w9:p1 with a live TUI on ses_A: Err(NotImplemented)
```
The one that passes is `ac33_the_fixture_is_an_executable_posix_sh_script`. It checks the committed fixture, a
precondition of the other tests and not a feature assertion, so it is meant to pass before F's code. The later assertions
in each test (call token lists, request lines, resolver counts, message contents) are what keep a bare "stop answering
`NotImplemented`" from passing them.

**`cargo test -p holler-adapter-opencode --test hermetic_test`**: `30 passed; 0 failed`. All 642a tests still pass with
AC 31's helper. `--lib`: no unit tests.

**Real tests, not opted in:** `cargo test ... --test real_opencode_test` gives `0 passed; 9 ignored`. With `-- --ignored`
it gives `9 passed` in `0.00s`: each test returns at once.

**Real tests, opted in** (OpenCode 1.18.35 at `/usr/local/bin/opencode`, tmux 3.7c):
`HOLLER_TEST_OPENCODE=1 cargo test -p holler-adapter-opencode --test real_opencode_test -- --ignored --test-threads=1`
gives `3 passed; 6 failed` in 27 s.
- **AC 12** fails on exactly the 8 TUI cases: `calls-to-unserved-port-are-unavailable` (its `attach_tui` leg),
  `shown-without-tui-is-none`, `attach-shows-the-session`, `attach-unknown-is-session-not-found`,
  `select-switches-the-shown-session`, `select-unknown-is-session-not-found`, `select-without-tui-fails` and
  `select-reaches-only-its-pane`. Each detail reads "got `not-implemented`". The 7 server-side cases pass, which shows that
  the rig, the shared data dir and the model guard work.
- **AC 13, 14, 15 and 19a** fail at their first `attach_tui` (`left: Err(NotImplemented)  right: Ok(())`). **AC 18** fails
  at step (2), `shown_session(w9:p1)` (`left: Err(NotImplemented)  right: Ok(None)`), before any raw pin is sent.
- **AC 16, 17 and 19 pass.** They are carried from 642a and use only the merged server side, so they are regression pins
  that no run had yet tested against a real OpenCode, not RED tests for 642b's code.
- Afterwards nothing leaked: no `/tmp/hlr642r-*`, no `opencode` process on 48100-48199, no private tmux server, no
  listener on 481xx.

**Also checked:** `rustfmt --check --edition 2021` on every test file and `src/tui.rs` passes. `bash scripts/lint.sh`
exits 0 (warns only: `hermetic_test.rs` 798, `attach_test.rs` 658; nothing at 900). `grep -rn "4700[0-9]\|--continue"
crates/holler-adapter-opencode` finds nothing. `git ls-files -s` shows the fixture at `100755`. The staged files are within
AC 22's list. No new test sets a socket option (`connect_timeout` only), and only the real rig's bounded polls sleep.

## Ready for F

Confirmed: RED is valid. F may implement against these tests. Notes for F:

1. **Replace the RED stubs in `src/tui.rs`** (the block headed "RED stubs (T, #642b Phase 4)") with the real builders and
   parsers. Keep their names and signatures, because the tests import them.
2. **An expired deadline answers `timeout` before anything is spawned or sent.** `ac30e` runs with `timeouts.call = 0`, so
   each tmux call must check the time left before it spawns. A runner that spawns and then races `try_wait` against a
   zero bound can flake on a loaded runner.
3. `ac30c_..._went_away_...` needs the death branch's re-`GET`: a 404 there is `session-not-found`, and anything else is
   `unavailable` with the exit status.
4. Messages may name the pane id, port, route, status and tmux's first stderr line, and nothing else from the call (Risk
   3). `echoes_nothing_it_did_not_author` checks the binary path, the directory, `sleep 3600`,
   `OPENCODE_DISABLE_TERMINAL_TITLE` and the format.
