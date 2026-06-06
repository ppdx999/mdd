# mdd-funnel

ファネル図プラグイン。営業ファネルやコンバージョンファネルなど、段階的に絞り込まれるプロセスを視覚化します。

## 使い方

```
cat input.funnel | mdd-funnel > output.svg
```

## 入力形式

```
title "タイトル"
stage ラベル : 値
stage ラベル2 : 値2
```

値を省略すると、各段階が固定比率で順に狭くなります。

```
stage 認知
stage 興味
stage 検討
stage 購入
```
