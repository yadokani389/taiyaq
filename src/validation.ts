import * as v from "valibot";
import { FLAVORS } from "./shared";

const day = v.pipe(
  v.string(),
  v.regex(/^\d{4}-\d{2}-\d{2}$/),
  v.check((value) => {
    const date = new Date(`${value}T00:00:00Z`);
    return (
      Number.isFinite(date.getTime()) &&
      date.toISOString().slice(0, 10) === value
    );
  }, "営業日が正しくありません"),
);

function counts(min: number) {
  const quantity = v.optional(
    v.pipe(v.number(), v.integer(), v.minValue(min), v.maxValue(100)),
    0,
  );
  return v.pipe(
    v.strictObject({
      tsubuan: quantity,
      custard: quantity,
      kurikinton: quantity,
      imoan: quantity,
      ujimatcha: quantity,
    }),
    v.check(
      (items) => Object.values(items).some((value) => value !== 0),
      "数量を選んでください",
    ),
  );
}

export const commandSchema = v.variant("type", [
  v.strictObject({ type: v.literal("order"), day, items: counts(0) }),
  v.strictObject({ type: v.literal("production"), day, items: counts(0) }),
  v.strictObject({ type: v.literal("adjust"), day, items: counts(-100) }),
  v.strictObject({
    type: v.literal("complete"),
    day,
    orderId: v.pipe(v.string(), v.uuid()),
  }),
  v.strictObject({
    type: v.literal("cancel"),
    day,
    orderId: v.pipe(v.string(), v.uuid()),
  }),
  v.strictObject({
    type: v.literal("flavor"),
    flavor: v.strictObject({
      id: v.picklist(FLAVORS.map(({ id }) => id)),
      enabled: v.boolean(),
      cookingMinutes: v.pipe(
        v.number(),
        v.integer(),
        v.minValue(1),
        v.maxValue(120),
      ),
      perBatch: v.pipe(v.number(), v.integer(), v.minValue(1), v.maxValue(20)),
    }),
  }),
]);

export const commandRequestSchema = v.strictObject({
  operationId: v.pipe(v.string(), v.uuid()),
  command: commandSchema,
});
