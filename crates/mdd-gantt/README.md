# mdd-gantt

`mdd` 用のガントチャートプラグイン。テキストベースの記法から SVG のガントチャートを生成する。

## 使い方

標準入力からガントチャート記法を受け取り、標準出力に SVG を出力する。

```sh
mdd-gantt < examples/simple.gantt > output.svg
```

`mdd` 経由で使う場合は、Markdown のコードブロックに `gantt` を指定する。

````md
```gantt
title スプリント1
unit day

タスクA : 2025-01-06, 3d
タスクB : 2025-01-06, 5d
タスクC : after タスクA, 4d
```
````

## 記法

### title

チャートのタイトルを定義する。

```
title プロジェクトX
```

### unit

時間軸の単位を指定する。`day`、`week`、`month` のいずれか。

```
unit week
```

### section

タスクをセクションでグループ化する。

```
section 設計
  要件定義 : 2025-01-06, 2w
  基本設計 : after 要件定義, 1w
```

### task

タスクを定義する。開始日と期間を指定する。

```
タスク名 : 2025-01-06, 3d
タスク名 : 2025-01-06, 2w
```

期間の単位は `d`(日)、`w`(週)。

### 依存関係

`after <タスク名>` で先行タスクの終了後に開始する。

```
結合テスト : after フロント実装, 2w
```

## サンプル

```sh
# シンプルなガントチャート
mdd-gantt < examples/simple.gantt > simple.svg

# プロジェクト計画（セクション付き）
mdd-gantt < examples/project.gantt > project.svg

# 大規模プロジェクト
mdd-gantt < examples/large.gantt > large.svg
```
