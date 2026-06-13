# API設計メモ

このドキュメントは、たい焼き管理システム `taiyaq` のAPI設計意図を残すためのメモです。
最新のAPI仕様、リクエスト、レスポンス、ステータスコードは、実装から生成されるOpenAPIを正とします。

- Swagger UI: `/swagger-ui/`
- OpenAPI JSON: `/openapi.json`
- OpenAPIの運用方法: `docs/openapi.md`

## APIの利用者

APIは主に次の利用者に分かれます。

- ユーザー・店頭ディスプレイ向け: 注文状況、受け取り可能番号、待ち時間を確認する
- スタッフ向け: 注文作成、焼き上がり登録、受け渡し完了、キャンセル、味ごとの設定を操作する
- Bot / Webhook向け: LINEやDiscordから通知登録、注文状況確認、通知送信を行う

## 認証方針

ユーザー・店頭ディスプレイ向けAPIは、店頭表示や注文番号による確認を想定して認証なしで公開します。

スタッフ向けAPIは、`Authorization: Bearer <STAFF_API_TOKEN>` による認証を必須にします。
トークンはサーバー側の環境変数 `STAFF_API_TOKEN` で設定します。

LINE webhookはLINE署名を検証します。

## 注文状態

注文状態は、注文受付から受け渡し完了までの業務フローを表します。

- `waiting`: 在庫割り当て待ち
- `cooking`: 調理中
- `ready`: 受け取り可能
- `completed`: 受け渡し完了
- `cancelled`: キャンセル済み

状態遷移や優先注文の扱いは、API handlerではなくusecase/domain層で決定します。
APIは現在の状態を返し、スタッフ操作をusecaseに渡す境界として扱います。

## 待ち時間

待ち時間は、現在の在庫、注文順、味ごとの調理時間、バッチサイズからバックエンドで計算します。
frontendは計算結果を表示するだけにし、待ち時間ロジックを重複実装しません。

## 通知

通知先は注文に紐付けます。
注文が `ready` になったタイミングで、登録済みの通知先へLINEまたはDiscordで通知します。

通知送信の成否はログとして保存し、注文状態そのものとは分離します。
通知失敗によって注文状態を巻き戻さない方針です。

## OpenAPI管理

OpenAPI定義は `utoipa` でRustコードから生成します。
手書きのendpoint一覧やJSON schemaはこのドキュメントでは管理しません。

APIを変更した場合は、handlerの `#[utoipa::path]` と `ToSchema` 対象の型を更新します。
現在は `backend/src/api/openapi.rs` の `build_openapi()` で単一のOpenAPI定義を生成します。

APIグループが増えて `openapi.rs` が見通しづらくなった場合は、handlerごとに `ApiDoc` を定義し、`build_openapi()` でmergeする構成へ移行します。
