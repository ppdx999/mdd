# mdd-pricetable

料金表プラグイン。プラン比較を表形式で可視化する。

## 使い方

```
cat input.pricetable | mdd-pricetable > output.svg
```

## 入力形式

```
title "料金プラン"
plan Free : "¥0/月"
- 基本機能
- メールサポート

plan* Pro : "¥2,980/月"
- 全機能
- チャットサポート
```

`plan*` はハイライト表示（おすすめプラン）。

## サンプル

![saas](examples/saas.svg)
