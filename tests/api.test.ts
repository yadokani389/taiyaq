import { afterAll, beforeAll, expect, it } from "vitest";
import { build } from "esbuild";
import { Miniflare } from "miniflare";
import { readFile, readdir } from "node:fs/promises";
import {
  zeroCounts,
  type Command,
  type CommandResult,
  type State,
} from "../src/shared";
import { execute, history, loadState } from "../src/server/store";

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
  for (const file of (await readdir("migrations")).sort()) {
    if (file === "0006_continuous_orders.sql") {
      await db.batch([
        db.prepare(
          "INSERT INTO days (id, last_number, next_sequence) VALUES ('2026-10-25', 7, 2)",
        ),
        db
          .prepare(
            "INSERT INTO inventory (day, counts) VALUES ('2026-10-25', ?)",
          )
          .bind(JSON.stringify({ ...zeroCounts(), tsubuan: 4 })),
        db
          .prepare(
            "INSERT INTO orders (id, day, sequence, number, status, items, allocated, created_at) VALUES (?, '2026-10-25', 1, 7, 'waiting', ?, ?, '2026-10-25T00:00:00Z')",
          )
          .bind(
            crypto.randomUUID(),
            JSON.stringify({ ...zeroCounts(), tsubuan: 1 }),
            JSON.stringify(zeroCounts()),
          ),
        db
          .prepare(
            "INSERT INTO operations (id, request, response, revision) VALUES (?, '{}', '{}', 1)",
          )
          .bind(crypto.randomUUID()),
        db.prepare("UPDATE app_state SET revision = 1 WHERE id = 1"),
        db.prepare(
          "UPDATE flavors SET cooking_minutes = 12 WHERE id = 'imoan'",
        ),
      ]);
    }
    const sql = await readFile(`migrations/${file}`, "utf8");
    await db.exec(sql.replace(/\n/g, " "));
  }
}, 30000);
afterAll(async () => {
  await mf?.dispose();
});

it("resets legacy orders, stock and numbering while preserving sales settings", async () => {
  const db = await mf.getD1Database("DB");
  expect(await loadState(db)).toMatchObject({
    revision: 0,
    lastNumber: 99,
    orders: [],
    stock: zeroCounts(),
    flavors: expect.arrayContaining([
      { id: "imoan", enabled: false, cookingMinutes: 12, perBatch: 2 },
    ]),
  });
  expect(
    await db.prepare("SELECT COUNT(*) AS count FROM operations").first("count"),
  ).toBe(0);
  expect(
    await db
      .prepare(
        "SELECT COUNT(*) AS count FROM sqlite_master WHERE type = 'table' AND name IN ('days', 'inventory')",
      )
      .first("count"),
  ).toBe(0);
});

it("runs concurrent operations atomically, deduplicates retries and preserves history", async () => {
  expect((await request("/api/staff/state", undefined, false)).status).toBe(
    401,
  );
  const status = await request("/api/status", undefined, false);
  expect(status.status).toBe(200);
  expect(await status.json()).toMatchObject({ orders: [] });
  const first = (
    await command({
      type: "order",
      items: { ...zeroCounts(), tsubuan: 1, custard: 1 },
    })
  ).order!;
  const production: Command = {
    type: "production",
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
      items: { ...zeroCounts(), tsubuan: 1 },
    })
  ).order!;
  expect(second.status).toBe("ready");
  const skip: Command = { type: "skip", orderId: first.id };
  const skipId = crypto.randomUUID();
  await Promise.all([command(skip, skipId), command(skip, skipId)]);
  const skipped = (await (await request("/api/staff/state")).json()) as State;
  expect(skipped.orders.find(({ id }) => id === first.id)).toMatchObject({
    status: "skipped",
    allocated: zeroCounts(),
    finishedAt: null,
  });
  expect(skipped.stock.tsubuan).toBe(1);
  expect(
    await (await request(`/api/orders/${first.id}`, undefined, false)).json(),
  ).toMatchObject({
    order: { id: first.id, number: 100, status: "ready" },
  });
  expect(
    await (await request("/api/status", undefined, false)).json(),
  ).toMatchObject({
    orders: expect.arrayContaining([
      { id: first.id, number: 100, status: "ready" },
    ]),
  });
  await command({ type: "resume", orderId: first.id });
  const restored = (await (await request("/api/staff/state")).json()) as State;
  expect(restored.orders.find(({ id }) => id === first.id)).toMatchObject({
    status: "waiting",
  });
  expect(restored.orders.find(({ id }) => id === second.id)!.status).toBe(
    "ready",
  );
  expect(
    await (await request(`/api/orders/${first.id}`, undefined, false)).json(),
  ).toMatchObject({
    order: { id: first.id, status: "waiting" },
  });
  const collision = await request("/api/staff/commands", {
    operationId,
    command: { ...production, items: { ...zeroCounts(), tsubuan: 3 } },
  });
  expect(collision.status).toBe(409);
  const concurrent = await Promise.all(
    Array.from({ length: 4 }, () =>
      command({
        type: "order",
        items: { ...zeroCounts(), custard: 1 },
      }),
    ),
  );
  expect(new Set(concurrent.map(({ order }) => order!.number)).size).toBe(4);
  expect(
    (
      await request("/api/staff/commands", {
        operationId: crypto.randomUUID(),
        command: { type: "complete", orderId: first.id },
      })
    ).status,
  ).toBe(409);
  await command({ type: "complete", orderId: second.id });
  const db = await mf.getD1Database("DB");
  await expect(
    db
      .prepare("UPDATE orders SET number = 0 WHERE id = ?")
      .bind(second.id)
      .run(),
  ).rejects.toThrow("CHECK");
  await expect(
    db
      .prepare("UPDATE orders SET number = ? WHERE id = ?")
      .bind(first.number, second.id)
      .run(),
  ).rejects.toThrow("UNIQUE");
  const snapshot = (await (await request("/api/staff/state")).json()) as State;
  expect(snapshot.stock.tsubuan).toBe(0);
  expect(
    snapshot.orders.find(({ id }) => id === first.id)!.allocated.tsubuan,
  ).toBe(1);
  const oldOrder = (await (
    await request(`/api/orders/${second.id}`, undefined, false)
  ).json()) as {
    order: { status: string };
  };
  expect(oldOrder.order.status).toBe("completed");
  expect(oldOrder.order).not.toHaveProperty("allocated");
  expect(oldOrder.order).not.toHaveProperty("sequence");
  expect(oldOrder.order).not.toHaveProperty("createdAt");
  expect(
    (await (await request("/api/staff/history")).json()) as unknown[],
  ).toHaveLength(6);
  await command({
    type: "flavor",
    flavor: { ...snapshot.flavors[1]!, cookingMinutes: 15, perBatch: 1 },
  });
  const priority = concurrent[3]!.order!;
  const nextPriority = concurrent[0]!.order!;
  const prioritize: Command = {
    type: "prioritize",
    orderId: priority.id,
  };
  const priorityOperationId = crypto.randomUUID();
  const [designated, duplicate] = await Promise.all([
    command(prioritize, priorityOperationId),
    command(prioritize, priorityOperationId),
  ]);
  expect(duplicate).toEqual(designated);
  await command({ type: "prioritize", orderId: nextPriority.id });
  await command(prioritize);
  const prioritized = (await (
    await request("/api/staff/state")
  ).json()) as State;
  const currentPriority = prioritized.orders.find(
    ({ id }) => id === priority.id,
  )!;
  expect(currentPriority.prioritySequence).toBe(
    designated.order!.prioritySequence,
  );
  expect(currentPriority.prioritySequence!).toBeLessThan(
    prioritized.orders.find(({ id }) => id === nextPriority.id)!
      .prioritySequence!,
  );
  expect(await history(db)).toContainEqual(currentPriority);
  const publicPriority = (await (
    await request(`/api/orders/${priority.id}`, undefined, false)
  ).json()) as {
    order: Record<string, unknown>;
    waitMinutes: number;
  };
  expect(publicPriority.waitMinutes).toBe(0);
  expect(publicPriority.order.status).toBe("waiting");
  expect(publicPriority.order).not.toHaveProperty("prioritySequence");
  expect(
    await (await request(`/api/orders/${first.id}`, undefined, false)).json(),
  ).toMatchObject({ waitMinutes: 30 });
  await command({
    type: "production",
    items: { ...zeroCounts(), custard: 1 },
  });
  const ready = (await (await request("/api/staff/state")).json()) as State;
  expect(ready.orders.find(({ id }) => id === priority.id)).toMatchObject({
    status: "ready",
    prioritySequence: null,
  });
  expect(await command(prioritize, priorityOperationId)).toEqual(designated);
  const beforeInvalidAdjustment = await loadState(db);
  const invalidAdjustment = await request("/api/staff/commands", {
    operationId: crypto.randomUUID(),
    command: {
      type: "adjust",
      items: { ...zeroCounts(), tsubuan: 1, custard: -1 },
    },
  });
  expect(invalidAdjustment.status).toBe(409);
  expect(await loadState(db)).toEqual(beforeInvalidAdjustment);
  for (const type of ["prioritize", "unprioritize"] as const) {
    expect(
      (
        await request("/api/staff/commands", {
          operationId: crypto.randomUUID(),
          command: { type, orderId: priority.id },
        })
      ).status,
    ).toBe(409);
  }
  await expect(
    db
      .prepare("UPDATE orders SET priority_sequence = 1 WHERE id = ?")
      .bind(priority.id)
      .run(),
  ).rejects.toThrow("CHECK");
  await command({ type: "prioritize", orderId: first.id });
  const finalState = await loadState(db);
  expect(finalState.orders.find(({ id }) => id === priority.id)).toMatchObject({
    status: "ready",
    allocated: { custard: 1 },
  });
}, 30000);

it("retains orders, stock, priorities, numbering and retry results across midnight", async () => {
  const db = await mf.getD1Database("DB");
  await db.prepare("UPDATE app_state SET last_number = 998 WHERE id = 1").run();
  const beforeMidnight = new Date("2040-05-01T14:59:59.999Z");
  const midnight = new Date("2040-05-01T15:00:00.000Z");
  const operationId = crypto.randomUUID();
  const request: Command = {
    type: "order",
    items: { ...zeroCounts(), tsubuan: 1, custard: 1 },
  };
  const first = await execute(db, operationId, request, beforeMidnight);
  expect(first.order!.number).toBe(999);
  await execute(
    db,
    crypto.randomUUID(),
    { type: "prioritize", orderId: first.order!.id },
    beforeMidnight,
  );
  await execute(
    db,
    crypto.randomUUID(),
    { type: "production", items: { ...zeroCounts(), tsubuan: 20 } },
    beforeMidnight,
  );
  const previous = await loadState(db);
  expect(
    previous.orders.find(({ id }) => id === first.order!.id),
  ).toMatchObject({
    status: "waiting",
    allocated: { tsubuan: 1, custard: 0 },
    prioritySequence: expect.any(Number),
  });
  const orders = await Promise.all(
    Array.from({ length: 2 }, () =>
      execute(
        db,
        crypto.randomUUID(),
        {
          type: "order",
          items: { ...zeroCounts(), tsubuan: 1 },
        },
        midnight,
      ),
    ),
  );
  expect(
    orders.map(({ order }) => order!.number).sort((a, b) => a - b),
  ).toEqual([1000, 1001]);
  const next = await loadState(db);
  expect(next.lastNumber).toBe(1001);
  expect(next.stock.tsubuan).toBe(previous.stock.tsubuan - 2);
  expect(next.flavors).toEqual(previous.flavors);
  expect(next.orders.find(({ id }) => id === first.order!.id)).toEqual(
    previous.orders.find(({ id }) => id === first.order!.id),
  );
  expect(await execute(db, operationId, request, midnight)).toEqual(first);
  const all = await history(db);
  expect(all).toHaveLength(9);
  expect(all.map(({ number }) => number)).toEqual([
    1001, 1000, 999, 105, 104, 103, 102, 101, 100,
  ]);
  expect(all.find(({ id }) => id === first.order!.id)?.createdAt).toBe(
    beforeMidnight.toISOString(),
  );
  expect(all[0]?.createdAt).toBe(midnight.toISOString());
});
