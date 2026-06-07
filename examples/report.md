# 営業レポート Q1 2025

売上は前年比 20% 増加しました。

ファネル分析の結果、商談化率の改善が主要因です。

```funnel
title "営業ファネル"
stage リード獲得 : 1000 : "広告・展示会"
stage 商談化 : 400 : "ヒアリング・提案"
stage 提案 : 200
stage 見積 : 100
stage 受注 : 40
```

# チーム構成

現在のエンジニアリング組織は以下の通りです。

```org
title "組織図"
member CEO : "代表取締役"
member CTO : "技術統括"
member CFO : "財務統括"
member VP_Eng : "開発部長"
member VP_Product : "プロダクト部長"
member Lead_FE : "FEリード"
member Lead_BE : "BEリード"
CEO -> CTO
CEO -> CFO
CTO -> VP_Eng
CTO -> VP_Product
VP_Eng -> Lead_FE
VP_Eng -> Lead_BE
```

# 開発プロセス

チームは PDCA サイクルで継続的に改善しています。

```cycle
title PDCA
step 計画 : "目標設定
行動計画策定"
step 実行 : "計画に基づき実施"
step 評価 : "結果の測定
分析・検証"
step 改善 : "改善策の立案
標準化"
```

# スプリント状況

```kanban
title "Sprint 12"
column Todo
card ユーザー通知 : "feature"
card バグ修正 #234 : "bug"

column In Progress
card API リファクタリング : "refactor"
card 検索機能改善 : "feature"

column Done
card ログイン画面 : "feature"
card CI/CD 設定 : "infra"
card パフォーマンス改善 : "perf"
```

# 市場シェア

```pie
title "市場シェア 2025 Q1"
slice 自社 : 35
slice 競合A : 25
slice 競合B : 20
slice その他 : 20
```

# スキル評価

```radar
title "チームスキル比較"
axis フロントエンド
axis バックエンド
axis インフラ
axis セキュリティ
axis コミュニケーション
data "田中" : 90, 70, 50, 60, 80
data "鈴木" : 60, 90, 80, 70, 60
```

# 料金プラン

```pricetable
title "料金プラン"
plan Free : "¥0/月"
- ユーザー5人まで
- ストレージ 1GB
- メールサポート

plan* Pro : "¥2,980/月"
- ユーザー無制限
- ストレージ 100GB
- チャットサポート
- API利用可
- カスタムドメイン

plan Enterprise : "お問い合わせ"
- 全Pro機能
- 専任サポート
- SLA 99.99%
- オンプレミス対応
```

# お客様の声

```quote
quote "導入して業務効率が3倍になりました。
もう手放せません。"
author "田中太郎"
role "CTO・株式会社テック"

quote "サポートの対応が素晴らしい。
問い合わせから1時間以内に解決してくれた。"
author "鈴木花子"
role "プロダクトマネージャー"
```

# よくある質問

```faq
title "FAQ"
q "無料プランはありますか？"
a "はい、基本機能は無料でご利用いただけます。"

q "データのバックアップはどうなっていますか？"
a "毎日自動バックアップを実施しています。
30日間保持されます。"

q "解約はいつでもできますか？"
a "はい、管理画面からいつでも解約可能です。
日割り返金にも対応しています。"
```

# Git ブランチ戦略

```gitgraph
commit "Initial commit" tag "v0.1"
commit "Add README"
branch feature/auth
checkout feature/auth
commit "Add login page"
commit "Add JWT auth"
checkout main
commit "Fix typo in docs"
branch feature/api
checkout feature/api
commit "Add REST endpoints"
commit "Add validation"
checkout main
merge feature/auth
commit "Update config" tag "v0.2"
merge feature/api
commit "Release prep" tag "v1.0"
```
