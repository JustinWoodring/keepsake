<div align="center">

<img src="crates/keepsake-app/src-tauri/icons/icon.png" width="128" alt="Keepsake" />

# 🛡 Keepsake

**An end-to-end-encrypted life organizer that you fully own.**

Local-first vault for accounts, credentials, documents, insurance, health,
finance, infrastructure decisions, scenario runbooks, work logs, and notes
— with optional CRDT-synced multi-device sync through a server that
**only ever sees ciphertext**.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](./LICENSE)
&nbsp;·&nbsp; Rust · Tauri · Solid · TypeScript

</div>

---

## Overview

Keepsake is a single-user, multi-device vault for the things you don't want
to trust to a third party: logins, government IDs, insurance policies, bank
and credit card details, vehicles, residences, subscriptions, infrastructure
decisions, runbooks, health notes, free-form notes, and dated work logs. The
data lives in an encrypted SQLite file on your device; an optional
self-hosted sync server keeps multiple devices converged without ever seeing
plaintext.

The design is deliberately boring. Records are typed (twenty schema-versioned
variants), validated on write, and stored in an SQL table sealed with a
per-vault key. That key is generated once at init, then sealed per-user under
each user's Argon2id-derived master key. The local vault has an append-only
audit chain; every state change is hash-linked to the previous one.

Sync, when enabled, is a **nested envelope** on the wire: the inner is the
on-disk local envelope (sealed by the local vault key), the outer is an
AEAD keyed by a per-`(passphrase, vault_id)` Argon2id-derived shared key.
A second device with the same passphrase re-derives the same shared key
independently — there is no key exchange, no registration, no account to
create. The server is a dumb encrypted blob store; the URL + vault id are
the only access boundary, and the passphrase is the only thing protecting
the server-side payload.

For the complete design rationale, see [`docs/sync-protocol.md`](./docs/sync-protocol.md)
and [`docs/threat-model.md`](./docs/threat-model.md).

## Why it exists

The category leaders in this space (1Password, Bitwarden, Dashlane) have
either a hosted component you don't control, an opaque sync protocol, or a
data model that doesn't extend past "logins". Keepsake tries to be a
replacement for the whole life-organizer stack, not just the password
manager: same encryption guarantees, but the schema covers real records
(insurance policies with renewal dates, vehicles with VINs, scenario
runbooks with step lists, work logs by date and project) and the sync
protocol is small enough to read in an afternoon and host on a $5 VPS.

## ✨ Highlights

- **Twenty record types** out of the box: logins, IDs, contacts, bank
  accounts, credit cards, investments, income, subscriptions, insurance,
  vehicles, residences, phones, addresses, documents, infrastructure,
  domains, runbooks, work logs, notes, health.
- **Multi-user per vault.** Add as many users as you want; each has their
  own password, each is sealed separately under the shared vault key, and
  any of them can unlock the same data.
- **CRDT merge on sync.** Concurrent edits on the same record are
  reconciled with LWW for structured fields and a per-text CRDT for free-form
  bodies. No merge conflicts to resolve.
- **Append-only audit chain.** Every state change is hash-linked; a
  `keepsake audit verify` (and the Audit page in the UI) walks the chain and
  flags any break.
- **Encryption choices you can audit.** XChaCha20-Poly1305 for AEAD,
  Argon2id for password-based KDF, BLAKE3 for hashing, all wrapped behind
  the same internal `AeadKey` / `MasterKey` / `VaultKey` types. See
  [`docs/crypto.md`](./docs/crypto.md) for the exact construction.
- **Dumb sync server.** The server is ~500 lines of axum; the client
  speaks JSON over HTTP. Host it on a $5 VPS, a Raspberry Pi, or skip sync
  entirely and use the local vault.
- **Auto-sync on unlock + every 30 minutes.** Set up once, never think
  about it. The loop releases the session lock before HTTP I/O so manual
  push/pull from the UI is never blocked.
- **Three first-run options.** Create a fresh vault, import a `.ksk`
  bundle exported from another device, or **recover from sync** by
  entering the server URL, vault id, and shared passphrase — the app
  creates a fresh vault, seals the shared setup into it, and pulls your
  records down.
- **Plaintext export bundles.** `keepsake export` writes a single
  passphrase-protected `.ksk` file containing every record, attachment,
  and the sealed vault key. Useful for migrating devices or keeping a
  cold-storage backup.
- **Tauri 2 desktop shell, no telemetry.** The renderer is Solid + TS;
  the Rust binary is statically linked; there is no analytics, no
  auto-update channel, no third-party CDN.
- **Search, links, insights.** A portable full-text search over title
  + body; `[[uuid]]` link markers in any text field resolve to the target
  record's title at render time; an Insights page surfaces vaults about
  to expire, recently edited, or otherwise worth a look.

## 🛰️ How it works

```
                      ┌──────── the vault (SQLite, sealed) ────────┐
                      │  records · attachments · audit chain        │
                      │  sealed_keys (per-user master → vault key)  │
                      │  shared_sync_keys (vault id → server URL)   │
                      └────────────────────────┬────────────────────┘
                                               │ vault_key (32 bytes)
              ┌────────────────────────────────┼─────────────────────────────┐
              │                                │                             │
   inner envelope (on disk)            shared_sync_key                recovery flow
   AEAD(vault_key,                      derive_shared_key(             (new device)
   inner_nonce,                         passphrase, vault_id)            server URL
   inner_aad,                              → Argon2id + HKDF             vault id
   record_json)                                                       passphrase
              │                                │                             │
              │   ┌──── wire format ─────────┐  │                             │
              └──▶│ outer_nonce (24)         │◀─┘                             │
                  │ AEAD(shared_key,         │                                │
                  │   outer_nonce,            │                                │
                  │   "keepsake/sync/        │                                │
                  │    payload/v1",          │                                │
                  │   inner_nonce +          │                                │
                  │   inner_aad +            │                                │
                  │   inner_ciphertext)      │                                │
                  └─────────────┬────────────┘                                │
                                ▼                                             │
                    server stores opaque bytes                                  │
                    (no auth, no per-user state)                                 │
                                                                                 │
                                  + create vault, seal shared setup, pull ◀────┘
```

The local envelope is byte-identical at sender and receiver — same key,
same nonce, same AAD, same ciphertext. The outer envelope is a fresh AEAD
per push keyed by the shared sync key, with a length prefix on the inner
AAD so the server can't tell apart records of different types.

## Requirements

- **Rust 1.91+** with `cargo`. Install via [rustup](https://rustup.rs).
- **Node 20+** and **npm** for the desktop UI build.
- **Tauri 2.x** system dependencies for your platform — see the
  [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).
- For the sync server: nothing. A $5 VPS running Ubuntu 22.04+ with a
  TLS-terminating reverse proxy (nginx or caddy) is enough. Full
  walkthrough in [`docs/deploy-sync-server.md`](./docs/deploy-sync-server.md).

Built and tested on macOS (arm64); the GitHub Actions release matrix also
targets Linux (x86_64 + aarch64) and Windows (x86_64). Linux AppImage,
Windows `.msi`/`.exe`, and macOS `.dmg`/`.app` are produced on every
`v*` tag.

## 🚀 Getting started

### Desktop app (recommended)

Download a release from
[the releases page](https://github.com/JustinWoodring/keepsake/releases),
or build from source:

```bash
git clone https://github.com/JustinWoodring/keepsake.git
cd keepsake
# Build the desktop app
cargo build --release -p keepsake-app
target/release/keepsake-app
```

Then in the app: **Create new vault**, **Import `.ksk` bundle**, or
**Recover from sync** (server URL + vault id + shared passphrase). The
unlock screen has all three.

### Standalone CLI

```bash
# Build
cargo build --release -p keepsake-cli

# Initialize a vault
./target/release/keepsake --vault ~/keepsake/vault.db init --username alice

# Unlock and use
./target/release/keepsake --vault ~/keepsake/vault.db unlock --username alice

# Or use the in-process REPL
./target/release/keepsake --vault ~/keepsake/vault.db repl --username alice
```

### Sync server

```bash
# Build
cargo build --release -p keepsake-server

# Run (env vars documented in docs/deploy-sync-server.md)
KEEPSAKE_ADDR=127.0.0.1:8485 KEEPSAKE_DB=/var/lib/keepsake/server.db \
  ./target/release/keepsake-server
```

The server is a single axum binary; there is no separate database to
install. SQLite is bundled.

### Library

```toml
[dependencies]
keepsake-core = { git = "https://github.com/JustinWoodring/keepsake" }
```

```rust
use keepsake_core::vault::Vault;
use keepsake_core::records::Note;

let mut v = Vault::open_or_create("vault.db")?;
v.unlock(&vault_key)?;
let id = Uuid::new_v4();
v.put_record(&header, &Note { id, title: "hello".into(), /* … */ })?;
```

### Useful scripts

| Command | Description |
|---------|-------------|
| `cargo test --workspace` | Run the full test suite (84+ tests) |
| `cargo build --release -p keepsake-app` | Build the desktop app |
| `cargo build --release -p keepsake-server` | Build the sync server |
| `python3 scripts/make-icons.py` | Regenerate the bundle icons |

## Configuration

There are **no config files**. Everything is configured in-app or via CLI
flags and persisted into the vault itself:

- **Per-user master key** — derived from the user's password with Argon2id
  (64 MiB, t=3, p=4 by default), sealed under the shared vault key.
- **Vault key** — generated once at init, 32 random bytes, sealed per-user
  in the `sealed_keys` table. Every record in the `records` table is
  sealed with this key.
- **Shared sync setup** — `(vault_id, passphrase, server_url)` is stored
  in the `shared_sync_keys` table, sealed under the vault key. The
  derived `shared_sync_key` lives in memory only, in the unlocked
  session.
- **Auto-sync cadence** — every 30 minutes by default; tunable via
  `SYNC_INTERVAL` in `crates/keepsake-app/src-tauri/src/auto_sync.rs`.

## 🔒 Security model

- **The local vault key never leaves the device unsealed.** It is wrapped
  by Argon2id + AEAD for every user. To compromise a vault, an attacker
  needs the vault file **and** at least one user's password.
- **The sync server only sees doubly-sealed ciphertext.** A forged or
  tampered payload fails the local-vault-key AEAD check on apply; a
  stolen server DB leaks zero plaintext.
- **Argon2id is the only KDF** for both user passwords (→ master key) and
  shared sync passphrases (→ shared sync key). The parameters are stored
  in the vault so they can be rotated independently.
- **Per-record AEAD includes a typed AAD** (`type || schema_version ||
  record_id`), so a record's type can't be silently changed without
  invalidating the MAC.
- **Every state change writes a hash-linked audit entry.** `keepsake
  audit verify` walks the chain and reports any break. The UI's Audit
  page shows the same chain.
- **No telemetry, no auto-update channel.** The Rust binary is
  statically linked, the renderer is built and bundled into the binary at
  build time, and the app does not phone home.
- **The threat model is fully written down** in
  [`docs/threat-model.md`](./docs/threat-model.md). Read it before
  trusting Keepsake with anything you wouldn't put in a `1Password`
  export.

## Project layout

```
crates/
  keepsake-core/    the library (vault, crypto, records, sync, export)
    src/
      vault/          SQLite-backed encrypted vault
      crypto/         AEAD, KDF, HKDF, hashing primitives
      records/        20 typed record variants + schemas
      identity/       master key, vault key, sealed envelopes
      sync/           CRDT, protocol types, client, push/pull
      export/         .ksk bundle import/export
      crdt/           per-text CRDT for free-form bodies
      audit/          hash-linked audit chain
  keepsake-cli/     ratatui TUI + clap command-line
    src/commands/    init · unlock · lock · add · edit · list ·
                     show · find · links · resolve · sync ·
                     export · import · audit · repl
  keepsake-app/     Tauri 2.x desktop shell
    src-tauri/       Rust backend + Tauri commands + auto-sync loop
    ui/              Solid + TypeScript + Vite frontend
  keepsake-server/  standalone axum sync server
    src/             axum router, storage layer
    tests/           integration tests against the running server
docs/                protocol, threat model, crypto, deployment
scripts/             make-icons.py — regenerate bundle icons
```

## 🤝 Contributing

Issues and pull requests are welcome. Please run `cargo test
--workspace` before opening a PR; CI runs the same on every push, along
with a release build matrix. The first build takes a few minutes
(Tauri pulls a lot of native code); subsequent incremental builds are
fast.

## License

[MIT](./LICENSE) © Justin Woodring

<sub>Built for people who want to own their records. 🛡</sub>
