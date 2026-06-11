CREATE TABLE orders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    status TEXT NOT NULL CHECK (status IN ('waiting', 'cooking', 'ready', 'completed', 'cancelled')),
    ordered_at TEXT NOT NULL,
    ready_at TEXT,
    completed_at TEXT,
    is_priority INTEGER NOT NULL DEFAULT 0 CHECK (is_priority IN (0, 1))
);

CREATE TABLE order_items (
    order_id INTEGER NOT NULL,
    flavor TEXT NOT NULL CHECK (flavor IN ('tsubuan', 'custard', 'kurikinton')),
    quantity INTEGER NOT NULL CHECK (quantity > 0),
    PRIMARY KEY (order_id, flavor),
    FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE
);

CREATE TABLE stock (
    flavor TEXT PRIMARY KEY CHECK (flavor IN ('tsubuan', 'custard', 'kurikinton')),
    unallocated_quantity INTEGER NOT NULL CHECK (unallocated_quantity >= 0)
);

CREATE TABLE flavor_configs (
    flavor TEXT PRIMARY KEY CHECK (flavor IN ('tsubuan', 'custard', 'kurikinton')),
    cooking_time_minutes INTEGER NOT NULL CHECK (cooking_time_minutes > 0),
    quantity_per_batch INTEGER NOT NULL CHECK (quantity_per_batch > 0)
);

CREATE TABLE notifications (
    order_id INTEGER NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('discord', 'line')),
    discord_channel_id TEXT,
    discord_user_id TEXT,
    line_user_id TEXT,
    PRIMARY KEY (order_id, kind, discord_channel_id, discord_user_id, line_user_id),
    FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE,
    CHECK (
        (kind = 'discord' AND discord_channel_id IS NOT NULL AND discord_user_id IS NOT NULL AND line_user_id IS NULL)
        OR (kind = 'line' AND discord_channel_id IS NULL AND discord_user_id IS NULL AND line_user_id IS NOT NULL)
    )
);
