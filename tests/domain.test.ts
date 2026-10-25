import { describe, expect, it } from "vitest";
import {
  FLAVORS,
  zeroCounts,
  type Command,
  type Counts,
  type State,
} from "../src/shared";
import { applyCommand } from "../src/server/domain";
import * as v from "valibot";
import { commandSchema } from "../src/validation";

function state(): State {
  return {
    revision: 0,
    day: { id: "2026-10-25", lastNumber: 0, nextSequence: 1 },
    stock: zeroCounts(),
    orders: [],
    flavors: FLAVORS.map(({ id }) => ({
      id,
      enabled: true,
      cookingMinutes: 15,
      perBatch: 10,
    })),
  };
}
const counts = (values: Partial<Counts>): Counts => ({
  ...zeroCounts(),
  ...values,
});
const run = (s: State, c: Command) =>
  applyCommand(s, c, "2026-10-25T01:00:00Z", crypto.randomUUID());
const order = (s: State, items: Partial<Counts>) =>
  run(s, { type: "order", day: s.day.id, items: counts(items) }).order!;

describe("order allocation", () => {
  it("reserves each flavor in arrival order while allowing independent flavors through", () => {
    const s = state();
    const first = order(s, { tsubuan: 2, custard: 1 });
    const second = order(s, { tsubuan: 1 });
    const third = order(s, { kurikinton: 1 });
    run(s, {
      type: "production",
      day: s.day.id,
      items: counts({ tsubuan: 1, kurikinton: 1 }),
    });
    expect(first.allocated.tsubuan).toBe(1);
    expect(second.allocated.tsubuan).toBe(0);
    expect(third.status).toBe("ready");
    run(s, {
      type: "production",
      day: s.day.id,
      items: counts({ tsubuan: 2 }),
    });
    expect(first.allocated.tsubuan).toBe(2);
    expect(first.status).toBe("waiting");
    expect(second.status).toBe("ready");
    run(s, { type: "cancel", day: s.day.id, orderId: first.id });
    expect(s.stock.tsubuan).toBe(2);
    expect(second.status).toBe("ready");
  });
  it("corrects inventory without restoring delivered items or completing unfinished orders", () => {
    const s = state();
    const first = order(s, { tsubuan: 1 });
    expect(() =>
      run(s, { type: "complete", day: s.day.id, orderId: first.id }),
    ).toThrow("全品");
    run(s, {
      type: "production",
      day: s.day.id,
      items: counts({ tsubuan: 2 }),
    });
    run(s, { type: "complete", day: s.day.id, orderId: first.id });
    expect(() =>
      run(s, { type: "cancel", day: s.day.id, orderId: first.id }),
    ).toThrow("終了");
    const second = order(s, { tsubuan: 1 });
    run(s, { type: "adjust", day: s.day.id, items: counts({ tsubuan: -1 }) });
    expect(second.status).toBe("waiting");
    expect(() =>
      run(s, {
        type: "adjust",
        day: s.day.id,
        items: counts({ tsubuan: -1 }),
      }),
    ).toThrow("受け渡し済み");
  });
  it("cycles tickets, skips occupied numbers and refuses a full pool", () => {
    const s = state();
    for (let n = 1; n <= 100; n++)
      expect(order(s, { tsubuan: 1 }).number).toBe(n);
    expect(() => order(s, { tsubuan: 1 })).toThrow("空き");
    run(s, { type: "cancel", day: s.day.id, orderId: s.orders[41]!.id });
    expect(order(s, { custard: 1 }).number).toBe(42);
    expect(s.orders[0]!.number).toBe(1);
  });
  it("keeps accepted orders when a flavor is stopped and rejects stale dates", () => {
    const s = state();
    const existing = order(s, { imoan: 1 });
    run(s, { type: "flavor", flavor: { ...s.flavors[3]!, enabled: false } });
    expect(() => order(s, { imoan: 1 })).toThrow("停止");
    run(s, { type: "production", day: s.day.id, items: counts({ imoan: 1 }) });
    expect(existing.status).toBe("ready");
    expect(() =>
      run(s, {
        type: "production",
        day: "2026-10-24",
        items: counts({ tsubuan: 1 }),
      }),
    ).toThrow("営業日");
  });
});

it("validates empty, fractional, unknown and out-of-range input at the API boundary", () => {
  for (const value of [
    null,
    { type: "order", day: "2026-02-30", items: { tsubuan: 1 } },
    { type: "order", day: "2026-99-99", items: { tsubuan: 1 } },
    { type: "order", day: "2026-10-25", items: {} },
    { type: "order", day: "2026-10-25", items: { tsubuan: -1 } },
    { type: "order", day: "2026-10-25", items: { tsubuan: 1.5 } },
    { type: "order", day: "2026-10-25", items: { tsubuan: 101 } },
    { type: "order", day: "2026-10-25", items: { other: 1 } },
    {
      type: "flavor",
      flavor: { id: "tsubuan", enabled: true, cookingMinutes: 0, perBatch: 10 },
    },
  ])
    expect(v.safeParse(commandSchema, value).success).toBe(false);
  expect(
    v.parse(commandSchema, {
      type: "order",
      day: "2026-10-25",
      items: { imoan: 2 },
    }),
  ).toEqual({
    type: "order",
    day: "2026-10-25",
    items: counts({ imoan: 2 }),
  });
});
