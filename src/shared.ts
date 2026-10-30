export const FLAVORS = [
  { id: "tsubuan", name: "粒あん" },
  { id: "custard", name: "カスタード" },
  { id: "kurikinton", name: "栗きんとん" },
  { id: "imoan", name: "いもあん" },
  { id: "ujimatcha", name: "宇治抹茶あん" },
] as const;

export type FlavorId = (typeof FLAVORS)[number]["id"];
export type Counts = Record<FlavorId, number>;
export type OrderStatus =
  | "waiting"
  | "ready"
  | "skipped"
  | "completed"
  | "cancelled";
export type Flavor = {
  id: FlavorId;
  enabled: boolean;
  cookingMinutes: number;
  perBatch: number;
};
export type Order = {
  id: string;
  number: number;
  status: OrderStatus;
  prioritySequence: number | null;
  items: Counts;
  allocated: Counts;
  createdAt: string;
  finishedAt: string | null;
};
export type State = {
  revision: number;
  lastNumber: number;
  flavors: Flavor[];
  stock: Counts;
  orders: Order[];
};
export type Command =
  | { type: "order"; items: Counts }
  | { type: "production"; items: Counts }
  | { type: "adjust"; items: Counts }
  | {
      type:
        | "complete"
        | "cancel"
        | "skip"
        | "resume"
        | "prioritize"
        | "unprioritize";
      orderId: string;
    }
  | { type: "flavor"; flavor: Flavor };
export type CommandRequest = { operationId: string; command: Command };
export type CommandResult = { order?: Order };
export type PublicOrder = Pick<Order, "id" | "number" | "items"> & {
  status: Exclude<OrderStatus, "skipped">;
};
export type PublicState = {
  flavors: Array<
    Pick<Flavor, "id" | "enabled"> & { waitMinutes: number | null }
  >;
  orders: Array<Pick<PublicOrder, "id" | "number" | "status">>;
};

export function compareAllocationOrder(a: Order, b: Order): number {
  return (
    Number(a.prioritySequence === null) - Number(b.prioritySequence === null) ||
    (a.prioritySequence ?? 0) - (b.prioritySequence ?? 0) ||
    a.number - b.number
  );
}

export function zeroCounts(): Counts {
  return Object.fromEntries(FLAVORS.map(({ id }) => [id, 0])) as Counts;
}

export function total(counts: Counts): number {
  return FLAVORS.reduce((sum, { id }) => sum + counts[id], 0);
}

export const STATUS_LABELS: Record<OrderStatus, string> = {
  waiting: "準備中",
  ready: "受け取り可能",
  skipped: "保留中",
  completed: "受け渡し済み",
  cancelled: "キャンセル済み",
};

export function estimateWait(
  state: State,
  flavorId: FlavorId,
  throughOrder?: Order,
): number | null {
  const flavor = state.flavors.find(({ id }) => id === flavorId);
  if (!flavor) return null;
  const demand = state.orders
    .filter(
      (order) =>
        order.status === "waiting" &&
        (!throughOrder || compareAllocationOrder(order, throughOrder) <= 0),
    )
    .reduce(
      (sum, order) => sum + order.items[flavorId] - order.allocated[flavorId],
      0,
    );
  return (
    Math.ceil(
      Math.max(0, demand + (throughOrder ? 0 : 1) - state.stock[flavorId]) /
        flavor.perBatch,
    ) * flavor.cookingMinutes
  );
}

export function estimateOrderWait(state: State, order: Order): number {
  if (order.status !== "waiting") return 0;
  return Math.max(
    0,
    ...state.flavors
      .filter(({ id }) => order.items[id] > order.allocated[id])
      .map(
        ({ id, cookingMinutes }) =>
          (estimateWait(state, id, order) ?? 0) - cookingMinutes,
      ),
  );
}
