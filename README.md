# MDD

MDD (Markdown with Diagrams) は軽量な Markdown プリプロセッサ。

Markdown のコードブロックをスキャンし、外部プラグインを呼び出して、ブロックを生成された SVG 画像に置換する。

プラグインは `$PATH` から発見される単純な実行可能コマンド。

コードブロック:

````markdown
```sequence
Alice -> Bob: Hello
```
````

に対して MDD は以下を実行する:

```bash
mdd-sequence
```

ブロックの内容は標準入力で渡され、プラグインは標準出力で SVG を返す。

```text
Markdown
    ↓
コードブロック
    ↓
mdd-{ブロック名}
    ↓
SVG
    ↓
Markdown
```

MDD 本体が担うのは以下のみ:

* Markdown のパース
* プラグインの発見
* プラグインの実行
* Markdown の生成

図の描画ロジックはすべてプラグイン側に属する。

## インストール

### ワンライナー（推奨）

```sh
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/ppdx999/mdd/main/install.sh | sh
```

```powershell
# Windows (PowerShell)
iwr https://raw.githubusercontent.com/ppdx999/mdd/main/install.ps1 -useb | iex
```

`~/.local/bin/` にインストールされる。`MDD_INSTALL_DIR` 環境変数でインストール先を変更可能。

### ソースからビルド

Rust ツールチェインが必要。

```sh
git clone https://github.com/ppdx999/mdd.git
cd mdd
make install
```

`~/.cargo/bin/` に `mdd` と全プラグインがインストールされる。

アンインストール:

```sh
make uninstall
```

## 公式プラグイン

### ユースケース図 ([mdd-usecase](crates/mdd-usecase/))

アクター、ユースケース、パッケージで構成されるユースケース図。

![usecase](crates/mdd-usecase/examples/complex.svg)

### DFD — データフロー図 ([mdd-dfd](crates/mdd-dfd/))

外部エンティティ、プロセス、データストア間のデータの流れを可視化する。データストアにはテーブル名と列名を記述可能。

![dfd](crates/mdd-dfd/examples/complex.svg)

### ツリー図 ([mdd-tree](crates/mdd-tree/))

組織図、ディレクトリ構造、分類体系などの階層構造をトップダウンで描画する。グループで複数ノードをまとめられる。

![tree](crates/mdd-tree/examples/org.svg)

### ER 図 ([mdd-er](crates/mdd-er/))

テーブル定義（主キー、列名）とリレーション（1:1, 1:N, N:M）を描画する。

![er](crates/mdd-er/examples/ecommerce.svg)

### シーケンス図 ([mdd-sequence](crates/mdd-sequence/))

参加者間のメッセージの時系列を描画する。同期メッセージ（実線）、応答メッセージ（破線）、自己メッセージに対応。

![sequence](crates/mdd-sequence/examples/auth.svg)

### 状態遷移図 ([mdd-state](crates/mdd-state/))

状態マシンの状態とラベル付き遷移を描画する。自己遷移にも対応。

![state](crates/mdd-state/examples/order.svg)

### インフラ構成図 ([mdd-infra](crates/mdd-infra/))

ネストしたグループ（AWS > VPC > サブネット）と種別付きコンポーネント（server, db, lb, cache, queue, storage, cdn 等）で構成されるインフラ構成図。

![infra](crates/mdd-infra/examples/aws.svg)

### ガントチャート ([mdd-gantt](crates/mdd-gantt/))

タスクの開始日・期間・依存関係を時系列で描画する。セクションによるグループ化に対応。

![gantt](crates/mdd-gantt/examples/project.svg)

### フローチャート ([mdd-flowchart](crates/mdd-flowchart/))

開始/終了（楕円）、処理（矩形）、分岐（ひし形）で構成されるフローチャート。業務フローやアルゴリズムの可視化に。

![flowchart](crates/mdd-flowchart/examples/validation.svg)

### スイムレーン図 ([mdd-swimlane](crates/mdd-swimlane/))

レーン（部門/担当者）ごとに分けたフローチャート。業務フローの責任分担を可視化する。

![swimlane](crates/mdd-swimlane/examples/order.svg)

### グリッド図 ([mdd-grid](crates/mdd-grid/))

RACI マトリクス、機能×チーム対応表、権限表などを色付きグリッドで可視化する。

![grid](crates/mdd-grid/examples/raci.svg)

### 分析図 ([mdd-analysis](crates/mdd-analysis/))

構成比や内訳を積み上げバーチャートやウォーターフォールチャートで可視化する。

![analysis](crates/mdd-analysis/examples/revenue.svg)

### ステップ図 ([mdd-steps](crates/mdd-steps/))

段階的な進行・成長を階段状に表現する。開発プロセスやスキル成長の可視化に。

![steps](crates/mdd-steps/examples/sdlc.svg)

### ランキング図 ([mdd-ranking](crates/mdd-ranking/))

順位付きリストを横棒グラフで可視化する。売上ランキング、優先度順位などに。

![ranking](crates/mdd-ranking/examples/sales.svg)

### グループ図 ([mdd-group-multi](crates/mdd-group-multi/))

多数のグループと要素をグリッド状に整理して配置する。部署一覧、技術スタック、カテゴリ分類などに。

![group-multi](crates/mdd-group-multi/examples/departments.svg)

### レイヤー図 ([mdd-layer](crates/mdd-layer/))

OSI参照モデル、アーキテクチャレイヤーなどの積層構造を可視化する。グループによるレイヤーのまとめ、右側への説明表示に対応。

![layer](crates/mdd-layer/examples/architecture.svg)

### タイムライン ([mdd-timeline](crates/mdd-timeline/))

プロジェクトのマイルストーン、会社沿革、リリース履歴などの時系列イベントを水平タイムラインで可視化する。

![timeline](crates/mdd-timeline/examples/company.svg)

### ビフォーアフター図 ([mdd-before-after](crates/mdd-before-after/))

変更前後の状態を左右に並べて対比する。業務改善、システム移行などの提案資料に。

![before-after](crates/mdd-before-after/examples/system.svg)

### サイクル図 ([mdd-cycle](crates/mdd-cycle/))

PDCA、DevOps、Scrum など循環するプロセスを円形に配置して可視化する。放射状の説明表示に対応。

![cycle](crates/mdd-cycle/examples/pdca.svg)

### プロセスフロー図 ([mdd-process](crates/mdd-process/))

横方向の矢印で繋いだプロセスフロー図。業務手順やワークフローの可視化に。カード下への説明表示に対応。

![process](crates/mdd-process/examples/simple.svg)

### ファネル図 ([mdd-funnel](crates/mdd-funnel/))

営業パイプライン、コンバージョン漏斗などのファネル図。値による幅の自動調整、右側への説明表示に対応。

![funnel](crates/mdd-funnel/examples/simple.svg)

### ピラミッド図 ([mdd-pyramid](crates/mdd-pyramid/))

階層構造の概念図。マズローの欲求階層、戦略ピラミッドなどに。右側への説明表示に対応。

![pyramid](crates/mdd-pyramid/examples/strategy.svg)

### トライアングル図 ([mdd-triangle](crates/mdd-triangle/))

3要素の三角関係を可視化する。QCD、スコープ・コスト・時間などに。

![triangle](crates/mdd-triangle/examples/qcd.svg)

### マトリクス図 ([mdd-matrix](crates/mdd-matrix/))

2軸で分類する2x2マトリクス図。アイゼンハワー・マトリクス、リスク分析などに。

![matrix](crates/mdd-matrix/examples/eisenhower.svg)

### 比較図 ([mdd-compare](crates/mdd-compare/))

2〜3案を並べて対比する。フレームワーク比較、料金プラン比較などに。

![compare](crates/mdd-compare/examples/framework.svg)

### 規模比較図 ([mdd-scale](crates/mdd-scale/))

数量や規模の大小を横棒グラフで視覚的に比較する。

![scale](crates/mdd-scale/examples/population.svg)

### SWOT分析図 ([mdd-swot](crates/mdd-swot/))

強み・弱み・機会・脅威の4象限で分析する SWOT 図。

![swot](crates/mdd-swot/examples/startup.svg)

### ベン図 ([mdd-venn](crates/mdd-venn/))

集合の重なりを可視化する。2〜3セットに対応。

![venn](crates/mdd-venn/examples/skills.svg)

### 放射図 ([mdd-radial](crates/mdd-radial/))

中心概念と周辺要素の関係をハブ&スポーク型で表現する。

![radial](crates/mdd-radial/examples/marketing.svg)

### 相関図・概念図 ([mdd-concept](crates/mdd-concept/))

概念間の自由な関係性を線と矢印で表現する。有向・無向リンクに対応。

![concept](crates/mdd-concept/examples/ecosystem.svg)

### マインドマップ ([mdd-mindmap](crates/mdd-mindmap/))

中心トピックから放射状に枝分かれするマインドマップ。ブレスト、アイデア整理に。

![mindmap](crates/mdd-mindmap/examples/planning.svg)

### パズル・ハニカム図 ([mdd-puzzle](crates/mdd-puzzle/))

六角形のハニカム構造で要素を配置する。チーム構成、構成要素の表現に。

![puzzle](crates/mdd-puzzle/examples/team.svg)

### グループ図 ([mdd-group](crates/mdd-group/))

2〜4グループの要素をカード形式で並べて表示する。

![group](crates/mdd-group/examples/roles.svg)

### テーブル ([mdd-table](crates/mdd-table/))

Markdown の表より視覚的にリッチな SVG テーブル。ヘッダー色分け、交互背景に対応。

![table](crates/mdd-table/examples/features.svg)

### 縦型リスト ([mdd-list-v](crates/mdd-list-v/))

番号バッジ付きの縦方向リスト。手順説明や設計原則の列挙に。

![list-v](crates/mdd-list-v/examples/principles.svg)

### 横型カードリスト ([mdd-list-h](crates/mdd-list-h/))

カード状の横方向リスト。企業バリュー、サービス一覧などに。

![list-h](crates/mdd-list-h/examples/values.svg)

### グリッドリスト ([mdd-list-grid](crates/mdd-list-grid/))

グリッド配置の項目一覧。ツール一覧、チェックリストなどに。

![list-grid](crates/mdd-list-grid/examples/tools.svg)

### KPI カード ([mdd-kpi](crates/mdd-kpi/))

数値ハイライトのメトリクスカード。ダッシュボード、KPI 表示に。

![kpi](crates/mdd-kpi/examples/dashboard.svg)

### 地図・マップ ([mdd-map](crates/mdd-map/))

拠点配置や地理的関係をピンとルートで簡易的に表現する。

![map](crates/mdd-map/examples/offices.svg)

### 数式 ([mdd-math](crates/mdd-math/))

数式をセリフフォントで SVG レンダリングする。Unicode 数学記号に対応。

![math](crates/mdd-math/examples/physics.svg)

## AGENTS.md 向けサンプル

AI エージェントにドキュメント内で図を生成させる際、`AGENTS.md` に以下のような記述を追加すると効果的。

````markdown
## 図の生成

このプロジェクトでは [mdd](https://github.com/ppdx999/mdd) を使って Markdown 内に図を埋め込む。
コードブロックの言語名に応じたプラグインが SVG を生成する。

### ユースケース図

```usecase
actor Customer
actor Admin

package "認証" {
  usecase Login
  usecase Logout
}

Customer -> Login
Admin -> Login
Admin -> Logout
```

### DFD（データフロー図）

```dfd
entity Customer
entity PaymentGateway

process HandleOrder
process ValidatePayment

datastore Orders {
  注文ID
  顧客ID
  合計金額
  ステータス
}

Customer -> HandleOrder : "注文情報"
HandleOrder -> Orders : "注文データ"
HandleOrder -> ValidatePayment : "支払い依頼"
ValidatePayment -> PaymentGateway : "決済リクエスト"
```
````
