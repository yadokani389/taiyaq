# バックエンドAPIテストコマンド

このドキュメントは、`taiyaq`バックエンドAPIエンドポイントをテストするための`curl`コマンドを提供します。
これらのコマンドを実行する前に、バックエンドサーバーが`http://127.0.0.1:38000`で実行されていることを確認してください。

## ベースURLとトークン

```bash
BASE_URL="http://127.0.0.1:38000"
# .envファイルなどで設定したSTAFF_API_TOKENをここに設定してください
STAFF_API_TOKEN="your_secret_token_here"
```

## 1. スタッフAPI: 味の設定

各味の調理時間とバッチサイズを設定します。

### `PUT /api/staff/flavors/{flavor}`

```bash
# つぶあんの設定
curl -X PUT "${BASE_URL}/api/staff/flavors/tsubuan" \
     -H "Authorization: Bearer ${STAFF_API_TOKEN}" \
     -H "Content-Type: application/json" \
     -d '{
          "cookingTimeMinutes": 15,
          "quantityPerBatch": 9
        }'

# カスタードの設定
curl -X PUT "${BASE_URL}/api/staff/flavors/custard" \
     -H "Authorization: Bearer ${STAFF_API_TOKEN}" \
     -H "Content-Type: application/json" \
     -d '{
          "cookingTimeMinutes": 15,
          "quantityPerBatch": 9
        }'

# 栗きんとんの設定
curl -X PUT "${BASE_URL}/api/staff/flavors/kurikinton" \
     -H "Authorization: Bearer ${STAFF_API_TOKEN}" \
     -H "Content-Type: application/json" \
     -d '{
          "cookingTimeMinutes": 15,
          "quantityPerBatch": 2
        }'
```

## 2. スタッフAPI: 注文の作成

まず、いくつかの注文を作成して、作業するデータを用意しましょう。

### `POST /api/staff/orders`

新しい注文を作成します。

```bash
# 注文1を作成 (IDは1になります)
curl -X POST "${BASE_URL}/api/staff/orders" \
     -H "Content-Type: application/json" \
     -H "Authorization: Bearer ${STAFF_API_TOKEN}" \
     -d '{
          "items": [
            {"flavor": "tsubuan", "quantity": 2},
            {"flavor": "custard", "quantity": 1}
          ]
        }'

# 注文2を作成 (IDは2になります)
curl -X POST "${BASE_URL}/api/staff/orders" \
     -H "Content-Type: application/json" \
     -H "Authorization: Bearer ${STAFF_API_TOKEN}" \
     -d '{
          "items": [
            {"flavor": "tsubuan", "quantity": 3}
          ]
        }'

curl -X POST "${BASE_URL}/api/staff/orders" \
     -H "Content-Type: application/json" \
     -H "Authorization: Bearer ${STAFF_API_TOKEN}" \
     -d '{
          "items": [
            {"flavor": "custard", "quantity": 5}
          ]
        }'
```

## 3. スタッフAPI: 全ての注文を取得

全ての注文を取得します。最初は全ての注文が「waiting」ステータスになります。

### `GET /api/staff/orders`

```bash
curl -X GET "${BASE_URL}/api/staff/orders" \
     -H "Authorization: Bearer ${STAFF_API_TOKEN}"
```

### `GET /api/staff/orders?status=waiting`

「waiting」ステータスの注文を取得します。

```bash
curl -X GET "${BASE_URL}/api/staff/orders?status=waiting" \
     -H "Authorization: Bearer ${STAFF_API_TOKEN}"
```

## 4. スタッフAPI: 生産状況の更新

焼き上がったたい焼きを報告します。これにより、十分な在庫があれば、待機中の注文が「ready」に移行するはずです。

### `POST /api/staff/production`

```bash
# 生産状況を報告: つぶあん5個、カスタード2個
curl -X POST "${BASE_URL}/api/staff/production" \
     -H "Content-Type: application/json" \
     -H "Authorization: Bearer ${STAFF_API_TOKEN}" \
     -d '{
          "items": [
            {"flavor": "tsubuan", "quantity": 5},
            {"flavor": "custard", "quantity": 2}
          ]
        }'
```

_予想:
注文1（つぶあん2個、カスタード1個）と注文2（つぶあん3個）は「ready」になるはずです。注文3（カスタード5個、栗きんとん1個）は、カスタードと栗きんとんが生産されていないため、「waiting」のままです。_

## 5. ユーザー/ディスプレイAPI: 表示用注文の取得

どの注文が準備完了または調理中かを確認します。

### `GET /api/orders/display`

```bash
curl -X GET "${BASE_URL}/api/orders/display"
```

_予想: 注文1と2が`ready`リストに表示されるはずです。_

## 6. ユーザー/ディスプレイAPI: 注文詳細の取得

特定の注文のステータスと推定待ち時間を確認します。

### `GET /api/orders/{id}`

```bash
# 注文1の詳細を確認 (readyになっているはず)
curl -X GET "${BASE_URL}/api/orders/1"

# 注文3の詳細を確認 (まだwaitingになっているはず)
curl -X GET "${BASE_URL}/api/orders/3"
```

## 6-2. ユーザー/ディスプレイAPI: 待ち時間の取得

現在のフレーバーごとの待ち時間を取得します。

### `GET /api/wait-times`

```bash
curl -X GET "${BASE_URL}/api/wait-times"
```

## 7. スタッフAPI: 注文への通知設定

注文の通知先を設定します。

### `PUT /api/staff/orders/{id}/notification`

```bash
# 注文3にDiscord通知を追加
curl -X PUT "${BASE_URL}/api/staff/orders/3/notification" \
     -H "Content-Type: application/json" \
     -H "Authorization: Bearer ${STAFF_API_TOKEN}" \
     -d '{
          "channel": "discord",
          "target": "<YOUR_DISCORD_USER_ID>"
     }'
```

## 8. スタッフAPI: 注文の完了

注文を完了としてマークします。

### `POST /api/staff/orders/{id}/complete`

```bash
# 注文1を完了
curl -X POST "${BASE_URL}/api/staff/orders/1/complete" \
     -H "Authorization: Bearer ${STAFF_API_TOKEN}"
```

## 9. スタッフAPI: 注文のキャンセル

注文をキャンセルします。

### `POST /api/staff/orders/{id}/cancel`

```bash
# 注文3をキャンセル
curl -X POST "${BASE_URL}/api/staff/orders/3/cancel" \
     -H "Authorization: Bearer ${STAFF_API_TOKEN}"
```
