# Keepsake desktop app

Tauri 2.x shell + Solid + TypeScript + Vite frontend.  All
business logic lives in `keepsake-core`; this crate is a
thin Rust shell that exposes the core's API to the frontend
as Tauri commands.

## Layout

* `src-tauri/` — Rust side.  `src/lib.rs` declares the
  Tauri commands; `src/session.rs` is the bridge to
  `keepsake-core`.
* `ui/` — frontend.  Solid + TypeScript + Vite, with
  `@solidjs/router` for routing.

## Build

```bash
# 1. install JS deps
cd ui
npm install

# 2. run in dev mode
npm run tauri dev    # or: cargo tauri dev

# 3. build a release bundle
npm run tauri build  # or: cargo tauri build
```

The dev server runs Vite on `localhost:1420`; Tauri spawns
a webview pointing at it.

## Frontend pages

* `/` — Overview: counts of records per category.
* `/c/:type` — Category list (e.g. `/c/credential`).
* `/c/:type/new` — New-record form.
* `/r/:id` — Record detail (sensitive fields masked by default).
* `/r/:id/edit` — Edit form.
* `/sync` — Configure the sync server base URL.
* `/audit` — Read or verify the audit chain.
* `/settings` — Vault path, current user, users on this device.

## Tauri commands

| Command          | Purpose                                       |
|------------------|-----------------------------------------------|
| `default_path`   | Return the per-OS default vault path          |
| `status`         | `{ unlocked, username }`                      |
| `list_users`     | List usernames on this device                 |
| `init`           | Initialize a new vault                        |
| `unlock`         | Unlock an existing vault                      |
| `lock`           | Lock the vault, zeroize the in-memory key     |
| `add_record`     | Add a record                                  |
| `update_record`  | Update a record                               |
| `delete_record`  | Delete a record                               |
| `list_records`   | List records of a type                        |
| `show_record`    | Fetch a record (sensitive fields masked)      |
| `find`           | Free-text search                              |
| `audit`          | Read or verify the audit chain                |
| `configure_sync` | Set the sync server base URL                  |

The frontend imports `invoke` from `@tauri-apps/api/core`
and calls these directly through the typed wrapper in
`ui/src/api.ts`.
