# mdd-state

`mdd` 用の状態遷移図プラグイン。テキストベースの記法から SVG の状態遷移図を生成する。

## 使い方

標準入力から状態遷移記法を受け取り、標準出力に SVG を出力する。

```sh
mdd-state < examples/simple.state > output.svg
```

`mdd` 経由で使う場合は、Markdown のコードブロックに `state` を指定する。

````md
```state
state 待機中
state 処理中
state 完了

待機中 -> 処理中 : "開始"
処理中 -> 完了 : "成功"
```
````

## 記法

### state

状態ノード（角丸矩形）を定義する。

```
state 待機中
```

### transition

状態間の遷移（有向エッジ）を `->` で定義する。`: "ラベル"` でラベルを付けられる。

```
待機中 -> 処理中 : "開始"
処理中 -> 完了
```

### 自己遷移

from と to が同じ状態の場合、右側にループ曲線で描画される。

```
処理中 -> 処理中 : "リトライ"
```

## サンプル

```sh
# シンプルな状態遷移
mdd-state < examples/simple.state > simple.svg

# 注文ステータス
mdd-state < examples/order.state > order.svg

# 認証フロー
mdd-state < examples/complex.state > complex.svg
```
