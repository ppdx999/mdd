# mdd-infra

`mdd` 用のインフラ構成図プラグイン。テキストベースの記法から SVG のインフラ構成図を生成する。

## 使い方

標準入力からインフラ記法を受け取り、標準出力に SVG を出力する。

```sh
mdd-infra < examples/simple.infra > output.svg
```

`mdd` 経由で使う場合は、Markdown のコードブロックに `infra` を指定する。

````md
```infra
node WebServer type=server
node Database type=db
WebServer -> Database : "SQL"
```
````

## 記法

### node

コンポーネントを定義する。`type=` で種別を指定するとアイコンと色が変わる。

```
node App type=server
node RDS type=db
node ALB type=lb
```

| type | 説明 | アイコン |
|---|---|---|
| server | サーバー | ラック |
| db / database | データベース | シリンダー |
| lb / loadbalancer | ロードバランサー | 分岐 |
| cache | キャッシュ | 稲妻 |
| queue | メッセージキュー | 矢印 |
| storage | オブジェクトストレージ | バケツ |
| cdn | CDN | 雲 |
| network / vpc / subnet | ネットワーク | 六角形 |
| user / client | ユーザー | 棒人間 |
| (省略 or 未知) | 汎用 | 矩形 |

### group

コンポーネントをグループ化する。ネスト可能。

```
group "AWS" {
  group "VPC" {
    node App type=server
    node DB type=db
  }
  node S3 type=storage
}
```

### edge

コンポーネント間の接続を定義する。ラベルはオプション。

```
App -> DB : "SQL"
ALB -> App
```

## サンプル

```sh
# シンプルな3層構成
mdd-infra < examples/simple.infra > simple.svg

# AWS 構成
mdd-infra < examples/aws.infra > aws.svg

# マイクロサービス構成
mdd-infra < examples/microservices.infra > microservices.svg
```
