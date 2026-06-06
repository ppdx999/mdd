# mdd-grid

`mdd` 用のグリッド図プラグイン。テキストベースの記法から色付きグリッド表を SVG で生成する。

## 使い方

標準入力からグリッド記法を受け取り、標準出力に SVG を出力する。

```sh
mdd-grid < examples/raci.grid > output.svg
```

`mdd` 経由で使う場合は、Markdown のコードブロックに `grid` を指定する。

````md
```grid
columns 設計, 実装, テスト

color R : blue, #e3f2fd
color A : red, #ffebee

田中 : R, A, R
佐藤 : A, R, R
```
````

## 記法

### columns

列ヘッダーをカンマ区切りで定義する。

```
columns 要件定義, 基本設計, 実装, テスト
```

### color

セル値に対する文字色（と任意で背景色）を定義する。全ての色は DSL 上で宣言的に指定する。

```
color R : blue
color A : red
color C : amber
color I : green
```

背景色も指定する場合:

```
color R : blue, #e3f2fd
color A : red, #ffebee
```

使える色名: `red`, `blue`, `green`, `amber`, `yellow`, `orange`, `teal`, `purple`, `pink`, `grey`, `lightgrey`, `black`。`#ff0000` のような HEX コードも直接指定可能。

### row

行ラベルとセル値をカンマ区切りで定義する。

```
PM : R, A, I, I
エンジニア : C, R, R, A
```

## サンプル

### RACI マトリクス

![raci](examples/raci.svg)

### 機能×チーム対応表

![feature-team](examples/feature-team.svg)

### 権限マトリクス

![permissions](examples/permissions.svg)
