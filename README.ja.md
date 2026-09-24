<div align="center">

# holler

[![CI](https://img.shields.io/github/actions/workflow/status/Performant-Labs/holler/ci.yml?branch=main&label=CI)](https://github.com/Performant-Labs/holler/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/Performant-Labs/holler)](https://github.com/Performant-Labs/holler/releases)
[![License](https://img.shields.io/github/license/Performant-Labs/holler)](./LICENSE)

**「holler」と呼びかければ、あなたのエージェントはすぐそこに —— 1 つのバイナリが hub にも body にもなる。**

[English](README.md) | [中文](README.zh-CN.md) | **日本語** | [Español](README.es.md) | [Deutsch](README.de.md) | [Français](README.fr.md)

</div>

Holler は、あなた自身が所有するマシン上で動くインタラクティブなコーディングセッションのための、
セルフホスト型・アウトバウンド専用の回線です。監督役になれる `hub` と、実際のコーディングエージェントと
並んで動く `body` を、同じ 1 つのバイナリ（役割違いで実行）がつなぎます。

> **このドキュメントは「入り口」部分（Why Holler？、Install、Quick Start）のみを翻訳したものです。**
> Harness recipes、Debug output、Contributing などの完全な技術リファレンスは現時点では英語版のみです。
> 翻訳の陳腐化によりコマンドやフラグの記載が不正確になることを避けるため、詳細は
> [README.md](README.md) を参照してください。

## Holler を選ぶ理由

- **セルフホストであり、ベンダーの中継ではない** —— hub もあなたが動かし、body もあなたが動かします。
  セッションを中継するクラウドの仲介者は存在しません。
- **アウトバウンド専用** —— body が hub に対して発信します。hub 側は body のネットワークから到達可能な
  インバウンドリスナーを一切必要としません。NAT やファイアウォールの内側でも構造上問題なく動作します。
- **マシンごとに発行され、失効可能な ID** —— 参加トークン（join token）がバインドされた資格情報になります
  （[ADR 0007](docs/adr/ADR-0007.md)）。共有シークレットはなく、「tailnet の IP アドレスだから本人だ」とは
  見なしません（tailnet や VPN はあくまで下層のネットワークであり、決して ID そのものではありません ——
  [ADR 0006](docs/adr/ADR-0006.md)）。
- **必要であれば監督できる hub** —— 監査ログや、必要な場面ではターン数・コストの上限を設定できますが、
  すべてのデプロイでそれを強制するわけではありません。
- **新しい harness の追加はコードではなく設定で** —— `[[session]]` の 1 行を新しい ACP 対応アダプタに
  向けるだけで済み、Holler 側のコード変更もリリースも不要です（[ADR 0012](docs/adr/ADR-0012.md)）。
- **ゼロから作ったプロトコルではなく、既存要素の組み合わせ** —— Holler はエージェント間通信や
  割り込みのセマンティクスを新たに発明しているわけではありません。body↔harness 間には ACP v2 を採用し、
  エージェント間（agent-to-agent）レイヤーにはあえて踏み込みません。詳しくは英語版の
  [Where Holler fits](README.md#where-holler-fits) を参照してください。

Holler は現在も活発に開発が進んでいます —— hub/body のコアとなる回線、attach モード、CLI インターフェースは
実際に動作し、リリース済みです（下記のインストール手順を参照）。カバレッジと負荷テストを追跡する
[テスト epic（#366）](https://github.com/Performant-Labs/holler/issues/366) は現在もオープンです。

## インストール

**Homebrew**（macOS/Apple Silicon、Linux/x86_64、または
[Homebrew on Linux](https://docs.brew.sh/Homebrew-on-Linux) 経由の Linux/arm64）：

```bash
brew tap Performant-Labs/tap
brew install holler
```

[Performant-Labs/homebrew-tap](https://github.com/Performant-Labs/homebrew-tap) —— 自前でホストしている
tap で、`homebrew-core` にはまだ入っていません。この tap の formula が更新されるたびに、
`brew upgrade holler` で新しいリリースを取得できます。

<details>
<summary>ワンライナーインストーラ、またはソースからビルド</summary>

**ワンライナーインストーラ**（Homebrew を使わない場合）：

```bash
curl -fsSL https://raw.githubusercontent.com/Performant-Labs/holler/main/install.sh | sh
```

お使いのプラットフォーム（macOS/Apple Silicon、Linux/x86_64、または Linux/arm64 —— Windows は
サポート対象外です。[#378](https://github.com/Performant-Labs/holler/issues/378) を参照）向けの
最新の[リリース](https://github.com/Performant-Labs/holler/releases)バイナリを `~/.local/bin/holler`
にダウンロードします。`HOLLER_VERSION=v0.3.0` で特定バージョンを固定したり、
`HOLLER_INSTALL_DIR=/usr/local/bin` でインストール先を変更したりできます（上記コマンドの前に
環境変数として指定してください）。

**ソースからビルドする場合：** `cargo build --release -p holler-cli` を実行し、バイナリは
`target/release/holler` に生成されます。

</details>

## クイックスタート

1 つのバイナリ、2 つの役割。まず、到達可能である必要があるマシン（**hub**）側で：

```bash
holler hub serve --listen 127.0.0.1:41807 --advertise <this-machine's-address>
holler hub token mint --label my-first-body
```

`token mint` は、そのまま実行できる `body join` コマンドを出力します —— これを、実際に
コーディングエージェントが動くマシン（**body**）側で実行し、続けて起動します：

```bash
holler body join --server wss://<hub-address> --token <token> --hub-key <hub-key>
holler body run --config sessions.toml
```

hub 側に戻り、セッションと対話します：

```bash
holler roster                          # 現在接続されているものを確認
holler say <session-name> "hello"      # ワンショットでプロンプトを送り、返答を表示
```

`sessions.toml` の具体的な形 —— どのセッションがどの harness で動くか、spawn モードか
attach モードか —— はコードではなく設定です（詳しくは英語版の
[Harness recipes](README.md#harness-recipes) と [ADR 0012](docs/adr/ADR-0012.md) を参照）。
1 つのローカルオーケストレーターと 1 つ以上のリモート attach モードセッションを、1 つの
ターミナルワークスペースにまとめたい場合は、英語版の
[Set up a Herdr workspace with an agent](README.md#set-up-a-herdr-workspace-with-an-agent)
を参照してください。

---

Documentation、Where Holler fits、Attach convenience、Harness recipes、Debug output、
Contributing、License などの完全な内容については、英語版の [README.md](README.md) を参照してください。
