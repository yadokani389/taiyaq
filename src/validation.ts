import * as v from "valibot";
import { FLAVORS } from "./shared";

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
  v.strictObject({ type: v.literal("order"), items: counts(0) }),
  v.strictObject({ type: v.literal("production"), items: counts(0) }),
  v.strictObject({ type: v.literal("adjust"), items: counts(-100) }),
  v.strictObject({
    type: v.literal("complete"),
    orderId: v.pipe(v.string(), v.uuid()),
  }),
  v.strictObject({
    type: v.literal("cancel"),
    orderId: v.pipe(v.string(), v.uuid()),
  }),
  v.strictObject({
    type: v.literal("skip"),
    orderId: v.pipe(v.string(), v.uuid()),
  }),
  v.strictObject({
    type: v.literal("resume"),
    orderId: v.pipe(v.string(), v.uuid()),
  }),
  v.strictObject({
    type: v.literal("prioritize"),
    orderId: v.pipe(v.string(), v.uuid()),
  }),
  v.strictObject({
    type: v.literal("unprioritize"),
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
