DROP TABLE orders;
DROP TABLE inventory;
DROP TABLE days;
DELETE FROM operations;
UPDATE app_state SET revision = 0;
ALTER TABLE app_state ADD COLUMN last_number INTEGER NOT NULL DEFAULT 99 CHECK (last_number >= 0);
ALTER TABLE app_state ADD COLUMN stock TEXT NOT NULL DEFAULT '{"tsubuan":0,"custard":0,"kurikinton":0,"imoan":0,"ujimatcha":0}' CHECK (json_valid(stock));
CREATE TABLE orders (
    id TEXT PRIMARY KEY,
    number INTEGER NOT NULL UNIQUE CHECK (number > 0),
    status TEXT NOT NULL CHECK (status IN ('waiting', 'ready', 'skipped', 'completed', 'cancelled')),
    priority_sequence INTEGER CHECK (priority_sequence IS NULL OR (priority_sequence > 0 AND status = 'waiting')),
    items TEXT NOT NULL CHECK (json_valid(items)),
    allocated TEXT NOT NULL CHECK (json_valid(allocated)),
    created_at TEXT NOT NULL,
    finished_at TEXT
);
