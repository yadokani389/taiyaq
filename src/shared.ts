export const MAX_TICKET_NUMBER = 99;

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
export type Day = {
  id: string;
  lastNumber: number;
  nextSequence: number;
};
export type Order = {
  id: string;
  day: string;
  sequence: number;
  number: number;
  status: OrderStatus;
  items: Counts;
  allocated: Counts;
  createdAt: string;
  finishedAt: string | null;
};
export type State = {
  revision: number;
  day: Day;
  flavors: Flavor[];
  stock: Counts;
  orders: Order[];
};
export type Command =
  | { type: "order"; day: string; items: Counts }
  | { type: "production"; day: string; items: Counts }
  | { type: "adjust"; day: string; items: Counts }
  | {
      type: "complete" | "cancel" | "skip" | "resume";
      day: string;
      orderId: string;
    }
  | { type: "flavor"; flavor: Flavor };
export type CommandRequest = { operationId: string; command: Command };
export type CommandResult = { order?: Order };
export type PublicOrder = Pick<Order, "id" | "day" | "number" | "items"> & {
  status: Exclude<OrderStatus, "skipped">;
};
export type PublicState = {
  day: string;
  flavors: Array<
    Pick<Flavor, "id" | "enabled"> & { waitMinutes: number | null }
  >;
  orders: Array<Pick<PublicOrder, "id" | "number" | "status">>;
};

export function zeroCounts(): Counts {
  return Object.fromEntries(FLAVORS.map(({ id }) => [id, 0])) as Counts;
}

export function total(counts: Counts): number {
  return FLAVORS.reduce((sum, { id }) => sum + counts[id], 0);
}

export function japanDate(now = new Date()): string {
  return new Intl.DateTimeFormat("sv-SE", { timeZone: "Asia/Tokyo" }).format(
    now,
  );
}

export const STATUS_LABELS: Record<OrderStatus, string> = {
  waiting: "準備中",
  ready: "受け取り可能",
  skipped: "保留中",
  completed: "受け渡し済み",
  cancelled: "キャンセル済み",
};
