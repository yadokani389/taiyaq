import { spawnSync } from "node:child_process";
import { databaseId } from "../cloudflare.config.ts";

const local = process.argv.includes("--local");
const result = spawnSync(
  "pnpm",
  [
    "exec",
    "cf",
    "d1",
    "migrations",
    "apply",
    databaseId,
    ...(local
      ? [
          "--local",
          "--persist-to",
          process.env.TAIYAQ_STATE_DIR ?? ".cloudflare/state",
        ]
      : []),
  ],
  { stdio: "inherit" },
);
if (result.error) throw result.error;
process.exit(result.status ?? 1);
