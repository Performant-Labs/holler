#!/usr/bin/env python3
"""A stream-json driver for spike #830: the hub owns the Claude process (no TUI).

Runs `claude -p --input-format stream-json --output-format stream-json --verbose`
under the same minimal environment as the shell probes, feeds it user messages on stdin
one turn at a time, and prints one summary line per event type it cares about. With
--interrupt-after SECONDS it sends a control_request interrupt that long after the
first message and reports what the stream did.

Usage (the shell probes call it): claude-stream.py CLAUDE_BIN CWD MODEL [options] MSG...
  --session-id UUID        choose the session id
  --resume UUID            resume a session
  --interrupt-after SEC    send {"type":"control_request","request":{"subtype":"interrupt"}}
  --allow-tools TOOLS      value for --allowed-tools
Env: only HOME and PATH are taken from the caller; nothing else is passed on.
"""
import json
import os
import subprocess
import sys
import threading
import time


def main():
    args = sys.argv[1:]
    claude, cwd, model = args[0], args[1], args[2]
    rest = args[3:]
    opts = {"session-id": None, "resume": None, "interrupt-after": None, "allow-tools": None}
    msgs = []
    i = 0
    while i < len(rest):
        if rest[i].startswith("--") and rest[i][2:] in opts:
            opts[rest[i][2:]] = rest[i + 1]
            i += 2
        else:
            msgs.append(rest[i])
            i += 1

    cmd = [claude, "--setting-sources", "project,local", "--model", model, "-p",
           "--input-format", "stream-json", "--output-format", "stream-json", "--verbose",
           "--permission-mode", "bypassPermissions"]
    if opts["session-id"]:
        cmd += ["--session-id", opts["session-id"]]
    if opts["resume"]:
        cmd += ["--resume", opts["resume"]]
    if opts["allow-tools"]:
        cmd += ["--allowed-tools", opts["allow-tools"]]
    env = {"HOME": os.environ["HOME"], "PATH": os.environ["PATH"], "TERM": "dumb", "LANG": "C.UTF-8"}
    p = subprocess.Popen(cmd, cwd=cwd, env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                         stderr=subprocess.DEVNULL, text=True, bufsize=1)

    events = []
    done = threading.Event()
    t0 = time.time()

    def reader():
        for line in p.stdout:
            line = line.strip()
            if not line:
                continue
            try:
                ev = json.loads(line)
            except json.JSONDecodeError:
                continue
            ev["_t"] = round(time.time() - t0, 2)
            events.append(ev)
            done.set()

    threading.Thread(target=reader, daemon=True).start()

    def user(text):
        p.stdin.write(json.dumps({"type": "user", "message": {"role": "user", "content": text}}) + "\n")
        p.stdin.flush()

    def wait_result(timeout):
        end = time.time() + timeout
        seen = len([e for e in events if e.get("type") == "result"])
        while time.time() < end:
            if len([e for e in events if e.get("type") == "result"]) > seen:
                return True
            time.sleep(0.1)
        return False

    for idx, m in enumerate(msgs):
        user(m)
        if idx == 0 and opts["interrupt-after"]:
            time.sleep(float(opts["interrupt-after"]))
            p.stdin.write(json.dumps({"type": "control_request", "request_id": "spike-int-1",
                                      "request": {"subtype": "interrupt"}}) + "\n")
            p.stdin.flush()
        wait_result(90)
    try:
        p.stdin.close()
    except OSError:
        pass
    try:
        p.wait(timeout=20)
    except subprocess.TimeoutExpired:
        p.kill()

    for ev in events:
        t = ev.get("type")
        if t == "system" and ev.get("subtype") == "init":
            chosen = opts["session-id"] or opts["resume"]
            print(f"init       t={ev['_t']} session_id_is_the_requested_one={ev.get('session_id') == chosen} caps={ev.get('capabilities')}")
        elif t == "control_response":
            print(f"control    t={ev['_t']} {json.dumps(ev.get('response'))[:200]}")
        elif t == "result":
            print(f"result     t={ev['_t']} subtype={ev.get('subtype')} is_error={ev.get('is_error')} "
                  f"session_id_is_the_requested_one={ev.get('session_id') == (opts['session-id'] or opts['resume'])} text={str(ev.get('result'))[:60]!r}")
        elif t == "system" and ev.get("subtype") not in ("init", None):
            print(f"system     t={ev['_t']} subtype={ev.get('subtype')}")
    print(f"exit       code={p.returncode} events={len(events)} types={sorted({e.get('type') for e in events})}")


if __name__ == "__main__":
    main()
