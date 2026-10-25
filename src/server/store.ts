import {
  FLAVORS,
  zeroCounts,
  japanDate,
  type Command,
  type CommandResult,
  type Counts,
  type Day,
  type Flavor,
  type Order,
  type State,
} from "../shared";
import { applyCommand, UserError } from "./domain";

type OrderRow = Omit<
  Order,
  "items" | "allocated" | "createdAt" | "finishedAt"
> & {
  items: string;
  allocated: string;
  created_at: string;
  finished_at: string | null;
};

export function fromOrderRow(row: OrderRow): Order {
  return {
    id: row.id,
    day: row.day,
    sequence: row.sequence,
    number: row.number,
    status: row.status,
    items: JSON.parse(row.items) as Counts,
    allocated: JSON.parse(row.allocated) as Counts,
    createdAt: row.created_at,
    finishedAt: row.finished_at,
  };
}

export async function loadState(
  db: D1Database,
  now = new Date(),
): Promise<State> {
  const today = japanDate(now);
  const [meta, days, stock, orders, flavors] = await db.batch([
    db.prepare("SELECT revision FROM app_state WHERE id = 1"),
    db
      .prepare("SELECT id, last_number, next_sequence FROM days WHERE id = ?")
      .bind(today),
    db.prepare("SELECT counts FROM inventory WHERE day = ?").bind(today),
    db
      .prepare(
        "SELECT * FROM orders WHERE day = ? AND status IN ('waiting', 'ready') ORDER BY sequence",
      )
      .bind(today),
    db.prepare("SELECT * FROM flavors"),
  ]);
  const day = days!.results[0] as
    | {
        id: string;
        last_number: number;
        next_sequence: number;
      }
    | undefined;
  const settings = flavors!.results as Array<{
    id: Flavor["id"];
    enabled: number;
    cooking_minutes: number;
    per_batch: number;
  }>;
  return {
    revision: (meta!.results[0] as { revision: number }).revision,
    day: day
      ? {
          id: day.id,
          lastNumber: day.last_number,
          nextSequence: day.next_sequence,
        }
      : { id: today, lastNumber: 0, nextSequence: 1 },
    stock: stock!.results[0]
      ? (JSON.parse((stock!.results[0] as { counts: string }).counts) as Counts)
      : zeroCounts(),
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
    const before = await loadState(db, now);
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
    statements.push(
      db
        .prepare(
          `INSERT INTO days (id, last_number, next_sequence) SELECT ?, ?, ? WHERE ${guard}
        ON CONFLICT(id) DO UPDATE SET last_number = excluded.last_number, next_sequence = excluded.next_sequence`,
        )
        .bind(
          after.day.id,
          after.day.lastNumber,
          after.day.nextSequence,
          operationId,
          revision,
        ),
    );
    statements.push(
      db
        .prepare(
          `INSERT INTO inventory (day, counts) SELECT ?, ? WHERE ${guard}
        ON CONFLICT(day) DO UPDATE SET counts = excluded.counts`,
        )
        .bind(after.day.id, JSON.stringify(after.stock), operationId, revision),
    );
    if (changedOrders.length) {
      statements.push(
        db
          .prepare(
            `INSERT INTO orders (id, day, sequence, number, status, items, allocated, created_at, finished_at)
        SELECT json_extract(value, '$.id'), json_extract(value, '$.day'), json_extract(value, '$.sequence'),
          json_extract(value, '$.number'), json_extract(value, '$.status'), json_extract(value, '$.items'),
          json_extract(value, '$.allocated'), json_extract(value, '$.createdAt'), json_extract(value, '$.finishedAt')
        FROM json_each(?) WHERE ${guard}
        ON CONFLICT(id) DO UPDATE SET status = excluded.status, allocated = excluded.allocated, finished_at = excluded.finished_at`,
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
          `UPDATE app_state SET revision = ? WHERE id = 1 AND revision = ? AND ${guard}`,
        )
        .bind(revision, before.revision, operationId, revision),
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

export async function history(db: D1Database, day: string): Promise<Order[]> {
  const result = await db
    .prepare("SELECT * FROM orders WHERE day = ? ORDER BY sequence DESC")
    .bind(day)
    .all<OrderRow>();
  return result.results.map(fromOrderRow);
}

export async function listDays(db: D1Database): Promise<Day[]> {
  const result = await db
    .prepare("SELECT id, last_number, next_sequence FROM days ORDER BY id DESC")
    .all<{
      id: string;
      last_number: number;
      next_sequence: number;
    }>();
  return result.results.map((row) => ({
    id: row.id,
    lastNumber: row.last_number,
    nextSequence: row.next_sequence,
  }));
}
