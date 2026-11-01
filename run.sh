#!/usr/bin/env bash
# Start the full local stack: API + engine + static UI (one process).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

if [[ ! -f .env ]]; then
  if [[ -f .env.example ]]; then
    cp .env.example .env
    echo "Created .env from .env.example"
  elif [[ -f plumb-api/.env ]]; then
    cp plumb-api/.env .env
    echo "Created .env from plumb-api/.env"
  fi
fi

# Export vars for the process (dotenv also loads .env from cwd).
if [[ -f .env ]]; then
  set -a
  # shellcheck disable=SC1091
  source .env
  set +a
fi

HOST="${HOST:-0.0.0.0}"
PORT="${PORT:-3000}"

mkdir -p data/uploads

echo "Building plumb-api…"
cargo build -p plumb-api

URL_HOST="$HOST"
if [[ "$URL_HOST" == "0.0.0.0" ]]; then
  URL_HOST="127.0.0.1"
fi

echo ""
echo "Plumb stack ready"
echo "  UI + API:  http://${URL_HOST}:${PORT}/"
echo "  Health:    http://${URL_HOST}:${PORT}/health"
echo "  Ctrl+C to stop"
echo ""

exec cargo run -p plumb-api --quiet
