CREATE TABLE days (
    id TEXT PRIMARY KEY,
    last_number INTEGER NOT NULL DEFAULT 0 CHECK (last_number BETWEEN 0 AND 100),
    next_sequence INTEGER NOT NULL DEFAULT 1 CHECK (next_sequence > 0),
    closed_at TEXT
);
CREATE TABLE app_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    revision INTEGER NOT NULL DEFAULT 0,
    active_day TEXT REFERENCES days(id)
);
INSERT INTO app_state (id) VALUES (1);
CREATE TABLE inventory (
    day TEXT PRIMARY KEY REFERENCES days(id),
    counts TEXT NOT NULL CHECK (json_valid(counts))
);
CREATE TABLE flavors (
    id TEXT PRIMARY KEY CHECK (id IN ('tsubuan', 'custard', 'kurikinton', 'imoan', 'ujimatcha')),
    enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
    cooking_minutes INTEGER NOT NULL CHECK (cooking_minutes BETWEEN 1 AND 120),
    per_batch INTEGER NOT NULL CHECK (per_batch BETWEEN 1 AND 20)
);
INSERT INTO flavors VALUES
    ('tsubuan', 1, 15, 9), ('custard', 1, 15, 9),
    ('kurikinton', 0, 15, 2), ('imoan', 0, 15, 2), ('ujimatcha', 0, 15, 2);
CREATE TABLE orders (
    id TEXT PRIMARY KEY,
    day TEXT NOT NULL REFERENCES days(id),
    sequence INTEGER NOT NULL CHECK (sequence > 0),
    number INTEGER NOT NULL CHECK (number BETWEEN 1 AND 100),
    status TEXT NOT NULL CHECK (status IN ('waiting', 'ready', 'completed', 'cancelled')),
    items TEXT NOT NULL CHECK (json_valid(items)),
    allocated TEXT NOT NULL CHECK (json_valid(allocated)),
    created_at TEXT NOT NULL,
    finished_at TEXT,
    UNIQUE (day, sequence)
);
CREATE UNIQUE INDEX active_ticket ON orders (day, number) WHERE status IN ('waiting', 'ready');
CREATE INDEX order_history ON orders (day, sequence DESC);
CREATE TABLE operations (
    id TEXT PRIMARY KEY,
    request TEXT NOT NULL,
    response TEXT NOT NULL CHECK (json_valid(response)),
    revision INTEGER NOT NULL UNIQUE
);
