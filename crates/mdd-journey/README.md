# mdd-journey

ユーザージャーニーマップ。ステージ・行動・感情を時系列で可視化する。

## 使い方

```
cat input.journey | mdd-journey > output.svg
```

## 入力形式

```
title "購入体験"
persona "30代会社員"
stage 認知 : "広告を見る" : 3
stage 検索 : "商品を検索" : 4
stage 購入 : "決済する" : 2
```

感情は 1（不満）〜 5（満足）。

## サンプル

![shopping](examples/shopping.svg)
