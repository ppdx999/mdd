# mdd-radial

放射図（ハブ&スポーク）プラグイン。中心となるコンセプトと、それを取り囲む関連要素を放射状に配置した図を生成します。

## 入力形式

```
title "オプションのタイトル"
center "中心コンセプト"
spoke 要素1
spoke 要素2
spoke 要素3
```

## 使い方

```sh
cat input.radial | mdd-radial > output.svg
```
