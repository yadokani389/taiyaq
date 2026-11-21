# 開発とデプロイ

## 開発

```sh
pnpm install
cp .dev.vars.example .dev.vars
pnpm dev
```

`http://127.0.0.1:5173`で起動します。
スタッフ画面には`.dev.vars`の`STAFF_API_TOKEN`でログインします。

```sh
pnpm check
pnpm test:e2e
```

ブラウザーテストにはChromiumが必要です（`pnpm exec playwright install chromium`）。

## 文化祭後の静的デモ

```sh
pnpm build:demo
pnpm preview:demo
pnpm test:demo
```

静的デモの公開対象は `dist/demo/` だけです。通常の `pnpm run deploy` は注文管理用の Worker を公開します。デモの公開には使いません。

本番データを非公開で退避してから、`dist/demo/` を Worker スクリプトや D1 接続のない Workers Static Assets として公開します。404 ページを有効にし、公開先に `/`、`/staff`、`/display`、`/order` があり、`/api/status` が 404 になることを確認します。旧 Worker のドメイン設定と稼働状態を確認してから、不要になった Workers Paid の契約を見直します。

## 注文管理用 Worker のデプロイ

接続先のD1は`cloudflare.config.ts`に設定します。
本番用の`STAFF_API_TOKEN`をWorkerのsecretに設定してください。
初回は`pnpm run deploy --secrets-file <ファイル>`で渡せます。

```sh
pnpm db:migrate:remote
pnpm run deploy
```

`0006_continuous_orders.sql`は、日付による管理を廃止する際に既存の注文と在庫、操作履歴をリセットします。
次の整理券番号は100になり、味ごとの販売設定は保持します。
適用前に必要なデータを退避してください。

`0007_restore_daily_orders.sql`は連番方式から日別管理へ移行します。
適用前にD1をバックアップし、スタッフ全員の操作を止め、未確認の操作を解決してください。旧版の保留リクエストは新APIにそのまま再送できません。
DB変更中は旧Workerへのリクエストと処理中の操作を停止・排出し、マイグレーション後は新Workerをデプロイしてから利用を再開してください。
注文・在庫・操作履歴を残し、移行当日は従来の連番を続けます。翌日から100で開始します。
旧方式の在庫には日付が記録されていないため、移行前にスタッフが在庫の対象日を確認してください。移行処理は在庫を実行当日に割り当てます。
既存注文に移行当日以外の日本時間の日付が含まれる場合は誤った日付に分類しないよう移行を拒否します。日付をまたいだ場合はデータの扱いを確認し、移行方法を見直してください。
