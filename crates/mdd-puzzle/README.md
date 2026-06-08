# mdd-puzzle

パズル・ハニカム構造図プラグイン

## 概要

`mdd-puzzle` は、相互に接続されたピースが全体を形成するパズル・ハニカム構造図を描画するプラグインです。各ピースは正六角形で表現され、ハニカム（蜂の巣）パターンで配置されます。

## 入力形式

```
ラベル1
ラベル2
ラベル3
ラベル4
```

- 各行でハニカムの各ピースを定義します。
- 最低2つのピースが必要です。

## 使い方

```sh
cat input.puzzle | mdd-puzzle > output.svg
```

## 例

```sh
cat examples/team.puzzle | cargo run -p mdd-puzzle > examples/team.svg
cat examples/simple.puzzle | cargo run -p mdd-puzzle > examples/simple.svg
```
