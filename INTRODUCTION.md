# 図の生成はすごく便利

認知負荷を下げるために、テキストだけでは伝わりにくい情報を図にすると便利。

```persona
actor エンジニア : "設計書にアーキテクチャ図を入れたい"
actor PM : "提案書にフロー図で流れを見せたい"
actor デザイナー : "仕様書に画面遷移図を付けたい"
```

# しかし、AIで図の生成は課題が多い


```persona
actor 悩み1 : "AIが思ったとおりに図を作ってくれない"
actor 悩み2 : "AIが1つの図を作るだけで数分かかる"
actor 悩み3 : "AIが作った図はバージョン管理が難しい"
```


# そもそも


```faq
q "AIに作図させるのはなぜこんなにも難しいのか？"
a "AIが情報を扱う上で、GUIはノイズでしかないから。
図はAIにとって不要な情報の塊でしかない"
q "ではどうすればAIに図をうまく扱えるか？"
a "AIにはデータ構造だけを管理させて
人間のためにいい感じに図にするツール(= プログラム)を作ったらいい"
```

# ということで作りました。


```github
repo ppdx999/mdd
desc "Markdown with Diagrams — テキストから図を生成する軽量プリプロセッサ"
lang Rust
license MIT
```

# MDD とは？

テキストからいい感じのSVGを生成するプログラム

## 例)

```code
lang usecase
---
actor 顧客
actor 管理者

package "認証" {
  usecase ログイン
  usecase ログアウト
}

顧客 -> ログイン
管理者 -> ログイン
管理者 -> ログアウト
```

```arrow
direction down
label "これをmddが変換すると..."
```

```usecase
actor 顧客
actor 管理者

package "認証" {
  usecase ログイン
  usecase ログアウト
}

顧客 -> ログイン
管理者 -> ログイン
管理者 -> ログアウト
```

# MDD の仕組み

```process
step テキスト(DSL) : "ただのテキスト。
AIが書いても人間が書いてもOK"
step MDD : "AIは不要。
これはただのプログラム。"
step SVG : "ブラウザでもPDFでも
簡単に誰でも見れる"
```

## 余談

このテキストから図を作る技術は1981年に出させた論文の技術が元になっています。詳しく知りたい人はsugiyama法で検索！

# この仕組みがなぜ良いのか？


```list-v
item "AIはデータの構造だけに集中できるので爆速"
item "レンダリングはただのプログラムがやるので爆速"
item "全ての元データがただのテキストになる"
```


# これの何が嬉しい？


```before-after
before "既存ツール" {
  AIが図を生成するのにすごく時間がかかる
  AIが思った通りの図を作ってくれない
  Gitなどでバージョン管理ができない
}

after "MDD" {
  AIが扱うのはただのテキスト(DSL)なので爆速
  図を作るのはただのプログラムなので思い通りの図を作ってくれる
  ただのテキストなのでGitでバージョン管理ができる！
}
```

# どんな図が作れる？ — フロー系

```flowchart
start 開始
process 要件定義
process 設計
process 実装
process テスト
end リリース

開始 -> 要件定義
要件定義 -> 設計
設計 -> 実装
実装 -> テスト
テスト -> リリース
```

# どんな図が作れる？ — 組織図

```org
title "エンジニアリング組織"
member CTO : "技術統括"
member VP_Eng : "開発部長"
member VP_Product : "プロダクト部長"
member FE_Lead : "FEリード"
member BE_Lead : "BEリード"
CTO -> VP_Eng
CTO -> VP_Product
VP_Eng -> FE_Lead
VP_Eng -> BE_Lead
```

# どんな図が作れる？ — 分析系

```pie
title "売上構成"
slice SaaS : 45
slice コンサル : 30
slice ライセンス : 15
slice その他 : 10
```

# どんな図が作れる？ — レーダーチャート

```radar
title "技術スタック評価"
axis 速度
axis 安全性
axis 学習コスト
axis エコシステム
axis 保守性
data "Rust" : 95, 95, 40, 60, 85
data "Go" : 85, 70, 80, 75, 75
```

# どんな図が作れる？ — カンバン

```kanban
title "Sprint Board"
column Backlog
card 認証機能 : "feature"
card 検索改善 : "feature"

column In Progress
card API設計 : "task"

column Review
card ログイン画面 : "feature"

column Done
card DB設計 : "task"
card CI/CD構築 : "infra"
```

# どんな図が作れる？ — まだまだ

```kpi
title "53種類のプラグイン"
metric "フロー系" : "8種類"
metric "構造・階層系" : "7種類"
metric "比較・分析系" : "10種類"
metric "関係・概念系" : "9種類"
metric "レイアウト系" : "9種類"
metric "特殊系" : "10種類"
```

# ユーザーの声

```quote
quote "AIにDSLを書かせるだけで図が出る。
ドキュメント作成が劇的に速くなった。"
author "エンジニア"
role "スタートアップCTO"

quote "Gitで差分レビューできるのが最高。
図の変更理由がコミットログに残る。"
author "テックリード"
role "大手SaaS企業"
```

# このスライドの正体

このスライド自体が、ただの Markdown テキストから生成されています。

```dirtree
INTRODUCTION.md : "このファイル（テキスト）"
  mdd slide/ : "変換コマンド"
    output.pdf : "このスライド（PDF）"
```

# このスライドの作り方

```process
step Markdownを書く : "テキストエディタで
DSLを記述"
step mdd slide : "コマンド一発で
PDF生成"
step 共有 : "GitにPushして
チームに共有"
```

# まとめ

```list-v
title "MDD まとめ"
item "テキストから図を生成" : "Markdownのコードブロックに書くだけ"
item "1秒以下で変換" : "AIワークフローに最適"
item "53種類のプラグイン" : "フロー、組織図、チャート、何でも"
item "Git管理可能" : "テキストだから差分・レビュー・履歴管理"
item "100%カスタマイズ" : "プラグインは自作可能"
item "スライドもMarkdownから" : "mdd slide で PDF エクスポート"
```

# Try MDD

```math
curl -fsSL https://raw.githubusercontent.com/ppdx999/mdd/main/install.sh | sh
```
