<div align="center">

# holler

[![CI](https://img.shields.io/github/actions/workflow/status/Performant-Labs/holler/ci.yml?branch=main&label=CI)](https://github.com/Performant-Labs/holler/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/Performant-Labs/holler)](https://github.com/Performant-Labs/holler/releases)
[![License](https://img.shields.io/github/license/Performant-Labs/holler)](./LICENSE)

**Tus agentes están a un grito de distancia — un solo binario, hub o body.**

[English](README.md) | [中文](README.zh-CN.md) | [日本語](README.ja.md) | **Español** | [Deutsch](README.de.md) | [Français](README.fr.md)

</div>

Holler es un circuito autoalojado y de solo salida para sesiones de programación interactivas en
máquinas que tú controlas — un `hub` que puede supervisar, y un `body` que se ejecuta junto al
agente de código real, conectados por un único binario que actúa en uno u otro rol.

> **Este documento traduce únicamente la parte de "entrada" (Why Holler?, Install, Quick Start).**
> La referencia técnica completa —Harness recipes, Debug output, Contributing, etc.— solo existe
> por ahora en inglés. Para evitar que una traducción desactualizada muestre comandos o flags
> incorrectos, consulta [README.md](README.md) para todo lo demás.

## ¿Por qué Holler?

- **Autoalojado, no un relé de un proveedor** — tú ejecutas el hub, tú ejecutas los bodies. Sin un
  intermediario en la nube enrutando tus sesiones.
- **Solo salida** — un body marca hacia el hub; el hub nunca necesita un puerto de escucha entrante
  accesible desde la red del body. Funciona detrás de NAT/firewalls por diseño.
- **Identidad acuñada por máquina y revocable** — un token de unión (join token) se convierte en una
  credencial vinculada ([ADR 0007](docs/adr/ADR-0007.md)); sin secretos compartidos, sin asumir que
  "la IP de la tailnet es quien dice ser" (una tailnet o VPN es la capa de red subyacente, nunca la
  identidad — [ADR 0006](docs/adr/ADR-0006.md)).
- **Un hub que puede supervisar, si lo quieres así** — un registro de auditoría y, cuando importa,
  límites de turnos/gasto, sin obligar a cada despliegue a activarlo.
- **Configuración, no código, para nuevos harnesses** — apuntar una fila `[[session]]` a un nuevo
  adaptador compatible con ACP no requiere cambios de código ni una nueva versión de Holler
  ([ADR 0012](docs/adr/ADR-0012.md)).
- **Composición, no un protocolo desde cero** — Holler no reinventa la mensajería entre agentes ni
  la semántica de interrupción; adopta ACP v2 para el salto body↔harness y se mantiene fuera de la
  capa agente-a-agente. Ver [Where Holler fits](README.md#where-holler-fits) (en inglés).

Este proyecto está en desarrollo activo — el circuito hub/body, el modo attach y la superficie de
la CLI son reales y ya están publicados (ver la instalación más abajo); el
[epic de pruebas (#366)](https://github.com/Performant-Labs/holler/issues/366) que hace seguimiento
de la cobertura y las pruebas de carga sigue abierto.

## Instalación

**Homebrew** (macOS/Apple Silicon, Linux/x86_64, o Linux/arm64 vía
[Homebrew on Linux](https://docs.brew.sh/Homebrew-on-Linux)):

```bash
brew tap Performant-Labs/tap
brew install holler
```

Desde [Performant-Labs/homebrew-tap](https://github.com/Performant-Labs/homebrew-tap), un tap
autoalojado — todavía no está en `homebrew-core`. `brew upgrade holler` recogerá las nuevas
versiones en cuanto se actualice la fórmula de este tap.

<details>
<summary>Instalador de una línea, o compilar desde el código fuente</summary>

**Instalador de una línea**, si no usas Homebrew:

```bash
curl -fsSL https://raw.githubusercontent.com/Performant-Labs/holler/main/install.sh | sh
```

Descarga el binario de la última [versión publicada](https://github.com/Performant-Labs/holler/releases)
para tu plataforma (macOS/Apple Silicon, Linux/x86_64, o Linux/arm64 — Windows no es una plataforma
soportada, ver [#378](https://github.com/Performant-Labs/holler/issues/378)) a `~/.local/bin/holler`.
Fija una versión específica con `HOLLER_VERSION=v0.2.0`, o cambia el directorio de instalación con
`HOLLER_INSTALL_DIR=/usr/local/bin` (antepón cualquiera de las dos como variable de entorno antes
del comando anterior).

**Desde el código fuente:** `cargo build --release -p holler-cli`, el binario queda en
`target/release/holler`.

</details>

## Inicio rápido

Un solo binario, dos roles. En la máquina que debería ser accesible (el **hub**):

```bash
holler hub serve --listen 127.0.0.1:41807 --advertise <this-machine's-address>
holler hub token mint --label my-first-body
```

`token mint` imprime un comando `body join` listo para usar — ejecútalo en la máquina donde
corre el agente de código real (el **body**), y luego inícialo:

```bash
holler body join --server wss://<hub-address> --token <token> --hub-key <hub-key>
holler body run --config sessions.toml
```

De vuelta en el lado del hub, habla con una sesión:

```bash
holler roster                          # ver qué está conectado
holler say <session-name> "hello"      # prompt de una sola vez, imprime la respuesta
```

La forma de `sessions.toml` — qué harness usa cada sesión, modo spawn o attach — es
configuración, no código (ver [Harness recipes](README.md#harness-recipes) en inglés y
[ADR 0012](docs/adr/ADR-0012.md)). Para un orquestador local más una o varias sesiones remotas
en modo attach dentro de un mismo espacio de trabajo de terminal, ver
[Set up a Herdr workspace with an agent](README.md#set-up-a-herdr-workspace-with-an-agent)
(en inglés).

---

Para el contenido completo — Documentation, Where Holler fits, Attach convenience, Harness
recipes, Debug output, Contributing, License — consulta el [README.md](README.md) en inglés.
