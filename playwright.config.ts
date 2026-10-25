import { defineConfig } from "@playwright/test";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

export default defineConfig({
  testDir: "tests/browser",
  outputDir: "/tmp/taiyaq-playwright-results",
  workers: 1,
  timeout: 60000,
  use: {
    launchOptions: {
      executablePath: process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH,
    },
    baseURL: "http://127.0.0.1:5174",
    viewport: { width: 1440, height: 1000 },
    trace: "retain-on-failure",
  },
  webServer: {
    command: "pnpm dev",
    url: "http://127.0.0.1:5174",
    timeout: 60000,
    env: {
      TAIYAQ_PORT: "5174",
      TAIYAQ_STATE_DIR: mkdtempSync(join(tmpdir(), "taiyaq-e2e-")),
    },
  },
});
