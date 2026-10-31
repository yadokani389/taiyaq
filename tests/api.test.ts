import { afterAll, beforeAll, expect, it } from "vitest";
import { build } from "esbuild";
import { Miniflare } from "miniflare";
import { readFile, readdir } from "node:fs/promises";
import {
  zeroCounts,
  japanDate,
  type Command,
  type CommandResult,
  type State,
} from "../src/shared";
import { execute, history, loadState } from "../src/server/store";

let mf: Miniflare;
const today = japanDate();
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
    external: ["cloudflare:workers"],
  });
  mf = new Miniflare({
    modules: true,
    script: worker.outputFiles[0]!.text,
    compatibilityDate: "2026-07-30",
    d1Databases: ["DB"],
    durableObjects: { STAFF_LIVE: { className: "StaffLive", useSQLite: true } },
    bindings: { STAFF_API_TOKEN: token },
  });
  const db = await mf.getD1Database("DB");
  for (const file of (await readdir("migrations")).sort()) {
    const sql = await readFile(`migrations/${file}`, "utf8");
    await db.exec(sql.replace(/\n/g, " "));
  }
}, 30000);
afterAll(async () => {
  await mf?.dispose();
});

it("runs concurrent operations atomically, deduplicates retries and preserves daily history", async () => {
  expect((await request("/api/staff/state", undefined, false)).status).toBe(
    401,
  );
  const status = await request("/api/status", undefined, false);
  expect(status.status).toBe(200);
  expect(await status.json()).toMatchObject({ day: today, orders: [] });
  const first = (
    await command({
      type: "order",
      day: today,
      items: { ...zeroCounts(), tsubuan: 1, custard: 1 },
    })
  ).order!;
  expect(first.number).toBe(100);
  const production: Command = {
    type: "production",
    day: today,
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
      day: today,
      items: { ...zeroCounts(), tsubuan: 1 },
    })
  ).order!;
  expect(second.number).toBe(101);
  expect(second.status).toBe("ready");
  const skip: Command = { type: "skip", day: today, orderId: first.id };
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
  await command({ type: "resume", day: today, orderId: first.id });
  const restored = (await (await request("/api/staff/state")).json()) as State;
  expect(restored.orders.find(({ id }) => id === first.id)).toMatchObject({
    sequence: first.sequence,
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
        day: today,
        items: { ...zeroCounts(), custard: 1 },
      }),
    ),
  );
  expect(
    concurrent.map(({ order }) => order!.number).sort((a, b) => a - b),
  ).toEqual([102, 103, 104, 105]);
  expect(
    (
      await request("/api/staff/commands", {
        operationId: crypto.randomUUID(),
        command: { type: "complete", day: today, orderId: first.id },
      })
    ).status,
  ).toBe(409);
  await command({ type: "complete", day: today, orderId: second.id });
  const db = await mf.getD1Database("DB");
  await expect(
    db
      .prepare("UPDATE days SET last_number = -1 WHERE id = ?")
      .bind(today)
      .run(),
  ).rejects.toThrow("CHECK");
  await expect(
    db
      .prepare("UPDATE orders SET number = 0 WHERE id = ?")
      .bind(second.id)
      .run(),
  ).rejects.toThrow("CHECK");
  const snapshot = (await (await request("/api/staff/state")).json()) as State;
  expect(snapshot.stock.tsubuan).toBe(0);
  expect(
    snapshot.orders.find(({ id }) => id === first.id)!.allocated.tsubuan,
  ).toBe(1);
  const oldOrder = (await (
    await request(`/api/orders/${second.id}`, undefined, false)
  ).json()) as {
    order: { status: string; day: string };
  };
  expect(oldOrder.order.status).toBe("completed");
  expect(oldOrder.order.day).toBe(today);
  expect(oldOrder.order).not.toHaveProperty("allocated");
  expect(oldOrder.order).not.toHaveProperty("sequence");
  expect(oldOrder.order).not.toHaveProperty("createdAt");
  expect(
    (await (await request(`/api/staff/history/${today}`)).json()) as unknown[],
  ).toHaveLength(6);
  await command({
    type: "flavor",
    flavor: { ...snapshot.flavors[1]!, cookingMinutes: 15, perBatch: 1 },
  });
  const priority = concurrent[3]!.order!;
  const nextPriority = concurrent[0]!.order!;
  const prioritize: Command = {
    type: "prioritize",
    day: today,
    orderId: priority.id,
  };
  const priorityOperationId = crypto.randomUUID();
  const [designated, duplicate] = await Promise.all([
    command(prioritize, priorityOperationId),
    command(prioritize, priorityOperationId),
  ]);
  expect(duplicate).toEqual(designated);
  await command({ type: "prioritize", day: today, orderId: nextPriority.id });
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
  expect(await history(db, today)).toContainEqual(currentPriority);
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
    day: today,
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
      day: today,
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
          command: { type, day: today, orderId: priority.id },
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
  await command({ type: "prioritize", day: today, orderId: first.id });
  const finalState = await loadState(db);
  expect(finalState.orders.find(({ id }) => id === priority.id)).toMatchObject({
    status: "ready",
    allocated: { custard: 1 },
  });
  const cancelled = (
    await command({
      type: "order",
      day: today,
      items: { ...zeroCounts(), tsubuan: 1 },
    })
  ).order!;
  expect(cancelled.number).toBe(106);
  await command({ type: "cancel", day: today, orderId: cancelled.id });
  const following = (
    await command({
      type: "order",
      day: today,
      items: { ...zeroCounts(), tsubuan: 1 },
    })
  ).order!;
  expect(following.number).toBe(107);
}, 30000);

it("switches at midnight in Japan while retaining history, settings and retry results", async () => {
  const db = await mf.getD1Database("DB");
  const beforeMidnight = new Date("2040-05-01T14:59:59.999Z");
  const midnight = new Date("2040-05-01T15:00:00.000Z");
  const operationId = crypto.randomUUID();
  const request: Command = {
    type: "order",
    day: "2040-05-01",
    items: { ...zeroCounts(), tsubuan: 1, custard: 1 },
  };
  const first = await execute(db, operationId, request, beforeMidnight);
  expect(first.order!.number).toBe(100);
  await execute(
    db,
    crypto.randomUUID(),
    {
      type: "production",
      day: request.day,
      items: { ...zeroCounts(), tsubuan: 3 },
    },
    beforeMidnight,
  );
  const previous = await loadState(db, beforeMidnight);
  await execute(
    db,
    crypto.randomUUID(),
    {
      type: "flavor",
      flavor: { ...previous.flavors[0]!, cookingMinutes: 12, perBatch: 8 },
    },
    beforeMidnight,
  );
  const next = await loadState(db, midnight);
  expect(next.day).toEqual({
    id: "2040-05-02",
    lastNumber: 99,
    nextSequence: 1,
  });
  expect(next.stock).toEqual(zeroCounts());
  expect(next.orders).toEqual([]);
  expect(next.soldCount).toBe(0);
  expect(next.flavors[0]).toMatchObject({ cookingMinutes: 12, perBatch: 8 });
  expect((await history(db, request.day))[0]).toMatchObject({
    id: first.order!.id,
    status: "waiting",
    allocated: { tsubuan: 1, custard: 0 },
  });
  expect((await loadState(db, beforeMidnight)).stock.tsubuan).toBe(2);
  expect(await execute(db, operationId, request, midnight)).toEqual(first);
  await expect(
    execute(db, crypto.randomUUID(), request, midnight),
  ).rejects.toThrow("営業日");
  const orders = await Promise.all(
    Array.from({ length: 2 }, () =>
      execute(
        db,
        crypto.randomUUID(),
        {
          type: "order",
          day: "2040-05-02",
          items: { ...zeroCounts(), tsubuan: 1 },
        },
        midnight,
      ),
    ),
  );
  expect(
    orders.map(({ order }) => order!.number).sort((a, b) => a - b),
  ).toEqual([100, 101]);
  expect((await loadState(db, midnight)).orders).toHaveLength(2);
  expect(await history(db, request.day)).toHaveLength(1);
});

it("authenticates staff streams, pushes snapshots to every client and restores state on reconnect", async () => {
  const { sign } = await import("hono/jwt");
  const open = (ticket: string, origin = "http://localhost") =>
    mf.dispatchFetch(
      `http://localhost/api/staff/live?ticket=${encodeURIComponent(ticket)}`,
      {
        headers: { Upgrade: "websocket", Origin: origin },
      },
    );
  expect((await request("/api/staff/live-ticket", {}, false)).status).toBe(401);
  expect((await open("invalid")).status).toBe(401);
  const expired = await sign(
    { aud: "http://localhost/api/staff/live", exp: 1 },
    token,
  );
  expect((await open(expired)).status).toBe(401);
  const wrongAudience = await sign(
    { aud: "other", exp: Math.floor(Date.now() / 1000) + 30 },
    token,
  );
  expect((await open(wrongAudience)).status).toBe(401);

  async function connect() {
    const { ticket } = (await (
      await request("/api/staff/live-ticket", {})
    ).json()) as { ticket: string };
    expect((await open(ticket, "https://other.example")).status).toBe(403);
    const response = await open(ticket);
    expect(response.status).toBe(101);
    const socket = response.webSocket!;
    const messages: string[] = [];
    socket.addEventListener("message", (event) =>
      messages.push(event.data as string),
    );
    socket.accept();
    async function next() {
      await expect.poll(() => messages.length).toBeGreaterThan(0);
      return messages.shift()!;
    }
    return { socket, next };
  }

  const first = await connect();
  const second = await connect();
  const initial = JSON.parse(await first.next()) as {
    type: string;
    state: State;
  };
  expect(initial.type).toBe("state");
  expect(JSON.parse(await second.next())).toEqual(initial);
  first.socket.send("ping");
  expect(await first.next()).toBe("pong");
  const flavor = initial.state.flavors[0]!;
  const operationId = crypto.randomUUID();
  const change: Command = {
    type: "flavor",
    flavor: { ...flavor, enabled: !flavor.enabled },
  };
  const result = await command(change, operationId);
  const updated = JSON.parse(await first.next()) as { state: State };
  expect(JSON.parse(await second.next())).toEqual(updated);
  expect(updated.state.revision).toBe(initial.state.revision + 1);
  expect(updated.state.flavors[0]!.enabled).toBe(!flavor.enabled);
  expect(await command(change, operationId)).toEqual(result);
  expect(JSON.parse(await first.next())).toEqual(updated);
  expect(JSON.parse(await second.next())).toEqual(updated);
  first.socket.close();
  const reconnected = await connect();
  expect(JSON.parse(await reconnected.next())).toEqual(updated);
  reconnected.socket.send("sync");
  expect(JSON.parse(await reconnected.next())).toEqual(updated);
  const closed = new Promise<number>((resolve) => {
    reconnected.socket.addEventListener(
      "close",
      (event) => resolve(event.code),
      { once: true },
    );
  });
  reconnected.socket.send("unsupported");
  expect(await closed).toBe(1008);
  reconnected.socket.close();
  second.socket.close();
}, 15000);
