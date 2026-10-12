# Handoff-F: Phase 6 (S rework, round 1) - #839 DRIVEN follows each pane's session of record

**Date:** 2026-10-12 (10:30 MDT)
**Branch:** `issue-839-implementation` (worktree `.claude/worktrees/0839-driven`, head `fde8eb8`). F committed nothing;
every change is in the working tree.
**Issue:** #839
**Reworks:** `handoff-S.md` (REWORK, `reworkKind: production`). This file covers this round only. The first round's
handoff, for the whole implementation, is in git: `git show d480424:docs/handoffs/839/handoff-F.md`.

**F verdict: done.**
- **REWORK 2 (production) is fixed.** `session-held` now quotes the body's advertised name. It builds
  `holler release <name>` only from a name that parses as a `RoutableName`.
- **REWORK 1 is test-only, so it is T's.** It is set out in full under "Tests that look wrong (for T)", because a
  production-kind S REWORK gives T-green no note of its own. I checked every clause of it against the production
  code, path by path, and production already does what each clause pins.

## What was done

- **`crates/holler-pane/src/hub.rs`:**
  - New `session_held(id, name, kind, reason) -> PaneError`, placed beside the `SESSION_HELD` code it raises.
  - The body session's name is printed with `findings::quoted`.
  - `holler release <name>` is built only from a name that parses as `holler_proto::RoutableName`. With any other
    name, or no name, the message is the existing fallback sentence: it names the id (quoted) and spells no name.
  - D4's content is kept: the hold's kind, the reason (quoted, as before) and the release.
  - The module doc gains one paragraph. `Advertised.name`'s doc now says that its session part is untrusted, as
    `harness_session_id`'s doc already said for the id.
- **`crates/holler-cli/src/pane/wiring/hub.rs`:**
  - `HubClient::held` keeps its one roster read and returns `session_held(..)`. `refused`'s `Held` arm calls it.
  - The `SESSION_HELD` import is gone.
- **`crates/holler-pane-testkit/src/hub.rs`:** `FakeHub::judge`'s hold arm returns
  `session_held(id, Some(&body.name), kind.as_str(), None)` instead of its own copy of the sentence. The module doc
  says so.
- **`docs/handoffs/839/evidence.md`:** four facts from unchanged code (see "Evidence appendix").

**The message, before and after.** The "after" column comes from a throwaway program in the session scratchpad,
outside the repo, that calls `session_held`. The "before" column is the old format string,
`"{name} is held ({kind} hold{reason}); run holler release {name} to lift it"`, applied to the same name.

| Case | Before | After |
|---|---|---|
| a valid name | `kiwi/demo-c1r1 is held (operator hold); run holler release kiwi/demo-c1r1 to lift it` | `"kiwi/demo-c1r1" is held (operator hold); run holler release kiwi/demo-c1r1 to lift it` |
| a reason with a quote and a newline | (the reason was quoted already) | `"kiwi/oc" is held (default hold, reason "night \"freeze\"\n"); run holler release kiwi/oc to lift it` |
| the name `kiwi/oc; rm -rf ~` | `kiwi/oc; rm -rf ~ is held (operator hold); run holler release kiwi/oc; rm -rf ~ to lift it` | `the session that advertises "ses_…01" is held (operator hold); release it with holler release` |
| a name with an ESC sequence and a newline | printed raw, ESC and newline included, twice | the same fallback |
| `kiwi/OC` (outside the name grammar) | `holler release kiwi/OC`, which works: `release` sends its text as it is (`cli.rs:566-568`), and the hub matches it exactly | the same fallback (see "Known issues") |
| no name (the roster read failed) | the fallback | the fallback, unchanged |

## Design decisions

- **One builder in `holler-pane`, called by both implementations of the port**, rather than a fix in `HubClient`
  alone.
  - **Why:** `FakeHub` carried a copy of the same sentence (A-dup's warn 2 names it). If only the real client were
    fixed, the fake would still spell the name raw, and the verbs' tests over `FakeHub` would pin words the real
    client no longer says. With one builder, `first.rs`'s pin (`holler release kiwi/demo-c1r1`) pins the real
    client's words.
  - **Placement:** beside `SESSION_HELD` in `holler_pane::hub`, the file that owns the code (D7). This is the same
    pattern as `driven::not_driven`, which builds `pane-not-driven`'s message in `holler-pane`. `holler-pane` already
    depends on `holler-proto` (for `PaneName`), so `RoutableName` adds no dependency.
  - **Rejected:** a `release_command` beside `switch_command` in `findings.rs`. It would have one caller, and the
    parse is one line inside `session_held`.
- **The subject is quoted even when the name parses** (D3: "every advertised name and id a message prints goes
  through `findings::quoted`"). The command spells the parsed `RoutableName` bare, as `driven::named` and
  `switch_command` do for an id.
- **A name that does not parse gets the fallback sentence.** This follows S's instruction, "keep the existing
  fallback sentence, which spells no name". `control/release` resolves a name by exact match against the roster's
  row names (`evidence.md`), so the command works for every name it is built from.
- **`kind` stays unquoted.**
  - The real client: `classify` admits only `operator` and `default` (`hub_refusal.rs:68-75`).
  - `FakeHub`: it passes `HoldKind::as_str()`.
  - So `kind` is never advertised text.

## Reuse / extend-vs-new

- **Extended:** `holler_pane::hub`, the story's own file that owns `SESSION_HELD`, now holds the builder.
- **Reused:** `findings::quoted` and `holler_proto::RoutableName`. There is no new module and no new dependency.
- **Removed:** a copy. `FakeHub`'s `session-held` sentence is gone; that is the second item of A-dup's warn 2. The
  other copies A-dup named (`FakeHub::why` and the pane gate's sentence) are untouched, because S's rework does not
  cover them.

## Architecture notes for A

- **One new public function,** `holler_pane::hub::session_held`, in a module this story created.
  - `HubPort` is unchanged.
  - No module, crate or dependency is new. The dependency direction is unchanged: `holler-cli` and
    `holler-pane-testkit` already depend on `holler-pane`, and `holler-pane` already depends on `holler-proto`.
- **The real client's `session-held` text changes:** the subject is quoted. The code, the exit (3) and every
  existing pin are unchanged.

## Deviations from spec / wireframe

None from the brief.

S named `wiring/hub.rs`. This round also touches `holler-pane/src/hub.rs` (the builder, and `Advertised.name`'s doc)
and the test kit's `hub.rs` (which now calls the builder). Both are this story's own new files; the reason is under
"Design decisions".

## Tier 1 self-check (incl. tests now GREEN)

```text
$ cargo build --workspace --all-targets                                          Finished (exit 0)
$ cargo clippy --workspace --all-targets -- -D warnings                          Finished (exit 0)
$ rustfmt --check --edition 2021 crates/holler-pane/src/hub.rs \
    crates/holler-cli/src/pane/wiring/hub.rs crates/holler-pane-testkit/src/hub.rs                (exit 0)
$ bash scripts/lint.sh                                (exit 0; size warnings only; the three files are 106, 223, 310 lines)
$ bash scripts/changelog-check.sh origin/main                                    changelog-check: ok

$ cargo test -p holler-pane                                                      every binary ok
$ cargo test -p holler-pane-testkit                                              every binary ok; fake_hub_test 13 passed
$ cargo test -p holler-cli --test pane_verbs                                     381 passed; 0 failed
    driven::first::reset_first_on_a_held_pane_is_session_held ... ok
    target_flags::every_other_held_error_falls_through_to_the_generic_arm ... ok
$ cargo test -p holler-cli --test driven_e2e_test                                11 passed; 0 failed
    the_hold_key_survives_a_rebind_and_release_lets_the_first_message_through ... ok
    the_real_client_passes_the_hub_conformance_suite ... ok

$ cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load
175 test binaries: 2721 passed; 0 failed; 36 ignored (exit 0). T-green's count, unchanged.

# After the last edit, a doc-only line on `Advertised.name` made while the suite ran:
$ cargo clippy --workspace --all-targets -- -D warnings                          Finished (exit 0)
$ RUSTDOCFLAGS="-D rustdoc::broken_intra_doc_links" cargo doc -p holler-pane --no-deps   (exit 0)
$ cargo test -p holler-pane                                                      163 passed; 0 failed
$ cargo test -p holler-pane-testkit --test fake_hub_test                         13 passed; 0 failed
$ cargo test -p holler-cli --test pane_verbs -- held                             2 passed; 0 failed
$ git diff --check                                                               (exit 0)
```

**What pins the `session-held` text,** all green:
- `first.rs::reset_first_on_a_held_pane_is_session_held`: `holler release kiwi/demo-c1r1`, over `FakeHub`, which now
  produces the real client's words;
- `first.rs::a_hold_carries_across_a_reset_and_stops_only_the_prompt`: the code;
- `fake_hub_test.rs::a_hold_refuses_prompts_and_not_a_rebind`: `holler release kiwi/a`;
- `driven_e2e_test.rs::the_hold_key_survives_a_rebind_and_release_lets_the_first_message_through`: the code, through
  the real client and a real hub.

## Evidence appendix

`docs/handoffs/839/evidence.md` has a new section, "What F's rework of S's REWORK 2 (`session-held`) relies on":
- the name is unchecked advertised text (`docs.rs:519-521`, `roster.rs:855`);
- the `RoutableName` grammar (`vocab.rs:123-128`, with the segment check quoted under T-6);
- `quoted` is `{:?}` cut at 64 characters (`findings.rs:419-425`, `error.rs:691-698`);
- `control/release` resolves by exact roster name (`control_hold.rs:29-37`).

## Tests that look wrong (for T)

**None of T's tests is wrong.** Every existing pin of the `session-held` text passes unchanged (see above).

### REWORK 1 is test-only, and it reaches T only through this section

S's REWORK is `production`, so the script sets no T-green note for it (`coding-pipeline.workflow.mjs:5557-5560`, set
only for `test-only`). The full text is item 1 of `handoff-S.md`'s "Verdict". S asks for the following, in
`crates/holler-cli/tests/driven_e2e_test.rs`, which already has a real hub, a real body and `--join-held`:

- **(a) P5: holds keep exit 4.** Hold the body that drives P twice: once with an operator hold
  (`holler hold kiwi/oc`), and once with a default hold (a hub started with `--join-held kiwi/*`).
  - Under each, `holler say --pane P TEXT` and `holler interrupt --pane P TEXT` exit 4 (`HELD_EXIT_CODE`).
  - Stderr contains neither `pane-not-driven` nor `session-held`.
  - `answer` has no held arm (E125), so it is not part of this pin.
- **(b) P5: a pane hold prints `PaneHeld`.** P is healthy. A second pane is parked, and its recorded DRIVEN is P's id.
  `holler say --pane P --json TEXT` prints the `PaneHeld` object and exits 3.
- **(c) `pane-not-driven` in all three verbs, checked first:**
  - `interrupt --pane Q` and `answer --pane Q 0` exit 3 `pane-not-driven` for the pane no body drives.
  - A pane whose id two live attach sessions advertise (for example `Rig::start(&[("oa", &id), ("ob", &id)])`) exits
    3 `pane-not-driven` for `say --pane` and for `interrupt --pane`, not 2.
  - A pane whose body is offline exits 3 `pane-not-driven`.

**What production does on each path.** I read these paths in the code; I ran no test for them, because F writes
none. They should pass as built:

| Clause | Path |
|---|---|
| (a) `say` | **Hub:** `talk::admit` runs the hold's fast path (`talk.rs:233`) before the pane gate (`:238`), which gives `-32011` with `hold_kind` `operator` or `default`.<br>**CLI:** `say_cmd::control_refusal` (`say_cmd.rs:195-231`) tries, in order:<br>- `pane_not_driven` (`:200`): `None`, because `classify` gives `Held`;<br>- `pane_hold_refusal` (`:205`): `None`, because the kind is not `pane`;<br>- `is_held` (`:213-216`): `HELD_EXIT_CODE`.<br>The held message is `hold_cmd`'s `session_held: … is held` (underscore), so it never contains `session-held`. |
| (a) `interrupt` with TEXT | **Hub:** `interrupt.rs:116-118` runs `would_refuse` before the cancel (`:125`), and `control_server.rs:337` passes the frame through.<br>**CLI:** `interrupt_cmd.rs:184-198`, in the same order, gives exit 4.<br>Without TEXT, a hold does not stop an interrupt (#442), so the pin needs TEXT. |
| (b) | **Hub:** `first_refusal` (`holler-pane/src/gate.rs:84-92`) judges every pane whose session of record or recorded DRIVEN is the id. The parked second pane refuses with `-32011`, `hold_kind: "pane"`, reason `pane-parked`.<br>**CLI:** `pane_hold_refusal` (`say_cmd.rs:554-579`) prints `PaneHeld` under `--json`, exit 3. P's own record is healthy, so the CLI's routing does not stop the call first. |
| (c) unknown id | **Hub:** `-32003`.<br>**CLI:** `classify` gives `NotDriven`, which is checked first (`interrupt_cmd.rs:184`, `answer_cmd.rs:93`). |
| (c) ambiguous | **Hub:** two matches give `Ambiguous` (`live/target.rs:38-40`). That becomes `-32003` with `reason: "ambiguous"` in `control_say.rs:74-77`, `control_server.rs:324` (interrupt) and `:371` (answer).<br>**CLI:** `classify` keys on the code, not the reason, so the answer is `NotDriven`. It is checked before `is_ambiguous`: `say_cmd.rs:200` comes before `:212`, and `interrupt_cmd.rs:184` before `:200`.<br>**Setup:** a body accepts two sessions on one id, since `config.rs:297` refuses only a duplicate name. |
| (c) offline | **Hub:** it keeps a disconnected body's last presence, so `offline_advertises` gives `NotConnected` (`live/target.rs:44`). That becomes `WireError::offline` in `control_say.rs:73` and `control_server.rs:321` and `:368`.<br>**CLI:** `classify` gives `NotDriven`.<br>**Setup:** the test must wait until the hub has seen the body go (its row leaves `connected`) before it runs the verb. |

**Suggested, not required by S: a pin for REWORK 2 itself.** No test gives `session_held` a name that is not a
routable name, so reverting the quoting would fail nothing. Two ways to add one:
- Over `FakeHub`: `with_body("tok", "kiwi/a b", &id, Idle)`, then `hold("kiwi/a b", Operator)`. The refusal is
  `session-held`, and its text contains neither `kiwi/a b` nor `holler release kiwi`.
- With a valid name, the text contains `"kiwi/a"`, quoted, and `holler release kiwi/a`.

## Known issues

- **S's advisory 1 is for O or the run's agent before merge, not F:** the branch conflicts with `origin/main` in
  ADR-0021's "Decisions taken", and #839's items renumber to 34 to 47.
- **`docs/pane-control.md:290`** says `session-held` "names the `holler release SESSION` that lifts it".
  - Two cases give the fallback, which names no session: a failed roster read (already so in round 1), and a name
    that is not a routable name (new, and reachable only through a crafted advertisement).
  - I did not edit the guide: the sentence describes the normal case, and `docs_pane_control_test` pins it.
- **A trade-off of S's rule: a harmless name outside the grammar loses its command.**
  - Take a name with no shell metacharacter that is still not a `RoutableName`, such as `kiwi/OC`. Before, the
    message printed `holler release kiwi/OC`, and that command works. Now the message falls back to the id.
  - That is the "built, never spelled" rule at work: the grammar is the one check that rules out every unsafe name,
    and a body that follows ADR 0005 never advertises such a name.
  - The operator still has the id, and `holler roster` shows the name.
- **The `profile_rename_faults_test` flake from #665** predates this story. Round 1's handoff covers it.

## Files changed

**Production:**
- `crates/holler-pane/src/hub.rs`
- `crates/holler-cli/src/pane/wiring/hub.rs`
- `crates/holler-pane-testkit/src/hub.rs`

**Pipeline scratch:** `docs/handoffs/839/handoff-F.md` (this round), `evidence.md` (appended) and `decisions.md`
(appended).
