import { fileURLToPath } from "node:url";
import { cloudflare } from "@cloudflare/vite-plugin";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite-plus";

export default defineConfig(({ mode }) => ({
  plugins: [
    react({ compiler: true }),
    mode !== "demo" &&
      !process.env.VITEST &&
      cloudflare({
        persistState: {
          path: process.env.TAIYAQ_STATE_DIR ?? ".cloudflare/state",
        },
      }),
  ],
  resolve: {
    alias:
      mode === "demo"
        ? [
            {
              find: /^\.\/(?:api|useStaffState)$/,
              replacement: fileURLToPath(
                new URL("./src/client/demo.ts", import.meta.url),
              ),
            },
          ]
        : [],
  },
  build: mode === "demo" ? { outDir: "dist/demo" } : undefined,
  server: { host: "127.0.0.1", port: Number(process.env.TAIYAQ_PORT ?? 5173) },
  test: { include: ["tests/**/*.test.ts"] },
  lint: {
    options: { typeAware: true, typeCheck: true },
    ignorePatterns: ["backend/**", "frontend/**", ".cloudflare/**"],
  },
  fmt: {
    printWidth: 80,
    ignorePatterns: [
      "backend/**",
      "frontend/**",
      ".cloudflare/**",
      "pnpm-lock.yaml",
    ],
  },
}));
