<div align="center">

# holler

[![CI](https://img.shields.io/github/actions/workflow/status/Performant-Labs/holler/ci.yml?branch=main&label=CI)](https://github.com/Performant-Labs/holler/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/Performant-Labs/holler)](https://github.com/Performant-Labs/holler/releases)
[![License](https://img.shields.io/github/license/Performant-Labs/holler)](./LICENSE)

**Vos agents ne sont qu'à un appel de distance — un seul binaire, hub ou body.**

[English](README.md) | [中文](README.zh-CN.md) | [日本語](README.ja.md) | [Español](README.es.md) | [Deutsch](README.de.md) | **Français**

</div>

Holler est un circuit auto-hébergé, en sortie uniquement, pour des sessions de programmation
interactives sur des machines qui vous appartiennent — un `hub` capable de superviser, et un
`body` qui s'exécute aux côtés de l'agent de code réel, reliés par un seul binaire qui joue l'un
ou l'autre rôle.

> **Ce document ne traduit que la partie « porte d'entrée » (Why Holler?, Install, Quick Start).**
> La référence technique complète — Harness recipes, Debug output, Contributing, etc. — n'existe
> pour l'instant qu'en anglais. Pour éviter qu'une traduction obsolète n'affiche des commandes ou
> des flags erronés, consultez le [README.md](README.md) pour tout le reste. Cette traduction a
> été réalisée avec l'aide d'une IA.

## Pourquoi Holler ?

- **Auto-hébergé, pas un relais d'éditeur** — vous exploitez le hub, vous exploitez les bodies.
  Aucun intermédiaire cloud n'achemine vos sessions.
- **En sortie uniquement** — un body se connecte au hub ; le hub n'a jamais besoin d'un listener
  entrant joignable depuis le réseau du body. Fonctionne derrière NAT/pare-feu par construction.
- **Identité émise par machine, révocable** — un join token devient un identifiant lié
  ([ADR 0007](docs/adr/ADR-0007.md)) ; pas de secret partagé, pas de « l'IP du tailnet fait foi »
  (un tailnet ou un VPN n'est que la couche réseau sous-jacente, jamais l'identité —
  [ADR 0006](docs/adr/ADR-0006.md)).
- **Un hub qui peut superviser, si vous le souhaitez** — un journal d'audit et, là où c'est
  important, des plafonds de tours et de dépenses, sans obliger chaque déploiement à les activer.
- **De la configuration, pas du code, pour les nouveaux harnesses** — pointer une ligne
  `[[session]]` vers un nouvel adaptateur compatible ACP ne nécessite ni modification du code de
  Holler ni nouvelle version ([ADR 0012](docs/adr/ADR-0012.md)).
- **De la composition, pas un protocole partant de zéro** — Holler ne réinvente ni la messagerie
  entre agents ni la sémantique d'interruption ; il adopte ACP v2 pour le saut body↔harness et
  reste en dehors de la couche agent-à-agent. Voir [Where Holler fits](README.md#where-holler-fits)
  (en anglais).

Ce projet est en développement actif — le circuit central hub/body, le mode attach et la surface
CLI sont réels et déjà publiés (voir [Installation](#installation)) ; l'[epic de
tests](https://github.com/Performant-Labs/holler/issues/366) qui suit la couverture et les
tests de charge est toujours ouvert.

## Installation

**Homebrew** (macOS/Apple Silicon, Linux/x86_64 ou Linux/arm64 — via [Homebrew on Linux](https://docs.brew.sh/Homebrew-on-Linux)) :

```bash
brew tap Performant-Labs/tap
brew install holler
```

Depuis [Performant-Labs/homebrew-tap](https://github.com/Performant-Labs/homebrew-tap), un tap
auto-hébergé — pas (encore) dans `homebrew-core`. `brew upgrade holler` récupère les nouvelles
versions dès que la formule du tap est mise à jour.

<details>
<summary>Installateur en une ligne, ou compilation depuis les sources</summary>

**Installateur en une ligne**, si vous n'utilisez pas Homebrew :

```bash
curl -fsSL https://raw.githubusercontent.com/Performant-Labs/holler/main/install.sh | sh
```

Télécharge le binaire de la dernière [version](https://github.com/Performant-Labs/holler/releases)
pour votre plateforme (macOS/Apple Silicon, Linux/x86_64 ou Linux/arm64 — Windows n'est pas une
cible prise en charge, voir [#378](https://github.com/Performant-Labs/holler/issues/378)) vers
`~/.local/bin/holler`. Vous pouvez figer une version précise avec `HOLLER_VERSION=v0.3.0`, ou
changer le répertoire d'installation avec `HOLLER_INSTALL_DIR=/usr/local/bin` (préfixez l'une ou
l'autre comme variable d'environnement devant la commande ci-dessus).

**Depuis les sources :** `cargo build --release -p holler-cli`, binaire dans
`target/release/holler`.

</details>

## Démarrage rapide

Un seul binaire, deux rôles. Sur la machine qui doit être joignable (le **hub**) :

```bash
holler hub serve --listen 127.0.0.1:41807 --advertise <this-machine's-address>
holler hub token mint --label my-first-body
```

`token mint` affiche une commande `body join` prête à l'emploi — exécutez-la sur la machine où
tourne l'agent de code réel (le **body**), puis démarrez-le :

```bash
holler body join --server <scheme>://<hub-address> --token <token> --hub-key <hub-key>
holler body run --config sessions.toml
```

`<scheme>` est celui que la commande affichée par `token mint` indiquait déjà — `ws://` pour un
`--advertise` en loopback (le cas mono-machine ci-dessous), `wss://` pour tout le reste
(ADR 0006). De retour côté hub, parlez à une session :

```bash
holler roster                          # voir ce qui est connecté
holler say <session-name> "hello"      # prompt ponctuel, affiche la réponse
```

### Un `sessions.toml` réel et fonctionnel

La forme de `sessions.toml` — quel harness exécute chaque session, mode spawn ou attach — relève
de la configuration, pas du code ([ADR 0012](docs/adr/ADR-0012.md)). Trois points de départ
réels, selon la façon dont vous avez obtenu `holler` et ce que vous ciblez :

**Compilé depuis les sources** (`cargo build --workspace`, et non le seul binaire
Homebrew/`install.sh`) : le workspace compile aussi `stub-acp`, un véritable agent de test
déterministe — sans modèle, sans clé d'API, sans réseau — qui est le moyen le plus rapide de voir
une session tourner pour de bon :

```toml
[[session]]
name = "hello"
harness = "opencode"
command = ["/absolute/path/to/target/release/stub-acp"]
```

```bash
holler roster
holler say my-first-body/hello "hello"
# -> a real reply, e.g. "stub chunk 0stub chunk 1stub chunk 2"
```

(Vérifié sur une exécution réelle `hub serve` → `token mint` → `body join` → `body run` → `say`,
le 2026-09-22.)

**Un véritable agent de code**, quelle que soit l'installation : voir
[Harness recipes](README.md#harness-recipes) pour la forme de `command` (par ex. la recette
`claude`/passerelle ACP — actuellement bloquée, voir la note propre à cette section) et la façon
dont `harness`/`command` correspondent à un vrai sous-processus.

**Se rattacher à une session déjà en cours d'exécution** (sans `sessions.toml` écrit à la main) :
voir [Attach convenience](README.md#attach-convenience) — `holler body attach init` écrit pour
vous la ligne `[[session]]` à partir d'un endpoint OpenCode actif.

Pour un orchestrateur local plus une ou plusieurs sessions distantes en mode attach dans un même
espace de travail de terminal, voir
[Set up a Herdr workspace with an agent](README.md#set-up-a-herdr-workspace-with-an-agent)
(en anglais).

---

Pour le contenu complet — Documentation, Where Holler fits, Attach convenience, Harness recipes,
Debug output, Contributing, License — consultez le [README.md](README.md) en anglais.
