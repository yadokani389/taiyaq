CREATE TABLE notification_delivery_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_id INTEGER NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('discord', 'line')),
    discord_channel_id TEXT,
    discord_user_id TEXT,
    line_user_id TEXT,
    message TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('sent', 'failed')),
    error_message TEXT,
    attempted_at TEXT NOT NULL,
    FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE,
    CHECK (
        (kind = 'discord' AND discord_channel_id IS NOT NULL AND discord_user_id IS NOT NULL AND line_user_id IS NULL)
        OR (kind = 'line' AND discord_channel_id IS NULL AND discord_user_id IS NULL AND line_user_id IS NOT NULL)
    )
);
