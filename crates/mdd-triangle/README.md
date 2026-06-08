# mdd-triangle

トライアングル図プラグイン

3つの要素を三角形の頂点に配置し、それぞれの辺にラベルを付けた関係図を描画します。QCDトライアングル、スコープ・コスト・スケジュールなどの三角形の関係を表現できます。

## 入力形式

```
node 上の要素
node 左下の要素
node 右下の要素
edge 0 -- 1 : "辺のラベル"
edge 1 -- 2 : "辺のラベル"
edge 0 -- 2 : "辺のラベル"
```

## 使い方

```bash
cat examples/qcd.triangle | cargo run -p mdd-triangle > output.svg
```
