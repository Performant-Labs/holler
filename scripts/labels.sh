#!/usr/bin/env bash
#
# Create the holler test-catalog label set once, by script, so catalog issues
# can be labeled consistently from day one (issue #164; the hlr-NNNN catalog
# contract depends on these labels existing).
#
# Idempotent: every label is created with `gh label create <name> --force`,
# which sets (or re-sets) the name/color/description rather than erroring on a
# label that already exists. Run it any number of times and the repo converges
# to the same set.
#
# What it touches, and what it deliberately does NOT:
#   - Creates/normalizes exactly the labels in the table below. Where a label
#     already exists (e.g. `adr`, `epic`, `rebuild`, `test-case` from earlier
#     stories), `--force` re-asserts this file's color/description, so the
#     catalog stays uniform even if an older story used a different color.
#   - Deletes NOTHING. Labels that exist but are not in the table (the GitHub
#     defaults `good first issue` / `help wanted` / `invalid`, and the retired
#     `rebuild now`) are left exactly as-is — the spec is "delete nothing that
#     exists".
#   - Never creates the `test-hlrsvr` / `test-hlrclnt` prefixes (those are the
#     old holler-server / holler-client repos' retired label prefixes).
#
# The canonical documentation of what each label means lives in docs/testing.md
# — written by a LATER story in this chain (the #357-equivalent); this story
# only creates the labels and notes the script's existence.
#
# Usage:
#   ./scripts/labels.sh            # create/normalize the full set on this repo
#
# Requirements: `gh` on PATH with a token that has `read:org`/`repo` scope
# (e.g. `gh auth login` or GH_TOKEN set). The repo is this one (the script is
# run from a checkout of holler, so it defaults to this repo).
#
# Color legend (the colors are in the table below; the per-label meanings are
# documented in docs/testing.md, written by a later story in this chain):
#   ADR 0E8A16, epic/rebuild 5319E7, test-case family 1D76DB, groups C5DEF5,
#   categories FBCA04, tags BFD4F2, and the six GitHub defaults unchanged.

set -euo pipefail

# Fail with a clear message rather than a bare "command not found" if gh is
# missing or unauthenticated.
if ! command -v gh >/dev/null 2>&1; then
    echo "error: 'gh' is not on PATH. Install it (https://cli.github.com/) and 'gh auth login' first." >&2
    exit 1
fi

# "name<TAB>color<TAB>description" — tab-delimited so descriptions may contain
# spaces. Color is a bare 6-hex value (e.g. 0E8A16), exactly as written in the
# issue's table: `gh label create --color` wants "a 6 character hex value" and
# will SILENTLY store a null color if you pass "#0E8A16" instead (gh 2.x),
# which is why the # must NOT be added here. For the six GitHub-default labels
# the stock color/description is passed explicitly so the file is
# self-documenting.
LABELS='
adr	0E8A16	Architecture decision record — governs the rest of the project
epic	5319E7	Tracking issue for a multi-story epic
rebuild	5319E7	Story in the single-binary rebuild epic
test-case	1D76DB	A reusable test-case definition
test-auto	1D76DB	Automated test (runs on CI, no human)
test-manual	1D76DB	Requires a human to run
test-hub	1D76DB	Applies to the hub role
test-body	1D76DB	Applies to the body role
test-run	1D76DB	Records one execution of the test-run design
test-grp-invoc	C5DEF5	Group: invocation (the x001 range)
test-grp-lifecycle	C5DEF5	Group: process lifecycle (the x010 range)
test-grp-logging	C5DEF5	Group: logging (the x020 range)
test-grp-io	C5DEF5	Group: Unix I/O & stdio (the x030 range)
test-grp-platform	C5DEF5	Group: platform (the x040 range)
test-grp-concurrency	C5DEF5	Group: concurrency (the x050 range)
test-grp-network	C5DEF5	Group: network (the x060 range)
test-grp-diagnostics	C5DEF5	Group: diagnostics (the x070 range)
test-grp-crypto	C5DEF5	Group: security & encryption (the x080 range)
test-grp-load	C5DEF5	Group: load (the x090 range)
test-cat-smoke	FBCA04	Category: smoke
test-cat-regression	FBCA04	Category: regression
test-cat-acceptance	FBCA04	Category: acceptance
test-cat-unit	FBCA04	Category: unit
test-tag-alters-db	BFD4F2	Tag: alters a database
test-tag-remote	BFD4F2	Tag: requires a remote endpoint
test-tag-needs-tunnel	BFD4F2	Tag: needs a tunnel/forward
test-tag-slow	BFD4F2	Tag: slow (excluded from the fast path)
test-tag-interop	BFD4F2	Tag: cross-implementation interop
bug	d73a4a	Something is not working
documentation	0075ca	Improvements or additions to documentation
enhancement	a2eeef	New feature or request
question	d876e3	Further information is requested
wontfix	ffffff	This will not be worked on
duplicate	cfd3d7	This issue or pull request already exists
'

# Parse the table one LINE at a time. A plain `for line in $LABELS` would
# word-split each tab-delimited row on spaces, so a description like
# "Architecture decision record — governs the rest of the project" would be
# torn into separate words and the color/description fields would be wrong.
# `IFS= read -r` keeps the whole line (tabs and spaces intact) so the tab
# splits below are the only ones that happen.
created=0
while IFS= read -r line; do
    [ -z "$line" ] && continue
    name="${line%%	*}"
    rest="${line#*	}"
    color="${rest%%	*}"
    desc="${rest#*	}"
    # --force is what makes the script idempotent AND re-runnable: without it,
    # the very first label (e.g. `adr`) that already exists makes `gh label
    # create` exit non-zero ("Label.name already exists"), and `set -e` would
    # abort the whole loop before the rest are created. With `--force`, gh
    # creates a missing label and updates (color/description) one that exists,
    # so every run converges the repo to this file's set.
    gh label create "$name" --force --color "$color" --description "$desc" >/dev/null
    echo "  $name"
    created=$((created + 1))
done <<< "$LABELS"

echo "Done: created/normalized $created labels."
echo "Note: this script deletes nothing. Labels not listed above (the GitHub"
echo "defaults such as 'good first issue'/'help wanted'/'invalid', and the"
echo "retired 'rebuild now') are left untouched."
