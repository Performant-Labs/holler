# The setup wizard's two-instance proof

Epic #726 lets the setup wizard build a second Holler instance beside a running first one.
This page explains the scenario that proves it (story #732), how to run it, and what it does
and does not show.

## Run it

```bash
cargo test -p holler-cli --test wizard_scripts proof::
```

It is part of the `wizard_scripts` target, so `cargo test -p holler-cli --test wizard_scripts`
runs it with the other wizard tests. The code is `crates/holler-cli/tests/wizard/proof.rs`;
its fakes are the small bash scripts in `crates/holler-cli/tests/wizard/proof_fakes/`.

## What the scenario does

Everything happens on one loopback host, in a temp directory, with high free loopback ports.

1. **Instance A** is brought up with the wizard's own steps: `instance.sh` validates its
   `[instance]` table, `inventory.sh` and `collide.sh` run the preflight, then a backend
   (stage 4), a hub with `tailscale serve` (stage 6), one body (stage 7) and one Herdr server
   (stage 8, only through `herdr.sh`) are started, each recorded in A's own ledger with
   `ledger.sh`. Before each backend or hub is started, `stop-owned.sh check-port` says the
   port is free.
2. A snapshot of A is taken: its ledger listing, its roster, the process rows of its
   recorded pids, the ports they hold plus its `tailscale serve` line, and a digest (mode,
   length and a hash of the content of every file) of its state directory.
3. **Instance B** is built the same way on the same host, with a different name, hub port,
   serve port, backend port, state directory, Herdr session and session name. A's snapshot is
   taken again and must equal the first.
4. **B is torn down** with `stop-owned.sh teardown <state_dir> --purge-state`. A's snapshot is
   taken a third time and must still equal the first. B's processes, listening ports, state
   directory and Herdr session must be gone, and a client command against B's state directory
   finds no hub.

Other checks in the same scenario:

- A **colliding config** (B with A's hub port, or with A's Herdr session name) is refused by
  `collide.sh` at the preflight. The refusal names the config key (`hub_port`,
  `herdr_session`). Nothing starts: the fake host's process, port, session and serve
  registries are unchanged, B's state directory is never created, no `holler` call carries it,
  and A's snapshot is unchanged. `herdr.sh check-session` independently refuses the shared
  session name.
- **No default state directory.** The temp `HOME` stays empty: neither instance writes to
  `$HOME/.holler`. Each hub keeps its info, identity key, token store and roster under its own
  `HOLLER_STATE_DIR`.
- **Client commands carry the instance's state directory.** Every call to the fake `holler`
  is logged with the `HOLLER_STATE_DIR` it carried; the test fails if one carried none or
  carried a directory that is not A's or B's. `holler roster` with B's directory shows only
  B's session, and with A's shows only A's.

## The fakes, and what that means

`ps -e`, `ss`, `lsof`, `tailscale`, `herdr`, `holler` and `opencode` are fake scripts placed
first on `PATH`. The real ones are never run, and no real process, port, state directory or
session is touched. The fake daemons register themselves in a private registry and turn
themselves into a harmless `sleep` with a command line such as `holler hub serve --listen
127.0.0.1:<port>`, so the real `inventory.sh`, `ledger.sh`, `collide.sh`, `herdr.sh` and
`stop-owned.sh` see the same shapes they see on a real host. Every such process is started
by the test and signalled only by the pid it recorded, through the ledger, never by name. The
fake `ps -e` lists only these registered processes, so a real process on the machine cannot
appear in an inventory.

## What it proves and what it does not

It proves that the wizard's scripts, driven as the skill tells the agent to drive them, keep
two instances apart on one host: the preflight, the ledger, the instance-scoped state and the
ownership-checked teardown leave the first instance's roster, processes, ports and state
exactly as they were, through B's whole run and B's teardown.

It does not prove anything about the real `holler` hub, `opencode`, Herdr or Tailscale.
Listening ports are those the fakes report, not real sockets. That is the live run on the
operator's workstation (#733) and the two-hubs-on-one-machine run (#734). `tailscale serve`
is not a process of the wizard, so teardown leaves B's serve entry in place by design; the
scenario's "ports gone" check covers listeners, not that entry.
