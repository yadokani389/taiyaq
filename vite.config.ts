import { cloudflare } from "@cloudflare/vite-plugin";
import { defineConfig } from "vite-plus";

export default defineConfig({
  plugins: [
    cloudflare({
      persistState: {
        path: process.env.TAIYAQ_STATE_DIR ?? ".cloudflare/state",
      },
    }),
  ],
  server: { host: "127.0.0.1", port: Number(process.env.TAIYAQ_PORT ?? 5173) },
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
});
