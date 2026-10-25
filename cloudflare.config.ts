import { bindings, defineConfig } from "cf/config";

export const databaseId = "c0768502-882a-46dc-9dc5-0a2d5c9af428";

export default defineConfig({
  worker: {
    name: "taiyaq",
    entrypoint: "./src/server/index.ts",
    compatibilityDate: "2026-07-30",
    domains: ["taiyaq.yadokani389.net"],
    workersDev: false,
    previewUrls: false,
    observability: { enabled: true },
    env: {
      DB: bindings.d1({ name: "taiyaq", id: databaseId }),
      STAFF_API_TOKEN: bindings.secret(),
    },
  },
});
