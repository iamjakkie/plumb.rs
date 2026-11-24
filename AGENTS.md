# AGENTS.md — plumb.rs

Deterministic event-time streaming and replay in Rust, aimed at sensor and telemetry data. The core engine is being rebuilt (see the P0 brief in hq); `plumb-api` and `plumb-ui` are frozen until P0 decides their fate.

## Commands
- Build: `cargo build --workspace --all-targets`
- Test: `cargo test --workspace`
- Lint: `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all --check`

## Layout
- `plumb-rs/` core engine (event model, windows, replay) · `plumb-src/` source connectors (CSV, Parquet) · `plumb-api/`, `plumb-ui/` legacy, frozen

## Repo-specific rules
- Determinism is the product: same input + same seed must give byte-identical output. Any change that could affect ordering needs a hash-comparison test.
- No async in the core engine; wrap sources at the edge.
- Do not edit `plumb-api/` or `plumb-ui/` unless the task says so.

## How tasks work
- Planning lives in the private hq repo (`iamjakkie/hq`). Each task here is an issue with **Context**, **Acceptance oracles**, **Allowed files** and a **Kind**; its first line links the hq original (`HQ copy: iamjakkie/hq#N`).
- Start from `gh issue view <n>`. Restate goal, oracles and allowed files, propose a plan, and wait for approval before editing.
- Behave according to **Kind**:
  - **Plumbing (agent)** – implement it, inside Allowed files only.
  - **Core (by hand)** – the human writes the logic. Do not write implementation code for it. Explain, point to references, review diffs, suggest test cases.
  - **Thinking** – produce the requested document; no code.
- Branch: `task/<n>-<short-slug>`. PR description: `Closes #<n>, closes iamjakkie/hq#<N>`, then what you verified (command + result) and what you did **not** verify. Tick only the oracles you actually checked.

## Always
- Run the build, tests and linters below before calling anything done.
- Keep the diff minimal and inside Allowed files.
- Cite the source (book/paper, section or equation) in a comment for any algorithm or equation.

## Ask first
- Adding or upgrading dependencies.
- Changing public APIs, feature flags or CI.
- Anything that needs a file outside Allowed files.

## Never
- Weaken, skip or delete a failing test to make CI pass. Edit existing tests only if the task's Allowed files include them.
- Edit files marked `// HAND-CORE` or `# HAND-CORE`.
- Commit secrets. LLM access goes through OpenRouter with the key in an env var (`OPENROUTER_API_KEY`).
