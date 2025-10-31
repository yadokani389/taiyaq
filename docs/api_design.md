# API設計

たい焼き管理システム `taiyaq` のAPI設計です。

## 1. データモデル

### Order (注文)

```json
{
    "id": "integer",            // 注文番号 (予約番号)
    "items": [
        {
            "flavor": "'tsubuan' | 'custard' | 'kurikinton'", // 味 (Enumとして管理)
            "quantity": "integer" // 個数
        }
    ],
    "status": "string",         // 'waiting' | 'cooking' | 'ready' | 'completed' | 'cancelled'
    "orderedAt": "string",      // 注文日時 (ISO 8601)
    "readyAt": "string | null",   // 準備完了日時 (ISO 8601)
    "completedAt": "string | null",// 受け渡し完了日時 (ISO 8601)
    "is_priority": "boolean",   // 優先注文フラグ (割り込み予約など)
    "notify": [
        {
            "channel": "discord" | "line",
            "target": "string" // DiscordのID、またはLINEユーザーID
        }
    ] // 通知設定 (配列)
}
```

## 2. APIエンドポイント

### 【ユーザー・店頭ディスプレイ向けAPI】

#### `GET /api/orders/display`

店頭ディスプレイやユーザー向けWebページで、呼び出し中・調理中の番号を表示するためのAPIです。

- **レスポンス:**

  ```json
  {
    "ready": [
      // 受け取り可能
      { "id": 101 },
      { "id": 102 }
    ],
    "cooking": [
      // 現在調理中
      { "id": 103 },
      { "id": 104 }
    ],
    "waiting": [
      // 待機中
      { "id": 105 },
      { "id": 106 }
    ]
  }
  ```

#### `GET /api/orders/{id}`

ユーザーが自分の予約番号で、現在の状況と推定待ち時間を確認するためのAPIです。

- **パスパラメータ:**
  - `id`: 注文番号 (integer)
- **レスポンス:**

  ```json
  {
    "id": 105,
    "items": [
      { "flavor": "tsubuan", "quantity": 2 },
      { "flavor": "custard", "quantity": 1 }
    ],
    "status": "waiting", // 現在のステータス
    "orderedAt": "2025-10-30T13:16:25.169400804Z",
    "estimatedWaitMinutes": 15 // 受け取り可能になるまでの推定時間(分)。ready, completedの場合はnull
  }
  ```

  _推定時間の計算は、前の注文数や厨房の生産能力からバックエンドで計算することを想定しています。_

#### `GET /api/wait-times`

現在のフレーバーごとの待ち時間を取得するためのAPIです。ユーザーが注文前に待ち時間を確認するのに使用します。

- **レスポンス:**

  ```json
  {
    "waitTimes": {
      "tsubuan": 15,
      "custard": 30,
      "kurikinton": null
    }
  }
  ```

  _`estimatedWaitMinutes`が`null`の場合は、そのフレーバーが現在提供不可（例：バッチサイズが0）であることを示します。_

### 【スタッフ向けAPI】

**認証:**

すべてのスタッフ向けAPIエンドポイントは、`Authorization: Bearer <TOKEN>` ヘッダーによる認証を必要とします。
`<TOKEN>` はサーバーで設定されたAPIトークンです。
正しいトークンが提供されない場合、APIは `401 Unauthorized` を返します。

#### `GET /api/staff/flavors/config`

各味の調理時間と一度に焼ける数（バッチサイズ）の設定を全て取得します。

- **レスポンス:**

  ```json
  {
    "tsubuan": {
      "cookingTimeMinutes": 15,
      "quantityPerBatch": 9
    },
    "custard": {
      "cookingTimeMinutes": 15,
      "quantityPerBatch": 9
    },
    "kurikinton": {
      "cookingTimeMinutes": 15,
      "quantityPerBatch": 2
    }
  }
  ```

#### `PUT /api/staff/flavors/{flavor}`

各味の調理時間と一度に焼ける数（バッチサイズ）を設定します。

- **パスパラメータ:**
  - `flavor`: `tsubuan` | `custard` | `kurikinton`
- **リクエストボディ:**

  ```json
  {
    "cookingTimeMinutes": 10,
    "quantityPerBatch": 8
  }
  ```

- **レスポンス:** `200 OK` と更新された設定オブジェクト

#### `GET /api/staff/orders`

スタッフが管理画面で現在の注文一覧を確認するためのAPIです。

- **クエリパラメータ (任意):**
  - `status`: `waiting` や `cooking` など、ステータスで絞り込み可能 (例: `?status=waiting,cooking`)
- **レスポンス:** `[Order]` (Orderオブジェクトの配列)

#### `GET /api/staff/stock`

現在の未割り当ての在庫数を取得します。

- **レスポンス:**

  ```json
  {
    "tsubuan": 10,
    "custard": 5,
    "kurikinton": 0
  }
  ```

#### `POST /api/staff/orders`

新しい注文を作成します。

- **リクエストボディ:**

  ```json
  {
    "items": [
      { "flavor": "tsubuan", "quantity": 5 },
      { "flavor": "custard", "quantity": 10 }
    ],
    "is_priority": false // (任意) 優先注文にする場合はtrue
  }
  ```

- **レスポンス:** `201 Created` と作成された `Order` オブジェクト

#### `POST /api/staff/production`

スタッフが焼き上がったたい焼きの数を報告します。バックエンドはこの情報をもとに、待ち状態の注文を自動で「準備完了(ready)」状態に更新します。

- **リクエストボディ:**

  ```json
  {
    "items": [
      { "flavor": "tsubuan", "quantity": 20 },
      { "flavor": "custard", "quantity": 20 }
    ]
  }
  ```

- **レスポンス:**

  ```json
  {
    // この報告によって新たに「準備完了」になった注文番号のリスト
    "newlyReadyOrders": [101, 102],
    // どの注文にも割り当てられなかった在庫
    "unallocatedItems": [{ "flavor": "tsubuan", "quantity": 5 }]
  }
  ```

#### `POST /api/staff/orders/{id}/complete`

お客様が商品を受け取った際に、注文を「受け渡し完了(completed)」にするためのAPIです。

- **パスパラメータ:**
  - `id`: 注文番号 (integer)
- **リクエストボディ:** (なし)
- **レスポンス:** 更新された `Order` オブジェクト

#### `POST /api/staff/orders/{id}/cancel`

注文をキャンセルするためのAPIです。

- **パスパラメータ:**
  - `id`: 注文番号 (integer)
- **リクエストボディ:** (なし)
- **レスポンス:** 更新された `Order` オブジェクト (statusが `cancelled` になる)

#### `PUT /api/staff/orders/{id}/priority`

既存の注文の優先度を更新します。

- **パスパラメータ:**
  - `id`: 注文番号 (integer)
- **リクエストボディ:**

  ```json
  {
    "is_priority": true
  }
  ```

- **レスポンス:** `200 OK` と更新された `Order` オブジェクト

#### `PUT /api/staff/orders/{id}/notification`

注文に通知先を登録します。

- **パスパラメータ:**
  - `id`: 注文番号 (integer)
- **リクエストボディ:**

  ```json
  {
      "channel": "discord" | "line",
      "target": "string" // DiscordのID、またはLINEユーザーID
  }
  ```

- **レスポンス:** `200 OK` と更新された `Order` オブジェクト

### 【LINE Bot向けWebhook】

#### `POST /line_callback`

LINE Messaging APIからのWebhookを受け取るエンドポイントです。

- **リクエストボディ:** LINE Messaging APIのWebhookイベントオブジェクト
- **機能:**
  - ユーザーからのメッセージを解析し、注文番号や通知設定用のメールアドレス/LINEユーザーIDを抽出します。
  - 抽出した情報に基づき、以下の処理を行います。
    - **通知設定**: ユーザーから提供されたメールアドレスまたはLINEユーザーIDを`PUT /api/orders/{id}/notification`エンドポイントを通じて注文に紐付け、通知先として設定します。
    - **注文ステータス照会**: ユーザーが問い合わせた注文番号の現在のステータスと推定待ち時間を`GET /api/orders/{id}`エンドポイントから取得し、LINEで返信します。
    - **その他**: 未知のコマンドや問い合わせに対しては、適切なヘルプメッセージを返信します。
- **レスポンス:** `200 OK`
