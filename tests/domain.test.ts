import { describe, expect, it } from "vitest";
import {
  FLAVORS,
  zeroCounts,
  type Command,
  type Counts,
  type State,
} from "../src/shared";
import { applyCommand, estimateWait } from "../src/server/domain";
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
  it("corrects waiting inventory without taking ready or delivered items", () => {
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
    const before = structuredClone(s);
    expect(() =>
      run(s, { type: "adjust", day: s.day.id, items: counts({ tsubuan: -1 }) }),
    ).toThrow("受け取り可能");
    expect(s).toEqual(before);
    const third = order(s, { tsubuan: 2 });
    run(s, {
      type: "production",
      day: s.day.id,
      items: counts({ tsubuan: 1 }),
    });
    run(s, { type: "adjust", day: s.day.id, items: counts({ tsubuan: -1 }) });
    expect(second.status).toBe("ready");
    expect(second.allocated.tsubuan).toBe(1);
    expect(third.status).toBe("waiting");
    expect(third.allocated.tsubuan).toBe(0);
    expect(() =>
      run(s, {
        type: "adjust",
        day: s.day.id,
        items: counts({ tsubuan: -1 }),
      }),
    ).toThrow("受け渡し済み");
  });
  it("moves partial reservations to a priority order while keeping ready orders ready", () => {
    const s = state();
    const first = order(s, { tsubuan: 2, custard: 1 });
    run(s, {
      type: "production",
      day: s.day.id,
      items: counts({ tsubuan: 1 }),
    });
    const priority = order(s, { tsubuan: 1 });
    run(s, { type: "prioritize", day: s.day.id, orderId: priority.id });
    expect(first.allocated.tsubuan).toBe(0);
    expect(priority).toMatchObject({
      status: "ready",
      prioritySequence: null,
      sequence: 2,
      number: 2,
      allocated: { tsubuan: 1 },
    });
    run(s, { type: "prioritize", day: s.day.id, orderId: first.id });
    expect(first.status).toBe("waiting");
    expect(priority.status).toBe("ready");
    expect(priority.allocated.tsubuan).toBe(1);
    for (const type of ["prioritize", "unprioritize"] as const) {
      expect(() =>
        run(s, { type, day: s.day.id, orderId: priority.id }),
      ).toThrow("準備中");
    }
  });
  it("keeps priority in designation order across batches and sends re-designations to the back", () => {
    const s = state();
    const regular = order(s, { tsubuan: 3 });
    const first = order(s, { tsubuan: 2, custard: 1 });
    const second = order(s, { tsubuan: 2 });
    run(s, { type: "prioritize", day: s.day.id, orderId: second.id });
    run(s, { type: "prioritize", day: s.day.id, orderId: first.id });
    const originalPriority = second.prioritySequence;
    run(s, { type: "prioritize", day: s.day.id, orderId: second.id });
    expect(second.prioritySequence).toBe(originalPriority);
    run(s, { type: "unprioritize", day: s.day.id, orderId: second.id });
    run(s, { type: "prioritize", day: s.day.id, orderId: second.id });
    expect(first.prioritySequence!).toBeLessThan(second.prioritySequence!);
    run(s, {
      type: "production",
      day: s.day.id,
      items: counts({ tsubuan: 2 }),
    });
    expect(first.allocated.tsubuan).toBe(2);
    expect(first.prioritySequence).not.toBeNull();
    expect(second.allocated.tsubuan).toBe(0);
    const independent = order(s, { kurikinton: 1 });
    run(s, {
      type: "production",
      day: s.day.id,
      items: counts({ kurikinton: 1, custard: 1 }),
    });
    expect(independent.status).toBe("ready");
    expect(first.status).toBe("ready");
    expect(first.prioritySequence).toBeNull();
    run(s, {
      type: "production",
      day: s.day.id,
      items: counts({ tsubuan: 1 }),
    });
    expect(second.status).toBe("waiting");
    expect(second.allocated.tsubuan).toBe(1);
    expect(second.prioritySequence).not.toBeNull();
    run(s, {
      type: "production",
      day: s.day.id,
      items: counts({ tsubuan: 1 }),
    });
    expect(second.status).toBe("ready");
    expect(second.prioritySequence).toBeNull();
    expect(regular.allocated.tsubuan).toBe(0);
  });
  it("recalculates partial reservations when priority is removed", () => {
    const s = state();
    const regular = order(s, { tsubuan: 2 });
    const priority = order(s, { tsubuan: 2 });
    run(s, { type: "prioritize", day: s.day.id, orderId: priority.id });
    run(s, {
      type: "production",
      day: s.day.id,
      items: counts({ tsubuan: 1 }),
    });
    expect(priority.allocated.tsubuan).toBe(1);
    run(s, { type: "unprioritize", day: s.day.id, orderId: priority.id });
    expect(priority.prioritySequence).toBeNull();
    expect(priority.allocated.tsubuan).toBe(0);
    expect(regular.allocated.tsubuan).toBe(1);
  });
  it.each(["skip", "cancel"] as const)(
    "clears priority and releases reservations on %s",
    (type) => {
      const s = state();
      const regular = order(s, { tsubuan: 1 });
      const priority = order(s, { tsubuan: 2 });
      run(s, { type: "prioritize", day: s.day.id, orderId: priority.id });
      run(s, {
        type: "production",
        day: s.day.id,
        items: counts({ tsubuan: 1 }),
      });
      run(s, { type, day: s.day.id, orderId: priority.id });
      expect(priority.prioritySequence).toBeNull();
      expect(priority.allocated).toEqual(zeroCounts());
      expect(regular.status).toBe("ready");
      expect(() =>
        run(s, { type: "prioritize", day: s.day.id, orderId: priority.id }),
      ).toThrow();
      if (type === "skip") {
        run(s, { type: "resume", day: s.day.id, orderId: priority.id });
        expect(priority.prioritySequence).toBeNull();
        expect(priority.status).toBe("waiting");
        expect(regular.status).toBe("ready");
      }
    },
  );
  it("estimates waits using allocation priority instead of ticket order", () => {
    const s = state();
    s.flavors[0]!.perBatch = 1;
    const regular = order(s, { tsubuan: 2 });
    const priority = order(s, { tsubuan: 1 });
    run(s, { type: "prioritize", day: s.day.id, orderId: priority.id });
    expect(estimateWait(s, "tsubuan", priority)).toBe(15);
    expect(estimateWait(s, "tsubuan", regular)).toBe(45);
    expect(estimateWait(s, "tsubuan")).toBe(60);
    run(s, {
      type: "production",
      day: s.day.id,
      items: counts({ tsubuan: 1 }),
    });
    expect(estimateWait(s, "tsubuan", regular)).toBe(30);
    expect(priority.status).toBe("ready");
  });
  it("cycles tickets, skips occupied numbers and refuses a full pool", () => {
    const s = state();
    for (let n = 1; n <= 99; n++)
      expect(order(s, { tsubuan: 1 }).number).toBe(n);
    expect(s.day.lastNumber).toBe(99);
    expect(() => order(s, { tsubuan: 1 })).toThrow("空き");
    run(s, { type: "cancel", day: s.day.id, orderId: s.orders[0]!.id });
    expect(order(s, { custard: 1 }).number).toBe(1);
    run(s, { type: "cancel", day: s.day.id, orderId: s.orders[41]!.id });
    expect(order(s, { custard: 1 }).number).toBe(42);
    expect(s.orders[0]!.number).toBe(1);
  });
  it("releases skipped orders and restores their original priority without taking ready items", () => {
    const s = state();
    const absent = order(s, { tsubuan: 2 });
    const second = order(s, { tsubuan: 1 });
    run(s, {
      type: "production",
      day: s.day.id,
      items: counts({ tsubuan: 2 }),
    });
    expect(absent.status).toBe("ready");
    run(s, { type: "skip", day: s.day.id, orderId: absent.id });
    expect(absent.status).toBe("skipped");
    expect(absent.allocated).toEqual(zeroCounts());
    expect(absent.finishedAt).toBeNull();
    expect(second.status).toBe("ready");
    expect(s.stock.tsubuan).toBe(1);
    expect(() =>
      run(s, { type: "skip", day: s.day.id, orderId: absent.id }),
    ).toThrow("保留");
    expect(() =>
      run(s, { type: "complete", day: s.day.id, orderId: absent.id }),
    ).toThrow("保留");
    s.day.lastNumber = 99;
    const third = order(s, { tsubuan: 1 });
    expect(third.number).toBe(3);
    expect(third.status).toBe("ready");
    const fourth = order(s, { tsubuan: 1 });
    run(s, { type: "resume", day: s.day.id, orderId: absent.id });
    run(s, { type: "adjust", day: s.day.id, items: counts({ custard: 1 }) });
    run(s, { type: "adjust", day: s.day.id, items: counts({ custard: -1 }) });
    expect(absent.sequence).toBe(1);
    expect(absent.number).toBe(1);
    expect(absent.status).toBe("waiting");
    expect(second.status).toBe("ready");
    expect(second.allocated.tsubuan).toBe(1);
    expect(third.status).toBe("ready");
    expect(third.allocated.tsubuan).toBe(1);
    expect(() =>
      run(s, { type: "resume", day: s.day.id, orderId: absent.id }),
    ).toThrow("保留");
    run(s, {
      type: "adjust",
      day: s.day.id,
      items: counts({ tsubuan: 1 }),
    });
    expect(absent.allocated.tsubuan).toBe(1);
    expect(absent.status).toBe("waiting");
    expect(fourth.allocated.tsubuan).toBe(0);
    expect(second.status).toBe("ready");
    expect(third.status).toBe("ready");
    run(s, {
      type: "production",
      day: s.day.id,
      items: counts({ tsubuan: 1 }),
    });
    expect(absent.status).toBe("ready");
    expect(fourth.status).toBe("waiting");
    run(s, { type: "skip", day: s.day.id, orderId: second.id });
    expect(fourth.status).toBe("ready");
    run(s, { type: "cancel", day: s.day.id, orderId: second.id });
    expect(second.status).toBe("cancelled");
    expect(second.finishedAt).not.toBeNull();
    expect(s.stock.tsubuan).toBe(0);
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
    { type: "prioritize", day: "2026-10-25", orderId: "invalid" },
    { type: "unprioritize", day: "2026-02-30", orderId: crypto.randomUUID() },
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
