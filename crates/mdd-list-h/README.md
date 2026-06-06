# mdd-list-h

横型カードリスト図プラグイン。

カードを横に並べて表示するSVGを生成します。各カードにはラベルと任意の説明文を設定できます。

## 入力フォーマット

```
title "タイトル"
card "ラベル" : "説明"
card "ラベル2" : "説明2"
card "ラベル3"
```

## 使い方

```sh
cat input.list-h | mdd-list-h > output.svg
```
