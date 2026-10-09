# Handoff-F: Phase 6 (the Workflow's implement phase; Phase 5 in the role doc) - #684 test kit slice e: `FakeHost`, `FakeHarness` and their conformance suites

**Date:** 2026-10-09
**Branch:** issue-684-implementation
**Issue:** #684

## What was done

All four files were stubs before this change.

- `crates/holler-pane-testkit/src/host.rs` (227 lines): `HostOp` with its `PortOp` names (`host.*`), and `FakeHost`. Its
  state is sessions by pane name (a `BTreeMap`, so `sessions()` is sorted), each with its cwd and pids, plus the argv log
  and a pid counter from 10 000. It also has the inspection methods (`sessions`, `cwd`, `runs`) and the scenario controls
  (`exit_process`, `end_session`).
- `crates/holler-pane-testkit/src/conformance/host.rs` (270 lines): `host_cases`, `run_host_conformance`, and the 9 cases
  of the brief's host table, in order, in one `CASES` table.
- `crates/holler-pane-testkit/src/harness.rs` (502 lines): `HarnessOp` (`harness.*`), `Quirk`, `ServerState`,
  `ServerView`, `TuiView` and `FakeHarness`. Its private `World` holds servers by port, a data directory per port (default
  `"default"`), the sessions of each directory in creation order, TUIs by pane, the quirks that are on, the abort log, and
  the session and pid counters.
- `crates/holler-pane-testkit/src/conformance/harness.rs` (426 lines): `HarnessRig` and `HarnessRig::sample`,
  `harness_cases`, `run_harness_conformance`, and the 15 cases of the brief's harness table, in order.
- `CHANGELOG.md`: one `### Enhancements` entry under `[Unreleased]`, right after the slice-a test-kit entry, linking
  [#684](https://github.com/Performant-Labs/holler/issues/684).
- `docs/handoffs/684/evidence.md` (new), this handoff, and an F entry in `docs/handoffs/684/decisions.md`.

## Design decisions

- **The public API is the brief's, item for item.** No extra public type, method or field; every helper is private.
  `HarnessRig` is in `conformance/harness.rs` with the 3-tuple `fresh`, as the brief and T's tests have it. A's W-1
  (align with #683's `HerdrFixture<H>` bundle) was not taken up by an amended brief, so it is left to the W-5 follow-up.
- **One rule per check, shared through small `World` helpers.** As the brief's Risks section suggests:
  - `reach(port, op)`: running goes on, frozen is the `timeout`, killed or never served is the unreachable `unavailable`.
  - `known(port, session)`: `session-not-found` when the port's data directory lacks the id.
  - `tui_port(pane)` and `show(pane, ..)` carry the no-TUI `unavailable`.
  - `signal(port, to)` covers `freeze`, `thaw` and `kill`: a port never served is unreachable, and so is a killed server,
    except that killing it again is `Ok`.

  Each port method is `enter`, then those checks in the brief's order, then one state change.
- **The quirks are one-line branches at the spot the contract would refuse.**
  - `AbortUnknownAcked` skips `known` (the abort is logged either way, as the brief says: "log it, `Ok`").
  - `SelectAckedWithoutTui` returns `Ok(())` before the no-TUI error, and changes nothing.

  Neither is a second code path.
- **The counters count what was minted** (`FIRST_PID + pids_minted`; the session number is `sessions_minted + 1`), so
  `HostState` and `World` derive `Default` and need no hand-written constructor. Session ids are
  `format!("ses_{n:026x}")`, 30 characters.
- **`health` reads the state directly** (`state(port) == Some(Running)`). It does not call `reach` and discard an error,
  and it fails only through the fault switch.
- **The host suite reads `ps` as a set.**
  - A `started` helper runs `sleep 30` and returns the pid it added; a run that adds none fails the case. Cases 3, 4, 6, 8
    and 9 use it, so none of them passes vacuously on a host whose `run` starts nothing.
  - Case 5 compares sorted lists, and the rest test containment. The port fixes no `ps` order, and a real tmux adapter may
    list a shell pid.
- **The host suite's case 6 re-ensures the session before it reads `ps` after `stop_owned`** (a real session may end with
  its last process). Case 7's doc states how it reads #641: "a missing session is a typed error" covers `run` and `ps`,
  not `stop_owned`.
- **The harness suite's case 13 builds on case 12** through one `switched` helper ("as 12, then ..." in the brief).
  Session lists are tested by containment or emptiness only (OpenCode lists the most recently updated first).
- **A's warnings, folded into the docs:**
  - W-2: both fake module docs say the fakes share no state, and how a test makes them agree.
  - W-3: the suite module docs state decisions 5 and 6 with their reasons. The host suite doc and case 7 give the reading
    of #641; the harness suite doc lists what binds #642.
  - W-4: `server()`'s doc names #644 as the consumer.
  - W-5(c): the suites parse pane names with `succeeds("PaneName::parse", PaneName::parse(..))`. There is no third
    `pane_name` copy.
  - W-6(c): the three ASSUMPTION comments are on the `impl HarnessPort for FakeHarness` methods in `harness.rs`.
- **No `harness/world.rs` split.** `harness.rs` is 502 lines, under the 600-line warning. The brief's fallback was not
  needed.

## Reuse / extend-vs-new

The plan extends slice a, as the brief's "Extend vs new" asks:

- Each fake holds a `FaultSwitch<Op>` and calls `enter` first in every port method (`fault.rs:93`).
- `HostOp` and `HarnessOp` implement the merged `PortOp::as_str`.
- Both suites run on `run_cases`, `succeeds`, `expect_code` and `expect_eq` (`conformance/mod.rs:47-100`), unchanged.
  The harness suite folds the rig into the subject, `((harness, rig), guard)`, as the brief specifies.
- The naming mirrors `FakePaneStore` and its suite (`<Port>Op`, `Fake<Port>`, `faults()`, `<port>_cases()`,
  `run_<port>_conformance`, one `CASES` table).
- The frozen `timeout` uses the fault switch's `Timeout { op: op.as_str() }` shape.
- The one lock line per fake is slice a's idiom (`fault.rs:105-107`).

New objects are only the ones the brief's API names. There is no second fault switch, case runner, assertion helper or
error code.

## Architecture notes for A

- **Layers.** `holler-pane-testkit` only. Its dependency is still `holler-pane` alone: no manifest, lock-file, `lib.rs` or
  `conformance/mod.rs` change. `cargo machete` is clean, and the AC 8 `cargo tree` grep prints nothing.
- **Public interfaces.** Added, none changed: the API of the brief's "Public API of slice e" section, as listed above.
- **Contracts that bind later stories.** Each is stated in the suite docs.
  - #641:
    - `run` and `ps` of a missing session are `pane-not-found`.
    - `stop_owned` of a missing session is `Ok`.
    - An empty argv is `usage`.
  - #642:
    - Reachability is checked before existence.
    - An unknown id is `session-not-found` for `abort`, `attach_tui` and `select_session`.
    - `select_session` on a pane with no TUI is `unavailable`, checked before the id.
    - `shown_session` with no TUI is `Ok(None)`.
- **Small private helpers.** Each suite has its own, as slice a's suite does: `ensured`, `ps`, `started`, `holds`,
  `lacks`, `sorted` in the host suite; `serve`, `create`, `list`, `attach`, `select`, `shown`, `holds`, `switched` in the
  harness suite. The two `holds` are six-line helpers over different element types (pids and session ids). A shared
  generic one would need a `conformance/mod.rs` edit, which this slice may not make, so I flag the pair for the W-5
  follow-up rather than add it.

## Deviations from spec / wireframe

- **ASSUMPTION comment layout.** The brief says "one line each". Each of the three is one `//` comment whose first line
  carries the verbatim prefix `// ASSUMPTION (#642 to confirm):` and the brief's text. It is wrapped at the file's
  100-column width rather than set as one ~200-character line. AC 7's
  `grep -c 'ASSUMPTION (#642 to confirm)' crates/holler-pane-testkit/src/harness.rs` prints `3`. Joining each onto one
  line is a mechanical change if S wants it literal.
- Otherwise none. No wireframe (no UI surface).

Behaviours the brief left open, decided here and recorded in `decisions.md`:

- `navigate` does not reach the server.
- `delete_session` sends home every TUI showing the id, on any port (ids are unique across data directories).
- Freezing a frozen server and thawing a running one are `Ok`.

No test drives any of them.

## Tier 1 self-check (incl. tests now GREEN)

Commands, run from the worktree root:

```
cargo test -p holler-pane-testkit
  fake_harness_test           19 passed; 0 failed
  fake_host_test              12 passed; 0 failed
  fake_pane_store_test        22 passed; 0 failed   (slice a, unchanged)
  harness_conformance_test    13 passed; 0 failed
  host_conformance_test       12 passed; 0 failed
  pane_store_conformance_test 10 passed; 0 failed   (slice a, unchanged)
  doc-tests                    0 (the suites' examples are `text` fences)

cargo clippy --workspace --all-targets -- -D warnings      exit 0, no diagnostics
bash scripts/lint.sh                                       exit 0 (no warning on a testkit file; the 600-line
                                                           warnings it prints are for other crates' existing files)
rustfmt --check --edition 2021 crates/holler-pane-testkit/src/{host,harness}.rs \
    crates/holler-pane-testkit/src/conformance/{host,harness}.rs      exit 0
cargo machete                                              no unused dependencies
bash scripts/changelog-check.sh                            changelog-check: ok
cargo tree -p holler-pane-testkit -e normal --prefix none | grep -E '^holler-(cli|hub|adapter)'   (prints nothing)
grep -c 'ASSUMPTION (#642 to confirm)' crates/holler-pane-testkit/src/harness.rs                   3
grep -nE '\.unwrap\(\)|\.expect\(|panic!|unreachable!|assert(_eq|_ne)?!|todo!|unimplemented!' \
    <the four src files> | grep -vE '^[^:]+:[0-9]+:\s*//'                                        (no match)
    (without the second grep, the only hits are the two `/// assert_eq!(` usage examples in the suites'
    `text`-fenced docs, conformance/host.rs:84 and conformance/harness.rs:133, the same pattern as slice a's
    conformance/pane_store.rs:96; a `text` fence is never compiled)
wc -l: host.rs 227, conformance/host.rs 270, harness.rs 502, conformance/harness.rs 426 (all under 600)
cargo build --workspace                                    exit 0
cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load   (CI's command)
                                                           exit 0: 109 test binaries, 1164 passed,
                                                           0 failed, 5 ignored
bash scripts/test-hooks.sh                                 exit 0 (11 ok, 0 not ok)
```

A mutation readout beyond T's tests: a throwaway crate outside the repo (it is not in the diff) ran every mutant of T's
two conformance files through the suites and printed every failure with its detail. Both fakes and both `Break::Nothing`
wrappers pass. Each mutant fails its named case for the intended reason, for example:

- `HealthAlwaysTrue`: "health of a port never served: expected false, got true".
- `SelectBroadcasts`: "shown_session of the other pane: expected Some(a), got Some(b)".
- `StopsEverySession`: "ps(b) after stop_owned(a) lacks pid 10001: []".

The extra failures are expected knock-ons:

- `RunIsNoop` fails every case that needs a started process.
- `PsOfMissingIsEmpty` also fails case 2's re-check.
- The separate data directory also fails case 15's attach on the second port.

## Evidence appendix

`docs/handoffs/684/evidence.md`. It covers:

- `enter`'s order, and the wedged `timeout` shape that the frozen `timeout` copies;
- the lock idiom;
- `run_cases`' per-case `fresh` and drop order;
- the closed code strings, and the refusal and failure classes;
- `PaneName`'s `Display` and `Ord`;
- `select_session` taking no port.

## Tests that look wrong (for T)

None. Every test of the four files passes against the brief's behaviour as written.

## Known issues

None against the acceptance criteria.

One run-environment note, outside the diff: concurrent runs share one scratchpad directory. My first workspace test log
used the generic name `ws-test.log`, which the #682 run was writing at the same time, and I reused a `probe/` directory
another run had created. I stopped my run and re-ran with private logs under `scratchpad/f684-host-harness/`. The
results above come only from those private logs and from commands run in the foreground.

## Files changed

- `crates/holler-pane-testkit/src/host.rs`
- `crates/holler-pane-testkit/src/harness.rs`
- `crates/holler-pane-testkit/src/conformance/host.rs`
- `crates/holler-pane-testkit/src/conformance/harness.rs`
- `CHANGELOG.md`

Pipeline artifacts (not production code): `docs/handoffs/684/handoff-F.md`, `docs/handoffs/684/evidence.md`,
`docs/handoffs/684/decisions.md`.
