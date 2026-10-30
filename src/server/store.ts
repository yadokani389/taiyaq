import {
  FLAVORS,
  type Command,
  type CommandResult,
  type Counts,
  type Flavor,
  type Order,
  type State,
} from "../shared";
import { applyCommand, UserError } from "./domain";

type OrderRow = Omit<
  Order,
  "items" | "allocated" | "createdAt" | "finishedAt" | "prioritySequence"
> & {
  items: string;
  allocated: string;
  created_at: string;
  finished_at: string | null;
  priority_sequence: number | null;
};

export function fromOrderRow(row: OrderRow): Order {
  return {
    id: row.id,
    number: row.number,
    status: row.status,
    prioritySequence: row.priority_sequence,
    items: JSON.parse(row.items) as Counts,
    allocated: JSON.parse(row.allocated) as Counts,
    createdAt: row.created_at,
    finishedAt: row.finished_at,
  };
}

export async function loadState(db: D1Database): Promise<State> {
  const [meta, orders, flavors] = await db.batch([
    db.prepare(
      "SELECT revision, last_number, stock FROM app_state WHERE id = 1",
    ),
    db.prepare(
      "SELECT * FROM orders WHERE status IN ('waiting', 'ready', 'skipped') ORDER BY number",
    ),
    db.prepare("SELECT * FROM flavors"),
  ]);
  const state = meta!.results[0] as {
    revision: number;
    last_number: number;
    stock: string;
  };
  const settings = flavors!.results as Array<{
    id: Flavor["id"];
    enabled: number;
    cooking_minutes: number;
    per_batch: number;
  }>;
  return {
    revision: state.revision,
    lastNumber: state.last_number,
    stock: JSON.parse(state.stock) as Counts,
    orders: (orders!.results as OrderRow[]).map(fromOrderRow),
    flavors: FLAVORS.map(({ id }) => {
      const flavor = settings.find((value) => value.id === id)!;
      return {
        id,
        enabled: flavor.enabled === 1,
        cookingMinutes: flavor.cooking_minutes,
        perBatch: flavor.per_batch,
      };
    }),
  };
}

async function replay(
  db: D1Database,
  operationId: string,
  request: string,
): Promise<CommandResult | null> {
  const operation = await db
    .prepare("SELECT request, response FROM operations WHERE id = ?")
    .bind(operationId)
    .first<{ request: string; response: string }>();
  if (!operation) return null;
  if (operation.request !== request)
    throw new UserError("同じ操作IDを別の内容に使用できません", 409);
  return JSON.parse(operation.response) as CommandResult;
}

export async function execute(
  db: D1Database,
  operationId: string,
  command: Command,
  now = new Date(),
): Promise<CommandResult> {
  const request = JSON.stringify(command);
  for (let attempt = 0; attempt < 5; attempt++) {
    const previous = await replay(db, operationId, request);
    if (previous) return previous;
    const before = await loadState(db);
    const after = structuredClone(before);
    let result: CommandResult;
    try {
      result = applyCommand(
        after,
        command,
        now.toISOString(),
        crypto.randomUUID(),
      );
    } catch (error) {
      const previous = await replay(db, operationId, request);
      if (previous) return previous;
      throw error;
    }
    const changedOrders = after.orders.filter(
      (order) =>
        JSON.stringify(order) !==
        JSON.stringify(before.orders.find(({ id }) => id === order.id)),
    );
    const changedFlavors = after.flavors.filter(
      (flavor) =>
        JSON.stringify(flavor) !==
        JSON.stringify(before.flavors.find(({ id }) => id === flavor.id)),
    );
    const revision = before.revision + 1;
    const guard =
      "EXISTS (SELECT 1 FROM operations WHERE id = ? AND revision = ?)";
    const statements = [
      db
        .prepare(
          "INSERT INTO operations (id, request, response, revision) SELECT ?, ?, ?, ? FROM app_state WHERE id = 1 AND revision = ?",
        )
        .bind(
          operationId,
          request,
          JSON.stringify(result),
          revision,
          before.revision,
        ),
    ];
    if (changedOrders.length) {
      statements.push(
        db
          .prepare(
            `INSERT INTO orders (id, number, status, priority_sequence, items, allocated, created_at, finished_at)
        SELECT json_extract(value, '$.id'), json_extract(value, '$.number'), json_extract(value, '$.status'), json_extract(value, '$.prioritySequence'), json_extract(value, '$.items'),
          json_extract(value, '$.allocated'), json_extract(value, '$.createdAt'), json_extract(value, '$.finishedAt')
        FROM json_each(?) WHERE ${guard}
        ON CONFLICT(id) DO UPDATE SET status = excluded.status, priority_sequence = excluded.priority_sequence, allocated = excluded.allocated, finished_at = excluded.finished_at`,
          )
          .bind(JSON.stringify(changedOrders), operationId, revision),
      );
    }
    if (changedFlavors.length) {
      statements.push(
        db
          .prepare(
            `INSERT INTO flavors (id, enabled, cooking_minutes, per_batch)
        SELECT json_extract(value, '$.id'), json_extract(value, '$.enabled'), json_extract(value, '$.cookingMinutes'), json_extract(value, '$.perBatch')
        FROM json_each(?) WHERE ${guard}
        ON CONFLICT(id) DO UPDATE SET enabled = excluded.enabled, cooking_minutes = excluded.cooking_minutes, per_batch = excluded.per_batch`,
          )
          .bind(JSON.stringify(changedFlavors), operationId, revision),
      );
    }
    statements.push(
      db
        .prepare(
          `UPDATE app_state SET revision = ?, last_number = ?, stock = ? WHERE id = 1 AND revision = ? AND ${guard}`,
        )
        .bind(
          revision,
          after.lastNumber,
          JSON.stringify(after.stock),
          before.revision,
          operationId,
          revision,
        ),
    );
    try {
      const committed = await db.batch(statements);
      if (committed[0]!.meta.changes === 1) return result;
    } catch (error) {
      const previous = await replay(db, operationId, request);
      if (previous) return previous;
      throw error;
    }
  }
  throw new UserError(
    "操作が重なりました。同じ内容でもう一度送信してください",
    409,
  );
}

export async function orderDetails(
  db: D1Database,
  id: string,
): Promise<Order | null> {
  const row = await db
    .prepare("SELECT * FROM orders WHERE id = ?")
    .bind(id)
    .first<OrderRow>();
  return row ? fromOrderRow(row) : null;
}

export async function history(db: D1Database): Promise<Order[]> {
  const result = await db
    .prepare("SELECT * FROM orders ORDER BY number DESC")
    .all<OrderRow>();
  return result.results.map(fromOrderRow);
}
