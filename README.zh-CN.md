<div align="center">

# holler

[![CI](https://img.shields.io/github/actions/workflow/status/Performant-Labs/holler/ci.yml?branch=main&label=CI)](https://github.com/Performant-Labs/holler/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/Performant-Labs/holler)](https://github.com/Performant-Labs/holler/releases)
[![License](https://img.shields.io/github/license/Performant-Labs/holler)](./LICENSE)

**只需一声“holler”，你的智能体就在身边 —— 一个二进制文件，身兼 hub 与 body 两种角色。**

[English](README.md) | **中文** | [日本語](README.ja.md) | [Español](README.es.md) | [Deutsch](README.de.md) | [Français](README.fr.md)

</div>

Holler 是一条自托管、仅出站的线路，用于连接你自己掌控的机器上的交互式编程会话 —— 一端是可以进行监督的
`hub`，另一端是与真正的编程智能体一起运行的 `body`，两者由同一个二进制文件（分别以不同角色运行）连接。

> **本文档仅翻译了「入门」部分（Why Holler？、Install、Quick Start）。** 完整的技术参考
> —— Harness recipes、Debug output、Contributing 等 —— 目前只有英文版本，请查阅
> [README.md](README.md)，避免因翻译滞后而出现命令或参数不准确的情况。

## 为什么选择 Holler？

- **自托管，而非厂商中转** —— hub 由你运行，body 也由你运行，没有云端中间人在路由你的会话。
- **仅出站连接** —— body 主动拨号连接到 hub；hub 永远不需要一个能被 body 所在网络访问到的入站监听端口，天生适配 NAT/防火墙环境。
- **按机器铸造、可撤销的身份** —— 一个加入令牌（join token）会成为一个绑定的凭证
  （[ADR 0007](docs/adr/ADR-0007.md)）；没有共享密钥，也不会把 tailnet 的 IP 地址当作身份凭证
  （tailnet 或 VPN 只是底层网络，从来不是身份 —— [ADR 0006](docs/adr/ADR-0006.md)）。
- **hub 可以选择进行监督** —— 如果你需要，可以有审计日志，乃至轮次/花费上限，但不会强制每一次部署都必须启用。
- **新增 harness 只需配置，无需改代码** —— 把一行 `[[session]]` 指向一个新的支持 ACP
  的适配器，不需要修改 Holler 代码，也不需要发布新版本（[ADR 0012](docs/adr/ADR-0012.md)）。
- **是一种组合，而非另起炉灶的协议** —— Holler 并没有重新发明智能体间通信或中断语义；
  在 body↔harness 这一跳上采用 ACP v2，并且有意不介入智能体间（agent-to-agent）这一层。
  详见英文版 [Where Holler fits](README.md#where-holler-fits)。

Holler 目前正在积极开发中 —— 核心的 hub/body 线路、attach 模式、以及 CLI 接口都已经真实可用并已发布
（见下方安装说明）；跟踪测试覆盖率与负载测试工作的
[测试 epic（#366）](https://github.com/Performant-Labs/holler/issues/366) 仍处于开放状态。

## 安装

**Homebrew**（macOS/Apple Silicon、Linux/x86_64，或通过
[Homebrew on Linux](https://docs.brew.sh/Homebrew-on-Linux) 在 Linux/arm64 上安装）：

```bash
brew tap Performant-Labs/tap
brew install holler
```

来自 [Performant-Labs/homebrew-tap](https://github.com/Performant-Labs/homebrew-tap)，一个
自托管的 tap —— 目前还没有（尚未）进入 `homebrew-core`。等这个 tap 的 formula 更新后，
`brew upgrade holler` 就能拿到新版本。

<details>
<summary>一行安装脚本，或从源码构建</summary>

**一行安装脚本**，如果你不使用 Homebrew：

```bash
curl -fsSL https://raw.githubusercontent.com/Performant-Labs/holler/main/install.sh | sh
```

会把适配你平台的最新[发行版](https://github.com/Performant-Labs/holler/releases)二进制文件
（macOS/Apple Silicon、Linux/x86_64，或 Linux/arm64 —— Windows 目前不是受支持的目标平台，
参见 [#378](https://github.com/Performant-Labs/holler/issues/378)）下载到
`~/.local/bin/holler`。可以用 `HOLLER_VERSION=v0.3.0` 固定某个特定版本，或用
`HOLLER_INSTALL_DIR=/usr/local/bin` 改变安装目录（在上面的命令前加上这个环境变量即可）。

**从源码构建：** `cargo build --release -p holler-cli`，二进制文件位于 `target/release/holler`。

</details>

## 快速开始

一个二进制文件，两种角色。在应当可被访问到的那台机器上（即 **hub**）：

```bash
holler hub serve --listen 127.0.0.1:41807 --advertise <this-machine's-address>
holler hub token mint --label my-first-body
```

`token mint` 会打印出一条现成可用的 `body join` 命令 —— 在真正运行编程智能体的那台机器上
（即 **body**）执行它，然后启动它：

```bash
holler body join --server wss://<hub-address> --token <token> --hub-key <hub-key>
holler body run --config sessions.toml
```

回到 hub 这一侧，与某个会话对话：

```bash
holler roster                          # 查看当前已连接的内容
holler say <session-name> "hello"      # 一次性发送提示词，并打印回复
```

`sessions.toml` 的具体结构 —— 每个会话运行哪种 harness、是 spawn 模式还是 attach 模式 —— 属于配置，而非代码
（详见英文版的 [Harness recipes](README.md#harness-recipes) 以及
[ADR 0012](docs/adr/ADR-0012.md)）。如果你想在同一个终端工作区中，用一个本地编排器（orchestrator）
搭配一个或多个远程 attach 模式会话，请参考英文版的
[Set up a Herdr workspace with an agent](README.md#set-up-a-herdr-workspace-with-an-agent)。

---

想了解完整内容 —— Documentation、Where Holler fits、Attach convenience、Harness recipes、
Debug output、Contributing、License —— 请查阅英文版 [README.md](README.md)。
