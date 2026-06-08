# mdd-list-grid

グリッドレイアウトリスト図プラグイン

## 概要

アイテムをグリッドレイアウトで表示するプラグインです。カード形式で項目を整理し、列数を指定してレイアウトを調整できます。

## 入力形式

```
columns 3
item "ラベル" { 説明 }
item "ラベル2"
```

- `columns` - 列数（省略時は3）
- `item` - アイテム。`{ 説明 }` で説明文を追加可能

## 使用例

```bash
cat input.list-grid | mdd-list-grid > output.svg
```
