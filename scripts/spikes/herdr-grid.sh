#!/usr/bin/env bash
# Herdr spike (issue #636): how Herdr expresses a pane's position, its axis order and base,
# the layout model (absolute cells or splits only) and the nesting rule. Builds a 2-row by
# 4-column layout from splits in a scratch session, places a marker pane at a known cell
# (row 2, column 3) and reads its position back. Usage: bash scripts/spikes/herdr-grid.sh
set -euo pipefail
# shellcheck source=scripts/spikes/herdr-lib.sh
. "$(dirname "$0")/herdr-lib.sh"

spike_start

split() { # split TARGET DIRECTION RATIO -> prints the new pane id
  h pane split "$1" --direction "$2" --ratio "$3" --no-focus | jq -r .result.pane.pane_id
}
label() { h pane rename "$1" "$2" >/dev/null; }

# Derive (row, col) two ways: by ranking the distinct rect x and y values of the tab, and by
# walking the exported split tree (a "down" chain of rows, each row a "right" chain of columns).
derive() { # derive TAB_ID
  local snap tree
  snap="$(h api snapshot)"
  tree="$(hsock "{\"id\":\"e\",\"method\":\"layout.export\",\"params\":{\"tab_id\":\"$1\"}}")"
  python3 -I -c '
import json, sys
snap = json.loads(sys.argv[1])["result"]["snapshot"]
tree = json.loads(sys.argv[2])["result"]["layout"]["root"]
tab = sys.argv[3]
labels = {p["pane_id"]: p.get("label") for p in snap["panes"]}
layout = next(l for l in snap["layouts"] if l["tab_id"] == tab)
xs = sorted({p["rect"]["x"] for p in layout["panes"]})
ys = sorted({p["rect"]["y"] for p in layout["panes"]})
def chain(node, direction):
    if node["type"] == "split" and node["direction"] == direction:
        return chain(node["first"], direction) + chain(node["second"], direction)
    return [node]
by_tree = {}
for r, row in enumerate(chain(tree, "down"), 1):
    for c, cell in enumerate(chain(row, "right"), 1):
        by_tree[cell.get("pane_id")] = (r, c) if cell["type"] == "pane" else None
print("%-7s %-12s %-24s %-14s %s" % ("pane", "label", "herdr rect {x,y,w,h}", "rank (row,col)", "tree (row,col)"))
for p in sorted(layout["panes"], key=lambda p: (p["rect"]["y"], p["rect"]["x"])):
    rc = p["rect"]
    rank = (ys.index(rc["y"]) + 1, xs.index(rc["x"]) + 1)
    t = by_tree.get(p["pane_id"])
    print("%-7s %-12s %-24s %-14s %s" % (p["pane_id"], labels[p["pane_id"]], "{%d,%d,%d,%d}" % (rc["x"], rc["y"], rc["width"], rc["height"]),
          "r%dc%d" % rank, "r%dc%d" % t if t else "not a rows-of-columns tree"))
print("area", layout["area"], " distinct x:", xs, " distinct y:", ys)
' "$snap" "$tree" "$1"
}

say "build 2 rows x 4 columns from splits only (rows first, then columns inside each row)"
ws="$(h workspace create --label grid --cwd "$SPIKE_WORK" --no-focus)"
T="$(jq -r .result.tab.tab_id <<<"$ws")"
R1C1="$(jq -r .result.root_pane.pane_id <<<"$ws")"
R2C1="$(split "$R1C1" down 0.5)"
R1C2="$(split "$R1C1" right 0.25)"; R1C3="$(split "$R1C2" right 0.3333)"; R1C4="$(split "$R1C3" right 0.5)"
R2C2="$(split "$R2C1" right 0.25)"; R2C3="$(split "$R2C2" right 0.3333)"; R2C4="$(split "$R2C3" right 0.5)"
label "$R1C1" r1c1; label "$R1C2" r1c2; label "$R1C3" r1c3; label "$R1C4" r1c4
label "$R2C1" r2c1; label "$R2C2" r2c2; label "$R2C3" r2c3; label "$R2C4" r2c4
label "$R2C3" marker
h pane run "$R2C3" 'echo marker-at-row-2-col-3'
echo "calls: split r1c1 down 0.5 -> r2c1; split r1c1 right 0.25; split r1c2 right 0.3333; split r1c3 right 0.5; the same for row 2"
echo "marker pane: $R2C3 (placed at row 2, column 3)"

say "read back: pane.layout for the marker (CLI: pane layout --pane ID)"
show "herdr pane layout --pane $R2C3"
h pane layout --pane "$R2C3" | jq -c --arg p "$R2C3" '.result.layout.panes[] | select(.pane_id == $p)'
show "herdr pane edges --pane $R2C3   (which outer edges the pane touches)"
h pane edges --pane "$R2C3" | jq -c '.result.edges | {pane_id, left, right, up, down}'
derive "$T"

say "the exported split tree (layout.export): ratio is the FIRST child's share; split ids encode a tree path"
hsock "{\"id\":\"e\",\"method\":\"layout.export\",\"params\":{\"tab_id\":\"$T\"}}" | jq -c '
  def walk_: if .type == "pane" then (.label // .pane_id) else {(.direction + " " + (.ratio|tostring)): [(.first|walk_), (.second|walk_)]} end;
  .result.layout.root | walk_'
h pane layout --pane "$R1C1" | jq -c '[.result.layout.splits[] | {id, direction, ratio}]'

say "splits only: there is no 'place at cell' call; only right and down splits exist"
set +e
h pane split "$R1C1" --direction left --no-focus >/dev/null 2>&1; echo "CLI --direction left: exit $?"
hsock "{\"id\":\"l\",\"method\":\"pane.split\",\"params\":{\"target_pane_id\":\"$R1C1\",\"direction\":\"left\"}}" | jq -c .
set -e

say "nesting rule: a split divides only the target pane's own cell"
before="$(h pane layout --pane "$R1C1" | jq -c '[.result.layout.panes[] | {p: .pane_id, r: .rect}]')"
N="$(split "$R1C2" down 0.5)"; label "$N" nested
after="$(h pane layout --pane "$R1C1" | jq -c '[.result.layout.panes[] | {p: .pane_id, r: .rect}]')"
echo "split r1c2 down -> $N; panes whose rect changed:"
jq -nc --argjson b "$before" --argjson a "$after" \
  '[$a[] as $x | ($b[] | select(.p == $x.p) | .r) as $old | select($old != $x.r) | {pane: $x.p, before: $old, after: $x.r}] + [$a[] | select(.p as $p | [$b[].p] | index($p) | not) | {pane: .p, new: .r}]'
derive "$T"
h pane close "$N" >/dev/null

say "unaligned rows: equal-ratio rows align, but each row is split independently"
ws2="$(h workspace create --label unaligned --cwd "$SPIKE_WORK" --no-focus)"
T2="$(jq -r .result.tab.tab_id <<<"$ws2")"
U1="$(jq -r .result.root_pane.pane_id <<<"$ws2")"; label "$U1" u-r1c1
U2="$(split "$U1" down 0.5)"; label "$U2" u-r2c1
label "$(split "$U1" right 0.5)" u-r1c2
label "$(split "$U2" right 0.3)" u-r2c2
derive "$T2"

say "position belongs to the layout slot, not to the pane id: swap the marker with r1c1"
hsock "{\"id\":\"s\",\"method\":\"pane.swap\",\"params\":{\"source_pane_id\":\"$R2C3\",\"target_pane_id\":\"$R1C1\"}}" | jq -c '.result.swap | {changed, source_pane_id, target_pane_id}'
derive "$T"
