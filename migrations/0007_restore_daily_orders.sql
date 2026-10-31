/* Refuse to guess dates if the continuous database spans multiple Japan days
   or if the cutover is delayed until tomorrow. Investigate before retrying. */
CREATE TABLE IF NOT EXISTS daily_migration_guard (same_day INTEGER NOT NULL CHECK (same_day = 1));
DELETE FROM daily_migration_guard;
INSERT INTO daily_migration_guard
SELECT NOT EXISTS (SELECT 1 FROM orders WHERE date(created_at, '+9 hours') IS NOT date('now', '+9 hours'));
DROP TABLE daily_migration_guard;

/* Continuous orders have no business-day field. The guard above ensures every
   existing order was created on the cutover's Japan date. */
CREATE TABLE days (
    id TEXT PRIMARY KEY,
    last_number INTEGER NOT NULL DEFAULT 99 CHECK (last_number >= 0),
    next_sequence INTEGER NOT NULL DEFAULT 1 CHECK (next_sequence > 0)
);
INSERT INTO days (id, last_number, next_sequence)
SELECT date('now', '+9 hours'), last_number, (SELECT COUNT(*) + 1 FROM orders)
FROM app_state WHERE id = 1;

CREATE TABLE inventory (
    day TEXT PRIMARY KEY REFERENCES days(id),
    counts TEXT NOT NULL CHECK (json_valid(counts))
);
INSERT INTO inventory (day, counts)
SELECT days.id, app_state.stock FROM days CROSS JOIN app_state WHERE app_state.id = 1;

CREATE TABLE orders_daily (
    id TEXT PRIMARY KEY,
    day TEXT NOT NULL REFERENCES days(id),
    sequence INTEGER NOT NULL CHECK (sequence > 0),
    number INTEGER NOT NULL CHECK (number > 0),
    status TEXT NOT NULL CHECK (status IN ('waiting', 'ready', 'skipped', 'completed', 'cancelled')),
    items TEXT NOT NULL CHECK (json_valid(items)),
    allocated TEXT NOT NULL CHECK (json_valid(allocated)),
    created_at TEXT NOT NULL,
    finished_at TEXT,
    priority_sequence INTEGER CHECK (priority_sequence IS NULL OR (priority_sequence > 0 AND status = 'waiting')),
    UNIQUE (day, sequence)
);
INSERT INTO orders_daily (id, day, sequence, number, status, items, allocated, created_at, finished_at, priority_sequence)
SELECT orders.id, days.id, ROW_NUMBER() OVER (ORDER BY orders.number), orders.number,
       orders.status, orders.items, orders.allocated, orders.created_at, orders.finished_at, orders.priority_sequence
FROM orders CROSS JOIN days;
DROP TABLE orders;
ALTER TABLE orders_daily RENAME TO orders;
CREATE UNIQUE INDEX ticket_number ON orders (day, number);
