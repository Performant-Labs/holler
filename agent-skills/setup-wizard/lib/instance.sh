#!/usr/bin/env bash
# instance.sh - validate and resolve the `[instance]` table of a setup-wizard sessions.toml.
#
# Usage: instance.sh <sessions.toml>
#
# Stage 1 of the setup wizard runs this. On success it prints the resolved instance (every key,
# defaults filled in) and each session's resolved backend port and endpoint, exit 0. On any problem it prints
# one message per problem to stderr, each naming the key, and exits 1. Exit 2 is a usage error.
# A config without an `[instance]` table resolves to all defaults (today's behaviour).
#
# The keys, defaults and rules are the contract of holler#726. bash 3.2 and POSIX awk only (no
# associative arrays, no GNU-only flags), so it runs on macOS and Linux.
set -u

if [ "$#" -ne 1 ]; then
    echo "usage: instance.sh <sessions.toml>" >&2
    exit 2
fi
if [ ! -r "$1" ]; then
    echo "instance.sh: cannot read config: $1" >&2
    exit 2
fi

awk -v home="${HOME:-}" '
function err(msg) { print "instance.sh: " msg > "/dev/stderr"; bad = 1 }
function trim(s) { sub(/^[ \t\r]+/, "", s); sub(/[ \t\r]+$/, "", s); return s }
function is_name(s) { return (s ~ /^[a-z][a-z0-9-]*$/ && length(s) <= 24) }
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
# Drops trailing slashes (keeps a lone "/").
function strip_slash(s) { while (length(s) > 1 && substr(s, length(s), 1) == "/") s = substr(s, 1, length(s) - 1); return s }
function is_default_state(s,   t) {
    t = strip_slash(s)
    if (t == "~/.holler") return 1
    return (home != "" && t == strip_slash(home) "/.holler")
}
function need_str(key, v) {
    if (substr(v, 1, 2) != "s:") { err("[instance] " key " must be a quoted string"); return 0 }
    return 1
}
function need_int(key, v, lo, hi,   t) {
    t = substr(v, 3)
    if (substr(v, 1, 2) != "n:" || t !~ /^[0-9]+$/ || length(t) > 9 || t + 0 < lo || t + 0 > hi) {
        err("[instance] " key " must be an integer from " lo " to " hi)
        return 0
    }
    return 1
}
BEGIN {
    D_hub_port = 41807; D_serve = 443; D_base = 47001
    section = ""; nsess = 0; has_instance = 0; seen_table = 0
}
{
    line = trim($0)
    if (line == "" || substr(line, 1, 1) == "#") next
    if (substr(line, 1, 1) == "[") {
        seen_table = 1
        if (line ~ /^\[instance\]/) {
            if (has_instance) err("[instance] appears more than once")
            has_instance = 1; section = "instance"
        } else if (line ~ /^\[\[session\]\]/) {
            section = "session"; nsess++; sname[nsess] = ""; sport[nsess] = ""; shost[nsess] = ""; send[nsess] = ""
        } else {
            section = "other"
            if (line ~ /^\[\[orchestrator\]\]/) seen_orch = 1
        }
        if (section == "instance" && (seen_orch || nsess > 0))
            err("[instance] must come before every [[orchestrator]] and [[session]] table")
        next
    }
    eq = index(line, "=")
    if (eq == 0) next
    key = trim(substr(line, 1, eq - 1))
    val = value_of(substr(line, eq + 1))
    if (section == "instance") {
        if (key != "name" && key != "prefix" && key != "hub_port" && key != "serve_https_port" &&
            key != "state_dir" && key != "herdr_session" && key != "backend_port_base") {
            if (key == "layout" || key == "hub_host")
                err("[instance] holds the key " key ": bare top-level keys must stay before every table header")
            else
                err("[instance] unknown key " key)
            next
        }
        set[key] = 1; v[key] = val
    } else if (section == "session") {
        if (key == "name") sname[nsess] = substr(val, 3)
        else if (key == "backend_port") sport[nsess] = val
        else if (key == "remote_host") shost[nsess] = substr(val, 3)
        else if (key == "endpoint") send[nsess] = substr(val, 3)
    }
}
END {
    name = "default"; prefix = ""; hub_port = D_hub_port; serve = D_serve
    state_dir = ""; herdr = ""; base = D_base
    nondefault = 0

    if (("name" in set) && need_str("name", v["name"])) {
        name = substr(v["name"], 3)
        if (!is_name(name)) err("[instance] name must match ^[a-z][a-z0-9-]{0,23}$")
        if (name != "default") nondefault = 1
    }
    prefix = name
    if (("prefix" in set) && need_str("prefix", v["prefix"])) {
        prefix = substr(v["prefix"], 3)
        if (!is_name(prefix)) err("[instance] prefix must match ^[a-z][a-z0-9-]{0,23}$")
        if (prefix != name) nondefault = 1
    }
    if (("hub_port" in set) && need_int("hub_port", v["hub_port"], 1024, 65535)) {
        hub_port = substr(v["hub_port"], 3) + 0
        if (hub_port != D_hub_port) nondefault = 1
    }
    if (("serve_https_port" in set) && need_int("serve_https_port", v["serve_https_port"], 1, 65535)) {
        serve = substr(v["serve_https_port"], 3) + 0
        if (serve != D_serve) nondefault = 1
    }
    if (("state_dir" in set) && need_str("state_dir", v["state_dir"])) {
        state_dir = substr(v["state_dir"], 3)
        if (state_dir != "") {
            if (state_dir !~ /^\// && state_dir !~ /^~\//) err("[instance] state_dir must be an absolute path or start with ~/")
            if (is_default_state(state_dir))
                err("[instance] state_dir must not be the default state directory (" state_dir "): a non-default instance would share the default instance token store, identity key, control socket and ledger")
            nondefault = 1
        }
    }
    if (("herdr_session" in set) && need_str("herdr_session", v["herdr_session"])) {
        herdr = substr(v["herdr_session"], 3)
        if (herdr != "") {
            if (!is_name(herdr)) err("[instance] herdr_session must match ^[a-z][a-z0-9-]{0,23}$")
            nondefault = 1
        }
    }
    if (("backend_port_base" in set) && need_int("backend_port_base", v["backend_port_base"], 1, 65535)) {
        base = substr(v["backend_port_base"], 3) + 0
        if (base != D_base) nondefault = 1
    }

    if (nondefault) {
        if (!("name" in set) || name == "default") err("[instance] a non-default instance must set name (to something other than default)")
        if (state_dir == "") err("[instance] a non-default instance must set state_dir")
        if (herdr == "") err("[instance] a non-default instance must set herdr_session")
    }

    if (nondefault) {
        if (hub_port == D_hub_port) err("[instance] hub_port must not be " D_hub_port " (the default) for a non-default instance: another instance may own it")
        if (serve == D_serve) err("[instance] serve_https_port must not be " D_serve " (the default) for a non-default instance: another instance may own it")
    }

    # Resolve each session backend port: its own backend_port, else base + index. With no
    # [instance] table the endpoint port is used, exactly as before this table existed.
    for (i = 1; i <= nsess; i++) {
        p = ""
        if (sport[i] != "") {
            if (substr(sport[i], 1, 2) == "n:" && substr(sport[i], 3) ~ /^[0-9]+$/ && length(sport[i]) <= 11 &&
                substr(sport[i], 3) + 0 >= 1 && substr(sport[i], 3) + 0 <= 65535)
                p = substr(sport[i], 3) + 0
            else err("session " sname[i] ": backend_port must be an integer from 1 to 65535")
        } else if (!has_instance && match(send[i], /:[0-9]+$/)) {
            p = substr(send[i], RSTART + 1) + 0
        } else {
            p = base + i - 1
            if (p > 65535) err("session " sname[i] ": backend port " p " is above 65535 (backend_port_base + index)")
        }
        rport[i] = p
    }
    allown = 1
    for (i = 1; i <= nsess; i++) if (sport[i] == "") allown = 0
    if (nondefault && !allown) for (i = 1; i <= nsess; i++)
        if (rport[i] != "" && rport[i] >= D_base && rport[i] < D_base + nsess)
            err("[instance] backend_port_base must keep session " sname[i] " off port " rport[i] " (the default range " D_base " to " (D_base + nsess - 1) ") unless every session sets its own backend_port")
    for (i = 1; i <= nsess; i++) for (j = i + 1; j <= nsess; j++)
        if (rport[i] != "" && rport[i] == rport[j] && shost[i] == shost[j])
            err("sessions " sname[i] " and " sname[j] " both resolve to backend port " rport[i] " on host " (shost[i] == "" ? "(unset)" : shost[i]))

    if (bad) exit 1
    if (has_instance) for (i = 1; i <= nsess; i++)
        if (match(send[i], /:[0-9]+$/) && substr(send[i], RSTART + 1) + 0 != rport[i])
            print "instance.sh: warning: session " sname[i] ": its endpoint port " (substr(send[i], RSTART + 1) + 0) \
                " differs from the resolved backend port " rport[i] "; the body is given http://127.0.0.1:" rport[i] \
                " (on a shared host the endpoint port may belong to another instance)" > "/dev/stderr"
    print "instance: name=" name " prefix=" prefix " hub_port=" hub_port " serve_https_port=" serve \
        " state_dir=" state_dir " herdr_session=" herdr " backend_port_base=" base
    for (i = 1; i <= nsess; i++) print "session " sname[i] ": backend_port=" rport[i] " endpoint=http://127.0.0.1:" rport[i]
}
' "$1"
