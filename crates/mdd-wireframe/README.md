# mdd-wireframe

ワイヤーフレームプラグイン。シンプルなUI モックアップを生成する。

## 使い方

```
cat input.wireframe | mdd-wireframe > output.svg
```

## 入力形式

```
title "ページ名"
header 見出し
text "説明文"
input "プレースホルダー"
button ボタン名
image "画像の説明"
---
- リスト項目1
- リスト項目2
```

## サンプル

![login](examples/login.svg)
