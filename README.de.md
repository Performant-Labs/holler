<div align="center">

# holler

[![CI](https://img.shields.io/github/actions/workflow/status/Performant-Labs/holler/ci.yml?branch=main&label=CI)](https://github.com/Performant-Labs/holler/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/Performant-Labs/holler)](https://github.com/Performant-Labs/holler/releases)
[![License](https://img.shields.io/github/license/Performant-Labs/holler)](./LICENSE)

**Deine Agenten sind nur einen Ruf entfernt — eine einzige Binärdatei, als Hub oder als Body.**

[English](README.md) | [中文](README.zh-CN.md) | [日本語](README.ja.md) | [Español](README.es.md) | **Deutsch** | [Français](README.fr.md)

</div>

Holler ist ein selbst gehosteter, ausschließlich ausgehender Kanal für interaktive
Coding-Sessions auf Maschinen, die dir gehören — ein `Hub`, der überwachen kann, und ein `Body`,
der zusammen mit dem eigentlichen Coding-Agenten läuft, verbunden durch dieselbe Binärdatei, die
je nach Rolle als Hub oder Body startet.

> **Dieses Dokument übersetzt nur den "Einstiegs"-Teil (Why Holler?, Install, Quick Start).**
> Die vollständige technische Referenz — Harness recipes, Debug output, Contributing usw. —
> gibt es aktuell nur auf Englisch. Damit eine veraltete Übersetzung nicht zu falschen Befehlen
> oder Flags führt, findest du alles Weitere in der [README.md](README.md).

## Warum Holler?

- **Selbst gehostet, kein Vendor-Relay** — du betreibst den Hub, du betreibst die Bodies. Kein
  Cloud-Mittelsmann, der deine Sessions routet.
- **Ausschließlich ausgehend** — ein Body wählt sich beim Hub ein; der Hub braucht nie einen
  eingehenden Listener, der aus dem Netz des Bodys erreichbar sein müsste. Funktioniert
  konstruktionsbedingt hinter NAT/Firewalls.
- **Pro Maschine vergebene, widerrufbare Identität** — ein Join-Token wird zu einem gebundenen
  Credential ([ADR 0007](docs/adr/ADR-0007.md)); kein gemeinsames Secret, keine Annahme, dass
  "die Tailnet-IP schon die richtige Identität ist" (ein Tailnet oder VPN ist nur die
  Netzwerkebene darunter, niemals die Identität selbst — [ADR 0006](docs/adr/ADR-0006.md)).
- **Ein Hub, der überwachen kann — wenn du das willst** — ein Audit-Log und, wo es wichtig ist,
  Limits für Turns/Ausgaben, ohne dass jedes Deployment dazu gezwungen wird.
- **Neue Harnesses per Konfiguration, nicht per Code** — eine `[[session]]`-Zeile auf einen neuen
  ACP-fähigen Adapter zu zeigen, erfordert keine Codeänderung und kein neues Release von Holler
  ([ADR 0012](docs/adr/ADR-0012.md)).
- **Eine Komposition, kein Protokoll von Grund auf** — Holler erfindet weder Agent-zu-Agent-
  Messaging noch Interrupt-Semantik neu; für den Body↔Harness-Hop wird ACP v2 übernommen, und
  bewusst wird die Agent-zu-Agent-Ebene ausgespart. Siehe [Where Holler fits](README.md#where-holler-fits)
  (auf Englisch).

Dieses Projekt wird aktiv weiterentwickelt — der Kern-Kanal zwischen Hub und Body, der
Attach-Modus und die CLI-Oberfläche sind real und bereits released (siehe Installation unten);
das [Test-Epic (#366)](https://github.com/Performant-Labs/holler/issues/366), das Coverage und
Lasttests nachverfolgt, ist weiterhin offen.

## Installation

**Homebrew** (macOS/Apple Silicon, Linux/x86_64, oder Linux/arm64 über
[Homebrew on Linux](https://docs.brew.sh/Homebrew-on-Linux)):

```bash
brew tap Performant-Labs/tap
brew install holler
```

Aus [Performant-Labs/homebrew-tap](https://github.com/Performant-Labs/homebrew-tap), einem selbst
gehosteten Tap — (noch) nicht in `homebrew-core`. `brew upgrade holler` holt neue Releases,
sobald die Formel dieses Taps aktualisiert wird.

<details>
<summary>Einzeiliger Installer oder Build aus dem Quellcode</summary>

**Einzeiliger Installer**, falls du kein Homebrew nutzt:

```bash
curl -fsSL https://raw.githubusercontent.com/Performant-Labs/holler/main/install.sh | sh
```

Lädt die neueste [Release](https://github.com/Performant-Labs/holler/releases)-Binärdatei für
deine Plattform (macOS/Apple Silicon, Linux/x86_64, oder Linux/arm64 — Windows wird nicht
unterstützt, siehe [#378](https://github.com/Performant-Labs/holler/issues/378)) nach
`~/.local/bin/holler` herunter. Eine bestimmte Version lässt sich mit `HOLLER_VERSION=v0.3.0`
festlegen, das Installationsverzeichnis mit `HOLLER_INSTALL_DIR=/usr/local/bin` ändern (jeweils
als Umgebungsvariable vor dem obigen Befehl setzen).

**Aus dem Quellcode:** `cargo build --release -p holler-cli`, die Binärdatei liegt danach unter
`target/release/holler`.

</details>

## Schnellstart

Eine Binärdatei, zwei Rollen. Auf der Maschine, die erreichbar sein soll (der **Hub**):

```bash
holler hub serve --listen 127.0.0.1:41807 --advertise <this-machine's-address>
holler hub token mint --label my-first-body
```

`token mint` gibt einen fertigen `body join`-Befehl aus — den führst du auf der Maschine aus,
auf der der eigentliche Coding-Agent läuft (der **Body**), und startest ihn anschließend:

```bash
holler body join --server wss://<hub-address> --token <token> --hub-key <hub-key>
holler body run --config sessions.toml
```

Zurück auf der Hub-Seite, mit einer Session sprechen:

```bash
holler roster                          # zeigt, was gerade verbunden ist
holler say <session-name> "hello"      # einmaliger Prompt, gibt die Antwort aus
```

Wie `sessions.toml` genau aufgebaut ist — welcher Harness pro Session läuft, Spawn- oder
Attach-Modus — ist Konfiguration, kein Code (siehe [Harness recipes](README.md#harness-recipes)
auf Englisch und [ADR 0012](docs/adr/ADR-0012.md)). Für einen lokalen Orchestrator plus eine
oder mehrere entfernte Attach-Sessions in einem gemeinsamen Terminal-Workspace siehe
[Set up a Herdr workspace with an agent](README.md#set-up-a-herdr-workspace-with-an-agent)
(auf Englisch).

---

Für den vollständigen Inhalt — Documentation, Where Holler fits, Attach convenience, Harness
recipes, Debug output, Contributing, License — siehe die englische [README.md](README.md).
