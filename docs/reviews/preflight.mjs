#!/usr/bin/env node
/**
 * Phase 1 table printer for the Holler review battery.
 * Run from the repo root: node docs/reviews/preflight.mjs
 *
 * Prints markdown tables with icon+HTML Result/Kind cells. The agent must
 * transcribe this output as-is and not reformat those cells.
 *
 * Adapted from Aftersight's reference implementation
 * (docs/reviews/review-battery-all.md's "Phase 1 — repo-specific deltas"):
 * Holler is a Rust workspace, not npm — the ADR range, product suite, and
 * build-junk/detritus patterns below differ from the npm-flavored canonical
 * defaults. Everything else (icon/color printing spec, table structure,
 * output rules) is canonical in playbook and unchanged here.
 *
 * Exit: 0 PASS (nothing to clean), 1 FAIL, 2 PASS/WARN with a cleanup offer.
 */
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const ROOT = findGitRoot(process.cwd());
const HOME = os.homedir();
const PLAYBOOK = process.env.PLAYBOOK_ROOT ?? path.join(HOME, 'Projects', 'playbook');
const BASE = process.env.REVIEW_BASE ?? 'origin/main';
const TZ = 'America/Boise';

const SKILLS = [
  ['thermo-nuclear-code-quality-review', 'Shape', false],
  ['thermo-nuclear-review', 'Truth+Safety', false],
  ['thermos', 'optional 1+2 together', true],
  ['code-review', 'Spec', false],
  ['security-review', 'Exploitability', false],
  ['tests-as-evidence', 'Tests', false],
];

const STUB_HOMES = [
  path.join(HOME, '.claude', 'skills'),
  path.join(HOME, '.grok', 'skills'),
  path.join(HOME, '.config', 'opencode', 'skills'),
  path.join(HOME, '.agents', 'skills'),
];

const HARNESS_DUMPS = [
  '.adal', '.aider-desk', '.augment', '.autohand', '.bob', '.codeartsdoer',
  '.codebuddy', '.codemaker', '.codestudio', '.commandcode', '.continue',
  '.cortex', '.crush', '.devin', '.forge', '.fx', '.goose', '.hermes',
  '.iflow', '.inferencesh', '.jazz', '.junie', '.kimchi', '.kiro', '.kode',
  '.lingma', '.mcpjam', '.minimax', '.moxby', '.mux', '.neovate', '.ona',
  '.openhands', '.pi', '.pochi', '.posit', '.qoder', '.qwen', '.reasonix',
  '.roo', '.rovodev', '.tabnine', '.terramind', '.tinycloud', '.trae',
  '.vibe', '.windsurf', '.zcode', '.zencoder',
];

// Holler-flavored (not Aftersight's npm dist/coverage/playwright/test-results):
// target/, holler-state/, *.log, sessions.toml, attach.toml (see .gitignore).
const BUILD_JUNK = [
  /^target\//, /^holler-state\//, /\.log$/, /^sessions\.toml$/, /^attach\.toml$/,
];

const SECRET_RES = [
  /-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----/,
  /AKIA[0-9A-Z]{16}/,
  /ghp_[A-Za-z0-9]{20,}/,
  /github_pat_[A-Za-z0-9_]{20,}/,
  /xox[baprs]-[A-Za-z0-9-]{10,}/,
  /AWS_SECRET_ACCESS_KEY\s*[:=]\s*\S+/,
];

const RESULT = {
  PASS: cell('#16a34a', '✅ PASS'),
  FAIL: cell('#dc2626', '❌ FAIL'),
  WARN: cell('#d97706', '⚠️ WARN'),
};
const KIND = {
  cli: cell('#2563eb', '🛠️ cli'),
  doc: cell('#6b7280', '📄 doc'),
  skill: cell('#7c3aed', '🧩 skill'),
  gate: cell('#dc2626', '🔒 gate'),
  advisory: cell('#d97706', '⚡ advisory'),
  info: cell('#2563eb', 'ℹ️ info'),
  tmp: cell('#6b7280', '🧹 tmp'),
  scratch: cell('#6b7280', '📝 scratch'),
  handoff: cell('#d97706', '📦 stale'),
  archived: cell('#6b7280', '📚 archived'),
  agent: cell('#d97706', '🤖 agent'),
  build: cell('#6b7280', '🏗️ build'),
  worktree: cell('#d97706', '🌳 worktree'),
  harness: cell('#dc2626', '🔌 harness'),
  other: cell('#6b7280', '📁 other'),
};
const ACTION = {
  delete: cell('#dc2626', '🗑️ delete'),
  gitignore: cell('#d97706', '🙈 gitignore'),
  leave: cell('#6b7280', '✋ leave'),
  commit: cell('#16a34a', '📌 commit'),
  reviewDelete: cell('#d97706', '🔍 review then delete'),
  archived: cell('#6b7280', '📚 leave (archived)'),
  live: cell('#6b7280', '✋ leave (live work)'),
  noEvidence: cell('#d97706', '⚠️ leave (no evidence)'),
};

function cell(color, text) {
  return `<span style="color:${color}">${text}</span>`;
}

function findGitRoot(start) {
  let dir = start;
  for (;;) {
    if (fs.existsSync(path.join(dir, '.git'))) return dir;
    const parent = path.dirname(dir);
    if (parent === dir) return start;
    dir = parent;
  }
}

function sh(cmd, args, opts = {}) {
  try {
    const out = execFileSync(cmd, args, {
      cwd: ROOT,
      encoding: 'utf8',
      timeout: opts.timeout ?? 15_000,
      stdio: ['ignore', 'pipe', 'pipe'],
      env: process.env,
    });
    return { ok: true, out: out.trim(), err: '' };
  } catch (e) {
    return {
      ok: false,
      out: String(e.stdout ?? '').trim(),
      err: String(e.stderr ?? e.message ?? '').trim().split('\n')[0],
      status: e.status,
    };
  }
}

function exists(p) {
  try {
    fs.accessSync(p);
    return true;
  } catch {
    return false;
  }
}

function firstLine(s) {
  return String(s || '').split('\n')[0].slice(0, 160);
}

function mdEscape(s) {
  return String(s ?? '').replace(/\|/g, '\\|').replace(/\n/g, ' ');
}

function table(headers, rows) {
  const head = `| ${headers.join(' | ')} |`;
  const sep = `| ${headers.map(() => '---').join(' | ')} |`;
  const body = rows.map((r) => `| ${r.map(mdEscape).join(' | ')} |`).join('\n');
  return `${head}\n${sep}\n${body || `| ${headers.map(() => '—').join(' | ')} |`}`;
}

function fmtWhen(iso) {
  if (!iso) return '—';
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  const local = new Intl.DateTimeFormat('en-US', {
    timeZone: TZ,
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
    timeZoneName: 'short',
  }).format(d);
  const utc = new Intl.DateTimeFormat('en-US', {
    timeZone: 'UTC',
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  }).format(d);
  return `${local} (${utc} UTC)`;
}

function bytes(n) {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KiB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MiB`;
}

function dirSize(p) {
  let total = 0;
  let count = 0;
  const walk = (d) => {
    let ents;
    try {
      ents = fs.readdirSync(d, { withFileTypes: true });
    } catch {
      return;
    }
    for (const e of ents) {
      const fp = path.join(d, e.name);
      if (e.isDirectory()) walk(fp);
      else {
        try {
          total += fs.statSync(fp).size;
          count += 1;
        } catch { /* skip */ }
      }
    }
  };
  try {
    const st = fs.statSync(p);
    if (st.isFile()) return { total: st.size, count: 1 };
  } catch {
    return { total: 0, count: 0 };
  }
  walk(p);
  return { total, count };
}

function currentIssueFromBranch(branch) {
  const m = branch.match(/(?:^|[-_/])(?:issue-)?0*(\d{1,5})(?:[-_]|$)/i);
  return m ? Number(m[1]) : null;
}

// --- probes ---

const tools = [];
const checks = [];
const detritus = [];
const handoffs = [];
const cleanup = [];

function addTool(name, kind, result, notes) {
  tools.push({ name, kind, result, notes });
}
function addCheck(name, kind, result, notes) {
  checks.push({ name, kind, result, notes });
}

const gitVer = sh('git', ['--version']);
const inRepo = sh('git', ['rev-parse', '--is-inside-work-tree']);
addTool('git', KIND.cli, gitVer.ok && inRepo.ok ? RESULT.PASS : RESULT.FAIL, gitVer.ok ? firstLine(gitVer.out) : gitVer.err || 'not a git work tree');

const ghVer = sh('gh', ['--version']);
const ghAuth = sh('gh', ['auth', 'status']);
const ghRepo = sh('gh', ['repo', 'view', '--json', 'nameWithOwner', '-q', '.nameWithOwner']);
let ghOk = ghVer.ok && ghAuth.ok && ghRepo.ok;
addTool(
  'gh',
  KIND.cli,
  ghOk ? RESULT.PASS : ghVer.ok ? RESULT.WARN : RESULT.FAIL,
  ghOk ? `${firstLine(ghVer.out)}; ${ghRepo.out}` : ghAuth.err || ghVer.err || 'gh not usable',
);

function addDoc(rel) {
  const p = path.join(ROOT, rel);
  addTool(rel, KIND.doc, exists(p) ? RESULT.PASS : RESULT.FAIL, exists(p) ? 'present' : 'missing');
}
addDoc('docs/reviews/review-battery.md');
addDoc('docs/agents/issue-tracker.md');

// Holler's ADR range is ADR-0001.md–ADR-0006.md (not playbook's canonical
// 0001.md–0013.md — a smaller, differently-named set for this repo).
const adrDir = path.join(ROOT, 'docs', 'adr');
if (exists(adrDir)) {
  const files = fs.readdirSync(adrDir).filter((f) => /^ADR-\d{4}\.md$/.test(f));
  addTool('docs/adr/ADR-0001.md–ADR-0006.md', KIND.doc, files.length >= 6 ? RESULT.PASS : RESULT.WARN, `${files.length} ADR files`);
} else {
  addTool('docs/adr/ADR-0001.md–ADR-0006.md', KIND.doc, RESULT.FAIL, 'docs/adr missing');
}

for (const [name, needed, optional] of SKILLS) {
  const canonical = path.join(PLAYBOOK, 'workflow', 'skills', name, 'SKILL.md');
  const have = exists(canonical);
  addTool(
    `playbook/workflow/skills/${name}/SKILL.md`,
    KIND.skill,
    have ? RESULT.PASS : optional ? RESULT.WARN : RESULT.FAIL,
    have ? needed : optional ? `missing (WARN — run 1 and 2 separately)` : `missing — needed for ${needed}`,
  );
}

const stubMiss = [];
for (const [name] of SKILLS) {
  let found = false;
  for (const home of STUB_HOMES) {
    const p = path.join(home, name, 'SKILL.md');
    if (!exists(p)) continue;
    const body = fs.readFileSync(p, 'utf8');
    if (body.includes('thin pointer')) {
      found = true;
      break;
    }
  }
  if (!found) stubMiss.push(name);
}
addTool(
  'harness stubs (~/.claude, ~/.grok, ~/.config/opencode, ~/.agents)',
  KIND.skill,
  stubMiss.length === 0 ? RESULT.PASS : RESULT.WARN,
  stubMiss.length === 0
    ? `thin pointers present for ${SKILLS.length} skills`
    : `no thin pointer for: ${stubMiss.join(', ')} — canonical still wins if playbook exists`,
);

const localSkillDirs = ['.agents/skills', '.claude/skills', '.grok/skills'];
const localPresent = localSkillDirs.filter((d) => exists(path.join(ROOT, d)));
addTool(
  './.agents/skills, ./.claude/skills, ./.grok/skills (must be absent)',
  KIND.skill,
  localPresent.length === 0 ? RESULT.PASS : RESULT.WARN,
  localPresent.length === 0
    ? 'absent — skills live in playbook; missing here is expected, not a fail'
    : `vendored copies still in ${localPresent.join(', ')} — delete; canonical is playbook/workflow/skills`,
);

// Holler-specific: scripts/lint.sh is the build guard gate (dead-code allows
// without an issue link, process::exit outside main.rs, the file-size guard,
// the dep-feature-comment rule). Aftersight (npm) has no direct equivalent —
// this is an extra row, not a substitution.
const lintSh = path.join(ROOT, 'scripts', 'lint.sh');
if (exists(lintSh)) {
  const lintRun = sh('bash', ['scripts/lint.sh'], { timeout: 60_000 });
  addCheck(
    'Build guards still pass',
    KIND.gate,
    lintRun.ok ? RESULT.PASS : RESULT.FAIL,
    lintRun.ok ? 'scripts/lint.sh: ok' : firstLine(lintRun.out || lintRun.err) || 'scripts/lint.sh failed',
  );
} else {
  addCheck('Build guards still pass', KIND.gate, RESULT.FAIL, 'scripts/lint.sh missing');
}

// Checks
addCheck('Inside a git work tree', KIND.gate, inRepo.ok && inRepo.out === 'true' ? RESULT.PASS : RESULT.FAIL, inRepo.out || inRepo.err);

const headRef = sh('git', ['symbolic-ref', '-q', 'HEAD']);
addCheck('HEAD is a branch, not detached', KIND.gate, headRef.ok ? RESULT.PASS : RESULT.FAIL, headRef.ok ? headRef.out : 'detached HEAD');

const gitDir = sh('git', ['rev-parse', '--git-dir']);
const gd = gitDir.ok ? path.resolve(ROOT, gitDir.out) : path.join(ROOT, '.git');
const inProgress = ['MERGE_HEAD', 'REBASE_HEAD', 'CHERRY_PICK_HEAD', 'REVERT_HEAD']
  .filter((n) => exists(path.join(gd, n)));
addCheck(
  'Not in the middle of merge/rebase/cherry-pick',
  KIND.gate,
  inProgress.length === 0 ? RESULT.PASS : RESULT.FAIL,
  inProgress.length === 0 ? 'clean' : inProgress.join(', '),
);

const branch = sh('git', ['branch', '--show-current']);
const branchName = branch.out || '';
const headSha = sh('git', ['rev-parse', 'HEAD']);
let baseSha = sh('git', ['rev-parse', BASE]);
let fetchNote = '';
if (!baseSha.ok) {
  const fetch = sh('git', ['fetch', 'origin', 'main'], { timeout: 20_000 });
  fetchNote = fetch.ok ? 'fetched origin/main' : `fetch failed: ${fetch.err}`;
  baseSha = sh('git', ['rev-parse', BASE]);
  if (!baseSha.ok) baseSha = sh('git', ['rev-parse', 'main']);
}
const sameAsBase = baseSha.ok && headSha.ok && baseSha.out === headSha.out;
const onMain = /^(main|master)$/.test(branchName);

if (!sh('git', ['rev-parse', BASE]).ok && sh('git', ['rev-parse', 'main']).ok) {
  addCheck('origin/main is fetchable', KIND.advisory, RESULT.WARN, `${fetchNote || 'origin/main missing'}; using local main`);
} else {
  addCheck(
    'origin/main is fetchable',
    KIND.gate,
    sh('git', ['rev-parse', BASE]).ok ? RESULT.PASS : RESULT.FAIL,
    fetchNote || BASE,
  );
}

const diffStat = sh('git', ['diff', `${BASE}...HEAD`, '--stat']);
const diffNames = sh('git', ['diff', `${BASE}...HEAD`, '--name-only']);
const emptyDiff = !diffNames.ok || diffNames.out === '';
const wholeCodebase = emptyDiff && (onMain || sameAsBase);
addCheck(
  'Review surface',
  KIND.info,
  RESULT.PASS,
  wholeCodebase
    ? `on ${branchName || 'main'} with no three-dot diff vs ${BASE} — reviewing the entire codebase, not a branch range`
    : emptyDiff
      ? `empty three-dot diff vs ${BASE} on ${branchName || '(detached)'}`
      : `${firstLine(diffStat.out) || `${diffNames.out.split('\n').length} files`} vs ${BASE}`,
);
if (emptyDiff && !wholeCodebase) {
  addCheck(
    'Diff vs base is non-empty',
    KIND.gate,
    RESULT.FAIL,
    `empty three-dot diff vs ${BASE} on ${branchName || '(detached)'} — not main, so there is no review surface`,
  );
}

const status = sh('git', ['status', '--porcelain=v1']);
const dirty = status.ok && status.out !== '';
addCheck(
  'Working tree is clean',
  KIND.gate,
  dirty ? RESULT.FAIL : status.ok ? RESULT.PASS : RESULT.FAIL,
  dirty ? status.out.split('\n').slice(0, 8).join('; ') : 'clean',
);

const junkInDiff = (diffNames.out || '').split('\n').filter((f) => f && BUILD_JUNK.some((re) => re.test(f)));
addCheck(
  'No ignored-but-present build junk in the diff',
  KIND.gate,
  junkInDiff.length ? RESULT.FAIL : RESULT.PASS,
  junkInDiff.length ? junkInDiff.slice(0, 6).join(', ') : 'none',
);

const issueList = sh('gh', ['issue', 'list', '--limit', '1']);
addCheck(
  'gh can file issues',
  KIND.gate,
  issueList.ok ? RESULT.PASS : RESULT.FAIL,
  issueList.ok ? 'ok' : issueList.err,
);

const issueNum = currentIssueFromBranch(branchName);
const logIssues = sh('git', ['log', `${BASE}..HEAD`, '--format=%s']);
const msgHasIssue = /#\d+/.test(logIssues.out || '');
addCheck(
  'Spec source is findable',
  KIND.advisory,
  wholeCodebase || issueNum || msgHasIssue ? RESULT.PASS : RESULT.WARN,
  wholeCodebase
    ? 'whole-codebase review — ADR-0001–ADR-0006 bind the entire codebase unconditionally, no issue number needed'
    : issueNum
      ? `branch issue #${issueNum}`
      : msgHasIssue
        ? 'issue in commit messages'
        : 'no issue number on branch or commits',
);

const diffText = emptyDiff ? '' : sh('git', ['diff', `${BASE}...HEAD`, '-U0']).out || '';
let secretHit = false;
for (const re of SECRET_RES) {
  if (re.test(diffText)) {
    secretHit = true;
    break;
  }
}
addCheck(
  'Secrets',
  KIND.gate,
  secretHit ? RESULT.FAIL : RESULT.PASS,
  secretHit ? 'possible secret in committed diff (value not printed)' : 'none detected',
);

let huge = [];
if (!emptyDiff) {
  for (const f of diffNames.out.split('\n').filter(Boolean)) {
    const size = sh('git', ['cat-file', '-s', `HEAD:${f}`]);
    const n = Number(size.out);
    if (size.ok && n > 500 * 1024) huge.push(`${f} (${bytes(n)})`);
  }
}
addCheck(
  'Huge blobs',
  KIND.advisory,
  huge.length ? RESULT.WARN : RESULT.PASS,
  huge.length ? huge.join(', ') : 'none > 500KB',
);

// Detritus — untracked / unignored. Holler-flavored classification: no
// ctrf/coverage/playwright-report concepts (Aftersight-specific); target/ and
// holler-state/ are normally gitignored so they will not appear here unless
// something broke the ignore rules, in which case 'build' below catches them.
const porcelain = (status.out || '').split('\n').filter(Boolean);
function classifyPath(rel) {
  const base = path.basename(rel);
  if (/\.(tmp|temp|swp)$/.test(base) || base.endsWith('~') || base === '.DS_Store' || base === 'Thumbs.db') {
    return 'tmp';
  }
  if (rel.startsWith('.scratch/') || rel === 'tmp' || rel.startsWith('tmp/') || /untitled/i.test(base)) return 'scratch';
  if (/\.(prompt\.txt|usage\.json)$/.test(base) || /compaction/.test(rel)) return 'agent';
  if (/^(target|holler-state)\//.test(rel) || /\.(orig|rej)$/.test(base) || base === 'sessions.toml' || base === 'attach.toml') {
    return 'build';
  }
  if (HARNESS_DUMPS.some((d) => rel === d || rel.startsWith(`${d}/`))) return 'harness';
  return 'other';
}

const seen = new Set();
for (const line of porcelain) {
  const rel = line.slice(3);
  if (!rel || seen.has(rel)) continue;
  seen.add(rel);
  const kindKey = classifyPath(rel);
  const abs = path.join(ROOT, rel);
  const { total, count } = exists(abs) ? dirSize(abs) : { total: 0, count: 0 };
  const tracked = !line.startsWith('??');
  let action;
  let cmd;
  if (kindKey === 'harness') {
    action = 'delete';
    cmd = tracked ? `git rm -r ${rel}` : `rm -rf ${rel}`;
  } else if (line.startsWith('??')) {
    action = 'delete';
    cmd = `rm -rf ${rel}`;
  } else {
    action = 'delete';
    cmd = `git rm -r --ignore-unmatch ${rel}`;
  }
  const skipNoise = kindKey === 'other' && !line.startsWith('??');
  if (skipNoise) continue;
  if (kindKey === 'other' && line.startsWith('??') && !/\.(tmp|temp|swp|orig|rej|prompt\.txt|usage\.json)$/.test(path.basename(rel))) {
    if (!HARNESS_DUMPS.includes(rel.split('/')[0])) {
      detritus.push({
        path: rel,
        kind: KIND.other,
        size: `${bytes(total)}${count > 1 ? ` / ${count} files` : ''}`,
        result: ACTION.leave,
      });
      continue;
    }
  }
  detritus.push({
    path: rel,
    kind: KIND[kindKey] || KIND.other,
    size: `${bytes(total)}${count > 1 ? ` / ${count} files` : ''}`,
    result: ACTION[action] || ACTION.leave,
  });
  if (cmd && action === 'delete') cleanup.push({ path: rel, cmd, reason: kindKey });
}

const wt = sh('git', ['worktree', 'list', '--porcelain']);
if (wt.ok) {
  const blocks = wt.out.split('\n\n').filter(Boolean);
  const merged = new Set(
    (sh('git', ['branch', '--merged']).out || '')
      .split('\n')
      .map((l) => l.replace(/^\*?\+?\s*/, '').trim())
      .filter(Boolean),
  );
  for (const block of blocks) {
    const wp = (block.match(/^worktree (.+)$/m) || [])[1];
    const br = (block.match(/^branch refs\/heads\/(.+)$/m) || [])[1];
    if (!wp || path.resolve(wp) === path.resolve(ROOT)) continue;
    if (br && merged.has(br)) {
      detritus.push({
        path: wp,
        kind: KIND.worktree,
        size: `branch ${br} already merged`,
        result: ACTION.leave,
      });
      cleanup.push({
        path: wp,
        cmd: `git worktree remove ${wp}`,
        reason: 'orphan worktree',
      });
    }
  }
}

// Handoffs — Holler has not adopted the docs/handoffs/<issue>-<slug>/
// convention as of this writing; this block is a no-op (exists() guard)
// until/unless that changes, kept for parity with the canonical mechanism.
const handoffRoot = path.join(ROOT, 'docs', 'handoffs');
if (exists(handoffRoot)) {
  const dirs = fs.readdirSync(handoffRoot, { withFileTypes: true }).filter((d) => d.isDirectory());
  for (const d of dirs) {
    const m = d.name.match(/^0*(\d+)-/);
    const n = m ? Number(m[1]) : null;
    if (issueNum && n === issueNum) continue;
    const rel = `docs/handoffs/${d.name}`;
    const log = sh('git', ['log', '--follow', '--format=%H %cI', '--', rel]);
    const commits = (log.out || '')
      .split('\n')
      .filter(Boolean)
      .map((line) => {
        const sp = line.indexOf(' ');
        return { hash: line.slice(0, sp), iso: line.slice(sp + 1) };
      });
    const last = commits[0];
    let issueState = '—';
    let classif = 'unresolved';
    let actionCell = ACTION.noEvidence;
    let notes = '';

    if (!n) {
      classif = 'unresolved';
      notes = 'no issue number in dir name';
      addCheck(`Handoff ${rel} resolvable`, KIND.advisory, RESULT.WARN, notes);
    } else if (!ghOk) {
      classif = 'unresolved';
      notes = 'gh not usable';
      addCheck(`Handoff ${rel} resolvable`, KIND.advisory, RESULT.WARN, notes);
    } else {
      const view = sh('gh', ['issue', 'view', String(n), '--json', 'state,title,closedAt']);
      if (!view.ok) {
        classif = 'unresolved';
        notes = view.err || 'gh issue view failed';
        addCheck(`Handoff ${rel} resolvable`, KIND.advisory, RESULT.WARN, notes);
      } else {
        let payload;
        try {
          payload = JSON.parse(view.out);
        } catch {
          payload = {};
        }
        issueState = payload.state || '—';
        if (issueState === 'OPEN') {
          classif = 'live work';
          actionCell = ACTION.live;
        } else if (issueState === 'CLOSED') {
          const closeMs = payload.closedAt ? Date.parse(payload.closedAt) : Date.now();
          const after = commits.filter((c) => Date.parse(c.iso) > closeMs + 1000);
          if (after.length === 0) {
            classif = 'closed story, archived';
            actionCell = ACTION.archived;
          } else {
            classif = 'stale candidate';
            actionCell = ACTION.reviewDelete;
            notes = `${after.length} commit(s) after close; spot-check newest file before delete`;
            const tracked = sh('git', ['ls-files', '--', rel]);
            cleanup.push({
              path: rel,
              cmd: tracked.out
                ? `git rm -r ${rel}`
                : `rm -r ${rel} && git add -u -- ${rel}`,
              reason: 'stale handoff',
              spotCheck: true,
            });
          }
        }
      }
    }

    handoffs.push({
      path: rel,
      issue: n ? `#${n}` : '—',
      state: issueState,
      lastTouched: last ? fmtWhen(last.iso) : '—',
      classif,
      action: actionCell,
      notes,
    });
  }
}

const failCount = [...tools, ...checks].filter((r) => r.result === RESULT.FAIL).length;
const warnCount = [...tools, ...checks].filter((r) => r.result === RESULT.WARN).length;
let verdictText = 'PRE-FLIGHT: PASS';
let exit = 0;
if (failCount) {
  verdictText = 'PRE-FLIGHT: FAIL';
  exit = 1;
} else if (warnCount || cleanup.length) {
  verdictText = 'PRE-FLIGHT: WARN';
  exit = 2;
}

console.log('## Tools');
console.log();
console.log(table(
  ['Tool', 'Kind', 'Result', 'Notes'],
  tools.map((t) => [t.name, t.kind, t.result, t.notes]),
));
console.log();
console.log('## Checks');
console.log();
console.log(table(
  ['Check', 'Kind', 'Result', 'Notes'],
  checks.map((c) => [c.name, c.kind, c.result, c.notes]),
));
console.log();
console.log('## Detritus');
console.log();
if (detritus.length === 0) {
  console.log('none found');
} else {
  console.log(table(
    ['Path', 'Kind', 'Size', 'Result'],
    detritus.map((d) => [d.path, d.kind, d.size, d.result]),
  ));
}
console.log();
console.log('## Handoffs (not the story under review)');
console.log();
if (handoffs.length === 0) {
  console.log(issueNum ? `none besides current issue #${issueNum}` : 'no docs/handoffs dirs (or none besides current story)');
} else {
  console.log(table(
    ['Path', 'Issue', 'State', 'Last touched', 'Classification', 'Suggested action'],
    handoffs.map((h) => [h.path, h.issue, h.state, h.lastTouched, h.classif, h.action]),
  ));
}
console.log();
console.log('## Verdict');
console.log();
console.log(cell(
  failCount ? '#dc2626' : exit === 2 ? '#d97706' : '#16a34a',
  `${failCount ? '❌' : exit === 2 ? '⚠️' : '✅'} ${verdictText}`,
));
console.log();

if (wholeCodebase) {
  console.log('We are reviewing the **entire codebase**. HEAD is on main (or matches origin/main), so there is no branch range. Phase 2 reviews the tree, not `git diff origin/main...HEAD`.');
  console.log();
}

if (failCount) {
  console.log('Failing rows:');
  for (const r of [...tools, ...checks].filter((x) => x.result === RESULT.FAIL)) {
    console.log(`- ${r.name}: ${r.notes}`);
  }
  console.log();
  console.log('Do not start Phase 2.');
}

const staleOffer = cleanup.filter((c) => c.reason === 'stale handoff');
const otherOffer = cleanup.filter((c) => c.reason !== 'stale handoff' && c.cmd);

if (!failCount && (otherOffer.length || staleOffer.length)) {
  console.log('Cleanup offer (do not run until the operator says yes):');
  for (const c of [...otherOffer, ...staleOffer]) {
    const extra = c.spotCheck ? ' — spot-check one claim in the newest file against current code/tests first' : '';
    console.log(`- \`${c.cmd}\`${extra}`);
  }
  console.log();
  console.log('Historical/archived and OPEN-other handoffs are not in this list.');
} else if (!failCount && !cleanup.length) {
  console.log('All systems go. Proceed to the reviews?');
}

console.log();
console.log(`<!-- preflight-exit=${exit} fail=${failCount} warn=${warnCount} cleanup=${cleanup.length} whole-codebase=${wholeCodebase ? 1 : 0} -->`);
process.exit(exit);
