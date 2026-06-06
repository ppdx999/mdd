# mdd-list-v

縦型リスト図プラグイン。

番号付きバッジとラベル・説明文を縦に並べたリスト図をSVGとして生成します。

## 入力形式

```
title "タイトル"
item "ラベル" : "説明文"
item "ラベル2"
```

- `title` は省略可能です。
- `item` は1つ以上必要です。
- `:` の後の説明文は省略可能です。

## 使い方

```sh
cat input.list-v | mdd-list-v > output.svg
```
