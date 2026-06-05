# mdd-tree

`mdd` 用のツリー図プラグイン。テキストベースの記法からトップダウンの階層構造図を SVG で生成する。

## 使い方

標準入力からツリー記法を受け取り、標準出力に SVG を出力する。

```sh
mdd-tree < examples/simple.tree > output.svg
```

`mdd` 経由で使う場合は、Markdown のコードブロックに `tree` を指定する。

````md
```tree
node CEO
node CTO
node CFO

CEO -> CTO
CEO -> CFO
```
````

## 記法

### node

単独ノード（角丸矩形）を定義する。

```
node CEO
```

### group

ノードをグループ化する。group 自体もエッジの端点になれる。

```
group "営業部" {
  node 部長
  node 社員A
  node 社員B
}
```

### edge

要素間の接続（無向、矢印なし）を `->` で定義する。トップダウン方向にレイアウトされる。

```
CEO -> "営業部"
CEO -> "開発部"
```

group 名にスペースが含まれる場合はエッジ側でも `"` で囲む。

## 描画

| 要素 | 形状 | 色 |
|---|---|---|
| node | 角丸矩形 | 薄い青 |
| group | 破線矩形 + ヘッダー + 子ノード | 薄いグレー |
| edge | 直線（矢印なし） | グレー |

## サンプル

```sh
# シンプルなツリー
mdd-tree < examples/simple.tree > simple.svg

# 組織図
mdd-tree < examples/org.tree > org.svg

# ディレクトリ構造
mdd-tree < examples/directory.tree > directory.svg
```
