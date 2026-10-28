import {
  FLAVORS,
  MAX_TICKET_NUMBER,
  zeroCounts,
  compareAllocationOrder,
  type Command,
  type CommandResult,
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
    if (order.status !== "waiting") continue;
    for (const { id } of FLAVORS) available[id] += order.allocated[id];
  }
  for (const order of state.orders.toSorted(compareAllocationOrder)) {
    if (order.status !== "waiting") continue;
    for (const { id } of FLAVORS) {
      order.allocated[id] = Math.min(order.items[id], available[id]);
      available[id] -= order.allocated[id];
    }
    order.status = FLAVORS.every(
      ({ id }) => order.allocated[id] === order.items[id],
    )
      ? "ready"
      : "waiting";
    if (order.status === "ready") order.prioritySequence = null;
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
        .filter(
          ({ status }) =>
            status === "waiting" || status === "ready" || status === "skipped",
        )
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
      prioritySequence: null,
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
          .filter(({ status }) => status === "waiting")
          .reduce((sum, order) => sum + order.allocated[id], 0);
      if (available + command.items[id] < 0)
        throw new UserError(
          "受け取り可能・受け渡し済みの商品は在庫から減らせません",
          409,
        );
    }
    for (const { id } of FLAVORS) state.stock[id] += command.items[id];
    allocate(state);
    return {};
  }
  const order = state.orders.find(({ id }) => id === command.orderId);
  if (!order) throw new UserError("注文が見つかりません", 404);
  if (order.status === "completed" || order.status === "cancelled")
    throw new UserError("すでに終了した注文です", 409);
  if (command.type === "prioritize" || command.type === "unprioritize") {
    if (order.status !== "waiting")
      throw new UserError("準備中の注文だけ優先を変更できます", 409);
    if (command.type === "prioritize") {
      order.prioritySequence ??=
        Math.max(
          0,
          ...state.orders.map((value) => value.prioritySequence ?? 0),
        ) + 1;
    } else order.prioritySequence = null;
  } else if (command.type === "resume") {
    if (order.status !== "skipped")
      throw new UserError("保留中の注文ではありません", 409);
    order.status = "waiting";
  } else {
    if (order.status === "skipped" && command.type !== "cancel")
      throw new UserError("保留中です。先に列に戻してください", 409);
    if (command.type === "complete" && order.status !== "ready")
      throw new UserError("まだ全品が揃っていません", 409);
    if (command.type === "cancel" || command.type === "skip") {
      for (const { id } of FLAVORS) state.stock[id] += order.allocated[id];
      order.allocated = zeroCounts();
    }
    order.prioritySequence = null;
    order.status =
      command.type === "skip"
        ? "skipped"
        : command.type === "complete"
          ? "completed"
          : "cancelled";
    if (command.type !== "skip") order.finishedAt = now;
  }
  allocate(state);
  return { order };
}
