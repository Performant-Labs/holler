# Holler review battery

How to run the multi-review setup, what each pass is for, and in which order. Canonical skills live in `~/Projects/playbook/workflow/skills/`. New-repo wiring (shims, overlays): `~/Projects/playbook/workflow/review-battery-setup.md`. Claude, Grok, and OpenCode load **global stubs** (`~/.claude/skills`, `~/.grok/skills`, `~/.config/opencode/skills`, `~/.agents/skills`) that point there — the same pattern as CI and the coding pipeline. Holler does not vendor copies (`./.agents/skills` missing is expected). Project deltas: `docs/reviews/overlays/`.

Want to run all of them? Use the prompt in this doc: [review-battery-all.md](review-battery-all.md)

Phase 1 tables come from `node docs/reviews/preflight.mjs` (do not hand-draw Result/Kind cells).

An extra pass is justified only when you can say what it is **allowed to ignore**. Style, formatting, and a second generic PR reviewer are not passes.

## How to file

Every pass below is a single agent prompt. **Keep the first sentence exactly as written.** The agent reviews, then files:

- one GitHub **epic** (parent issue) titled `[review] <axis> — <branch> vs <base>`
- one **child issue** per high-conviction finding (file + region + failure mode)
- `Part of #<epic>` on each child if native sub-issues cannot be created

Do not implement the findings. Do not merge axes into one epic. Base is `origin/main` unless named otherwise. Spec sources are the branch's GitHub issue, then `docs/adr/`, then `docs/protocol/v2.md` for wire contracts. Tracker: `docs/agents/issue-tracker.md`.

Two findings are the same finding when they share file + region + failure mode, not wording. If a later pass rediscovers an earlier one, comment on the existing child instead of opening a duplicate.

Cursor `/thermos` (or `/thermos` here) runs passes 1 and 2 in parallel. If you use it, run the **Thermos (1+2)** prompt and skip the two solo Thermos prompts.

## Order

1. Shape — or Thermos (1+2) instead of 1 and 2
2. Truth + Safety (skip if Thermos already ran)
3. Spec
4. Exploitability
5. Tests as evidence
6. Conditional passes whose trigger matrix row fired, in the order listed under Conditional

---

### Thermos (1+2 together)

**Accomplishes:** Shape and Truth+Safety in parallel, then a short synthesis. Use this *or* the next two prompts, not both.

Create an epic and issues with the results of the following review: run `/thermos` on `git diff origin/main...HEAD`. File **two** epics from the synthesis, one per axis — `[review] Shape — …` for maintainability, file-size, spaghetti, and abstractions; `[review] Truth+Safety — …` for bugs, breakages, security, DX regressions, and protocol-contract leaks in added or modified code only. Ignore nits, whole-file rewrites of unchanged lines, and perf. Holler evidence: `docs/adr/`, `docs/protocol/v2.md`; the `holler-proto`/`holler-hub`/`holler-body`/`holler-cli` crate boundaries; the build guards in `scripts/lint.sh` and `clippy.toml`.

---

### 1. Shape

**Accomplishes:** Maintainability. Code judo, files crossing the guard thresholds (warn 600 / fail 900 lines per `scripts/lint.sh`), spaghetti growth, abstractions that do not earn their keep. Skill: `/thermo-nuclear-code-quality-review`.

Create an epic and issues with the results of the following review: run `/thermo-nuclear-code-quality-review` on `git diff origin/main...HEAD`. Allowed to ignore bugs, exploits, missing tests, and ticket fidelity. Binding evidence: `docs/adr/` (crate boundaries — `holler-proto` wire types, `holler-hub` server, `holler-body` agent runtime, `holler-cli` entrypoint), and the file-size/lint thresholds in `scripts/lint.sh` and `clippy.toml`. File one epic `[review] Shape — <branch> vs origin/main` and one child issue per high-conviction structural finding. Do not implement.

---

### 2. Truth + Safety (diff-scoped)

**Accomplishes:** Correctness and safety of the hunk: bugs, breakages, security, DX regressions, protocol-contract leaks. Only added or modified code. Skill: `/thermo-nuclear-review`.

Create an epic and issues with the results of the following review: run `/thermo-nuclear-review` on `git diff origin/main...HEAD`. Allowed to ignore whole-file rewrites of unchanged code, structure nits, and perf. Binding evidence: `docs/protocol/v2.md` (Holler protocol v2 JSON-RPC wire contract), `crates/holler-proto/src/error.rs` (error table), the `SessionManager` actor model (`crates/holler-body/src/session_manager.rs`), the ACP v2 driver (`crates/holler-body/src/acp_driver.rs`). File one epic `[review] Truth+Safety — <branch> vs origin/main` and one child issue per high-conviction breakage or safety finding. Do not implement.

---

### 3. Spec

**Accomplishes:** Whether the diff does what the issue and ADRs asked — missing requirements, wrong implementation, scope creep. Skill: `/code-review` (Matt Pocock). **File issues only from the Spec axis.** Drop the Standards axis; it duplicates Shape.

Create an epic and issues with the results of the following review: run `/code-review` against `origin/main` and keep **only the Spec report**. Spec sources: the GitHub issue on the branch, then `docs/adr/ADR-0001.md`–`ADR-0006.md`, then `docs/protocol/v2.md` for wire-contract stories. A story that changes the wire format without updating `docs/protocol/v2.md` and the golden files together is a Spec fail, not a Shape fail. Allowed to ignore style, exploitability, and how the code is shaped. File one epic `[review] Spec — <branch> vs origin/main` and one child issue per missing, wrong, or unasked requirement. Do not implement.

---

### 4. Exploitability

**Accomplishes:** High-confidence exploitable vulnerabilities only (attacker-controlled input, confirmed sink). Skill: `/security-review` (Sentry). Do not also run Trail of Bits `differential-review` on the same PR unless this pass and Thermos disagree on a real sink.

Create an epic and issues with the results of the following review: run `/security-review` on the current branch. Report HIGH confidence only. Holler sinks: the hub's control socket and WebSocket auth handshake (`circuit/authenticate`), untrusted body/hub JSON-RPC payloads on the wire, the ACP subprocess boundary (spawned agent processes, `session/request_permission` / `elicitation/create`), and anything that could leak a session token or agent working-directory path in logs. Allowed to ignore maintainability, nits, and theoretical CWE lists. File one epic `[review] Exploitability — <branch> vs origin/main` and one child issue per exploitable finding. Do not implement.

---

### 5. Tests as evidence

**Accomplishes:** Whether tests lock the new behavior (including removed behavior) and how this dies in production. Skill: `/tests-as-evidence`.

Create an epic and issues with the results of the following review: run `/tests-as-evidence` on `git diff origin/main...HEAD`. `docs/reviews/overlays/tests-as-evidence.md`: real hub+body processes only for integration/e2e claims; golden-file blessing must not launder unrelated drift; `#[ignore]`d tests are not evidence. Default death modes the diff can actually hit: session actor panic/silent exit, hub/body liveness-timeout asymmetry, dropped-WebSocket reconnect/resume, ACP subprocess crash/hang. Allowed to ignore structure and speculative refactors. File one epic `[review] Tests — <branch> vs origin/main` and one child issue per unproven behavior or untested death mode. Do not implement.

---

### Conditional — run only when the matrix row fires

Each of these is still one prompt, in this order after pass 5.

#### Protocol / wire contract

**Fires on:** `crates/holler-proto/src/**`, `crates/holler-proto/tests/golden/**`, `docs/protocol/v2.md`. **Accomplishes:** JSON-RPC v2 wire contract vs implementation; golden-file fidelity; error-table completeness.

Create an epic and issues with the results of the following review: review the wire-format hunks for JSON-RPC v2 contract drift against `docs/protocol/v2.md`, error-table (`error.rs`) completeness, and golden-file fidelity (flag any blessing run that touched files outside its own story's scope). Allowed to ignore CLI UX and file size. Skip comment-only hunks. File one epic `[review] Protocol — <branch> vs origin/main`. Do not implement.

#### Session lifecycle / concurrency

**Fires on:** `crates/holler-body/src/session_manager/**`, `crates/holler-body/src/acp_driver/**`, `crates/holler-hub/src/circuit.rs`, `crates/holler-hub/src/serve.rs`. **Accomplishes:** actor isolation, busy-turn queueing, liveness/heartbeat symmetry between hub and body, join-handle/shutdown correctness.

Create an epic and issues with the results of the following review: review session-actor and ACP-driver hunks for crash isolation (one panicking session must not affect others), busy-turn queue correctness (`-32009 session_busy`), liveness-timeout symmetry between `circuit.rs::session_loop` and `connection.rs::live_loop`, and clean task shutdown (no busy-poll wake_by_ref patterns; a real `.await` join). Allowed to ignore CLI ergonomics. Skip comment-only hunks. File one epic `[review] Concurrency — <branch> vs origin/main`. Do not implement.

#### Build guards / CI

**Fires on:** `scripts/lint.sh`, `clippy.toml`, `.github/workflows/**`, `Cargo.toml` (`[workspace.lints]`). **Accomplishes:** guards still fail closed; thresholds still enforced; no new `#[allow]` without a `// #NNN` link.

Create an epic and issues with the results of the following review: review build-guard and CI-workflow hunks for fail-closed behavior — every `#[allow(...)]` carries a `// #NNN` link, `process::exit` stays out of non-`main.rs` files, file-size guard thresholds (warn 600 / fail 900) are unchanged unless the story says so, and CI still runs `scripts/lint.sh` before the test canary. Allowed to ignore product logic. Skip script comments. File one epic `[review] Rollout — <branch> vs origin/main`. Do not implement.

#### Scope hygiene (meta)

**Fires on:** `.opencode/**`, `.claude/**`, `.agents/**`, `AGENTS.md`, `CLAUDE.md`, or a PR that mixes unrelated work. **Accomplishes:** whether this change is smuggling product work, or is unreviewable.

Create an epic and issues with the results of the following review: review the PR as a change — mixed refactor plus feature, missing issue/ADR pointer, or harness docs smuggling product code. Allowed to ignore product security on a pure role-doc sync. File one epic `[review] Scope — <branch> vs origin/main`. Do not implement.

---

## Trigger matrix

Scan the diff. Each matching row **adds** a pass after 1–5 (or confirms a default pass must actually run). No match → skip that axis.

| Diff signal (any file in the hunk) | Extra axis | Pass | Skip when |
|---|---|---|---|
| `crates/holler-proto/src/**`, `crates/holler-proto/tests/golden/**`, `docs/protocol/v2.md` | Protocol / wire contract | Protocol | Comment-only, or fixture-only JSON with no schema change |
| `crates/holler-body/src/session_manager/**`, `crates/holler-body/src/acp_driver/**`, `crates/holler-hub/src/circuit.rs`, `crates/holler-hub/src/serve.rs` | Session lifecycle / concurrency | Concurrency | Comment-only |
| `scripts/lint.sh`, `clippy.toml`, `.github/workflows/**`, `Cargo.toml` `[workspace.lints]` | Build guards / CI | Rollout | Script/workflow comment-only |
| `docs/adr/**` | Docs fidelity | Pass 3 both directions | Typo in an ADR |
| `.opencode/**`, `.claude/**`, `.agents/**`, `AGENTS.md`, `CLAUDE.md` | Meta | Scope | Pure role-doc sync |
| `Cargo.toml`, `Cargo.lock` dependency additions | Supply-chain | Fold into Shape (no dedicated pass yet) | In-range lockfile bump on green CI |

### Conditional axes with no Holler glob yet

Keep them off the default list. Turn them on by hand when the issue text says so:

| Axis | Turn on when |
|---|---|
| Threat model | New trust boundary (public-facing hub listener beyond loopback, plugin/agent surface, outbound webhook) |
| Architecture / boundaries | A new crate, a new wire transport, or a library that brings its own server |
| Release / rollout | First non-loopback production deploy, TLS-proxy config changes (ADR-0006) |

Cursor siblings that stay **out** of the default battery: `unslop`/`deslop`, `blast-radius`, `interrogate`, `review-and-ship`. Use `interrogate` when Spec comes back muddy.

## Deduping findings

Fan-out, then merge across epics.

- Shape vs Truth+Safety: keep both if one is "this file cannot be maintained" and the other is "this branch is wrong/exploitable".
- Sentry vs Truth+Safety: drop the Thermos security line if Sentry has a higher-confidence exploit path for the same sink; keep Thermos if it is a breakage/DX/protocol-leak, not an exploit.
- Spec vs everything: a missing ADR/issue requirement is Spec even if Thermos also noticed the hole.

## Installed skills

Canonical copies: `~/Projects/playbook/workflow/skills/`. Sync stubs: `bash ~/Projects/playbook/workflow/skills/install-stubs.sh`.

| Pass | Skill | Source |
|---|---|---|
| 1 Shape | `thermo-nuclear-code-quality-review` | cursor/plugins → playbook |
| 2 Truth+Safety | `thermo-nuclear-review` | cursor/plugins → playbook |
| 1+2 | `thermos` | cursor/plugins → playbook |
| 3 Spec | `code-review` (Spec axis only) | mattpocock/skills → playbook |
| 4 Exploitability | `security-review` | getsentry/skills → playbook |
| 5 Tests | `tests-as-evidence` | playbook + `docs/reviews/overlays/tests-as-evidence.md` |

Do not install a second generic code-review skill next to Thermos. Superpowers `requesting-code-review` / `receiving-code-review` are meta (how to ask, how to take a review), not a Truth pass.

Trail of Bits `differential-review` is the alternate for pass 4 when the diff is auth, the ACP subprocess boundary, or the wire handshake and you want a tie-break — not installed. Add on demand into **playbook**, then re-run `install-stubs.sh`. Do not `npx skills add` into this repo.
