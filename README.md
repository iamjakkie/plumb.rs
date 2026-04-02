# plumb.rs

A lightweight stream analytics testing tool built in Rust. Point it at any data source — CSV files, WebSockets, or your own connector — and get a live pipeline to experiment with stream processing logic without needing a full Kafka/Flink setup.

## Architecture

The project is a Cargo workspace with three crates and a UI:

| Crate | Description |
|---|---|
| `plumb-api` | Axum HTTP API server — manages pipelines, nodes, and connectors |
| `plumb-src` | Connector library — pluggable data sources with validation |
| `plumb-rs` | Core engine |
| `plumb-ui` | Vanilla JS frontend served statically by the API |

## Getting Started

**Prerequisites:** Rust (2024 edition), SQLite

```bash
git clone https://github.com/iamjakkie/plumb.rs.git
cd plumb.rs

# Copy and configure environment
cp .env.example .env  # set HOST, PORT as needed

cargo run -p plumb-api
```

The server starts on `http://0.0.0.0:3000` by default and serves the UI at `/`.

## API

| Method | Endpoint | Description |
|---|---|---|
| `GET` | `/health` | Health check |
| `GET` | `/api/pipelines` | List all pipelines |
| `POST` | `/api/pipelines` | Create a pipeline |
| `GET` | `/api/pipelines/:id` | Get pipeline details |

## Connectors

Available data source connectors (in `plumb-src`):

- **CSV** — read from CSV files
- **WebSocket** — stream data from a WebSocket endpoint

Each connector exposes a config schema, constraint schema, display name, and description via the `ConnectorMeta` trait.

## License

MIT
