CREATE TABLE pipelines (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE nodes (
    id INTEGER PRIMARY KEY,
    pipeline_id INTEGER REFERENCES pipelines(id),
    node_type TEXT NOT NULL,  -- 'connector', 'transformation', 'destination'
    name TEXT NOT NULL,
    config JSON,
    constraints JSON  -- Store NodeType constraints as JSON
);

CREATE TABLE edges (
    id INTEGER PRIMARY KEY,
    pipeline_id INTEGER REFERENCES pipelines(id),
    from_node INTEGER REFERENCES nodes(id),
    to_node INTEGER REFERENCES nodes(id)
);

CREATE TABLE node_states (
    node_id INTEGER REFERENCES nodes(id),
    status TEXT NOT NULL,
    description TEXT,
    error TEXT,
    last_activity TIMESTAMP,
    metrics JSON,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);