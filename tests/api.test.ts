import { afterAll, beforeAll, expect, it } from "vitest";
import { build } from "esbuild";
import { Miniflare } from "miniflare";
import { readFile } from "node:fs/promises";
import {
  zeroCounts,
  type Command,
  type CommandResult,
  type State,
} from "../src/shared";

let mf: Miniflare;
const token = crypto.randomUUID();
async function request(path: string, body?: unknown, authenticated = true) {
  return mf.dispatchFetch(`http://localhost${path}`, {
    method: body ? "POST" : "GET",
    headers: {
      "Content-Type": "application/json",
      ...(authenticated ? { Authorization: `Bearer ${token}` } : {}),
    },
    ...(body ? { body: JSON.stringify(body) } : {}),
  });
}
async function command(
  command: Command,
  operationId = crypto.randomUUID(),
): Promise<CommandResult> {
  const response = await request("/api/staff/commands", {
    operationId,
    command,
  });
  const body = await response.json();
  expect(response.status, JSON.stringify(body)).toBe(200);
  return body as CommandResult;
}
beforeAll(async () => {
  const worker = await build({
    entryPoints: ["src/server/index.ts"],
    bundle: true,
    write: false,
    format: "esm",
    target: "es2022",
  });
  mf = new Miniflare({
    modules: true,
    script: worker.outputFiles[0]!.text,
    compatibilityDate: "2026-07-30",
    d1Databases: ["DB"],
    bindings: { STAFF_API_TOKEN: token },
  });
  const db = await mf.getD1Database("DB");
  const sql = await readFile("migrations/0001_initial.sql", "utf8");
  await db.exec(sql.replace(/\n/g, " "));
}, 30000);
afterAll(async () => {
  await mf?.dispose();
});

it("runs concurrent operations atomically, deduplicates retries and preserves daily history", async () => {
  expect((await request("/api/staff/state", undefined, false)).status).toBe(
    401,
  );
  expect((await request("/api/status", undefined, false)).status).toBe(200);
  await command({ type: "start", day: "2026-10-25" });
  const first = (
    await command({
      type: "order",
      day: "2026-10-25",
      items: { ...zeroCounts(), tsubuan: 1, custard: 1 },
    })
  ).order!;
  const production: Command = {
    type: "production",
    day: "2026-10-25",
    items: { ...zeroCounts(), tsubuan: 2 },
  };
  const operationId = crypto.randomUUID();
  await Promise.all([
    command(production, operationId),
    command(production, operationId),
  ]);
  const second = (
    await command({
      type: "order",
      day: "2026-10-25",
      items: { ...zeroCounts(), tsubuan: 1 },
    })
  ).order!;
  expect(second.status).toBe("ready");
  const collision = await request("/api/staff/commands", {
    operationId,
    command: { ...production, items: { ...zeroCounts(), tsubuan: 3 } },
  });
  expect(collision.status).toBe(409);
  const concurrent = await Promise.all(
    Array.from({ length: 4 }, () =>
      command({
        type: "order",
        day: "2026-10-25",
        items: { ...zeroCounts(), custard: 1 },
      }),
    ),
  );
  expect(new Set(concurrent.map(({ order }) => order!.number)).size).toBe(4);
  expect(
    (
      await request("/api/staff/commands", {
        operationId: crypto.randomUUID(),
        command: { type: "complete", day: "2026-10-25", orderId: first.id },
      })
    ).status,
  ).toBe(409);
  await command({ type: "complete", day: "2026-10-25", orderId: second.id });
  let snapshot = (await (await request("/api/staff/state")).json()) as State;
  expect(snapshot.stock.tsubuan).toBe(0);
  expect(
    snapshot.orders.find(({ id }) => id === first.id)!.allocated.tsubuan,
  ).toBe(1);
  for (const order of snapshot.orders)
    await command({ type: "cancel", day: "2026-10-25", orderId: order.id });
  await command({ type: "close", day: "2026-10-25" });
  await command({ type: "start", day: "2026-10-26" });
  snapshot = (await (await request("/api/staff/state")).json()) as State;
  expect(snapshot.stock).toEqual(zeroCounts());
  expect(snapshot.orders).toHaveLength(0);
  expect(
    (
      await command({
        type: "order",
        day: "2026-10-26",
        items: { ...zeroCounts(), custard: 1 },
      })
    ).order!.number,
  ).toBe(1);
  const oldOrder = (await (
    await request(`/api/orders/${second.id}`, undefined, false)
  ).json()) as {
    order: { status: string; day: string };
  };
  expect(oldOrder.order.status).toBe("completed");
  expect(oldOrder.order.day).toBe("2026-10-25");
  expect(oldOrder.order).not.toHaveProperty("allocated");
  expect(oldOrder.order).not.toHaveProperty("sequence");
  expect(oldOrder.order).not.toHaveProperty("createdAt");
  expect(
    (await (
      await request("/api/staff/history/2026-10-25")
    ).json()) as unknown[],
  ).toHaveLength(6);
}, 30000);
