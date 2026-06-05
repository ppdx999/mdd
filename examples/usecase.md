# ECサイト ユースケース図

```usecase
actor 顧客
actor 管理者

package "商品管理" {
  usecase 商品検索
  usecase 商品詳細表示
  usecase 在庫確認
}

package "注文" {
  usecase カートに追加
  usecase 注文確定
  usecase 注文履歴確認
}

顧客 -> 商品検索
顧客 -> 商品詳細表示
顧客 -> カートに追加
顧客 -> 注文確定
顧客 -> 注文履歴確認
管理者 -> 在庫確認
商品詳細表示 -> 在庫確認
```
