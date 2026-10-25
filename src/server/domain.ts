import {
  FLAVORS,
  MAX_TICKET_NUMBER,
  zeroCounts,
  type Command,
  type CommandResult,
  type FlavorId,
  type State,
} from "../shared";

export class UserError extends Error {
  constructor(
    message: string,
    public status: 400 | 401 | 404 | 409 | 503 = 400,
  ) {
    super(message);
  }
}

export function allocate(state: State): void {
  const available = { ...state.stock };
  for (const order of state.orders) {
    if (order.status !== "waiting" && order.status !== "ready") continue;
    for (const { id } of FLAVORS) available[id] += order.allocated[id];
  }
  for (const order of state.orders.toSorted(
    (a, b) => a.sequence - b.sequence,
  )) {
    if (order.status !== "waiting" && order.status !== "ready") continue;
    for (const { id } of FLAVORS) {
      order.allocated[id] = Math.min(order.items[id], available[id]);
      available[id] -= order.allocated[id];
    }
    order.status = FLAVORS.every(
      ({ id }) => order.allocated[id] === order.items[id],
    )
      ? "ready"
      : "waiting";
  }
  state.stock = available;
}

export function applyCommand(
  state: State,
  command: Command,
  now: string,
  newId: string,
): CommandResult {
  if (command.type === "flavor") {
    const index = state.flavors.findIndex(({ id }) => id === command.flavor.id);
    if (index < 0) throw new UserError("味が見つかりません", 404);
    state.flavors[index] = command.flavor;
    return {};
  }
  if (state.day.id !== command.day) {
    throw new UserError("営業日が変わっています。画面を更新してください", 409);
  }
  if (command.type === "order") {
    for (const { id, enabled } of state.flavors) {
      if (!enabled && command.items[id] > 0)
        throw new UserError("受付を停止している味が含まれています", 409);
    }
    const occupied = new Set(
      state.orders
        .filter(({ status }) => status === "waiting" || status === "ready")
        .map(({ number }) => number),
    );
    let number = state.day.lastNumber;
    for (let attempts = 0; attempts < MAX_TICKET_NUMBER; attempts++) {
      number = (number % MAX_TICKET_NUMBER) + 1;
      if (!occupied.has(number)) break;
    }
    if (occupied.has(number))
      throw new UserError("整理券番号に空きがありません", 409);
    const order = {
      id: newId,
      day: state.day.id,
      sequence: state.day.nextSequence++,
      number,
      status: "waiting" as const,
      items: command.items,
      allocated: zeroCounts(),
      createdAt: now,
      finishedAt: null,
    };
    state.day.lastNumber = number;
    state.orders.push(order);
    allocate(state);
    return { order };
  }
  if (command.type === "production" || command.type === "adjust") {
    for (const { id } of FLAVORS) {
      const available =
        state.stock[id] +
        state.orders
          .filter(({ status }) => status === "waiting" || status === "ready")
          .reduce((sum, order) => sum + order.allocated[id], 0);
      if (available + command.items[id] < 0)
        throw new UserError("受け渡し済みの商品は在庫から減らせません", 409);
    }
    for (const { id } of FLAVORS) state.stock[id] += command.items[id];
    allocate(state);
    return {};
  }
  const order = state.orders.find(({ id }) => id === command.orderId);
  if (!order) throw new UserError("注文が見つかりません", 404);
  if (order.status === "completed" || order.status === "cancelled")
    throw new UserError("すでに終了した注文です", 409);
  if (command.type === "complete" && order.status !== "ready")
    throw new UserError("まだ全品が揃っていません", 409);
  if (command.type === "cancel") {
    for (const { id } of FLAVORS) state.stock[id] += order.allocated[id];
    order.allocated = zeroCounts();
  }
  order.status = command.type === "complete" ? "completed" : "cancelled";
  order.finishedAt = now;
  allocate(state);
  return { order };
}

export function estimateWait(
  state: State,
  flavorId: FlavorId,
  throughSequence = Infinity,
): number | null {
  const flavor = state.flavors.find(({ id }) => id === flavorId);
  if (!flavor) return null;
  const demand = state.orders
    .filter(
      ({ status, sequence }) =>
        status === "waiting" && sequence <= throughSequence,
    )
    .reduce(
      (sum, order) => sum + order.items[flavorId] - order.allocated[flavorId],
      0,
    );
  return (
    Math.ceil(
      Math.max(
        0,
        demand + (throughSequence === Infinity ? 1 : 0) - state.stock[flavorId],
      ) / flavor.perBatch,
    ) * flavor.cookingMinutes
  );
}
