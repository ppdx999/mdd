# MDD — Markdown with Diagrams

ドキュメントに図を入れたい。でも、もっと簡単に。

# なぜ図が必要か

認知負荷を下げるために、テキストだけでは伝わりにくい情報を図にしたい。

```persona
actor エンジニア : "設計書にアーキテクチャ図を
入れたいけど面倒..."
actor PM : "提案書にフロー図が
ほしいけど時間がない..."
actor デザイナー : "仕様書に画面遷移図を
付けたいけどツールが重い..."
```

# 現状の選択肢

```compare
title "既存ツールの課題"
option "Draw.io" {
  操作が複雑
  共有にエクスポートが必要
  差分管理が困難
}
option "PowerPoint" {
  起動・編集が重い
  バージョン管理不可
  AIが扱いにくい
}
option "Canva" {
  ネット必須
  細かい調整が面倒
  テキスト出力できない
}
```

# 既存ツールの問題点

```swot
title "既存ツールの現状"
strengths {
  リッチな表現力
  GUIで直感的
}
weaknesses {
  レンダリングが遅い
  バイナリ形式で差分管理不可
  AIのトークン消費が大きい
  共有・レビューが面倒
}
opportunities {
  AI活用で自動生成
  ドキュメント自動化
}
threats {
  チーム間の認識齟齬
  ドキュメント陳腐化
}
```

# MDD の登場

テキストから図を生成する、軽量な Markdown プリプロセッサ。

```process
step テキスト入力 : "Markdownに
DSLを書く"
step MDD変換 : "コマンド一発
1秒以下"
step SVG出力 : "美しい図が
自動生成"
step 共有 : "Git管理
差分レビュー"
```

# MDD の仕組み

```dirtree
input.md : "Markdownファイル"
  code-block/ : "コードブロックを検出"
    funnel : "mdd-funnel を実行"
    org : "mdd-org を実行"
    cycle : "mdd-cycle を実行"
output.md : "SVG埋め込み済みMarkdown"
```

# MDD の3つの強み

```list-h
title "Why MDD?"
card "爆速生成" : "1秒以下でSVG生成。AIの待ち時間ゼロ"
card "トークン節約" : "テキストDSLだからAIの入出力が最小限"
card "100%カスタマイズ" : "プラグインはただのCLI。自作も簡単"
```

# 強み① 爆速生成

```steps
step Draw.io : "エクスポート 5〜10秒"
step PowerPoint : "起動だけで数秒"
step Canva : "ネット往復 3〜5秒"
step MDD : "0.1秒以下"
```

# 強み② トークン節約

AIに図を生成させるとき、MDD は最小限のテキストで済む。

```scale
title "AIトークン消費量の比較"
unit "tokens"
item Draw.io XML : 5000
item SVG直接生成 : 3000
item MDD DSL : 200
```

# 強み③ 100%カスタマイズ可能

```layer
layer プラグイン : "stdin → SVG → stdout"
layer MDD本体 : "コードブロック検出・プラグイン実行"
layer Markdown : "テキストファイル"
layer Git : "バージョン管理・差分レビュー"
```

# どんな図が作れる？ — フロー系

```flowchart
start 開始
process 入力受付
decision 有効？
process 処理実行
process エラー表示
end 完了

開始 -> 入力受付
入力受付 -> 有効？
有効？ -> 処理実行 : "Yes"
有効？ -> エラー表示 : "No"
処理実行 -> 完了
エラー表示 -> 入力受付
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

# どんな図が作れる？ — Git ブランチ

```gitgraph
commit "Initial commit" tag "v0.1"
branch feature/auth
checkout feature/auth
commit "Add login"
commit "Add JWT"
checkout main
merge feature/auth
commit "Release" tag "v1.0"
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
