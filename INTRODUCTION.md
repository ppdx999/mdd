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

要件定義 : 2025-04-01, 3d
基本設計 : after 要件定義, 3d
詳細設計 : after 基本設計, 2d
開発 : after 詳細設計, 5d
テスト : after 開発, 3d
移行 : after テスト, 2d
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
2025-04-01 : 要件定義開始
2025-07-01 : 基本設計完了
2025-09-01 : 詳細設計完了
2026-01-01 : 開発完了
2026-03-01 : テスト完了
2026-06-01 : 本番移行
```

# どんな図が作れる？ — リスク分析

```matrix
title "プロジェクトリスクマトリクス"
x-axis "影響度：小" "影響度：大"
y-axis "発生確率：低" "発生確率：高"
quadrant 1 : "監視"
quadrant 2 : "最優先対応"
quadrant 3 : "許容"
quadrant 4 : "軽減策検討"
```

# どんな図が作れる？ — インフラ構成

```infra
node CDN type=cdn

group "AWS" {
  node ALB type=lb
  group "VPC" {
    node AP1 type=server
    node AP2 type=server
    node RDS type=db
    node Redis type=cache
  }
}

CDN -> ALB
ALB -> AP1
ALB -> AP2
AP1 -> RDS : "SQL"
AP2 -> RDS : "SQL"
AP1 -> Redis
AP2 -> Redis
```

# どんな図が作れる？ — SWOT分析

```swot
title "プロジェクト SWOT"
strengths {
  チームの技術力が高い
  既存業務知識が豊富
}
weaknesses {
  レガシーシステムとの依存
  テスト環境が不十分
}
opportunities {
  クラウド移行でコスト削減
  DXによる業務効率化
}
threats {
  納期遅延リスク
  要件変更の頻発
}
```

# どんな図が作れる？ — ディレクトリ構成

```dirtree
backend/ : "バックエンド"
  src/
    controllers/ : "APIエンドポイント"
    services/ : "ビジネスロジック"
    repositories/ : "データアクセス"
    models/ : "エンティティ定義"
  tests/
  Dockerfile
frontend/ : "フロントエンド"
  src/
    components/
    pages/
    hooks/
  package.json
docker-compose.yml : "開発環境構成"
```

# (おまけ) このスライドの正体

このスライド自体が、ただの Markdown テキストから自動生成されています。

```code
title "INTRODUCTION.md"
---
# MDD — Markdown with Diagrams

ドキュメントに図を入れたい。でも、もっと簡単に。

# 図の生成はすごく便利

```usecase
actor 顧客
actor 管理者
package "認証" {
  usecase ログイン
}
顧客 -> ログイン
```
