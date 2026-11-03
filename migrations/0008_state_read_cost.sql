ALTER TABLE days ADD COLUMN sold_count INTEGER NOT NULL DEFAULT 0 CHECK (sold_count >= 0);

UPDATE days SET sold_count = (
    SELECT COALESCE(SUM(item.value), 0)
    FROM orders, json_each(orders.items) AS item
    WHERE orders.day = days.id AND orders.status = 'completed'
);

CREATE INDEX active_orders ON orders (day, sequence)
    WHERE status IN ('waiting', 'ready', 'skipped');
