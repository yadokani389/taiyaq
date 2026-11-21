import { useSyncExternalStore } from "react";
import {
  FIRST_TICKET_NUMBER,
  FLAVORS,
  estimateOrderWait,
  estimateWait,
  total,
  zeroCounts,
  type CommandResult,
  type PublicOrder,
  type PublicState,
  type StaffState,
  type State,
} from "../shared";
import { applyCommand, UserError } from "../server/domain";
import { commandRequestSchema } from "../validation";
import * as v from "valibot";

const key = "taiyaq:demo:v1";
const day = "2026-11-21";
type Snapshot = { state: State; updatedAt: number };

function createInitial(): Snapshot {
  const state: State = {
    day: { id: day, lastNumber: FIRST_TICKET_NUMBER - 1, nextSequence: 1 },
    flavors: FLAVORS.map(({ id }) => ({
      id,
      enabled: true,
      cookingMinutes: 5,
      perBatch: 6,
    })),
    stock: zeroCounts(),
    orders: [],
    revision: 0,
  };
  const seededAt = `${day}T10:00:00+09:00`;
  applyCommand(
    state,
    { type: "order", day, items: { ...zeroCounts(), tsubuan: 1 } },
    seededAt,
    "demo-ready",
  );
  applyCommand(
    state,
    { type: "production", day, items: { ...zeroCounts(), tsubuan: 1 } },
    seededAt,
    "demo-stock",
  );
  applyCommand(
    state,
    { type: "order", day, items: { ...zeroCounts(), custard: 2 } },
    seededAt,
    "demo-waiting",
  );
  return { state, updatedAt: Date.now() };
}

let initial: Snapshot | undefined;
let snapshot: Snapshot | undefined;
let previous: string | null = null;

function getSnapshot(): Snapshot {
  initial ??= createInitial();
  try {
    const raw = localStorage.getItem(key);
    if (raw !== previous) {
      snapshot = raw ? (JSON.parse(raw) as Snapshot) : initial;
      previous = raw;
    }
    return snapshot ?? initial;
  } catch {
    return initial;
  }
}

const listeners = new Set<() => void>();
function notify() {
  for (const listener of listeners) listener();
}
function onStorage(event: StorageEvent) {
  if (event.key === key || event.key === null) notify();
}
function subscribe(listener: () => void) {
  if (!listeners.size) window.addEventListener("storage", onStorage);
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
    if (!listeners.size) window.removeEventListener("storage", onStorage);
  };
}

function runWrite<T>(write: () => T): Promise<T> {
  return navigator.locks
    ? navigator.locks.request(key, write)
    : Promise.resolve().then(write);
}

export function resetDemo(): Promise<void> {
  return runWrite(() => {
    localStorage.removeItem(key);
    localStorage.removeItem("taiyaq:demo:order");
    notify();
  });
}

export { UserError as ApiError };

export function sendCommand(input: unknown): Promise<CommandResult> {
  return runWrite(() => {
    const parsed = v.safeParse(commandRequestSchema, input);
    if (!parsed.success) throw new UserError(parsed.issues[0].message);
    const state = structuredClone(getSnapshot().state);
    const result = applyCommand(
      state,
      parsed.output.command,
      new Date().toISOString(),
      parsed.output.operationId,
    );
    state.revision++;
    localStorage.setItem(
      key,
      JSON.stringify({ state, updatedAt: Date.now() } satisfies Snapshot),
    );
    notify();
    return result;
  });
}

function activeOrders(state: State) {
  return state.orders.filter(
    ({ status }) => status !== "completed" && status !== "cancelled",
  );
}

function publicOrder({
  id,
  day,
  number,
  items,
  status,
}: State["orders"][number]): PublicOrder {
  return {
    id,
    day,
    number,
    items,
    status: status === "skipped" ? "ready" : status,
  };
}

function read(state: State, path: string) {
  if (path === "/api/status") {
    return {
      day: state.day.id,
      flavors: state.flavors.map(({ id, enabled }) => ({
        id,
        enabled,
        waitMinutes: enabled ? estimateWait(state, id) : null,
      })),
      orders: activeOrders(state).map(({ id, number, status }) => ({
        id,
        number,
        status: status === "skipped" ? "ready" : status,
      })),
    } satisfies PublicState;
  }
  if (path === "/api/staff/days") return [state.day];
  if (path === `/api/staff/history/${state.day.id}`)
    return state.orders.toReversed();
  const prefix = "/api/orders/";
  if (path.startsWith(prefix)) {
    const id = decodeURIComponent(path.slice(prefix.length));
    const order = state.orders.find((entry) => entry.id === id);
    if (order)
      return {
        order: publicOrder(order),
        waitMinutes: estimateOrderWait(state, order),
      };
    throw new UserError(
      "デモの注文が見つかりません。整理券番号を選び直してください",
      404,
    );
  }
  throw new UserError("ページが見つかりません", 404);
}

export function usePolling<T>(path: string | null) {
  const { state, updatedAt } = useSyncExternalStore(subscribe, getSnapshot);
  try {
    return {
      data: path ? (read(state, path) as T) : undefined,
      updatedAt,
      error: undefined,
    };
  } catch (cause) {
    if (!(cause instanceof UserError)) throw cause;
    return { data: undefined, updatedAt, error: cause.message };
  }
}

export function useStaffState() {
  const { state, updatedAt } = useSyncExternalStore(subscribe, getSnapshot);
  const data: StaffState = {
    ...state,
    orders: activeOrders(state),
    soldCount: state.orders.reduce(
      (sum, order) =>
        sum + (order.status === "completed" ? total(order.items) : 0),
      0,
    ),
  };
  return { data, updatedAt, error: undefined, live: true, refresh: notify };
}
