# mdd-org

組織図プラグイン。メンバーと上下関係を階層的に可視化する。

## 使い方

```
cat input.org | mdd-org > output.svg
```

## 入力形式

```
member CEO : "代表取締役"
member CTO : "技術統括"
member CFO : "財務統括"

CEO -> CTO
CEO -> CFO
```

## サンプル

![company](examples/company.svg)
