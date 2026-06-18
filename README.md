# Keepsake

End-to-end-encrypted life organizer: accounts, credentials, documents, insurance,
health, finance, infrastructure decisions, scenario runbooks, work logs, and notes.

Rust library + CLI + Tauri (Solid + TS) desktop app. CRDT-synced over a
self-hosted server that only ever sees ciphertext.

## Workspace layout

- `crates/keepsake-core` — the library (vault, crypto, records, sync, export)
- `crates/keepsake-cli` — `ratatui` TUI + `clap` command-line
- `crates/keepsake-app` — Tauri 2.x desktop shell (Solid + TypeScript + Vite)
- `crates/keepsake-server` — standalone axum sync server

## Quick start

```bash
# build the library + CLI
cargo build -p keepsake-core -p keepsake-cli

# initialize a vault
cargo run -p keepsake-cli -- init

# unlock and use
cargo run -p keepsake-cli -- unlock
```

See `docs/threat-model.md`, `docs/crypto.md`, and `docs/sync-protocol.md`.

## License

MIT. See `LICENSE`.
