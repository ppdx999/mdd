# mdd-concept

相関図・概念図プラグイン

概念やエンティティ間の関係を可視化する相関図を生成します。ノード間のラベル付き接続を描画し、有向矢印（`->`）と無向線（`--`）をサポートします。

## 入力形式

```
node ノード名A
node ノード名B
link ノード名A -> ノード名B : "ラベル"
link ノード名A -- ノード名B : "ラベル"
```

## 使い方

```bash
cat input.concept | mdd-concept > output.svg
```
