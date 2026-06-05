# mdd-er

`mdd` 用の ER 図（Entity-Relationship Diagram）プラグイン。テキストベースの記法から SVG の ER 図を生成する。

## 使い方

標準入力から ER 記法を受け取り、標準出力に SVG を出力する。

```sh
mdd-er < examples/simple.er > output.svg
```

`mdd` 経由で使う場合は、Markdown のコードブロックに `er` を指定する。

````md
```er
table Users {
  * id
  name
  email
}

table Posts {
  * id
  user_id
  title
}

Users 1--* Posts
```
````

## 記法

### table

テーブルを定義する。`*` プレフィックスで主キー（PK）を示す。

```
table Users {
  * id
  name
  email
  created_at
}
```

### relation

テーブル間のリレーションをカーディナリティ付きで定義する。

```
Users 1--* Orders
Orders 1--* OrderItems
Products 1--* OrderItems
Categories 1--1 Settings
Tags *--* Posts
```

| 記法 | 意味 |
|---|---|
| `1--1` | 一対一 |
| `1--*` | 一対多 |
| `*--1` | 多対一 |
| `*--*` | 多対多 |

## 描画

| 要素 | 形状 |
|---|---|
| table ヘッダー | 濃い青背景 + 白テキスト |
| table ボディ | 白背景 + 列名リスト（PK は鍵アイコン + 太字） |
| relation | 線 + 端にカーディナリティ記号（`1` or `*`） |

列が多い場合は自動的に多段カラム表示になる。エッジはノード回避ルーティングとベジェ曲線で描画。

## サンプル

```sh
# シンプルな図
mdd-er < examples/simple.er > simple.svg

# EC サイトのスキーマ
mdd-er < examples/ecommerce.er > ecommerce.svg
```
