# SQLite / sqlx

## 概要

バックエンドはSQLiteを永続化に使います。
SQLは `sqlx::query!` でコンパイル時に検証します。

`query!` はビルド時にDBスキーマが必要です。
このリポジトリでは `.sqlx/` をコミットし、`SQLX_OFFLINE=true` でDBファイルなしでも検証できるようにします。

## デフォルト設定

デフォルトの `DATABASE_URL` は次の値です。

```text
sqlite://data/taiyaq.sqlite
```

起動時に `backend/migrations` のmigrationを実行します。

## SQL変更時の手順

`backend/migrations` または `sqlx::query!` のSQLを変更したら、`.sqlx/` を更新してください。

```sh
DATABASE_URL=sqlite://data/sqlx-prepare.sqlite sqlx database create
DATABASE_URL=sqlite://data/sqlx-prepare.sqlite sqlx migrate run --source backend/migrations
SQLX_OFFLINE=false DATABASE_URL=sqlite://data/sqlx-prepare.sqlite cargo sqlx prepare --manifest-path backend/Cargo.toml -- --all-targets
```

既存のprepare用DBを作り直したい場合は、`data/sqlx-prepare.sqlite` を削除してから同じ手順を実行します。
`data/` は `.gitignore` 済みなので、prepare用DBはコミットされません。

## 検証

通常の検証はオフラインメタデータを使います。
`backend/.cargo/config.toml` で `SQLX_OFFLINE=true` を設定しているため、通常は環境変数を明示する必要はありません。

```sh
cargo check --manifest-path backend/Cargo.toml
cargo clippy --manifest-path backend/Cargo.toml --all-targets
```

## 注意点

- `.sqlx/` は生成物ですが、ビルド再現性のためにコミットします。
- SQLやmigrationを変更して `.sqlx/` を更新し忘れると、オフラインビルドが古いスキーマを参照します。
- SQLiteの `INTEGER` はRust側では基本的に `i64` として受け取り、必要な箇所で `u32` や `usize` に変換します。
- SQLiteのnullable推論は保守的なので、DB制約上 `NOT NULL` のつもりでもRust側では `Option<T>` になる場合があります。
