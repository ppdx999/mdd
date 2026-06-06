# mdd-pyramid

ピラミッド図プラグイン

ピラミッド/階層図（マズローの欲求階層、戦略ピラミッドなど）をSVGとしてレンダリングします。

## 使い方

```
cat input.pyramid | mdd-pyramid > output.svg
```

## 入力形式

```
title "タイトル"
level 頂点レベル
level 第二レベル
level 第三レベル
level 基盤レベル
```

説明付き:

```
level 戦略 : "長期的な方向性"
level 戦術 : "四半期の施策"
level 実行 : "日々のオペレーション"
```
