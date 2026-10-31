import { expect, it } from "vitest";
import { Miniflare } from "miniflare";
import { readdir, readFile } from "node:fs/promises";
import { execute, loadState } from "../src/server/store";
import { zeroCounts } from "../src/shared";

it("restarts numbers at 100 for each separate daily sales day", async () => {
  const mf = new Miniflare({
    modules: true,
    script: "export default {fetch(){return new Response('ok')}}",
    compatibilityDate: "2026-07-30",
    d1Databases: ["DB"],
  });
  try {
    const db = await mf.getD1Database("DB");
    for (const file of (await readdir("migrations")).sort())
      await db.exec(
        (await readFile(`migrations/${file}`, "utf8")).replace(/\n/g, " "),
      );
    const before = new Date("2040-05-01T14:59:59Z");
    const after = new Date("2040-05-01T15:00:00Z");
    const items = { ...zeroCounts(), tsubuan: 1 };
    const first = await execute(
      db,
      crypto.randomUUID(),
      { type: "order", day: "2040-05-01", items },
      before,
    );
    expect(first.order?.number).toBe(100);
    expect((await loadState(db, after)).orders).toEqual([]);
    const second = await execute(
      db,
      crypto.randomUUID(),
      { type: "order", day: "2040-05-02", items },
      after,
    );
    expect(second.order?.number).toBe(100);
    expect((await loadState(db, after)).orders.map((o) => o.id)).toEqual([
      second.order?.id,
    ]);
    const concurrent = await Promise.all([
      execute(
        db,
        crypto.randomUUID(),
        { type: "order", day: "2040-05-01", items },
        before,
      ),
      execute(
        db,
        crypto.randomUUID(),
        { type: "order", day: "2040-05-02", items },
        after,
      ),
    ]);
    expect(
      concurrent.map(({ order }) => order!.number).sort((a, b) => a - b),
    ).toEqual([101, 101]);
  } finally {
    await mf.dispose();
  }
});
