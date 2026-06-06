# mdd-matrix

`mdd` 用のマトリクス図プラグイン。テキストベースの記法から色付きマトリクス表を SVG で生成する。

## 使い方

標準入力からマトリクス記法を受け取り、標準出力に SVG を出力する。

```sh
mdd-matrix < examples/raci.matrix > output.svg
```

`mdd` 経由で使う場合は、Markdown のコードブロックに `matrix` を指定する。

````md
```matrix
columns 設計, 実装, テスト

田中 : R, A, C
佐藤 : A, R, R
```
````

## 記法

### columns

列ヘッダーをカンマ区切りで定義する。

```
columns 要件定義, 基本設計, 実装, テスト
```

### row

行ラベルとセル値をカンマ区切りで定義する。

```
PM : R, A, I, I
エンジニア : C, R, R, A
```

### セル値の色

| 値 | 色 | RACI での意味 |
|---|---|---|
| R | 青 | Responsible（実行責任） |
| A | 赤 | Accountable（説明責任） |
| C | 黄 | Consulted（協業） |
| I | 緑 | Informed（報告） |
| ○ | 青 | あり / 対応 |
| ◎ | ティール | 主担当 |
| △ | 黄 | 一部 / 要相談 |
| × / - | グレー | なし / 対象外 |

## サンプル

### RACI マトリクス

![raci](examples/raci.svg)

### 機能×チーム対応表

![feature-team](examples/feature-team.svg)

### 権限マトリクス

![permissions](examples/permissions.svg)
