# mdd-arrow

矢印プラグイン。ラベル付きの方向矢印を SVG で描画する。

## 使い方

```
cat input.arrow | mdd-arrow > output.svg
```

## 入力形式

```
direction down
label "mdd-usecase"
```

`direction` は `down`（デフォルト）、`up`、`right`、`left`。`label` は省略可能。

## サンプル

![down](examples/down.svg)
