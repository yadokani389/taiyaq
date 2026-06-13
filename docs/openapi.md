# OpenAPI / Swagger UI

バックエンドは `utoipa` でOpenAPIを生成し、`utoipa-swagger-ui` でSwagger UIを提供します。

## URL

バックエンドを起動した状態で、次のURLを参照します。

```text
http://127.0.0.1:38000/openapi.json
http://127.0.0.1:38000/swagger-ui/
```

`/openapi.json` はOpenAPI 3.1 JSONです。
`/swagger-ui/` はブラウザで確認するAPIドキュメントです。

## 認証

スタッフ向けAPIは `Authorization` ヘッダーにBearer tokenが必要です。

```text
Authorization: Bearer <STAFF_API_TOKEN>
```

`STAFF_API_TOKEN` はサーバー側の環境変数で設定します。

## 更新方法

APIのリクエスト、レスポンス、パス、ステータスコードを変更したら、Rustコード側のOpenAPI定義も同時に更新します。

- request / response型を変更した場合は `ToSchema` 対象の型を更新する
- endpointを追加・変更した場合は handler の `#[utoipa::path]` を更新する
- schemaやpathの一覧を変更した場合は `backend/src/api/openapi.rs` を更新する
- Swagger UIで使うOpenAPIは `build_openapi()` から生成する

## 設計方針

現在の規模では、`backend/src/api/openapi.rs` に単一のOpenAPI定義を置いています。
将来APIグループが増えた場合は、handlerごとに `ApiDoc` を定義して `build_openapi()` でmergeする構成へ移行します。

移行の目安は次の通りです。

- handlerが5個以上になる
- endpointが20個を超える
- `openapi.rs` のpath/schema一覧が探しづらくなる
- APIグループごとに変更頻度や担当範囲が分かれる

## 注意点

`EnumMap` は `utoipa` が直接schema化できないため、一部のレスポンスはOpenAPI上で `object` として表現しています。
より厳密なschemaが必要になった場合は、API専用DTOへ変換する形にします。
