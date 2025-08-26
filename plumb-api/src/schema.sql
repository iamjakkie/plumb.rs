CREATE TABLE IF NOT EXISTS pipelines (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    status TEXT DEFAULT 'idle'
);

CREATE TABLE IF NOT EXISTS nodes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pipeline_id INTEGER REFERENCES pipelines(id),
    node_type TEXT NOT NULL,  -- 'connector', 'transformation', 'destination'
    name TEXT NOT NULL,
    config JSON,
    constraints JSON  -- Store NodeType constraints as JSON
    status TEXT DEFAULT 'idle',
    last_updated TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS edges (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pipeline_id INTEGER REFERENCES pipelines(id),
    from_node INTEGER REFERENCES nodes(id),
    to_node INTEGER REFERENCES nodes(id)
);

CREATE TABLE IF NOT EXISTS node_states (
    node_id INTEGER REFERENCES nodes(id),
    status TEXT NOT NULL,
    last_updated TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (node_id)
    -- description TEXT,
    -- error TEXT,
    -- last_activity TIMESTAMP,
    -- metrics JSON,
    -- updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);