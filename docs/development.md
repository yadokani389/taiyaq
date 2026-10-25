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

## デプロイ

接続先のD1は`cloudflare.config.ts`に設定します。
本番用の`STAFF_API_TOKEN`をWorkerのsecretに設定してください。
初回は`pnpm run deploy --secrets-file <ファイル>`で渡せます。

```sh
pnpm db:migrate:remote
pnpm run deploy
```
