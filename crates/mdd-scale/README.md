# mdd-scale

規模比較図プラグイン。

数値データを横棒グラフとして描画し、項目間の相対的な規模を視覚的に比較できます。

## 入力形式

```
title "タイトル"
unit "単位"
item ラベル : 値
item ラベル : 値
```

- `title` — 図のタイトル（省略可）
- `unit` — 値の単位（省略可）
- `item` — 比較する項目（最低2つ必要）

## 使い方

```sh
cat input.scale | mdd-scale > output.svg
```

## 例

```sh
cat examples/population.scale | cargo run -p mdd-scale > examples/population.svg
cat examples/storage.scale | cargo run -p mdd-scale > examples/storage.svg
```
