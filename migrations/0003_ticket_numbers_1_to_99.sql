CREATE TABLE days_new (
    id TEXT PRIMARY KEY,
    last_number INTEGER NOT NULL DEFAULT 0 CHECK (last_number BETWEEN 0 AND 99),
    next_sequence INTEGER NOT NULL DEFAULT 1 CHECK (next_sequence > 0)
);
INSERT INTO days_new SELECT id, last_number, next_sequence FROM days;

CREATE TABLE inventory_new (
    day TEXT PRIMARY KEY REFERENCES days_new(id),
    counts TEXT NOT NULL CHECK (json_valid(counts))
);
INSERT INTO inventory_new SELECT day, counts FROM inventory;

CREATE TABLE orders_new (
    id TEXT PRIMARY KEY,
    day TEXT NOT NULL REFERENCES days_new(id),
    sequence INTEGER NOT NULL CHECK (sequence > 0),
    number INTEGER NOT NULL CHECK (number BETWEEN 1 AND 99),
    status TEXT NOT NULL CHECK (status IN ('waiting', 'ready', 'completed', 'cancelled')),
    items TEXT NOT NULL CHECK (json_valid(items)),
    allocated TEXT NOT NULL CHECK (json_valid(allocated)),
    created_at TEXT NOT NULL,
    finished_at TEXT,
    UNIQUE (day, sequence)
);
INSERT INTO orders_new SELECT id, day, sequence, number, status, items, allocated, created_at, finished_at FROM orders;

DROP TABLE orders;
DROP TABLE inventory;
DROP TABLE days;
ALTER TABLE days_new RENAME TO days;
ALTER TABLE inventory_new RENAME TO inventory;
ALTER TABLE orders_new RENAME TO orders;
CREATE UNIQUE INDEX active_ticket ON orders (day, number) WHERE status IN ('waiting', 'ready');
CREATE INDEX order_history ON orders (day, sequence DESC);
