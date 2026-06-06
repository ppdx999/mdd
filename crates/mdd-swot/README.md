# mdd-swot

SWOT分析図プラグイン。

Strengths（強み）、Weaknesses（弱み）、Opportunities（機会）、Threats（脅威）を2x2グリッドで可視化します。

## 使い方

```
cat input.swot | mdd-swot > output.svg
```

## 入力形式

```
title "タイトル"
strengths {
  項目1
  項目2
}
weaknesses {
  項目1
}
opportunities {
  項目1
}
threats {
  項目1
}
```

各セクション（strengths, weaknesses, opportunities, threats）は省略可能ですが、少なくとも1つのセクションに項目が必要です。
