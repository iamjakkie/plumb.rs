# plumb.rs

A local, single-process stream lab in Rust. Point a pipeline at a CSV file or WebSocket, add a few transforms, hit **Run**, and watch JSON records stream into the UI — no Kafka/Flink required.

This is an early working prototype, not a distributed streaming platform.

## Architecture

Cargo workspace:

| Crate / dir | Role |
|---|---|
| `plumb-api` | Axum HTTP API, SQLite, runtime manager, static UI |
| `plumb-src` | Connectors (CSV, WebSocket) + transforms + preview destination |
| `plumb-rs` | In-process DAG engine |
| `plumb-ui` | Vanilla JS canvas editor + live events panel |

## Getting started

**Prerequisites:** Rust (edition 2021), SQLite (via `rusqlite`)

```bash
git clone https://github.com/iamjakkie/plumb.rs.git
cd plumb.rs

cp .env.example .env   # HOST, PORT, DATABASE_PATH

cargo run -p plumb-api
```

Open `http://localhost:3000`.

### Quick lab loop

1. **New Pipeline**
2. Add a **CSV** connector — Browse/upload a file (sample under `data/uploads/`)
3. Add **Rate Limit** (e.g. 5 records/sec) so replay is watchable
4. Optionally add **Filter** / **Map**
5. Add a **Preview** destination
6. Draw edges: CSV → Rate Limit → … → Preview
7. **Validate** → **Run** — records appear in **Live Events**
8. **Stop** to cancel

## API

| Method | Endpoint | Description |
|---|---|---|
| `GET` | `/health` | Health check |
| `GET` | `/api/connectors` | Connector metadata + config schemas |
| `GET` | `/api/operators` | Transform + destination schemas |
| `GET` | `/api/pipelines` | List pipelines (includes `node_count`) |
| `POST` | `/api/pipelines` | Create (or clone) a pipeline |
| `GET` | `/api/pipelines/:id` | Full pipeline with nodes/edges |
| `DELETE` | `/api/pipelines/:id` | Delete pipeline |
| `POST` | `/api/pipelines/:id/nodes` | Add node (`config.type` required) |
| `POST` | `/api/pipelines/:id/edges` | Add edge `{ from, to }` |
| `GET` | `/api/pipelines/:id/validate` | Graph + config checks |
| `POST` | `/api/pipelines/:id/trigger` | Start engine |
| `POST` | `/api/pipelines/:id/stop` | Cancel running pipeline |
| `GET` | `/api/pipelines/:id/events` | SSE live `EngineEvent` stream |
| `POST` | `/api/upload` | Multipart upload → `./data/uploads/` |

## Operators

**Sources:** `csv`, `websocket`  
**Transforms:** `filter`, `map`, `rate_limit`  
**Destination:** `preview` (feeds the Live Events panel)

Node configs must include `"type": "<operator id>"` (the UI sets this when you pick a type).

## Connector smoke tests

```bash
cargo run -p plumb-src --bin test_csv
cargo run -p plumb-src --bin test_binance   # needs network
```

## Vision / backlog

Longer-term ideas (AI-assisted connectors, ONNX, distributed fittings) live in `TUTOR.md` and are **not** implemented. The near-term goal is a solid local experimentation loop.

## License

MIT
