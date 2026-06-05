# mdd-sequence

`mdd` 用のシーケンス図プラグイン。テキストベースの記法から SVG のシーケンス図を生成する。

## 使い方

標準入力からシーケンス記法を受け取り、標準出力に SVG を出力する。

```sh
mdd-sequence < examples/simple.sequence > output.svg
```

`mdd` 経由で使う場合は、Markdown のコードブロックに `sequence` を指定する。

````md
```sequence
Alice -> Bob : "Hello"
Bob --> Alice : "Hi there"
```
````

## 記法

### participant

参加者を明示的に定義する。省略した場合はメッセージで初出時に自動追加される。

```
participant Client
participant Server
```

### message

参加者間のメッセージを定義する。

```
Client -> Server : "リクエスト"
Server --> Client : "レスポンス"
```

| 記法 | 意味 | 描画 |
|---|---|---|
| `->` | 同期メッセージ | 実線 + 塗りつぶし矢印 |
| `-->` | 応答/非同期メッセージ | 破線 + 開き矢印 |

### 自己メッセージ

from と to が同じ参加者の場合、右にループする矢印で描画される。

```
Server -> Server : "内部処理"
```

### ラベル

`: "ラベル"` はオプション。省略するとラベルなしの矢印になる。

## サンプル

```sh
# シンプルな図
mdd-sequence < examples/simple.sequence > simple.svg

# 認証フロー
mdd-sequence < examples/auth.sequence > auth.svg

# 複雑なフロー
mdd-sequence < examples/complex.sequence > complex.svg
```
