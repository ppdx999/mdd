# AGENTS.md — MDD プラグイン開発ガイド

MDD は Markdown のコードブロックを SVG に変換する軽量プリプロセッサ。
図の描画ロジックはすべてプラグイン側に属する。このドキュメントは公式プラグインの作法を定義する。

## アーキテクチャ

```
Markdown → mdd (コア) → mdd-{言語名} (プラグイン) → SVG → Markdown
```

- コアはコードブロックの言語名から `mdd-{言語名}` コマンドを `$PATH` で探して実行する
- プラグインは**標準入力**でテキストを受け取り、**標準出力**で SVG を返す
- エラーは**標準エラー出力**に書き、**終了コード 1** で終了する

## ディレクトリ構成

```
crates/mdd-{name}/
├── Cargo.toml
├── README.md
├── examples/
│   ├── simple.{name}      # 最小限の例（必須）
│   ├── simple.svg
│   ├── {other}.{name}     # 追加の例
│   └── {other}.svg
└── src/
    └── main.rs             # 単一ファイル
```

### ルール

- ソースは `src/main.rs` の単一ファイルに収める。`lib.rs` は作らない
- example ファイルの拡張子はプラグイン名と一致させる（例: `.flowchart`, `.gantt`）
- `simple.{name}` は最小限の動作例として必須。3〜7 行程度
- 追加の example は実用的なシナリオを示す

## Cargo.toml

```toml
[package]
name = "mdd-{name}"
version = "0.1.0"
edition = "2024"

[dependencies]
# レイアウトが必要なら rust-sugiyama = "0.4"
# 依存は最小限に保つ
```

- バージョンは `0.1.0`、エディションは `2024`
- 依存は必要最小限。グラフレイアウトには `rust-sugiyama = "0.4"` を使う

## ソースコード構成

`main.rs` は以下の 3 層で構成する。

### 1. データ構造

```rust
#[derive(Debug)]
struct Node { name: String, kind: NodeKind }

#[derive(Debug)]
struct Edge { from: usize, to: usize, label: String }

#[derive(Debug)]
struct Diagram { nodes: Vec<Node>, edges: Vec<Edge> }
```

- ノード/要素の識別には `String` 名と `usize` インデックスを併用する
- `HashMap<String, usize>` で名前→ID のルックアップを持つ

### 2. パーサー

```rust
fn parse(input: &str) -> Result<Diagram, String> { ... }
```

- 行指向のパース。空行はスキップ
- エラーは `Result<_, String>` で返す。具体的なメッセージを付ける
- 要素定義行（`start`, `process`, `node` 等）とエッジ定義行（`A -> B : "label"`）を分ける

### 3. レンダラー

```rust
fn render_svg(diagram: &Diagram) -> String { ... }
```

- SVG XML を直接文字列で組み立てる（テンプレートエンジンは使わない）
- スタイルはインラインで記述する

### main 関数

```rust
fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).expect("Failed to read stdin");

    let diagram = match parse(&input) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("mdd-{name}: {}", e);
            std::process::exit(1);
        }
    };

    let svg = render_svg(&diagram);
    print!("{}", svg);
}
```

この形を厳守する。引数は取らず、stdin/stdout のみで動作する。

## DSL 記法の設計指針

### 要素定義

```
{種別キーワード} {名前}
```

例: `start 開始`, `process 処理`, `actor 顧客`, `table Users { ... }`

- キーワードは英小文字
- 名前は日本語・英語いずれも使えるようにする
- 属性が必要な場合は `key=value` 形式（例: `node LB type=lb`）

### エッジ定義

```
{from} -> {to}
{from} -> {to} : "{label}"
```

- 矢印は `->` で統一する（破線は `-->` など、意味に応じたバリアントは可）
- ラベルはダブルクォートで囲む

### グループ / ブロック

```
{group-keyword} "{name}" {
  ...
}
```

- 開き括弧 `{` は同じ行に書く
- ネスト可能にする場合はインデントではなく括弧で表現する

## SVG レンダリング規約

### 背景

- SVG のルート直下に白背景の `<rect>` を置く（ダークモード対応）

### デザイン原則

**テキストは基本的に黒（`#333`）を使う。** 背景色は控えめにし、使う場合も必ず薄い色にする。濃い背景色 + 白文字の組み合わせは避ける。

- ヘッダーや見出しの背景は薄い色（`#e8eaf6` 等）にして、文字は濃い色（`#333`）
- セルや要素の差別化はテキストの色で表現するか、非常に薄い背景色（`#e3f2fd`, `#fff8e1` 等）で行う
- 強調が必要な場合でもパステルトーンの範囲に留める

### 色パレット

| 用途 | 背景色 | テキスト色 |
|---|---|---|
| ノード（デフォルト） | `#e3f2fd`（薄い青） | `#333` |
| 開始 | `#e8f5e9`（薄い緑） | `#333` |
| 終了 | `#f5f5f5`（薄いグレー） | `#333` |
| 分岐 | `#fff8e1`（薄い黄） | `#333` |
| グループ | `#fafafa`（薄いグレー）+ 破線枠 | `#333` |
| ヘッダー | `#e0e0e0` / `#e8eaf6`（薄いグレー/インディゴ） | `#333` |
| 線 | — | `#666` / `#999` |
| エラー/重要 | `#ffebee`（薄い赤） | `#c62828` |

色は図の種類に応じて調整してよいが、全体のトーンは「薄い背景 + 濃い文字」で統一する。濃い背景色は使わない。

### テキスト

- フォント: `sans-serif`
- フォントサイズ: `13px`（基本）
- テキスト幅の推定: ASCII 文字 `~8px`、日本語文字 `~14px`

### 形状

- ノード: 角丸矩形（`rx="8"`）
- 開始/終了: 角丸楕円
- 分岐: ひし形
- グループ: 破線矩形 + ヘッダー

## テスト

- `#[cfg(test)] mod tests` を `main.rs` の末尾に書く
- パース結果の検証テスト（正常系・異常系）を含める
- SVG 出力の構造テスト（要素の存在確認）を含める
- `cargo test` で全プラグインのテストが通ること

## README.md

日本語で記述する。以下の構成に従う。

```markdown
# mdd-{name}

`mdd` 用の{図の種類}プラグイン。テキストベースの記法から SVG の{図の種類}を生成する。

## 使い方

（stdin/stdout の使い方 + mdd 経由の使い方）

## 記法

（各要素の定義方法とエッジ定義を小見出しで説明）

## 描画

（要素ごとの形状・色の対応表）

## サンプル

（examples/ の SVG を画像として埋め込む）
```

## Makefile への登録

新しいプラグインを追加したら `Makefile` の `CRATES` 変数に追加する。

```makefile
CRATES := mdd mdd-usecase ... mdd-{name}
```

## ルート README.md への登録

`README.md` の「公式プラグイン」セクションにエントリを追加する。

```markdown
### {図の名前} ([mdd-{name}](crates/mdd-{name}/))

{1行の説明}

![{name}](crates/mdd-{name}/examples/{representative}.svg)
```

## チェックリスト

新しいプラグインを追加する際の確認事項:

- [ ] `crates/mdd-{name}/` にディレクトリを作成した
- [ ] `Cargo.toml` が規約に従っている
- [ ] `src/main.rs` が parse → render → output パターンに従っている
- [ ] stdin から読み、stdout に SVG を出力する
- [ ] エラー時は stderr に出力し exit code 1 で終了する
- [ ] `examples/simple.{name}` と `examples/simple.svg` がある
- [ ] 実用的な追加 example がある
- [ ] テストがある（パース + SVG 構造）
- [ ] `README.md` が規約に従っている
- [ ] `Makefile` の `CRATES` に追加した
- [ ] ルート `README.md` の公式プラグインセクションに追加した
- [ ] `cargo test` が通る
- [ ] SVG に白背景 rect がある
