# Deploying the keepsake sync server

A runbook for taking the `keepsake-server` binary from a fresh
Ubuntu VPS to a working sync endpoint reachable from your
desktop and laptop.

## TL;DR

```bash
# 1. On the VPS:
sudo apt update && sudo apt install -y nginx certbot python3-certbot-nginx
sudo tee /etc/systemd/system/keepsake-server.service >/dev/null <<'UNIT'
[Unit]
Description=keepsake sync server
After=network.target

[Service]
Type=simple
User=keepsake
Group=keepsake
WorkingDirectory=/var/lib/keepsake
Environment=KEEPSAKE_ADDR=127.0.0.1:8484
Environment=KEEPSAKE_DB=/var/lib/keepsake/server.db
ExecStart=/usr/local/bin/keepsake-server
Restart=on-failure
RestartSec=5
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/keepsake
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
UNIT
sudo useradd -r -s /usr/sbin/nologin -d /var/lib/keepsake keepsake
sudo mkdir -p /var/lib/keepsake
sudo cp target/release/keepsake-server /usr/local/bin/
sudo chown -R keepsake:keepsake /var/lib/keepsake
sudo systemctl daemon-reload
sudo systemctl enable --now keepsake-server

# 2. Nginx reverse proxy with TLS (replace sync.example.com):
sudo tee /etc/nginx/sites-available/keepsake >/dev/null <<'NGINX'
server {
    listen 80;
    server_name sync.example.com;
    location / {
        return 301 https://$host$request_uri;
    }
}
server {
    listen 443 ssl http2;
    server_name sync.example.com;
    client_max_body_size 64m;

    ssl_certificate     /etc/letsencrypt/live/sync.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/sync.example.com/privkey.pem;

    location / {
        proxy_pass http://127.0.0.1:8484;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_read_timeout 60s;
    }
}
NGINX
sudo ln -sf /etc/nginx/sites-available/keepsake /etc/nginx/sites-enabled/
sudo certbot --nginx -d sync.example.com
sudo systemctl reload nginx

# 3. Verify:
curl https://sync.example.com/v1/health
# {"kind":"ok","message":"ok"}
```

That's it. Below is the rationale and the operational details.

## Architecture

```
+-------------+         HTTPS          +---------------+        HTTP          +-------------------+
|  keepsake   |  ------------------->  |  nginx        |  ---------------->   |  keepsake-server  |
|  desktop    |   POST /v1/vaults/.. |  (TLS term)   |   127.0.0.1:8484    |  axum + SQLite     |
|  or CLI     |                       |               |                       |                   |
+-------------+                       +---------------+                       +-------------------+
                                                                                       |
                                                                                       v
                                                                          /var/lib/keepsake/server.db
                                                                          (encrypted payloads only)
```

The server is a **dumb encrypted blob store**.  It never sees
plaintext, never holds per-user state, and does not authenticate
clients.  The only access boundary is the `vault_id` URL segment.

### How it works

* Each **vault** is a shared encrypted space.  Everyone who knows
  the vault's passphrase can read and write to it; the server has
  no idea who they are.
* The vault's encryption key is derived from the shared
  passphrase + `vault_id` using Argon2id (client-side, in
  `keepsake-core`).  Same passphrase + same `vault_id` ⇒ same
  key, on every device, with no key exchange.
* Records are pushed as opaque AEAD ciphertext keyed by that
  shared key.  The server stores the bytes, a `(ts_millis,
  actor)` ordering tuple, and a per-actor lamport clock.
* Pull returns either the **current state** (one row per record,
  LWW-picked by `(ts_millis, actor)`) when `since` is empty, or
  the **change feed since** a vector clock when `since` is set.
  No merge happens on the server — the client applies LWW /
  per-text CRDT logic locally.
* Blobs (attachments) are addressed by SHA-256 of the encrypted
  ciphertext, scoped to the vault.  `PUT /v1/vaults/:id/blobs/:sha256`
  is idempotent.

The server **never** has any way to decrypt a payload, so a
server compromise leaks ciphertext and metadata (lamport clocks,
actor IDs, change counts) but not vault content.

## Server configuration

The server takes two env vars (both optional):

| Var | Default | Notes |
|---|---|---|
| `KEEPSAKE_ADDR` | `127.0.0.1:8484` | Listen address. Keep this on localhost; nginx fronts it. |
| `KEEPSAKE_DB` | `./keepsake-server.db` | SQLite path. Use `/var/lib/keepsake/server.db` in production. |

There is no `KEEPSAKE_TLS` env var, no auth token config, no
registration flag.  The Rust binary does not terminate TLS; put
nginx or caddy in front of it.

## Host requirements

* **CPU/memory**: tiny.  The server is a single-threaded axum
  process holding one SQLite file.  1 vCPU / 512 MB is plenty.
* **Storage**: SQLite grows with change volume, not vault
  size.  1 GB is plenty for years of personal use.
* **Network**: HTTPS in, port 443 open.
* **Hostname**: A domain you control, with DNS pointing at
  the VPS.  The TLS cert requires a real domain.  (You can
  use a self-signed cert on the LAN for testing, but the
  desktop app will reject self-signed without extra config.)

## Setup walkthrough

### 1. Provision the VPS

Any Linux VPS works.  Tested on Ubuntu 22.04 / 24.04.

```bash
ssh root@sync.example.com
apt update && apt install -y nginx certbot python3-certbot-nginx ufw
ufw allow OpenSSH
ufw allow 'Nginx Full'
ufw enable
```

### 2. Create a service user

```bash
sudo useradd -r -s /usr/sbin/nologin -d /var/lib/keepsake keepsake
sudo mkdir -p /var/lib/keepsake
sudo chown -R keepsake:keepsake /var/lib/keepsake
```

### 3. Install the binary

Build it locally (release build, statically linked if you
want to avoid libc mismatches):

```bash
cargo build --release -p keepsake-server
# Produces target/release/keepsake-server
```

Copy to the VPS:

```bash
scp target/release/keepsake-server root@sync.example.com:/usr/local/bin/
ssh root@sync.example.com "chmod 755 /usr/local/bin/keepsake-server"
```

### 4. systemd unit

The unit file from the TL;DR runs the server as the
unprivileged `keepsake` user, with `KEEPSAKE_ADDR` pinned to
localhost (so it isn't directly reachable from the
internet — only via nginx).  Enable and start it:

```bash
sudo tee /etc/systemd/system/keepsake-server.service >/dev/null <<'UNIT'
[Unit]
Description=keepsake sync server
After=network.target

[Service]
Type=simple
User=keepsake
Group=keepsake
WorkingDirectory=/var/lib/keepsake
Environment=KEEPSAKE_ADDR=127.0.0.1:8484
Environment=KEEPSAKE_DB=/var/lib/keepsake/server.db
ExecStart=/usr/local/bin/keepsake-server
Restart=on-failure
RestartSec=5
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/keepsake
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
UNIT

sudo systemctl daemon-reload
sudo systemctl enable --now keepsake-server
sudo systemctl status keepsake-server
```

`status` should show `active (running)`.  Logs go to
journald; view with `journalctl -u keepsake-server -f`.

### 5. TLS via nginx + Let's Encrypt

The unit binds to localhost; nginx fronts it.  Point DNS
for `sync.example.com` at the VPS, then:

```bash
sudo tee /etc/nginx/sites-available/keepsake >/dev/null <<'NGINX'
server {
    listen 80;
    server_name sync.example.com;
    location / {
        return 301 https://$host$request_uri;
    }
}
server {
    listen 443 ssl http2;
    server_name sync.example.com;
    client_max_body_size 64m;

    ssl_certificate     /etc/letsencrypt/live/sync.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/sync.example.com/privkey.pem;

    location / {
        proxy_pass http://127.0.0.1:8484;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_read_timeout 60s;
    }
}
NGINX

sudo ln -sf /etc/nginx/sites-available/keepsake /etc/nginx/sites-enabled/
sudo certbot --nginx -d sync.example.com
sudo systemctl reload nginx
```

Certbot sets up auto-renewal via a systemd timer; verify with
`sudo systemctl list-timers | grep certbot`.

### 6. Verify

```bash
curl -s https://sync.example.com/v1/health
# {"kind":"ok","message":"ok"}
```

If that returns `ok`, the server is up and reachable over TLS.

## Pointing the desktop app at the server

1. Open Keepsake on the desktop.
2. Go to **Sync** in the sidebar.
3. Enter `https://sync.example.com` in the **server URL** field.
4. Enter a **vault id** (any string you and your collaborators
   agree on, e.g. `family` or `team-shared`).  The vault id is
   the only access boundary on the server.
5. Enter the shared **passphrase** in the unlock flow — this
   derives the vault's AEAD key client-side.
6. Click **Push** to upload every local change.
7. Repeat on the second device, entering the same server URL,
   vault id, and passphrase.  **Pull** to fetch.

The same flow works on the CLI:

```bash
keepsake unlock                  # enter the shared passphrase
keepsake sync push --server https://sync.example.com --vault family
keepsake sync pull --server https://sync.example.com --vault family
```

If `--server` and `--vault` are omitted, the CLI prompts
interactively.

## Operations

### Backups

The server's only state is `server.db`.  It contains:

* **Public** data: vault ids, vector clocks, change counts,
  actor IDs.
* **Encrypted** data: AEAD-encrypted record payloads (the
  server can't read these), opaque blobs.

A nightly `cp /var/lib/keepsake/server.db /backups/keepsake-$(date +%F).db`
is enough.  Encrypted payloads in the DB are useless to anyone
who doesn't know the shared passphrase, so the backup file
itself is safe to keep around (e.g. encrypted with age/gpg at
rest, or stored in an encrypted backup target).

Restoration is `cp /backups/<file> /var/lib/keepsake/server.db && systemctl restart keepsake-server`.

### Log rotation

`journald` handles log rotation.  The `StandardOutput=journal`
default is fine; adjust retention with `journalctl --vacuum-time=30d`.

### Backups of the *vault* itself

This server is **not** a backup of the local vault file.  It's a
sync endpoint.  The vault on each device is the source of truth.
The server just keeps ciphertext so multiple devices can converge.
A real backup (e.g. nightly encrypted sync of `~/keepsake/vault.db`
to a cold-storage bucket) is a separate concern; out of scope for
this doc.

### Database growth

SQLite grows as new `Change` rows are appended.  `Change` rows
are tiny (UUID, lamport, ts, record_id, payload).  A 50-record
vault that's been pushed weekly for a year is on the order of
tens of kilobytes.  No compaction is needed; the schema doesn't
currently expose one, but the audit chain can be trimmed in a
future version if it ever becomes a concern.

### Updates

```bash
# Build a new binary locally.
cargo build --release -p keepsake-server

# Push it to the VPS.
scp target/release/keepsake-server root@sync.example.com:/usr/local/bin/keepsake-server.new

# On the VPS, swap it in and restart.
ssh root@sync.example.com '
    install -m 755 /usr/local/bin/keepsake-server.new /usr/local/bin/keepsake-server &&
    rm /usr/local/bin/keepsake-server.new &&
    systemctl restart keepsake-server
'
```

There is no data migration between server versions in v1.  The
SQLite schema is stable; future versions will add tables, never
drop or rename them.

### Monitoring

A few signals worth checking:

```bash
# Is the process up?
systemctl is-active keepsake-server

# Recent errors?
journalctl -u keepsake-server --since "1 hour ago" -p err

# Disk usage of the SQLite file?
ls -lh /var/lib/keepsake/server.db
```

A simple uptime check (e.g. `curl -fsS https://sync.example.com/v1/health`
from an external monitor every 5 minutes) is enough for personal
use.

## Threat model recap

* **Confidentiality**: payloads are AEAD-encrypted (XChaCha20-Poly1305)
  with a key derived from the shared passphrase + `vault_id`
  (Argon2id → HKDF) before leaving the device.  The server stores
  opaque ciphertext.  TLS protects data in transit.  A server
  compromise leaks ciphertext, lamport clocks, and actor IDs —
  not vault content.
* **Authentication of *readers***: the server does not authenticate
  clients.  Anyone who knows the `vault_id` *and* the shared
  passphrase can read and write the vault.  This is the intended
  design — the server is a dumb blob store, and membership is
  controlled entirely by the passphrase.  Treat the vault id
  the same way you'd treat a public channel name and the
  passphrase the way you'd treat a shared key.
* **Integrity**: every `Change` payload is an AEAD ciphertext,
  so a server cannot tamper with the contents without invalidating
  the MAC.  A malicious server could *drop* or *reorder* changes;
  the client detects this via vector-clock comparison and refetches
  the current state on divergence.
* **Availability**: this is one VPS.  If it goes down, sync
  stops; local vaults continue to work.  For multi-region
  availability, the server would need to be replicated
  (out of scope for v1).
* **Server compromise**: the attacker gets ciphertext, vector
  clocks, vault ids, and actor IDs.  They cannot decrypt any
  payload without each vault's shared passphrase.  They can deny
  service by deleting the database.  They can impersonate the
  server and feed forged changes to clients, but AEAD MACs on
  the payloads make forged changes undecryptable.

## What this guide doesn't cover

* **Vault discovery**: there is no "list of vaults" endpoint.
  Each device must be told the `vault_id` and passphrase
  out-of-band.  The server treats every `vault_id` as a valid
  namespace and will happily accept garbage ids; that is by
  design.
* **Per-user key management**: every member of a shared vault
  derives the same vault key from the same passphrase.  Adding
  or removing a member is a passphrase rotation.  The old
  passphrase's ciphertext is no longer readable; clients that
  still hold the old passphrase can read old changes but will
  not see new ones.
* **Per-user conflict resolution across users**: the CRDT layer
  (LWW per record, per-text CRDT) handles the case where the
  *same* member edits the same record on two devices.  It does
  not yet handle the case where two *different* members edit
  the same record simultaneously — the first LWW writer wins
  across the whole vault.  In practice this is a v2 problem.
* **Sync at the record type level**: the server stores
  ciphertext for every record type.  The client pushes and
  pulls everything.  Per-type sync (e.g. "sync only notes")
  is a future feature.
