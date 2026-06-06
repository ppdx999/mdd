# mdd-group

グループ図プラグイン

## 概要

グループ図を生成するための `mdd` プラグインです。2〜4つのグループとその要素をシンプルに可視化します。

## 入力形式

```
title "タイトル"
group "グループ名" {
  要素1
  要素2
  要素3
}
```

## 使い方

```sh
cat input.group | mdd-group > output.svg
```
