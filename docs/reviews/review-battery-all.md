# Run the full Holler review battery

Paste this prompt to Claude (or another agent) as the whole message.

You are running the Holler review battery. Do not implement product findings. Do not merge review axes. The binding runbook is docs/reviews/review-battery.md — follow it. Tracker: docs/agents/issue-tracker.md. Base is origin/main unless I name another. Phase 2 is idempotent: reuse prior epics when they are still fresh, and run the selected passes in parallel. Do not skip a pass that is on the agreed list, and do not invent extra reviewers.

Phase 1 is a hard gate. Run every pre-flight check; do not stop at the first failure. Print the full tables. Do not start Phase 2 until pre-flight is PASS, or the operator has waived each FAIL in writing. Do not run the product test suite (cargo test, cargo clippy, cargo fmt --check) as part of pre-flight.

---

Filing rules, Phase 2's plan/run mechanics, and Phase 3 (Findings report) are canonical in playbook — read and follow `~/Projects/playbook/workflow/review-battery-all.md` verbatim for those sections; do not hand-copy them here, they will drift.

---

## Phase 1 — repo-specific deltas

Playbook's canonical Phase 1 pre-flight table is npm-flavored; Holler is a Rust workspace, so these
rows differ from playbook's canonical Phase 1 table. Everything else in Phase 1 (the icon/color
printing spec, the general table structure, the output rules) is canonical there — follow it
verbatim.

- ADR range: `docs/adr/ADR-0001.md`–`docs/adr/ADR-0006.md` (not playbook's `0001.md`–`0013.md`).
- Product suite: `cargo test`, `cargo clippy`, `cargo fmt --check` (not `npm test`, `npm run
  check`, `npm run test:e2e`).
- Build-junk / detritus patterns: `target/`, `holler-state/`, `*.log`, `sessions.toml`,
  `attach.toml` (not `dist/`, `coverage/`, `*.tsbuildinfo`, `playwright-report/`,
  `test-results/`).
- Extra check row (Holler has no npm-flavored equivalent): "Build guards still pass | gate |
  `scripts/lint.sh` | lint.sh fails (dead-code allow without issue link, `process::exit` outside
  `main.rs`, file-size guard, dep-feature-comment)".
- Phase 2a plan-step axis list: `Shape, Truth+Safety, Spec, Exploitability, Tests, Protocol,
  Concurrency, Rollout, Scope` (no webapp/a11y-dormant line, and shorter than Aftersight's list).

---

## Default battery pass prompts (Holler)

Use these verbatim as the five default-battery passes in playbook's Phase 2b. Conditional prompts (only if the trigger-matrix row fired) are in docs/reviews/review-battery.md under "Conditional — run only when the matrix row fires". Use those verbatim. Do not invent extra reviewers.

1. Shape — skill /thermo-nuclear-code-quality-review
   Create an epic and issues with the results of the following review: run thermo-nuclear-code-quality-review on git diff origin/main...HEAD. Allowed to ignore bugs, exploits, missing tests, and ticket fidelity. Binding evidence: docs/adr/ (crate boundaries — holler-proto/holler-hub/holler-body/holler-cli), scripts/lint.sh and clippy.toml thresholds. File epic [review] Shape — <branch> vs origin/main.

2. Truth+Safety — skill /thermo-nuclear-review
   Create an epic and issues with the results of the following review: run thermo-nuclear-review on git diff origin/main...HEAD. Only added or modified code. Allowed to ignore whole-file rewrites of unchanged code, structure nits, and perf. Binding evidence: docs/protocol/v2.md, crates/holler-proto/src/error.rs, the SessionManager actor model, the ACP v2 driver. File epic [review] Truth+Safety — <branch> vs origin/main.

3. Spec — skill /code-review (Matt Pocock)
   Create an epic and issues with the results of the following review: run code-review against origin/main and keep only the Spec report. Drop the Standards axis (it duplicates Shape). Spec sources: the GitHub issue on this branch, then docs/adr/ADR-0001–ADR-0006, then docs/protocol/v2.md for wire-contract stories. A story that changes the wire format without updating docs/protocol/v2.md and the golden files together is a Spec fail. Allowed to ignore style, exploitability, and how the code is shaped. File epic [review] Spec — <branch> vs origin/main.

4. Exploitability — skill /security-review (Sentry)
   Create an epic and issues with the results of the following review: run security-review on this branch. HIGH confidence only. Sinks: the hub control socket and WebSocket auth handshake, untrusted body/hub JSON-RPC payloads, the ACP subprocess boundary, secrets or paths leaking in logs. Allowed to ignore maintainability, nits, and theoretical CWE lists. Do not also run Trail of Bits differential-review unless this pass and Thermos disagree on a real sink. File epic [review] Exploitability — <branch> vs origin/main.

5. Tests as evidence — skill /tests-as-evidence
   Create an epic and issues with the results of the following review: run tests-as-evidence on git diff origin/main...HEAD. docs/reviews/overlays/tests-as-evidence.md: real hub+body processes for integration/e2e claims, golden-file blessing must not launder unrelated drift, #[ignore]d tests are not evidence. Death modes the diff can actually hit: session actor panic/silent exit, hub/body liveness-timeout asymmetry, dropped-WebSocket reconnect/resume, ACP subprocess crash/hang. Allowed to ignore structure and speculative refactors. File epic [review] Tests — <branch> vs origin/main.

Skills live in ~/Projects/playbook/workflow/skills. Harness stubs in ~/.claude/skills, ~/.grok/skills, ~/.config/opencode/skills, and ~/.agents/skills point there. If a skill cannot be invoked as a slash command, read the playbook SKILL.md (via the stub) and follow it. Apply docs/reviews/overlays/<name>.md when present.
