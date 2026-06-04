# MDD

MDD (Markdown with Diagrams) は軽量な Markdown プリプロセッサ。

Markdown のコードブロックをスキャンし、外部プラグインを呼び出して、ブロックを生成された SVG 画像に置換する。

プラグインは `$PATH` から発見される単純な実行可能コマンド。

コードブロック:

````markdown
```sequence
Alice -> Bob: Hello
```
````

に対して MDD は以下を実行する:

```bash
mdd-sequence
```

ブロックの内容は標準入力で渡され、プラグインは標準出力で SVG を返す。

```text
Markdown
    ↓
コードブロック
    ↓
mdd-{ブロック名}
    ↓
SVG
    ↓
Markdown
```

MDD 本体が担うのは以下のみ:

* Markdown のパース
* プラグインの発見
* プラグインの実行
* Markdown の生成

図の描画ロジックはすべてプラグイン側に属する。

## インストール

Rust ツールチェインが必要。

```sh
git clone https://github.com/ppdx999/mdd.git
cd mdd
make install
```

`~/.cargo/bin/` に `mdd` と全プラグインがインストールされる。

アンインストール:

```sh
make uninstall
```

## プラグイン

| プラグイン | 説明 |
|---|---|
| [mdd-usecase](crates/mdd-usecase/) | ユースケース図 |
| [mdd-dfd](crates/mdd-dfd/) | DFD（データフロー図） |

## AGENTS.md 向けサンプル

AI エージェントにドキュメント内で図を生成させる際、`AGENTS.md` に以下のような記述を追加すると効果的。

````markdown
## 図の生成

このプロジェクトでは [mdd](https://github.com/ppdx999/mdd) を使って Markdown 内に図を埋め込む。
コードブロックの言語名に応じたプラグインが SVG を生成する。

### ユースケース図

```usecase
actor Customer
actor Admin

package "認証" {
  usecase Login
  usecase Logout
}

Customer -> Login
Admin -> Login
Admin -> Logout
```

### DFD（データフロー図）

```dfd
entity Customer
entity PaymentGateway

process HandleOrder
process ValidatePayment

datastore Orders {
  注文ID
  顧客ID
  合計金額
  ステータス
}

Customer -> HandleOrder : "注文情報"
HandleOrder -> Orders : "注文データ"
HandleOrder -> ValidatePayment : "支払い依頼"
ValidatePayment -> PaymentGateway : "決済リクエスト"
```
````
