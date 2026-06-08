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

# どんな図が作れる？ — プロジェクト計画

```gantt
title 基幹システム刷新PJ
unit day

要件定義 : 2025-04-01, 15d
基本設計 : after 要件定義, 15d
開発 : after 基本設計, 30d
テスト : after 開発, 15d
```

# どんな図が作れる？ — ER図

```er
table 顧客マスタ {
  * 顧客ID
  顧客名
  メールアドレス
  電話番号
}

table 受注 {
  * 受注ID
  顧客ID
  受注日
  合計金額
  ステータス
}

table 受注明細 {
  * 明細ID
  受注ID
  商品ID
  数量
  単価
}

table 商品マスタ {
  * 商品ID
  商品名
  カテゴリ
  定価
}

顧客マスタ 1--* 受注
受注 1--* 受注明細
商品マスタ 1--* 受注明細
```

# どんな図が作れる？ — 業務フロー

```swimlane
lane 顧客
lane 営業部
lane 経理部
lane 物流部

顧客: start 注文
営業部: process 受注処理
営業部: decision 在庫確認
経理部: process 請求書発行
物流部: process 出荷手配
物流部: process 配送
顧客: end 受領

注文 -> 受注処理
受注処理 -> 在庫確認
在庫確認 -> 請求書発行 : "在庫あり"
請求書発行 -> 出荷手配
出荷手配 -> 配送
配送 -> 受領
```

# どんな図が作れる？ — 組織図

```org
title "プロジェクト体制図"
member PM : "プロジェクトマネージャー"
member AP_Lead : "APリーダー"
member Infra_Lead : "インフラリーダー"
member QA_Lead : "QAリーダー"
member AP1 : "AP開発メンバー"
member AP2 : "AP開発メンバー"
member Infra1 : "インフラメンバー"
member QA1 : "QAメンバー"
PM -> AP_Lead
PM -> Infra_Lead
PM -> QA_Lead
AP_Lead -> AP1
AP_Lead -> AP2
Infra_Lead -> Infra1
QA_Lead -> QA1
```

# どんな図が作れる？ — 技術比較

```compare
title "フレームワーク比較"
option "Spring Boot" {
  言語: Java
  実績: 豊富
  学習コスト: 中
  エコシステム: 大規模
  保守性: 高い
}
option "Ruby on Rails" {
  言語: Ruby
  実績: 豊富
  学習コスト: 低
  エコシステム: 中規模
  保守性: 中
}
option "Next.js" {
  言語: TypeScript
  実績: 増加中
  学習コスト: 中
  エコシステム: 大規模
  保守性: 高い
}
```

# どんな図が作れる？ — タイムライン

```timeline
title "基幹システム刷新ロードマップ"

2025-04 : 要件定義開始
2025-07 : 基本設計完了
2025-09 : 詳細設計完了
2026-01 : 開発完了
2026-03 : 結合テスト完了
2026-04 : ユーザー受入テスト
2026-06 : 本番移行
2026-07 : 旧システム停止
```

# どんな図が作れる？ — まだまだ

```kpi
title "55種類以上のプラグイン"
metric "フロー・業務系" : "フローチャート, スイムレーン, ガント, etc"
metric "構造・設計系" : "ER図, 組織図, レイヤー図, ツリー, etc"
metric "比較・分析系" : "比較表, SWOT, レーダー, 円グラフ, etc"
metric "その他" : "Git図, カンバン, FAQ, 料金表, etc"
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
