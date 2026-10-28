ALTER TABLE orders ADD COLUMN priority_sequence INTEGER
    CHECK (priority_sequence IS NULL OR (priority_sequence > 0 AND status = 'waiting'));
