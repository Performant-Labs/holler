#!/usr/bin/env bash
# Tests for the Aftersight claude-pipeline adoption (issue #674): the committed pin, the
# project config and the pipeline section of CLAUDE.md. Pure file checks; no model run and no
# network. The resolved release (.aftersight/releases/claude) is a gitignored link that CI does
# not have, so the checks that read it are skipped when it is absent.
set -uo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
cd "$root"

fail=0
check() { # check <description> <command...>
  local desc=$1; shift
  if "$@" >/dev/null 2>&1; then echo "ok   - $desc"; else echo "FAIL - $desc"; fail=1; fi
}
py() { python3 -I -c "$@"; }

pin=.aftersight/pipeline-pin.json
cfg=.aftersight/pipeline.config.json

check "the pin parses and names a claude-pipeline tag, its version and a 40-hex commit" \
  py "
import json, re, sys
c = json.load(open('$pin'))['claude']
ok = (re.fullmatch(r'claude-pipeline-v(\d+\.\d+\.\d+)', c['tag'])
      and c['tag'] == 'claude-pipeline-v' + c['version']
      and re.fullmatch(r'[0-9a-f]{40}', c['commit']))
sys.exit(0 if ok else 1)"
# A `releases/` rule does not match a symlink (a worktree links the release in), so the rule is read.
check ".aftersight/.gitignore ignores releases/" grep -qx 'releases/' .aftersight/.gitignore
check "the pin is tracked" git ls-files --error-unmatch "$pin"

check "pipeline.config.json has test.unit.command as a cargo test argv of non-empty strings" \
  py "
import json, sys
a = json.load(open('$cfg'))['test']['unit']['command']
ok = (isinstance(a, list) and a[:2] == ['cargo', 'test']
      and all(isinstance(x, str) and x for x in a))
sys.exit(0 if ok else 1)"

# CLAUDE.md drives the pipeline from the release, not from the Playbook checkout.
for f in coding-pipeline.workflow.mjs review-models.sh sync-role-docs.mjs \
         workflow-coding-pipeline.md pipeline-conventions.md; do
  check "CLAUDE.md names no \$WORKFLOW_ROOT path for $f" \
    bash -c "! grep -E 'WORKFLOW_ROOT[^ ]*/$f' CLAUDE.md"
  check "CLAUDE.md names the release's $f" \
    grep -q ".aftersight/releases/claude/workflow/$f" CLAUDE.md
done
check "CLAUDE.md passes argsVersion 2 and the stages table to the Workflow" \
  bash -c "grep -q 'argsVersion: 2' CLAUDE.md && grep -q 'stages:' CLAUDE.md"

if [ -d .aftersight/releases/claude ]; then
  missing=$(grep -oE '\.aftersight/releases/claude/[A-Za-z0-9_./-]+' CLAUDE.md \
    | sed -E 's/[.,;:]+$//' | sort -u | while read -r p; do [ -e "$p" ] || echo "$p"; done)
  if [ -z "$missing" ]; then echo "ok   - every release path CLAUDE.md names exists in the resolved release"
  else echo "FAIL - release paths in CLAUDE.md that do not exist: $missing"; fail=1; fi
else
  echo "skip - no resolved release at .aftersight/releases/claude (CI), path-existence check skipped"
fi

exit $fail
