# Run the full Holler review battery

Paste this prompt to Claude (or another agent) as the whole message.

You are running the Holler review battery. Do not implement product findings. Do not merge review axes. The binding runbook is docs/reviews/review-battery.md — follow it. Tracker: docs/agents/issue-tracker.md. Base is origin/main unless I name another. Phase 2 is idempotent: reuse prior epics when they are still fresh, and run the selected passes in parallel. Do not skip a pass that is on the agreed list, and do not invent extra reviewers.

Phase 1 is a hard gate. Run every pre-flight check; do not stop at the first failure. Print the full tables. Do not start Phase 2 until pre-flight is PASS, or the operator has waived each FAIL in writing. Do not run the product test suite (cargo test, cargo clippy, cargo fmt --check) as part of pre-flight.

Goal after Phase 1: for each review pass, review the current branch, then create a GitHub epic and child issues from the findings. Keep this first sentence on every review pass, including conditionals:

Create an epic and issues with the results of the following review

Filing rules
- One epic per pass, title: [review] <axis> — <branch> vs origin/main
- One child issue per high-conviction finding (file + region + failure mode)
- If native sub-issues fail, put Part of #<epic> at the top of each child body
- Do not implement findings
- Do not merge axes into one epic
- Two findings are the same when they share file + region + failure mode, not wording. If a later pass rediscovers an earlier one, comment on the existing child; do not open a duplicate
- Style, formatting, and a second generic PR reviewer are not passes

---

## Phase 1 — Pre-flight

Purpose: prove the tree is reviewable. A dirty, tool-less, or junk-filled checkout produces a false review.

Run all of the following. **Do not hand-format the tables.** From the repo root:

    node docs/reviews/preflight.mjs

If that script does not exist yet in this repo, run the probes below by hand and print the same tables — the probe lists are the spec the script would implement. Transcribe stdout as-is; do not rewrite Result or Kind cells. Then stop and follow the Phase 1 output rules (FAIL / cleanup offer / All systems go).

### How to print Result and Kind

Distinguish cells with **icons first**. Color is extra. Markdown tables often strip CSS, so a Result of `PASS` or `PASS (green)` looks the same as every other row — that is a miss. Never write a parenthetical color name.

Every Result and Kind cell is copied exactly from "Print as": a distinctive icon, then the label, wrapped in `<span style="color:…">` so tinted text appears when the renderer allows it.

Result (tools Status maps onto this; checks verdict maps onto this):

| Value | Print as (the entire cell) |
|---|---|
| PASS or installed | `<span style="color:#16a34a">✅ PASS</span>` |
| FAIL or missing | `<span style="color:#dc2626">❌ FAIL</span>` |
| WARN or broken | `<span style="color:#d97706">⚠️ WARN</span>` |

Kind on the **checks** table — class of the row, not the verdict:

| Value | When | Print as |
|---|---|---|
| gate | a FAIL would hard-stop | `<span style="color:#dc2626">🔒 gate</span>` |
| advisory | WARN only | `<span style="color:#d97706">⚡ advisory</span>` |
| info | never fails this check | `<span style="color:#2563eb">ℹ️ info</span>` |

Kind on the **tools** table:

| Value | Print as |
|---|---|
| cli | `<span style="color:#2563eb">🛠️ cli</span>` |
| doc | `<span style="color:#6b7280">📄 doc</span>` |
| skill | `<span style="color:#7c3aed">🧩 skill</span>` |

Kind on the **detritus** table:

| Value | Print as |
|---|---|
| tmp | `<span style="color:#6b7280">🧹 tmp</span>` |
| scratch | `<span style="color:#6b7280">📝 scratch</span>` |
| handoff | `<span style="color:#d97706">📦 stale</span>` — **stale candidate** only |
| archived | `<span style="color:#6b7280">📚 archived</span>` — closed story, not detritus |
| agent | `<span style="color:#d97706">🤖 agent</span>` |
| report | `<span style="color:#6b7280">📊 report</span>` |
| worktree | `<span style="color:#d97706">🌳 worktree</span>` |
| harness | `<span style="color:#dc2626">🔌 harness</span>` |
| other | `<span style="color:#6b7280">📁 other</span>` |

Suggested action (Result-style cell): `<span style="color:#dc2626">🗑️ delete</span>`, `<span style="color:#d97706">🙈 gitignore</span>`, `<span style="color:#6b7280">✋ leave</span>`, `<span style="color:#16a34a">📌 commit</span>`, `<span style="color:#d97706">🔍 review then delete</span>` (stale handoffs only). Historical handoffs print `<span style="color:#6b7280">📚 leave (archived)</span>` and are never on the cleanup offer.

### Tools table

Probe each row. Result is installed / missing / broken (present but auth or version fails). Include the version or error in Notes. Print columns: Tool, Kind, Result, Notes.

| Tool | Kind | Needed for | How to probe |
|---|---|---|---|
| git | cli | status, diff vs origin/main, branch name | git --version; git rev-parse --is-inside-work-tree |
| gh | cli | file epics and child issues | gh --version; gh auth status; gh repo view |
| docs/reviews/review-battery.md | doc | runbook | file exists |
| docs/agents/issue-tracker.md | doc | issue create/view | file exists |
| docs/adr/ADR-0001.md–ADR-0006.md | doc | Spec pass | files exist |
| ~/Projects/playbook/workflow/skills/thermo-nuclear-code-quality-review/SKILL.md | skill | Shape | canonical file exists |
| ~/Projects/playbook/workflow/skills/thermo-nuclear-review/SKILL.md | skill | Truth+Safety | same |
| ~/Projects/playbook/workflow/skills/thermos/SKILL.md | skill | optional 1+2 together | same; missing is WARN, not FAIL — run 1 and 2 separately |
| ~/Projects/playbook/workflow/skills/code-review/SKILL.md | skill | Spec | same |
| ~/Projects/playbook/workflow/skills/security-review/SKILL.md | skill | Exploitability | same |
| ~/Projects/playbook/workflow/skills/tests-as-evidence/SKILL.md | skill | Tests | same |
| stub ~/.claude/skills/<name>/SKILL.md (or ~/.grok, ~/.config/opencode, ~/.agents) | skill | harness discovery | at least one harness stub exists and contains "thin pointer"; missing stubs are WARN if canonical exists |
| ./.agents/skills, ./.claude/skills, ./.grok/skills in this repo | skill | must be **absent** | **PASS if missing.** Those trees were moved to playbook. Do not FAIL thermos, code-review, security-review, tests-as-evidence (or any sibling) for not existing here. WARN only if the directory is present (vendored copy). |

A skill is installed if the **playbook** SKILL.md exists. Stubs are user-global pointers (`~/.agents/skills`, not `./.agents/skills`). FAIL the canonical row if playbook is missing that skill. A missing `./.agents/skills/<name>/SKILL.md` is expected and is not a fail.

### Checks table

Every row runs. FAIL is a hard stop. WARN is shown and does not block unless the operator says it should. Print columns: Check, Kind, Result, Notes. Kind is gate unless the row is documented as WARN-only (advisory) or never fails (info). Render Kind and Result with the HTML+icon cells above.

Probe reference (not the printed table):

| Check | Kind | How | FAIL when |
|---|---|---|---|
| Inside a git work tree | gate | git rev-parse | not a repo |
| HEAD is a branch, not detached | gate | git symbolic-ref -q HEAD | detached HEAD |
| Not in the middle of merge/rebase/cherry-pick | gate | look for MERGE_HEAD, REBASE_HEAD, CHERRY_PICK_HEAD | any in-progress git operation |
| Review surface | info | git branch --show-current; git diff origin/main...HEAD --stat | never FAIL. On main (or HEAD == origin/main) with an empty three-dot diff: tell the operator we are reviewing the **entire codebase**, not a branch range. Phase 2 then reviews the tree, not `git diff origin/main...HEAD`. |
| origin/main is fetchable | advisory if origin unreachable and local main exists; otherwise gate | git fetch origin main (or git remote show origin) | cannot reach origin; then compare to local main and WARN that the base may be stale |
| Diff vs base is non-empty | gate | git diff origin/main...HEAD --stat | FAIL only on a **non-main** branch with an empty three-dot diff (nothing to review). Empty on main is the whole-codebase case above, not an error. |
| Working tree is clean | gate | git status --porcelain=v1 | any output: uncommitted staged, unstaged, or untracked files that are not ignored. Thorough review means the commit graph is the review surface. |
| No ignored-but-present build junk in the diff | gate | git diff origin/main...HEAD --name-only | committed target/, holler-state/, *.log, sessions.toml, attach.toml |
| gh can file issues | gate | gh issue list --limit 1 | auth missing, wrong repo, or permission denied |
| Spec source is findable | advisory | commit messages / branch name / gh | WARN if no issue number and no ADR-only story; Spec pass will ask. Not FAIL. |
| Secrets | gate | scan the diff for private keys, AWS keys, tokens, .env bodies | FAIL if a secret appears in the committed diff; do not print the secret |
| Huge blobs | advisory | git diff origin/main...HEAD --stat | WARN on any file > 500KB in the diff |
| Build guards still pass | gate | scripts/lint.sh | lint.sh fails (dead-code allow without issue link, process::exit outside main.rs, file-size guard, dep-feature-comment) |

### Detritus

Scan for leftover work that will pollute the review or get committed by accident. Do **not** delete anything until the operator says yes.

Look for, at least:

- Untracked or unignored: *.tmp, *.temp, *.swp, *~, .DS_Store, Thumbs.db
- Scratch: .scratch/, tmp/, /tmp copies inside the repo, untitled dumps
- Pipeline / agent leftovers: *.prompt.txt, *.usage.json, compaction dumps
- Generated reports sitting untracked: target/, *.orig, *.rej
- Orphan git worktrees: git worktree list — merged or gone branches still checked out (this repo runs multiple concurrent per-issue worktrees; check each still maps to an open issue/PR)
- Accidental harness skill dumps (directories named for unused agents at repo root). Do not touch .agents, .claude, .grok, .opencode, .cursor
- node_modules / target that is not expected in a worktree (note only; do not offer to rm -rf)

Handoffs that are not the story under review: `docs/handoffs/*` not matching the current issue number, if this repo has adopted that convention. For each such dir, classify before listing it:

1. Extract its issue number from the dir name (`NNNN-slug` or `NNN-slug`). `gh issue view <N> --json state,title`.
2. `git log --follow --format=%H -- docs/handoffs/<dir>` — the commit list for that path.
3. Classify:
   - Issue CLOSED, and every commit touching the dir predates (or is) the commit that closed it (no edits after close) → **historical, not detritus**. Leave it out of the cleanup offer; note it in the table as "closed story, archived" only.
   - Issue CLOSED, but a commit touched the dir after the closing commit → **stale candidate**. The handoff may describe a state the merged code no longer matches — flag for the cleanup offer below, do not delete without confirming the drift (spot-check one claim in the newest touched file against current code/tests before offering deletion).
   - Issue OPEN and not the branch under review → leave alone, not detritus (someone else's live work).
   - No issue number resolvable, or `gh issue view` errors (deleted/private/wrong repo) → WARN, list with reason, do not offer deletion (no evidence to act on).
4. Print one row per candidate dir in a handoffs table (this is part of detritus output, not a separate gate): path, issue #, issue state, last-touched commit date (America/Boise, UTC in parens), classification, suggested action. Only "stale candidate" rows get a suggested action of "review then delete"; "historical, not detritus" rows get "leave (archived)". OPEN-other rows may be omitted from the table or shown as "leave (live work)" with Result ✋ leave — never offered for deletion. Unresolvable rows: ⚠️ WARN, suggested action "leave (no evidence)".

Print the general detritus table with columns: Path, Kind, Size, Result. Kind uses the detritus symbols above. Result is the suggested action (🗑️ delete / 🙈 gitignore / ✋ leave / 📌 commit). Do **not** put historical handoffs in that table as junk.

If the general list is non-empty **or** any "stale candidate" handoff rows exist: **offer to clean** — name each path and the exact command you would run. For stale handoff dirs, the command is `git rm -r docs/handoffs/<dir>` (or `rm -r` plus `git add -u` if untracked), same as today's offer. Wait. Do not delete without the operator's yes. "Historical, not detritus" rows are never offered for deletion; they are not junk, they are closed-story records worth keeping. If the operator agrees, clean, then re-run git status and show it clean.

### Phase 1 output (print all of this)

1. Tools table: Tool, Kind, Result, Notes
2. Checks table: Check, Kind, Result, Notes
3. Detritus table: Path, Kind, Size, Result — or "none found". Plus the handoffs table (one row per `docs/handoffs/*` dir that is not the story under review), if this repo has any: path, issue #, issue state, last-touched, classification, suggested action.
4. Verdict line using the same Result icons: `<span style="color:#16a34a">✅ PRE-FLIGHT: PASS</span>` or `<span style="color:#dc2626">❌ PRE-FLIGHT: FAIL</span>` (and `<span style="color:#d97706">⚠️ PRE-FLIGHT: WARN</span>` if the only issues are WARN/detritus)
5. If FAIL: list every failing row; stop. Do not start Phase 2.
6. If general detritus exists or any handoff row is a **stale candidate**, and the tree is otherwise PASS: stop and ask whether to clean those paths only, then re-run status. Do not include historical/archived or OPEN-other handoffs in that question.
7. If PASS and no detritus (or detritus waived): stop and ask exactly: All systems go. Proceed to the reviews? Do not start Phase 2 until the operator answers yes.

---

## Phase 2 — Reviews

Idempotent. Prior runs of this battery on this branch already filed `[review] <axis> — <branch> vs origin/main` epics. Do not open a second epic with the same title unless the operator asks for a fresh one.

### 2a. Plan (stop and wait)

1. Review surface: if Phase 1 said **entire codebase**, use the tree at HEAD (not an empty `git diff origin/main...HEAD`). Otherwise `git diff origin/main...HEAD` and the file list. Scan the trigger matrix in docs/reviews/review-battery.md. Candidate passes = default battery 1–5 plus every conditional whose row fired.
2. Find prior epics. `gh issue list --state all --limit 100 --json number,title,createdAt,url,state` and keep issues whose title matches `[review] <axis> — <current-branch>`. Match axis names exactly: Shape, Truth+Safety, Spec, Exploitability, Tests, Protocol, Concurrency, Rollout, Scope.
3. Print a table, one row per candidate pass. Show times in America/Boise (MDT or MST) with UTC in parens if you have the raw UTC timestamp.

| Pass | Prior epic | Run at | Age | Proposed action |
|---|---|---|---|---|
| Shape | #123 | 7:34 PM MDT (01:34 UTC) | 3h | skip — ran < 24h ago |
| Spec | — | — | — | run — no prior epic |
| Tests | #140 | 4:10 PM MDT yesterday | 30h | re-run — older than 24h |

Proposed action rules:
- No prior epic → **run** (create the epic as usual).
- Prior epic created less than 24 hours ago → **skip** (reuse it). Link it.
- Prior epic created 24 hours or more ago → **re-run** (suggest running again). Re-run appends new child issues to the existing epic and comments that this is a re-run at the current HEAD SHA. Do not open a duplicate epic with the same title.
- Closed prior epic → **re-run** on that epic (reopen only if the operator asks).

4. Offer the operator changes: add or drop a pass, force re-run of a fresh skip, force skip of a stale re-run, or ask for a **fresh** epic (new issue, same axis, title suffixed with the date) instead of appending. Wait. Do not start any review until they confirm the list.

### 2b. Run (only after the list is confirmed)

Launch every pass whose action is **run** or **re-run** **in parallel**. Do not wait for one pass to finish before starting another. `/thermos` may stand in for Shape and Truth+Safety together only if **both** are on the confirmed list as run/re-run; otherwise launch those two skills on their own.

Each parallel pass still begins with:

Create an epic and issues with the results of the following review

On **run**, create the epic. On **re-run**, do not create a second epic; comment on the existing one and add children. On **skip**, do nothing.

After every parallel pass has finished, print a rollup: epic URLs, child counts this run, skipped passes and why, unused conditionals and why.

Default battery (always)

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

Conditional prompts (only if the matrix row fired) are in docs/reviews/review-battery.md under "Conditional — run only when the matrix row fires". Use those verbatim. Do not invent extra reviewers.

Skills live in ~/Projects/playbook/workflow/skills. Harness stubs in ~/.claude/skills, ~/.grok/skills, ~/.config/opencode/skills, and ~/.agents/skills point there. If a skill cannot be invoked as a slash command, read the playbook SKILL.md (via the stub) and follow it. Apply docs/reviews/overlays/<name>.md when present.

When you are done, do not start fixing. Hand back the Phase 1 verdict plus the Phase 2 rollup only.
