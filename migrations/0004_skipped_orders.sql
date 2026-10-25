CREATE TABLE orders_new (
    id TEXT PRIMARY KEY,
    day TEXT NOT NULL REFERENCES days(id),
    sequence INTEGER NOT NULL CHECK (sequence > 0),
    number INTEGER NOT NULL CHECK (number BETWEEN 1 AND 99),
    status TEXT NOT NULL CHECK (status IN ('waiting', 'ready', 'skipped', 'completed', 'cancelled')),
    items TEXT NOT NULL CHECK (json_valid(items)),
    allocated TEXT NOT NULL CHECK (json_valid(allocated)),
    created_at TEXT NOT NULL,
    finished_at TEXT,
    UNIQUE (day, sequence)
);
INSERT INTO orders_new SELECT id, day, sequence, number, status, items, allocated, created_at, finished_at FROM orders;
DROP TABLE orders;
ALTER TABLE orders_new RENAME TO orders;
CREATE UNIQUE INDEX active_ticket ON orders (day, number) WHERE status IN ('waiting', 'ready', 'skipped');
CREATE INDEX order_history ON orders (day, sequence DESC);
