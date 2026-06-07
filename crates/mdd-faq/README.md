# mdd-faq

FAQ プラグイン。Q&A 形式のよくある質問を可視化する。

## 使い方

```
cat input.faq | mdd-faq > output.svg
```

## 入力形式

```
title "よくある質問"
q "質問テキスト"
a "回答テキスト"

q "別の質問"
a "複数行の回答
2行目"
```

## サンプル

![product](examples/product.svg)
