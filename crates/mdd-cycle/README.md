# mdd-cycle

`mdd` 用のサイクル図プラグイン。テキストベースの記法から SVG のサイクル図を生成する。

## 使い方

```bash
# 直接実行
cat input.cycle | mdd-cycle > output.svg

# mdd 経由
mdd input.md > output.md
```

## 記法

### タイトル（オプション）

```
title PDCA
```

中央にタイトルを表示する。

### ステップ定義

```
step 計画
step 実行
step 評価
step 改善
```

定義された順番に円形に配置され、最後のステップから最初のステップへ矢印が戻る。最低2つのステップが必要。

## 描画

| 要素 | 形状 | 背景色 | テキスト色 |
|---|---|---|---|
| ステップ | 角丸矩形 | ステップごとに異なるパステルカラー | `#333` |
| タイトル | テキスト（中央） | — | `#333` |
| 矢印 | 曲線矢印 | — | `#666` |

## サンプル

### PDCA サイクル

![pdca](examples/pdca.svg)

### DevOps サイクル

![devops](examples/devops.svg)

### Scrum サイクル

![scrum](examples/scrum.svg)
