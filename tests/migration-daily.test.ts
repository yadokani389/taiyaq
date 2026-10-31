import { afterEach, expect, it } from "vitest";
import { Miniflare } from "miniflare";
import { readFile, readdir } from "node:fs/promises";
import { japanDate, zeroCounts } from "../src/shared";
import { execute, history, loadState } from "../src/server/store";

let mf: Miniflare;
async function database() {
  mf = new Miniflare({
    modules: true,
    script: "export default { fetch() { return new Response('ok') } }",
    compatibilityDate: "2026-07-30",
    d1Databases: ["DB"],
  });
  const db = await mf.getD1Database("DB");
  for (const file of (await readdir("migrations"))
    .sort()
    .filter((f) => f < "0007")) {
    await db.exec(
      (await readFile(`migrations/${file}`, "utf8")).replace(/\n/g, " "),
    );
  }
  return db;
}
async function migrate(db: D1Database) {
  const sql = await readFile(
    "migrations/0007_restore_daily_orders.sql",
    "utf8",
  );
  await db.exec(sql.replace(/\n/g, " "));
}
afterEach(async () => {
  await mf?.dispose();
});

it("retains continuous live data, numbers and operations in the current Japan day", async () => {
  const db = await database();
  const stock = JSON.stringify({ ...zeroCounts(), tsubuan: 7 });
  const items = JSON.stringify({ ...zeroCounts(), tsubuan: 2 });
  const allocated = JSON.stringify({ ...zeroCounts(), tsubuan: 1 });
  await db.batch([
    db
      .prepare(
        "UPDATE app_state SET revision = 42, last_number = 150, stock = ? WHERE id = 1",
      )
      .bind(stock),
    db.prepare("UPDATE flavors SET cooking_minutes = 12 WHERE id = 'imoan'"),
    db
      .prepare("INSERT INTO operations VALUES ('op', ?, ?, 42)")
      .bind('{"type":"old"}', '{"order":{"number":150}}'),
    ...[
      ["waiting", 101, "waiting", 3, null],
      ["ready", 110, "ready", null, null],
      ["skipped", 120, "skipped", null, null],
      ["completed", 130, "completed", null, "2026-01-01T02:00:00.000Z"],
      ["cancelled", 150, "cancelled", null, "2026-01-01T03:00:00.000Z"],
    ].map(([id, number, status, priority, finished]) =>
      db
        .prepare("INSERT INTO orders VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(
          id,
          number,
          status,
          priority,
          items,
          allocated,
          new Date().toISOString(),
          finished,
        ),
    ),
  ]);
  const oldOrders = (
    await db.prepare("SELECT * FROM orders ORDER BY number").all()
  ).results;
  const oldOperation = await db
    .prepare("SELECT * FROM operations WHERE id = 'op'")
    .first();
  const today = japanDate();
  await migrate(db);
  expect(japanDate()).toBe(today); // If migration crosses Japan midnight, retry the test.
  const state = await loadState(db);
  expect(state.revision).toBe(42);
  expect(state.stock).toEqual(JSON.parse(stock));
  expect(state.day).toEqual({ id: today, lastNumber: 150, nextSequence: 6 });
  expect(state.orders.map((order) => order.id)).toEqual([
    "waiting",
    "ready",
    "skipped",
  ]);
  expect(
    (
      await db
        .prepare(
          "SELECT id, number, status, priority_sequence, items, allocated, created_at, finished_at FROM orders ORDER BY number",
        )
        .all()
    ).results,
  ).toEqual(oldOrders);
  expect(
    await db.prepare("SELECT * FROM operations WHERE id = 'op'").first(),
  ).toEqual(oldOperation);
  expect(state.flavors.find((f) => f.id === "imoan")?.cookingMinutes).toBe(12);
  expect((await history(db, today)).map((order) => order.sequence)).toEqual([
    5, 4, 3, 2, 1,
  ]);
  const result = await execute(db, "new-op", {
    type: "order",
    day: today,
    items: { ...zeroCounts(), custard: 1 },
  });
  expect(result.order).toMatchObject({ number: 151, sequence: 6, day: today });
  expect((await loadState(db)).revision).toBe(43);
  expect(await execute(db, "op", { type: "old" } as never)).toEqual({
    order: { number: 150 },
  });
  const tomorrow = new Date(Date.now() + 24 * 60 * 60 * 1000);
  expect(await loadState(db, tomorrow)).toMatchObject({
    day: { id: japanDate(tomorrow), lastNumber: 99, nextSequence: 1 },
    stock: zeroCounts(),
    orders: [],
  });
  const tomorrowOrder = await execute(
    db,
    "tomorrow-op",
    {
      type: "order",
      day: japanDate(tomorrow),
      items: { ...zeroCounts(), custard: 1 },
    },
    tomorrow,
  );
  expect(tomorrowOrder.order).toMatchObject({ number: 100, sequence: 1 });
  expect(await history(db, today)).toHaveLength(6);
  expect((await db.prepare("PRAGMA foreign_key_check").all()).results).toEqual(
    [],
  );
  const plan = await db
    .prepare(
      "EXPLAIN QUERY PLAN SELECT * FROM orders WHERE day = ? ORDER BY sequence DESC",
    )
    .bind(today)
    .all<{ detail: string }>();
  expect(
    plan.results.some(({ detail }) => detail.includes("USING INDEX")),
  ).toBe(true);
  expect(
    plan.results.some(({ detail }) => detail.includes("TEMP B-TREE")),
  ).toBe(false);
});

it("refuses to relabel previous-day orders as the cutover day", async () => {
  const db = await database();
  const yesterday = new Date(Date.now() - 24 * 60 * 60 * 1000).toISOString();
  const counts = JSON.stringify(zeroCounts());
  await db
    .prepare(
      "INSERT INTO orders VALUES ('old', 100, 'waiting', NULL, ?, ?, ?, NULL)",
    )
    .bind(counts, counts, yesterday)
    .run();
  await expect(migrate(db)).rejects.toThrow();
  expect(await db.prepare("SELECT COUNT(*) AS n FROM orders").first("n")).toBe(
    1,
  );
});

it("migrates an empty continuous database and starts daily numbering at 100", async () => {
  const db = await database();
  await migrate(db);
  const state = await loadState(db);
  expect(state.day).toEqual({
    id: japanDate(),
    lastNumber: 99,
    nextSequence: 1,
  });
  expect(state.orders).toEqual([]);
  expect(state.stock).toEqual(zeroCounts());
  const result = await execute(db, "first", {
    type: "order",
    day: state.day.id,
    items: { ...zeroCounts(), tsubuan: 1 },
  });
  expect(result.order).toMatchObject({ number: 100, sequence: 1 });
});

it("preserves more than 99 active tickets and continues the migrated counter", async () => {
  const db = await database();
  const counts = JSON.stringify(zeroCounts());
  await db.batch([
    db.prepare("UPDATE app_state SET last_number = 219 WHERE id = 1"),
    ...Array.from({ length: 120 }, (_, index) =>
      db
        .prepare(
          "INSERT INTO orders VALUES (?, ?, 'waiting', NULL, ?, ?, datetime('now'), NULL)",
        )
        .bind(`order-${index + 100}`, index + 100, counts, counts),
    ),
  ]);
  await migrate(db);
  const state = await loadState(db);
  expect(state.orders).toHaveLength(120);
  expect(state.day).toMatchObject({ lastNumber: 219, nextSequence: 121 });
  const result = await execute(db, "overflow", {
    type: "order",
    day: state.day.id,
    items: { ...zeroCounts(), tsubuan: 1 },
  });
  expect(result.order?.number).toBe(220);
  expect((await loadState(db)).orders).toHaveLength(121);
});
