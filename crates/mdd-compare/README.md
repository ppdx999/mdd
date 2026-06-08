# mdd-compare

比較図プラグイン。複数の選択肢を横並びで比較するSVG図を生成します。

## 使い方

```
cat input.compare | mdd-compare > output.svg
```

## 入力形式

```
option "オプションA" {
  項目1
  項目2
}
option "オプションB" {
  項目1
  項目2
}
```

- `option` は2つ以上3つまで指定できます
