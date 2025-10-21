# バックエンドAPIテストコマンド

このドキュメントは、`taiyaq`バックエンドAPIエンドポイントをテストするための`curl`コマンドを提供します。
これらのコマンドを実行する前に、バックエンドサーバーが`http://127.0.0.1:38000`で実行されていることを確認してください。

## ベースURL

```bash
BASE_URL="http://127.0.0.1:38000"
```

## 1. スタッフAPI: 注文の作成

まず、いくつかの注文を作成して、作業するデータを用意しましょう。

### `POST /api/staff/orders`

新しい注文を作成します。

```bash
# 注文1を作成 (IDは1になります)
curl -X POST "${BASE_URL}/api/staff/orders" \
     -H "Content-Type: application/json" \
     -d '{
          "items": [
            {"flavor": "anko", "quantity": 2},
            {"flavor": "custard", "quantity": 1}
          ]
        }'

# 注文2を作成 (IDは2になります)
curl -X POST "${BASE_URL}/api/staff/orders" \
     -H "Content-Type: application/json" \
     -d '{
          "items": [
            {"flavor": "anko", "quantity": 3}
          ]
        }'

# 注文3を作成 (IDは3になります)
curl -X POST "${BASE_URL}/api/staff/orders" \
     -H "Content-Type: application/json" \
     -d '{
          "items": [
            {"flavor": "custard", "quantity": 5}
          ]
        }'
```

## 2. スタッフAPI: 全ての注文を取得

全ての注文を取得します。最初は全ての注文が「waiting」ステータスになります。

### `GET /api/staff/orders`

```bash
curl -X GET "${BASE_URL}/api/staff/orders"
```

### `GET /api/staff/orders?status=waiting`

「waiting」ステータスの注文を取得します。

```bash
curl -X GET "${BASE_URL}/api/staff/orders?status=waiting"
```

## 3. スタッフAPI: 生産状況の更新

焼き上がったたい焼きを報告します。これにより、十分な在庫があれば、待機中の注文が「ready」に移行するはずです。

### `POST /api/staff/production`

```bash
# 生産状況を報告: あんこ5個、カスタード2個
curl -X POST "${BASE_URL}/api/staff/production" \
     -H "Content-Type: application/json" \
     -d '{
          "items": [
            {"flavor": "anko", "quantity": 5},
            {"flavor": "custard", "quantity": 2}
          ]
        }'
```

*予想: 注文1（あんこ2個、カスタード1個）と注文2（あんこ3個）は「ready」になるはずです。注文3（カスタード5個）は、カスタードが2個しか生産されていないため、「waiting」のままです。*

## 4. ユーザー/ディスプレイAPI: 表示用注文の取得

どの注文が準備完了または調理中かを確認します。

### `GET /api/orders/display`

```bash
curl -X GET "${BASE_URL}/api/orders/display"
```

*予想: 注文1と2が`ready`リストに表示されるはずです。*

## 5. ユーザー/ディスプレイAPI: 注文詳細の取得

特定の注文のステータスと推定待ち時間を確認します。

### `GET /api/orders/{id}`

```bash
# 注文1の詳細を確認 (readyになっているはず)
curl -X GET "${BASE_URL}/api/orders/1"

# 注文3の詳細を確認 (まだwaitingになっているはず)
curl -X GET "${BASE_URL}/api/orders/3"
```

## 6. ボットAPI: 通知の追加

注文の通知先を設定します。

### `PUT /api/orders/{id}/notification`

```bash
# 注文3にメール通知を追加
curl -X PUT "${BASE_URL}/api/orders/3/notification" \
     -H "Content-Type: application/json" \
     -d '{
          "channel": "email",
          "target": "user3@example.com"
        }'
```

## 7. スタッフAPI: 注文の完了

注文を完了としてマークします。

### `POST /api/staff/orders/{id}/complete`

```bash
# 注文1を完了
curl -X POST "${BASE_URL}/api/staff/orders/1/complete"
```

## 8. スタッフAPI: 注文のキャンセル

注文をキャンセルします。

### `POST /api/staff/orders/{id}/cancel`

```bash
# 注文3をキャンセル
curl -X POST "${BASE_URL}/api/staff/orders/3/cancel"
```
