#!/usr/bin/env bash
# panes.sh - the setup wizard's pane config: check it, and print the Holler calls that build and
# tear down its panes (story #803, epic #633).
#
# Usage: panes.sh check|grid|env <config>
#        panes.sh plan <config> <profile> [--rerun]
#        panes.sh teardown <config> <profile>
#
#   check     exit 0 and one summary line when the config is valid. Otherwise one message per
#             problem on stderr, each naming the key, and exit 1. `hub_host` is accepted and
#             ignored, with one warning line on stderr.
#   grid      tab-separated lines: `workspace LABEL`, `extent RxC`, `profile P`, then one
#             `cell rRcC NAME HARNESS ROLE DIR PORT` per pane in layout order (DIR with `~/`
#             expanded, PORT `-` for a Claude Code pane, which has no port).
#   env       shell assignments that eval in bash: WIZARD_HERDR_WORKSPACES='LABEL=RxC' always,
#             then WIZARD_HERDR_SOCKET for the default instance or WIZARD_TMUX_SOCKET for any other.
#   plan      one line per Holler call, `TAG ARGS...`, the arguments shell-quoted and `holler`
#             left out (the skill runs each through `herdr.sh holler`). The tags, in order:
#             profile-create (left out with --rerun, when the profile exists), spec (one per pane,
#             in layout order: a `--spec-only` launch, which touches nothing live), dry-run, apply.
#   teardown  the same form: `pane close NAME --profile P` per pane in reverse layout order (tag
#             close), then `profile delete P` (tag profile-delete).
#
# Every verb checks the config first and prints nothing on stdout from a config with a problem
# (exit 1). Exit 2 is a usage error, a profile that is not this config's, or no instance.sh. It
# runs no Herdr, tmux or Holler: it reads one file and HOME / XDG_CONFIG_HOME, and prints text.
#
# The profile P the wizard builds is the workspace label (`workspace`, default `holler`); `grid`
# prints it, so the agent never chooses it, and `plan` / `teardown` refuse any other.
#
# What it reads (docs/setup-wizard.md, "The config file"): the bare top-level `layout` (columns of
# pane names, left to right, each top to bottom; no column taller than the column to its left,
# because Herdr adds a row only below a row of one pane), the top-level defaults `model`,
# `effort`, `ctx_soft` and `ctx_hard`, and every `[[pane]]` table (`name`, `dir`, `harness`,
# `role`, `agent`, `backend_port`, `model`, `effort`, `ctx_soft`, `ctx_hard`). A Claude Code pane
# never inherits the top-level `model` (it names an OpenCode model). `[[orchestrator]]`,
# `[[session]]` and their keys are retired and refused by name.
#
# The `[instance]` table and `workspace` are instance.sh's: this script runs it (in WIZARD_LIB,
# else its own directory) and reads only the lines it prints. Its value parser (trim, value_of)
# is a copy of instance.sh's and accepts exactly the same TOML subset. The port rule is
# instance.sh's too, applied to the OpenCode panes in layout order: a pane's own backend_port,
# else backend_port_base + its index (a pane with its own port keeps its index), at most 65535,
# no two panes on one port, and a non-default instance stays out of the first instance's range
# (the base instance.sh prints for a config with no table, up the pane count) unless every
# OpenCode pane sets its own port.
#
# bash 3.2 and POSIX awk (no associative arrays in bash, no GNU-only flags), so it runs on macOS
# and Linux.
set -u

usage() {
  echo "usage: panes.sh check|grid|env <config> | plan <config> <profile> [--rerun] | teardown <config> <profile>" >&2
  exit 2
}

verb="${1:-}"
profile=""
rerun=""
case "$verb" in
  check | grid | env) [ "$#" -eq 2 ] || usage ;;
  plan)
    if [ "$#" -eq 4 ] && [ "$4" = "--rerun" ]; then
      rerun=1
    elif [ "$#" -ne 3 ]; then
      usage
    fi
    profile="$3"
    ;;
  teardown)
    [ "$#" -eq 3 ] || usage
    profile="$3"
    ;;
  *) usage ;;
esac
config="$2"
case "$verb" in
  plan | teardown) [ -n "$profile" ] || usage ;;
esac
if [ ! -r "$config" ]; then
  echo "panes.sh: cannot read config: $config" >&2
  exit 2
fi
lib_dir="${WIZARD_LIB:-$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)}"
if [ ! -r "$lib_dir/instance.sh" ]; then
  echo "panes.sh: no instance.sh in $lib_dir (WIZARD_LIB names the skill's lib directory)" >&2
  exit 2
fi

# The first instance's backend port base: what instance.sh resolves for a config with no
# [instance] table, so the range refused below cannot drift from instance.sh's own.
defaults="$(bash "$lib_dir/instance.sh" /dev/null 2>/dev/null)"
# This config's instance, as instance.sh prints it; its messages go straight to stderr.
inst_ok=1
resolved="$(bash "$lib_dir/instance.sh" "$config")" || inst_ok=""

PANES_VERB="$verb" PANES_PROFILE="$profile" PANES_RERUN="$rerun" PANES_INSTANCE="$resolved" \
  PANES_INSTANCE_OK="$inst_ok" PANES_DEFAULTS="$defaults" LC_ALL=C awk '
function err(msg) { print "panes.sh: " msg > "/dev/stderr"; bad = 1 }
function trim(s) { sub(/^[ \t\r]+/, "", s); sub(/[ \t\r]+$/, "", s); return s }
# Returns "s:<text>" for a quoted string, "n:<text>" for anything else, comments removed.
function value_of(raw,   q, rest) {
    raw = trim(raw)
    if (substr(raw, 1, 1) == "\"") {
        rest = substr(raw, 2)
        q = index(rest, "\"")
        if (q == 0) return "x:" raw
        return "s:" substr(rest, 1, q - 1)
    }
    sub(/[ \t]*#.*$/, "", raw)
    return "n:" trim(raw)
}
function is_str(v) { return substr(v, 1, 2) == "s:" }
function num_in(v, lo, hi,   t) {
    t = substr(v, 3)
    return (substr(v, 1, 2) == "n:" && t ~ /^[0-9]+$/ && length(t) <= 10 && t + 0 >= lo && t + 0 <= hi)
}
function noslash(s) { while (length(s) > 1 && substr(s, length(s), 1) == "/") s = substr(s, 1, length(s) - 1); return s }
# A shell word: bare when it is safe as it is, else single-quoted.
function q(s) { return (s ~ SAFE ? s : qq(s)) }
function qq(s,   n, parts, i, out) {
    n = split(s, parts, SQ)
    out = (n > 0 ? parts[1] : "")
    for (i = 2; i <= n; i++) out = out SQ "\\" SQ SQ parts[i]
    return SQ out SQ
}
# The bracket depth of an array value, outside quoted strings.
function depth_of(s,   i, c, d, inq) {
    d = 0; inq = 0
    for (i = 1; i <= length(s); i++) {
        c = substr(s, i, 1)
        if (c == "\"") inq = !inq
        else if (!inq && c == "[") d++
        else if (!inq && c == "]") d--
    }
    return d
}
function strip_comment(s,   i, c, inq) {
    inq = 0
    for (i = 1; i <= length(s); i++) {
        c = substr(s, i, 1)
        if (c == "\"") inq = !inq
        else if (!inq && c == "#") return substr(s, 1, i - 1)
    }
    return s
}
# Reads `layout` into ncol, clen[c] and lname[c, r]; 0 when it is not a nested array of quoted names.
function parse_layout(s,   i, c, need, close_q) {
    need = "open"; ncol = 0
    for (i = 1; i <= length(s); i++) {
        c = substr(s, i, 1)
        if (c == " " || c == "\t" || c == "\r") continue
        if (need == "open") { if (c != "[") return 0; need = "col"; continue }
        if (need == "col" || need == "colsep") {
            if (c == "]") { need = "end"; continue }
            if (need == "colsep") { if (c != ",") return 0; need = "col"; continue }
            if (c != "[") return 0
            ncol++; clen[ncol] = 0; need = "name"; continue
        }
        if (need == "name" || need == "namesep") {
            if (c == "]") { need = "colsep"; continue }
            if (need == "namesep") { if (c != ",") return 0; need = "name"; continue }
            if (c != "\"") return 0
            close_q = index(substr(s, i + 1), "\"")
            if (close_q == 0) return 0
            clen[ncol]++; lname[ncol, clen[ncol]] = substr(s, i + 1, close_q - 1)
            i += close_q; need = "namesep"; continue
        }
        return 0
    }
    return (need == "end")
}
# The text after " key=" on an instance.sh line, up to " next_key=" (or the end).
function field(line, key, next_key,   a, rest, b) {
    a = index(line, " " key "=")
    if (a == 0) return ""
    rest = substr(line, a + length(key) + 2)
    if (next_key == "") return rest
    b = index(rest, " " next_key "=")
    return (b ? substr(rest, 1, b - 1) : rest)
}
function header(line) {
    if (line ~ /^\[\[pane\]\][ \t]*(#.*)?$/) {
        section = "pane"; np++; pline[np] = NR
    } else if (line ~ /^\[instance\]/) {
        section = "instance"
    } else if (line ~ /^\[\[orchestrator\]\]/) {
        section = "retired"; retired_tables++
        if (!told_orch++)
            err("[[orchestrator]] is retired: an orchestrator is now a [[pane]] with role = \"orchestrator\" that runs opencode or claude, and its cmd is not carried (the wizard no longer builds attach-mode or remote sessions; Stage 1 shows the conversion)")
    } else if (line ~ /^\[\[session\]\]/) {
        section = "retired"; retired_tables++
        if (!told_sess++)
            err("[[session]] is retired: the wizard no longer builds attach-mode or remote sessions, so each [[session]] is dropped (Stage 1 shows the conversion; attach mode and holler body still serve a remote session set up by hand)")
    } else if (line ~ /^\[\[?(session\.)?ext\./) {
        section = "skip"
    } else {
        section = "skip"
        err("unknown table " line " (the config holds bare top-level keys, an optional [instance] table and [[pane]] tables)")
    }
}
function retired_key(key, where) {
    if (!(key in rwhere)) { nrk++; rkey[nrk] = key; rwhere[key] = "" }
    rwhere[key] = rwhere[key] SUBSEP where
}
function top_key(key, raw) {
    if (key == "layout") {
        if (has_layout) { err("layout is given twice"); return }
        has_layout = 1; lraw = strip_comment(raw)
        if (depth_of(lraw) > 0) inlayout = 1
        return
    }
    if (key == "workspace") return
    if (key == "hub_host") { warn_hub = 1; return }
    if (index(RETIRED, " " key " ")) { retired_key(key, 0); return }
    if (index(DEFAULTS, " " key " ")) {
        if (key in tv) err(key " is given twice at the top level")
        else tv[key] = value_of(raw)
        return
    }
    err("unknown top-level key " key " (the top level takes layout, workspace, model, effort, ctx_soft and ctx_hard)")
}
function pane_key(key, raw) {
    if (index(RETIRED, " " key " ")) { retired_key(key, np); return }
    if (key == "layout" || key == "workspace" || key == "hub_host") {
        err("a [[pane]] table holds the key " key ": bare top-level keys must come before every table header")
        return
    }
    if (!index(PANE_KEYS, " " key " ")) { punknown[np] = punknown[np] " " key; return }
    if ((np, key) in pv) { err("[[pane]] number " np " gives " key " twice"); return }
    pv[np, key] = value_of(raw)
}
# How a message names pane i: its name when it has a quoted one, else its number.
function short(i,   v) {
    if ((i, "name") in pv) {
        v = pv[i, "name"]
        if (is_str(v) && substr(v, 3) != "") return substr(v, 3)
    }
    return "number " i
}
function who(i) { return "pane " short(i) }
function retired_why(key) {
    if (key == "cmd") return "a pane runs its harness (opencode or claude), never a free-form cmd"
    if (key == "remote_host" || key == "remote_tailnet_host") return "every pane runs on this machine"
    if (key == "mode") return "every pane is a registry pane"
    return "the port and session of record of a pane come from the registry"
}
function report_retired(   k, key, n, w, i, where, names) {
    for (k = 1; k <= nrk; k++) {
        key = rkey[k]; names = ""
        n = split(substr(rwhere[key], 2), w, SUBSEP)
        for (i = 1; i <= n; i++) {
            where = (w[i] + 0 == 0 ? "the top level" : who(w[i] + 0))
            if (index(", " names ", ", ", " where ", ") == 0) names = names (names == "" ? "" : ", ") where
        }
        err(key " (" names ") is retired: the wizard no longer builds attach-mode or remote sessions, and " retired_why(key) " (Stage 1 shows the conversion)")
    }
}
# The top-level defaults: tdef[key] the value, or "" when absent or wrong (wrong is reported).
function check_defaults(   k, key, v) {
    for (k = 1; k <= 4; k++) {
        key = KEYS[k]; tdef[key] = ""
        if (!(key in tv)) continue
        v = tv[key]
        if (key == "model" && !(is_str(v) && substr(v, 3) ~ MODEL)) { err("model at the top level must be a quoted provider/id, for example \"provider/model\""); tbad[key] = 1 }
        else if (key == "effort" && !(is_str(v) && substr(v, 3) != "")) { err("effort at the top level must be a quoted, non-empty string"); tbad[key] = 1 }
        else if ((key == "ctx_soft" || key == "ctx_hard") && !num_in(v, 1, 4294967295)) { err(key " at the top level must be a whole number from 1 to 4294967295"); tbad[key] = 1 }
        else tdef[key] = substr(v, 3)
    }
}
# One resolved value of pane i: its own, else the top-level default; "" (reported) when none.
function resolve(i, key, h,   v, w) {
    w = who(i)
    if ((i, key) in pv) {
        v = pv[i, key]
        if (key == "model" && !(is_str(v) && substr(v, 3) ~ MODEL)) { err("model of " w " must be a quoted provider/id, for example \"provider/model\""); return "" }
        if (key == "effort" && !(is_str(v) && substr(v, 3) != "")) { err("effort of " w " must be a quoted, non-empty string"); return "" }
        if ((key == "ctx_soft" || key == "ctx_hard") && !num_in(v, 1, 4294967295)) { err(key " of " w " must be a whole number from 1 to 4294967295"); return "" }
        return substr(v, 3)
    }
    if (key == "model" && h == "claude") {
        err("model: " w " runs claude and needs a model of its own (the top-level model is an OpenCode model, read from the OpenCode configuration)")
        return ""
    }
    if (tdef[key] != "") return tdef[key]
    if (!(key in tbad)) missing[key] = missing[key] (missing[key] == "" ? "" : ", ") short(i)
    return ""
}
function check_panes(   i, v, nm, w, d, h, r) {
    if (np == 0) err("no [[pane]] table: every pane of the grid is a [[pane]] with a name and a dir")
    for (i = 1; i <= np; i++) {
        w = who(i); nm = ""
        if (!((i, "name") in pv)) err("[[pane]] number " i " (line " pline[i] ") has no name")
        else if (!is_str(pv[i, "name"])) err("name of [[pane]] number " i " must be a quoted string")
        else {
            nm = substr(pv[i, "name"], 3); known[nm] = 1
            if (nm !~ NAME || length(nm) > 32)
                err("name \"" nm "\" is not a valid pane name: lower-case letters and digits, - between them, 1 to 32 characters")
            else if (nm in byname) err("name " nm " is given to two [[pane]] tables")
            else byname[nm] = i
        }
        pname[i] = nm
        if (i in punknown) err("unknown key(s)" punknown[i] " in " w " (a [[pane]] takes name, dir, harness, role, agent, backend_port, model, effort, ctx_soft and ctx_hard)")
        d = ""
        if (!((i, "dir") in pv)) err("dir is missing in " w)
        else if (!is_str(pv[i, "dir"])) err("dir of " w " must be a quoted string")
        else {
            d = substr(pv[i, "dir"], 3)
            if (d == "~" || substr(d, 1, 2) == "~/") {
                if (home !~ /^\//) err("dir of " w " starts with ~ but HOME is not an absolute path")
                else d = noslash(home) substr(d, 2)
            } else if (substr(d, 1, 1) != "/") err("dir of " w " must be an absolute path or start with ~/: " d)
        }
        pdir[i] = d
        h = "opencode"
        if ((i, "harness") in pv) {
            v = pv[i, "harness"]
            if (is_str(v) && (substr(v, 3) == "opencode" || substr(v, 3) == "claude")) h = substr(v, 3)
            else err("harness of " w " must be \"opencode\" or \"claude\"")
        }
        phar[i] = h
        r = "agent"
        if ((i, "role") in pv) {
            v = pv[i, "role"]
            if (is_str(v) && (substr(v, 3) == "agent" || substr(v, 3) == "orchestrator")) r = substr(v, 3)
            else err("role of " w " must be \"agent\" or \"orchestrator\"")
        }
        prole[i] = r
        if (r == "orchestrator") norch++
        pagent[i] = ""
        if ((i, "agent") in pv) {
            v = pv[i, "agent"]
            if (h == "claude") err("agent: " w " runs claude, which takes no agent (a Claude Code launch refuses --agent as agent-unsupported)")
            else if (!(is_str(v) && substr(v, 3) ~ /^[A-Za-z0-9_-]+$/)) err("agent of " w " must be a quoted OpenCode agent key: letters, digits, - and _")
            else pagent[i] = substr(v, 3)
        }
        pown[i] = ""
        if ((i, "backend_port") in pv) {
            if (h == "claude") err("backend_port: " w " runs claude, which has no port (its port policy is none)")
            else if (!num_in(pv[i, "backend_port"], 1, 65535)) err("backend_port of " w " must be an integer from 1 to 65535")
            else pown[i] = substr(pv[i, "backend_port"], 3) + 0
        }
        pmodel[i] = resolve(i, "model", h)
        peff[i] = resolve(i, "effort", h)
        psoft[i] = resolve(i, "ctx_soft", h)
        phard[i] = resolve(i, "ctx_hard", h)
        if (psoft[i] != "" && phard[i] != "" && psoft[i] + 0 > phard[i] + 0)
            err("ctx_soft of " w " (" psoft[i] ") is above its ctx_hard (" phard[i] ")")
    }
    for (i = 1; i <= 4; i++)
        if (missing[KEYS[i]] != "")
            err("no " KEYS[i] " for pane(s) " missing[KEYS[i]] ": set " KEYS[i] " at the top level (the default for every pane) or on the pane")
}
# The layout against the panes: every name placed once, defined, and a staircase. 1 when sound.
function check_layout(   c, r, nm, i, ok) {
    if (!has_layout) { err("layout is missing: a bare top-level layout = [[...], ...] names every pane, columns left to right, each top to bottom"); return 0 }
    if (inlayout || !parse_layout(lraw)) { err("layout must be a nested array of quoted pane names, columns left to right and each top to bottom, for example [[\"r1c1\", \"r2c1\"], [\"r1c2\"]]"); return 0 }
    if (ncol == 0) { err("layout has no column"); return 0 }
    ok = 1
    for (c = 1; c <= ncol; c++) {
        if (clen[c] == 0) { err("layout: column " c " is empty"); ok = 0 }
        if (c > 1 && clen[c] > clen[c - 1]) {
            err("layout: column " c " has " clen[c] " pane(s), more than the " clen[c - 1] " of column " (c - 1) ": no column may be taller than the column to its left, because Herdr adds a row only below a row of one pane (put the taller column first)")
            ok = 0
        }
        for (r = 1; r <= clen[c]; r++) {
            nm = lname[c, r]
            if (nm in placed) { err("layout names " nm " twice"); ok = 0; continue }
            placed[nm] = 1; norder++; order[norder] = nm; pos[nm] = "r" r "c" c
            if (!(nm in known)) { err("layout names " nm ", which no [[pane]] defines"); ok = 0 }
        }
    }
    for (i = 1; i <= np; i++)
        if (pname[i] != "" && (pname[i] in byname) && byname[pname[i]] == i && !(pname[i] in placed)) {
            err("layout does not place pane " pname[i] ": every [[pane]] name appears in layout exactly once")
            ok = 0
        }
    return ok
}
# The OpenCode ports: the rule instance.sh applies to sessions, over the panes in layout order.
function check_ports(   k, i, n, all_own, j, p) {
    n = 0; all_own = 1
    for (k = 1; k <= norder; k++) {
        i = byname[order[k]]
        if (phar[i] == "claude") { pport[i] = "-"; continue }
        if (pown[i] != "") pport[i] = pown[i]
        else {
            all_own = 0; pport[i] = base + n
            if (pport[i] > 65535) err("pane " pname[i] ": backend port " pport[i] " is above 65535 (backend_port_base + index)")
        }
        n++; open[n] = i
    }
    nopen = n
    if (nondefault && !all_own)
        for (k = 1; k <= n; k++) {
            p = pport[open[k]]
            if (p >= dbase && p < dbase + n)
                err("[instance] backend_port_base must keep pane " pname[open[k]] " off port " p " (the default range " dbase " to " (dbase + n - 1) ") unless every OpenCode pane sets its own backend_port")
        }
    for (k = 1; k <= n; k++) for (j = k + 1; j <= n; j++)
        if (pport[open[k]] == pport[open[j]])
            err("backend_port: panes " pname[open[k]] " and " pname[open[j]] " both resolve to port " pport[open[k]])
}
function read_instance(   n, l, i, line) {
    n = split(ENVIRON["PANES_INSTANCE"], l, "\n")
    for (i = 1; i <= n; i++) {
        line = l[i]
        if (substr(line, 1, 10) == "instance: ") {
            iname = field(line, "name", "prefix")
            istate = field(line, "state_dir", "herdr_session")
            iherdr = field(line, "herdr_session", "backend_port_base")
            base = field(line, "backend_port_base", "") + 0
            seen_inst = 1
        } else if (substr(line, 1, 11) == "workspace: ") label = substr(line, 12)
    }
    n = split(ENVIRON["PANES_DEFAULTS"], l, "\n")
    for (i = 1; i <= n; i++)
        if (substr(l[i], 1, 10) == "instance: ") dbase = field(l[i], "backend_port_base", "") + 0
    if (!seen_inst) { err("instance.sh printed no instance line"); return }
    if (dbase == 0) { err("cannot read the default backend_port_base from instance.sh"); return }
    nondefault = (iname != "default")
}
function config_home(   x) {
    x = noslash(ENVIRON["XDG_CONFIG_HOME"])
    if (x ~ /^\//) return x
    if (home ~ /^\//) return noslash(home) "/.config"
    return ""
}
function spec(i, nm,   s) {
    s = "pane launch " q(nm) " --profile " q(P) " --spec-only --workspace " q(label) " --grid " pos[nm] " --project " q(pdir[i])
    if (phar[i] == "claude") s = s " --harness claude"
    s = s " --role " prole[i] " --model " q(pmodel[i]) " --effort " q(peff[i]) " --ctx-soft " psoft[i] " --ctx-hard " phard[i]
    s = s " --port-policy " (phar[i] == "claude" ? "none" : "fixed:" pport[i])
    if (pagent[i] != "") s = s " --agent " q(pagent[i])
    return s
}
BEGIN {
    SQ = "\047"; SAFE = "^[A-Za-z0-9_./:=@%+,-]+$"
    NAME = "^[a-z0-9]([a-z0-9-]*[a-z0-9])?$"; MODEL = "^[^/ \t]+/[^ \t]+$"
    RETIRED = " endpoint mode session_id remote_host remote_tailnet_host cmd "
    PANE_KEYS = " name dir harness role agent backend_port model effort ctx_soft ctx_hard "
    DEFAULTS = " model effort ctx_soft ctx_hard "
    split("model effort ctx_soft ctx_hard", KEYS, " ")
    verb = ENVIRON["PANES_VERB"]; want = ENVIRON["PANES_PROFILE"]; rerun = (ENVIRON["PANES_RERUN"] != "")
    home = ENVIRON["HOME"]; inst_ok = (ENVIRON["PANES_INSTANCE_OK"] != "")
    section = ""; np = 0; label = "holler"
}
{
    line = trim($0)
    if (inlayout) {
        lraw = lraw " " strip_comment(line)
        if (depth_of(lraw) <= 0) inlayout = 0
        next
    }
    if (line == "" || substr(line, 1, 1) == "#") next
    if (substr(line, 1, 1) == "[") { header(line); next }
    if (section == "instance" || section == "retired" || section == "skip") next
    eq = index(line, "=")
    if (eq == 0) { err("line " NR " is neither key = value nor a table header: " line); next }
    key = trim(substr(line, 1, eq - 1))
    if (section == "pane") pane_key(key, substr(line, eq + 1))
    else top_key(key, substr(line, eq + 1))
}
END {
    report_retired()
    if (warn_hub) print "panes.sh: warning: hub_host is no longer used (the hub listens on loopback and nothing remote dials it); it is ignored" > "/dev/stderr"
    # A config of the old shape gets only the retirement messages: the rest would be about it.
    if (retired_tables) exit 1
    if (!inst_ok) bad = 1
    else read_instance()
    check_defaults()
    check_panes()
    layout_ok = check_layout()
    if (np > 0 && norch == 0) err("role: no [[pane]] has role = \"orchestrator\"; at least one pane must be an orchestrator")
    if (layout_ok && inst_ok && seen_inst) check_ports()
    if (bad) exit 1
    P = label
    if ((verb == "plan" || verb == "teardown") && want != P) {
        print "panes.sh: the profile " qq(want) " is not the profile of this config, " qq(P) " (its workspace label, which grid prints)" > "/dev/stderr"
        exit 2
    }
    S = (nondefault ? iherdr : "default")
    if (verb == "check") {
        print "ok: " norder " panes in a " clen[1] "x" ncol " grid (" nopen " OpenCode, " (norder - nopen) " Claude Code), " norch " orchestrator(s); workspace " label ", profile " P
    } else if (verb == "grid") {
        print "workspace\t" label; print "extent\t" clen[1] "x" ncol; print "profile\t" P
        for (k = 1; k <= norder; k++) {
            i = byname[order[k]]
            print "cell\t" pos[order[k]] "\t" order[k] "\t" phar[i] "\t" prole[i] "\t" pdir[i] "\t" pport[i]
        }
    } else if (verb == "env") {
        if (nondefault) {
            sd = istate
            if (sd == "~" || substr(sd, 1, 2) == "~/") {
                if (home !~ /^\//) { print "panes.sh: state_dir starts with ~ but HOME is not an absolute path" > "/dev/stderr"; exit 1 }
                sd = noslash(home) substr(sd, 2)
            }
            sock_name = "WIZARD_TMUX_SOCKET"; sock = noslash(sd) "/tmux.sock"
        } else {
            ch = config_home()
            if (ch == "") { print "panes.sh: neither XDG_CONFIG_HOME nor HOME is an absolute directory, so the default Herdr socket has no path" > "/dev/stderr"; exit 1 }
            sock_name = "WIZARD_HERDR_SOCKET"; sock = ch "/herdr/herdr.sock"
        }
        print "WIZARD_HERDR_WORKSPACES=" qq(label "=" clen[1] "x" ncol)
        print sock_name "=" qq(sock)
    } else if (verb == "plan") {
        if (!rerun) print "profile-create profile create " q(P)
        for (k = 1; k <= norder; k++) print "spec " spec(byname[order[k]], order[k])
        print "dry-run profile apply " q(P) " --dry-run --herdr-session " q(S) " --format=json"
        print "apply profile apply " q(P) " --herdr-session " q(S)
    } else if (verb == "teardown") {
        for (k = norder; k >= 1; k--) print "close pane close " q(order[k]) " --profile " q(P)
        print "profile-delete profile delete " q(P)
    }
}
' "$config"
