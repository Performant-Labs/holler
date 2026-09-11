# Deploying a hub (v1)

This is the doc the hub's own refusal message points to:

```
refusing to bind 0.0.0.0:0 as plain ws: put a TLS-terminating proxy in front (docs/deploy.md) — non-loopback plain ws is not allowed (ADR 0006)
```

The short version: **the hub only ever binds loopback.** If you need bodies
to reach it from another machine, put something in front of it that
terminates TLS and forwards to the loopback port. This document covers why,
and how. The decision record is [ADR 0006](adr/ADR-0006.md) — this doc does
not add requirements beyond what that ADR decided.

## Why the hub refuses non-loopback binds

`hub serve --listen <addr>` validates every `--listen` address before
anything binds. A loopback address (`127.0.0.1`/`[::1]`, any port) is
accepted; anything else — a LAN or public IP, a wildcard like `0.0.0.0`, or
the name `localhost` (which is never resolved) — is refused with exit 3
before the socket opens.

The hub speaks plain WebSocket (`ws://127.0.0.1:41807`), not `wss://`. It has no TLS implementation in
v1 (native hub TLS is tracked for v2 — see ADR 0006's "Not decided here").
So a hub bound to a non-loopback address would be an unencrypted WebSocket
reachable off the machine, which ADR 0006 rules out categorically rather
than gating on a flag. The proxy hop from a TLS-terminating front end down
to `ws://127.0.0.1:41807` is still compliant, because that hop is loopback.

## The supported path: a Tailscale tailnet

For v1 this is the supported way to expose a hub to bodies on other
machines. On the hub machine:

```
tailscale serve --bg 41807
```

Enable HTTPS certificates and MagicDNS in the Tailscale admin console for
your tailnet. `tailscale serve` terminates TLS 1.3 on the hub machine using
a Let's Encrypt certificate issued for the tailnet's MagicDNS name, and
forwards the WebSocket upgrade to the hub's loopback listener. Bodies dial:

```
wss://<hub-machine>.<your-tailnet>.ts.net
```

That hostname resolves to a certificate the body's TLS stack already
trusts — bodies use `rustls-tls-native-roots`, and the tailnet's Let's
Encrypt certificate validates against the same OS trust store, so there is
no certificate to distribute by hand.

Tailscale is strictly an underlay and a proxy here — never identity. A
tailnet IP or MagicDNS name is never treated as authentication; a body
still has to `body join` with a minted join token over the connection.
Holler itself never calls the Tailscale API.

On the hub side, start the hub bound to loopback as usual and let the proxy
do the rest:

```
holler hub serve --listen 127.0.0.1:41807
```

## Other proxies work too

ADR 0006 treats `tailscale serve` as the documented v1 path, but any
TLS-terminating reverse proxy in front of the loopback listener is equally
acceptable — Caddy, Traefik, and nginx are all fine as long as they:

- **run on a machine you control** (see the warning below),
- terminate TLS 1.3 themselves,
- forward the WebSocket upgrade to `ws://127.0.0.1:41807` (or whatever port
  the hub is bound to), and
- hold connections open well past the hub's 15 s presence beat — set the
  proxy's idle/read timeout to **at least 120 s** so a proxy timeout doesn't
  masquerade as a dropped body.

> **The proxy is a trusted component, not merely a transport.** TLS terminates
> *at the proxy*, and the proxy→hub hop is plaintext `ws` on loopback — so the
> proxy sees every frame in the clear: the join secret, the body credential,
> and every prompt and reply. The `tailscale serve` path above is safe by
> construction because it runs **on the hub machine itself**; the proxy and the
> hub are one trust domain.
>
> That property does not survive moving the proxy somewhere else. Do **not**
> terminate Holler's TLS on a host you do not control — a shared ingress, a
> managed load balancer, or a third-party edge — because doing so hands that
> operator your credentials and your conversations. This is ADR 0006 point 6;
> nothing in the hub can detect or enforce it for you.

### Example: Caddy

```
hub.example.com {
    reverse_proxy 127.0.0.1:41807
}
```

Caddy terminates TLS with an automatically-issued certificate and proxies
WebSocket upgrades without extra configuration.

### Example: nginx

```
server {
    listen 443 ssl;
    server_name hub.example.com;

    ssl_certificate     /etc/letsencrypt/live/hub.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/hub.example.com/privkey.pem;

    location / {
        proxy_pass http://127.0.0.1:41807;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_read_timeout 120s;
    }
}
```

Whichever proxy you use, bodies then dial the proxy's `wss://` hostname,
never the hub's loopback address directly.

## What this doesn't cover

Native TLS in the hub (`--tls-cert`/`--tls-key`) is v2 work per ADR 0006 and
isn't available yet — there's no way to skip the proxy hop in v1. Prefix-
scoped tokens are reserved grammar (ADR 0005) but not part of the
deployment model. The old SSH-tunnel scripts (ADR 0002) are retired; don't
resurrect them as a workaround.

## A note on hub log addresses

Because the hub only ever sees the proxy's loopback connection, peer
addresses in hub logs are the proxy's, not the body's real origin.
Authentication and authorization are by join token, not by source address,
so this doesn't weaken anything — but don't expect hub logs to tell you
where a body is actually connecting from.
