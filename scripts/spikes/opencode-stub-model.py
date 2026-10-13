#!/usr/bin/env python3
"""Local stand-in for a model provider, for the OpenCode compaction spike (#878).

Speaks just enough OpenAI-compatible streamed chat completions on 127.0.0.1 for
OpenCode to run turns and a summary against it. No network beyond the loopback
listener; no key is read, checked or logged.

  opencode-stub-model.py PORT LOGFILE

Behaviour:
  POST /v1/chat/completions   streamed reply with a fixed `usage` block and fixed text.
                              A request carrying OpenCode's summarization system
                              prompt ("context summarization agent") is a "summary" and gets the
                              fixed summary text and the smaller summary usage.
  POST /_ctl  {"hold": bool, "fail": bool, "hold_summary_only": bool}
                              hold: keep replies open (no bytes after the first) until
                              released; fail: answer HTTP 500; fail_summary_only and
                              hold_summary_only limit either to summary requests.
  GET  /_log                  the request log (one JSON object per line).
Every chat request is appended to LOGFILE as {n, kind, model, stream, keys, nmsg, chars, held, failed}.
"""
import json, sys, threading, time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

PORT, LOG = int(sys.argv[1]), sys.argv[2]
CTL = {"hold": False, "fail": False, "hold_summary_only": False, "fail_summary_only": False}
LOCK = threading.Lock()
COUNT = [0]
TURN_USAGE = {"prompt_tokens": 1000, "completion_tokens": 50, "total_tokens": 1050}
SUMMARY_USAGE = {"prompt_tokens": 400, "completion_tokens": 60, "total_tokens": 460}
TURN_TEXT = "Noted."
SUMMARY_TEXT = "## Summary\nThe user ran three short turns; nothing else happened. (fixed stub summary)"


def text_of(messages):
    out = []
    for m in messages:
        c = m.get("content")
        if isinstance(c, str):
            out.append(c)
        elif isinstance(c, list):
            out += [p.get("text", "") for p in c if isinstance(p, dict)]
    return "\n".join(out)


class H(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *a):
        pass

    def _json(self, code, obj):
        b = json.dumps(obj).encode()
        self.send_response(code)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(b)))
        self.end_headers()
        self.wfile.write(b)

    def do_GET(self):
        if self.path == "/_log":
            b = open(LOG, "rb").read() if __import__("os").path.exists(LOG) else b""
            self.send_response(200)
            self.send_header("content-length", str(len(b)))
            self.end_headers()
            self.wfile.write(b)
        elif self.path.endswith("/models"):
            self._json(200, {"object": "list", "data": []})
        else:
            self._json(404, {"error": "stub"})

    def do_POST(self):
        n = int(self.headers.get("content-length") or 0)
        raw = self.rfile.read(n) if n else b"{}"
        try:
            body = json.loads(raw or b"{}")
        except Exception:
            body = {}
        if self.path == "/_ctl":
            CTL.update({k: bool(v) for k, v in body.items() if k in CTL})
            return self._json(200, CTL)
        if not self.path.endswith("/chat/completions"):
            return self._json(404, {"error": "stub"})
        allt = text_of(body.get("messages", []))
        summary = "context summarization agent" in allt.lower()
        msgs = body.get("messages", [])
        with LOCK:
            COUNT[0] += 1
            seq = COUNT[0]
            hold = CTL["hold"] or (CTL["hold_summary_only"] and summary)
            fail = CTL["fail"] or (CTL["fail_summary_only"] and summary)
            with open(LOG, "a") as f:
                f.write(json.dumps({"n": seq, "kind": "summary" if summary else "turn",
                                    "model": body.get("model"), "stream": body.get("stream"),
                                    "keys": sorted(body.keys()), "nmsg": len(msgs), "chars": len(allt),
                                    "held": hold, "failed": fail}) + "\n")
        if fail:
            return self._json(500, {"error": {"message": "stub failure on request", "type": "stub"}})
        self.send_response(200)
        self.send_header("content-type", "text/event-stream")
        self.send_header("transfer-encoding", "chunked")
        self.end_headers()

        def chunk(obj):
            d = ("data: " + (obj if isinstance(obj, str) else json.dumps(obj)) + "\n\n").encode()
            self.wfile.write(b"%x\r\n" % len(d) + d + b"\r\n")
            self.wfile.flush()

        base = {"id": "chatcmpl-stub", "object": "chat.completion.chunk", "created": 0,
                "model": body.get("model")}
        chunk({**base, "choices": [{"index": 0, "delta": {"role": "assistant", "content": ""}, "finish_reason": None}]})
        while True:
            with LOCK:
                still = CTL["hold"] or (CTL["hold_summary_only"] and summary)
            if not (hold and still):
                break
            time.sleep(0.1)
        text = SUMMARY_TEXT if summary else TURN_TEXT
        chunk({**base, "choices": [{"index": 0, "delta": {"content": text}, "finish_reason": None}]})
        chunk({**base, "choices": [{"index": 0, "delta": {}, "finish_reason": "stop"}]})
        chunk({**base, "choices": [], "usage": SUMMARY_USAGE if summary else TURN_USAGE})
        chunk("[DONE]")
        self.wfile.write(b"0\r\n\r\n")
        self.wfile.flush()


ThreadingHTTPServer.daemon_threads = True
ThreadingHTTPServer(("127.0.0.1", PORT), H).serve_forever()
