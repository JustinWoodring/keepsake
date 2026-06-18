# Sync protocol

The sync server is a **dumb encrypted blob store**.  It does not
authenticate clients, does not hold per-user state, and stores
opaque bytes.  Membership in a vault is controlled entirely by
knowledge of a shared sync passphrase, and the data is
double-sealed on the wire.

## Threat model recap

| Adversary | What they get | What they don't get |
|---|---|---|
| Network observer (TLS) | Encrypted TLS bytes | Anything |
| Server operator | `vault_id`, `(record_id, lamport, ts, author)`, doubly-sealed payload bytes | Plaintext records, vault key, sync passphrase |
| Server impersonator (MITM) | Same as above, but feeds forged payload bytes to clients | Decryptable plaintext (AEAD MAC fails) |
| Compromised server DB | Same as operator | Plaintext |
| Stolen vault file (no master key) | Sealed user envelopes, sealed sync keys, encrypted record rows | Anything usable |

The protocol assumes TLS is terminated by a reverse proxy in
front of the server (see `docs/deploy-sync-server.md`).  The
Rust binary does not terminate TLS.

## The three keys

Three 32-byte keys exist.  All are derived, none are stored in
the clear.

### 1. `master_key` — per-user

Derived from the user's password via Argon2id:

```text
master_key = Argon2id(password, salt, m_kib, t, p)
```

Where `salt` and the KDF parameters are stored in the
`sealed_key_rows` table (one row per user, alongside the
sealed vault key).  Never written in the clear.  Zeroized
on lock.

### 2. `vault_key` — per-vault, **shared across all users**

Generated once at vault init as 32 random bytes.  Sealed
once per user under that user's `master_key`.  Stored in
the `sealed_key_rows` table; never written in the clear.
Zeroized on lock.

**All users of a vault share the same `vault_key`.**  This is
why "different passwords → same vault key": each user derives
a different `master_key` from their password, but every
`master_key` can unseal the same `vault_key` blob.

### 3. `shared_sync_key` — per-(passphrase, vault_id), shared across all users of a vault

Derived from a sync passphrase + `vault_id`:

```text
salt       = blake3("keepsake/shared-vault/v1\n" || vault_id)[:16]
master     = Argon2id(sync_passphrase, salt, m_kib=8MiB, t=3, p=1)
shared_key = HKDF(master, salt, "keepsake/shared-vault/key/v1", 32)
```

Implementation: `keepsake_core::sync::client::derive_shared_key`.

The `shared_sync_key` is **stored inside the local vault**
sealed under the `vault_key` (in the `shared_sync_keys` table
— see `docs/vault-schema.md` if/when it exists, or
`keepsake-core/src/vault/mod.rs`).  This means:

- After `unlock`, the `vault_key` is in memory, which
  unseals the `shared_sync_key`.  **No passphrase re-entry
  is needed for sync.**
- Every user of the vault has the same `shared_sync_key`
  (they all hold the same `vault_key`, which sealed it).
- Rotating the sync passphrase rotates the `shared_sync_key`
  but doesn't touch the `vault_key` or any user's
  `master_key`.

## Local envelope: how a record is stored in the SQLite vault

Every row in the `records` table is:

```sql
CREATE TABLE records (
    id          BLOB PRIMARY KEY,
    type        TEXT NOT NULL,
    schema_ver  INTEGER NOT NULL,
    created_by  TEXT NOT NULL,
    updated_by  TEXT NOT NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    aead_nonce  BLOB NOT NULL,    -- 24 bytes
    aead_aad    BLOB NOT NULL,    -- variable
    ciphertext  BLOB NOT NULL     -- variable
);
```

The encryption is:

```text
aad =
    "keepsake/record/v1\n" ||
    type || 0x00 ||
    schema_version (u32 LE) || 0x00 ||
    record_id (16 bytes)
nonce       = 24 random bytes
ciphertext  = AEAD_encrypt(
                key:       vault_key,
                nonce:     nonce,
                plaintext: serde_json::to_vec(record),
                aad:       aad)
```

The local vault's `put_record` and `get_record` use exactly
this format.  See `keepsake-core/src/vault/mod.rs:build_aad`
and `keepsake-core/src/vault/mod.rs:put_record`.

The "inner envelope" of a record is the four-tuple
`(aead_nonce, aead_aad, ciphertext)`.  This tuple is what
gets nested inside the outer sync envelope.

## Wire envelope: how a record is sent to the sync server

Each `Change` carries a `payload` field.  The bytes of
`payload` are the **nested** encryption of the local
envelope:

```text
inner_len     = 4 (aad_len as u32 LE)
                + 24 (nonce)
                + aad_len
                + ciphertext_len
inner_plain   = inner_len (u32 LE, big-endian? see below)
                || aead_nonce (24 bytes)
                || aad_len    (u32 LE)
                || aead_aad
                || ciphertext
outer_nonce   = 24 random bytes
payload       = outer_nonce
                || AEAD_encrypt(
                     key:       shared_sync_key,
                     nonce:     outer_nonce,
                     plaintext: inner_plain,
                     aad:       "keepsake/sync/payload/v1")
```

Length prefixing is required because `aead_aad` is
variable-length (it contains the record `type` string).

The server stores `payload` as opaque bytes.  The server
**cannot** peel either layer: it doesn't have
`shared_sync_key`, and even if it did, peeling the outer
would only expose the local envelope — which is sealed
under the `vault_key` that the server also doesn't have.

## Endpoints

```
GET  /v1/health
POST /v1/vaults/:id/sync/push        body: { new_clock, changes: [..] }  → 204
POST /v1/vaults/:id/sync/pull        body: { since?: VectorClock }       → { changes: [..], current_clock }
PUT  /v1/vaults/:id/blobs/:sha256    body: opaque ciphertext             → 204
GET  /v1/vaults/:id/blobs/:sha256                                       → opaque ciphertext
```

`vault_id` is a string matching `[A-Za-z0-9_-]+` of length
1..=64.  There is no auth, no per-user state, and no
registration step.

### Push semantics

* Client sends `{ new_clock, changes: [..] }`.
* Each `Change` is `{ id, lamport, ts, author, record_id, payload }`
  where `payload` is the nested envelope described above.
* Server `INSERT OR IGNORE`s each `Change.id` (idempotent
  dedup).  It also updates the per-actor lamport clock in
  the `clocks` table.
* `lamport` must be strictly greater than the actor's last
  seen clock for that `vault_id`, else the request is
  rejected with 409 (out-of-order push).

### Pull semantics

* Empty `since` (or missing) → return **current state**:
  one row per `record_id`, picked by LWW over
  `(ts_millis ASC, author ASC)`.  Returns the union of all
  `vault_id`-scoped current records.
* Non-empty `since` → return the **change feed**:
  all `Change` rows whose `lamport` is greater than
  `since.counters.values().max()` (or all of them, if the
  map is empty).
* Response includes `current_clock` so the client can use
  it as the next `since`.

## What the server stores per vault

```sql
CREATE TABLE changes (
    id          BLOB PRIMARY KEY,   -- UUID v4 of the Change row
    vault_id    TEXT NOT NULL,
    lamport     INTEGER NOT NULL,
    ts_millis   INTEGER NOT NULL,
    author      TEXT NOT NULL,
    record_id   BLOB,               -- nullable; record-type changes
    payload     BLOB NOT NULL       -- nested envelope bytes
);
CREATE INDEX changes_vault_lamport_idx ON changes(vault_id, lamport);

CREATE TABLE clocks (
    vault_id    TEXT NOT NULL,
    actor       TEXT NOT NULL,
    lamport     INTEGER NOT NULL,
    PRIMARY KEY (vault_id, actor)
);

CREATE TABLE blobs (
    vault_id    TEXT NOT NULL,
    sha256      BLOB NOT NULL,      -- 32 bytes
    ciphertext  BLOB NOT NULL,
    PRIMARY KEY (vault_id, sha256)
);
```

Indexes: `(vault_id, lamport)` for change-feed scans;
`(vault_id, record_id)` for current-state aggregation.

## What the client does on push

1. For each local record the user wants to publish:
   1. Read the inner envelope `(aead_nonce, aead_aad, ciphertext)`
      directly from the local `records` table — **no
      decryption, no plaintext in flight**.
   2. Build `inner_plain` (length-prefixed) per the wire
      format above.
   3. Generate a fresh `outer_nonce`.
   4. Compute `payload = outer_nonce || AEAD(shared_sync_key, outer_nonce, inner_plain, "keepsake/sync/payload/v1")`.
   5. Build a `Change { id: uuid, lamport, ts, author, record_id, payload }`.
2. POST to `/v1/vaults/:id/sync/push` with
   `{ new_clock, changes }`.

## What the client does on pull

1. POST to `/v1/vaults/:id/sync/pull` with
   `{ since: <client_clock> }` (or empty for current state).
2. For each `Change`:
   1. Peel the outer: `inner_plain = AEAD_decrypt(shared_sync_key, outer_nonce, ciphertext, "keepsake/sync/payload/v1")`.
   2. Parse the length prefix; split into
      `(aead_nonce, aead_aad, ciphertext)`.
   3. **Validate** by attempting `AEAD_decrypt(vault_key, aead_nonce, ciphertext, aead_aad)`.  If the
      vault key can't decrypt it, the change is from a
      different vault and must be skipped.  (This should
      not happen in v1; the check is defense-in-depth.)
   4. Run the change through the CRDT layer (LWW per
      record, per-text CRDT) to decide whether to apply.
   5. If applying, write the inner envelope
      `(aead_nonce, aead_aad, ciphertext)` directly to the
      local `records` table via `vault.put_record_envelope`
      — **no re-encryption**, no plaintext on disk.
3. Update the local `VectorClock` to the server's
   `current_clock`.

## Why nesting (not re-encryption)

I keep wanting to "decrypt with `vault_key`, re-encrypt with
`shared_sync_key`, send plaintext" or vice versa.  **Don't.**
Reasons:

* All users of a vault share the same `vault_key`.  The
  inner envelope sealed by user A is byte-identical to what
  user B would write, and decryptable by user B with the
  same `vault_key`.  Nesting works without re-keying.
* Re-encryption requires touching plaintext on at least
  one side.  Nesting never does.
* The server can't tell whether two pushes came from the
  same user or different users — the inner envelope
  contents are identical for a given record content.  This
  is good for cross-user dedup via `record_id`.
* Forward secrecy: compromising `vault_key` later doesn't
  retroactively reveal server-stored ciphertext, because
  the server ciphertext is keyed by `shared_sync_key` (a
  separate derivation).
* Key rotation: rotating the sync passphrase rotates
  `shared_sync_key` without touching the `vault_key` or
  any local envelope.  Rotating the `vault_key` rotates
  every local envelope but leaves server payloads
  untouched.

## What the protocol does NOT do

* **No per-user authentication.**  Anyone who can reach
  the URL and knows the `vault_id` can push and pull.
  Encryption (not authentication) is the access boundary.
* **No per-record access control.**  All members of a vault
  see all records.
* **No conflict resolution across distinct users.**  The
  CRDT layer (LWW per record) handles same-user
  multi-device.  Distinct-user concurrent edits resolve
  LWW, which may not be what you want.
* **No server-side compaction.**  The change log grows
  monotonically.  A future version can prune old
  `Change` rows once all clients have advanced past them.
* **No blob-level encryption beyond AEAD.**  Blobs are
  sealed with `shared_sync_key` in the same nested
  pattern.  Server stores `(vault_id, sha256, ciphertext)`.

## Local storage of the shared sync setup

The `shared_sync_keys` table lives inside the **local vault
SQLite file** and is sealed under the `vault_key`.  It is the
single source of truth for "what is the sync setup for this
vault?"

```sql
CREATE TABLE shared_sync_keys (
    vault_id     TEXT PRIMARY KEY,        -- e.g. "family"
    passphrase   BLOB NOT NULL,           -- sealed by vault_key (see below)
    passphrase_nonce BLOB NOT NULL,       -- 24 bytes
    kdf_salt     BLOB NOT NULL,           -- 16 bytes, blake3(vault_id)[:16]
    kdf_m_kib    INTEGER NOT NULL,        -- Argon2id memory
    kdf_t        INTEGER NOT NULL,        -- Argon2id iterations
    kdf_p        INTEGER NOT NULL,        -- Argon2id parallelism
    created_at   INTEGER NOT NULL,
    rotated_at   INTEGER                  -- nullable; set when passphrase rotates
);
```

The `passphrase` column stores the literal sync passphrase
the user entered, sealed under the `vault_key`:

```text
aad          = "keepsake/shared-sync-passphrase/v1\n" || vault_id
passphrase_nonce = 24 random bytes
sealed       = AEAD_encrypt(
                 key:       vault_key,
                 nonce:     passphrase_nonce,
                 plaintext: passphrase_utf8_bytes,
                 aad:       aad)
```

**Why store the passphrase in the clear (sealed) when we
also store the derived key?**  Because the user needs to be
able to re-share the passphrase with a new device, and the
passphrase cannot be recovered from the derived key.
Argon2id is one-way.  Storing the sealed passphrase is
consistent with the rest of the vault's threat model:
anyone with the vault file **and** a user's `master_key`
can already read every record, so allowing them to also
read the sync passphrase doesn't expand the attack surface.

**Why seal under the `vault_key` (not the user's
`master_key`)?**  Because the `vault_key` is shared across
all users.  This way, *any* user who unlocks the vault can
reveal the sync passphrase — they don't need to be the
user who originally set it up.

## UI: setup, reveal, rotate

The Sync page in the desktop app exposes three actions
once the vault is unlocked:

### Setup (one-time)

* User enters `vault_id` (free text; URL-safe) and
  `passphrase` (free text, no length constraint enforced
  by the server, but Argon2id works best with at least 12
  characters of entropy).
* Client validates `vault_id` matches `[A-Za-z0-9_-]+`
  and 1..=64 chars (same constraint the server enforces).
* Client derives `shared_sync_key = derive_shared_key(passphrase, vault_id)`.
* Client calls `vault.set_shared_sync_key(vault_id, passphrase, &shared_sync_key)`, which seals and stores the row.
* Client runs a first push to publish the local records to
  the new server vault.

### Reveal

* User clicks "Show sync setup".
* Client calls `vault.get_shared_sync_setup(vault_id) -> { vault_id, passphrase, kdf_salt, kdf_params, created_at, rotated_at }`.
* The UI displays `vault_id` and `passphrase` next to copy
  buttons, plus a warning that the passphrase grants
  read/write access to the entire vault and should be
  shared out-of-band only.
* The passphrase is shown in plain text; the user is
  responsible for not screenshotting it.  (A future
  enhancement: optionally a "recovery bundle" base64 blob
  that contains the same fields in a single copy.)

### Rotate

* User clicks "Rotate passphrase", enters a new passphrase.
* Client derives a new `shared_sync_key` from the new
  passphrase + the same `vault_id`.
* Client calls `vault.set_shared_sync_key(vault_id, new_passphrase, &new_shared_sync_key)` with `rotated_at` set.
* Client re-pushes.  All devices that have the **old**
  passphrase can no longer pull new changes (the new
  server-side ciphertext is keyed by the new
  `shared_sync_key`); they will see "decrypt failed" on
  pull until the user re-enters the new passphrase on
  those devices.
* Old devices can still read historical changes **only if
  the new device chose to also re-push them under the new
  key**, which is not done automatically in v1.  In
  practice, rotation is a "fork"; users should rotate
  rarely.

## Multi-user semantics

Because the `shared_sync_keys` table is sealed under the
`vault_key`, and the `vault_key` is the same for all users
of a vault:

* User A sets up sync.  Sealed passphrase + sealed key
  land in the table.
* User B unlocks the vault (with their own password).
  Their `master_key` unseals the same `vault_key`.  The
  same `vault_key` unseals the `shared_sync_keys` row.
  User B can reveal the passphrase and the vault_id.
* User B can then set up sync on their own device using
  the same `vault_id` and passphrase.
* No "invite flow" is needed: the second user just needs
  the `vault_id` and passphrase out-of-band from the
  first user, plus access to the vault itself (i.e. their
  own account on it).

## Threat-model delta from storing the passphrase

* **Compromised vault file + one user's password**: same
  as before — attacker can read all records, audit
  chain, and the sync passphrase.  The attacker could
  always have just observed the passphrase on the wire
  during a prior push, so this is not a meaningful
  expansion of attack surface.
* **Compromised vault file, no password**: attacker gets
  sealed envelopes, which they cannot unseal.  No
  change.
* **Compromised master password alone**: attacker can
  unseal the `vault_key`, then unseal the
  `shared_sync_keys` row.  They learn the sync passphrase
  and can join the server vault.  This is a real
  expansion: previously, knowing one user's password
  gave access only to that user's records, not to the
  shared sync state.  The mitigation is that the attacker
  also needs the **server URL + vault_id** to do anything
  with the passphrase, and the passphrase is the only
  thing protecting the server-side ciphertext.

If the multi-user risk above is unacceptable, the
alternative is to seal the `shared_sync_keys` row per-user
under each `master_key` instead of under the `vault_key`.
This trades convenience (any user can reveal) for
isolation (only the user who set it up can reveal).  We
chose the convenient option for v1.

## Design invariants — DO NOT VIOLATE

These are hard rules of the protocol.  Before changing any
of them, write a doc explaining why and get review.

### INV-1: `vault_key` is stored ONLY in the local vault.

The `vault_key` is generated locally at vault init as 32
random bytes and sealed inside `sealed_keys` against each
user's `master_key`.  It is **never** serialized, exported,
re-derived from a passphrase, or transmitted to the server
in any form.  All cross-device crypto derives from the
existing `master_key → vault_key` chain — not from any
synthesized "vault passphrase" or "shared secret".

If a feature seems to require the `vault_key` to leave the
device, that feature is mis-designed.  Find a way that uses
the existing `sealed_keys` table instead.

### INV-2: Cross-device sync reuses the existing user/password chain.

There is no "register device" flow and no out-of-band
shared secret required to add a device.  A new device joins
the sync group exactly the same way a new user joins the
local vault: it must already have (or be assigned) a user
account in the vault, and it must know that user's password.

The flow on a fresh device:

1. The new user already exists in the vault (was added on
   another device).
2. Their `sealed_keys` row (sealed under their `master_key`)
   is uploaded to the server the first time sync is set up
   (and on every new user addition).
3. The new device downloads the `sealed_keys` blob from
   the server, derives `master_key` from the user's
   password, unseals the `vault_key`, and uses it for
   their local vault.
4. **No passphrase exchange.  No key export.  No
   derivation from a shared secret.**  Just the existing
   password.

The user types their own password.  That's how it works
locally; that's how it works on a new device.

### INV-3: The server never holds plaintext keys.

The server sees only ciphertext.  The `vault_key` is
sealed under `master_key` (a per-user key Bob and Alice
never share with the server).  Records are sealed under
`vault_key`.  The wire envelope wraps records under
`shared_sync_key` (a separate derivation, distinct from
any key that decrypts the records).

The server can be compromised and leak only ciphertext.
Breaking the wire seal requires the user's `shared_sync_key`
which the server doesn't have.  Breaking the inner seal
requires the user's `vault_key` which the server doesn't
have.  These are independent, separately-rotatable secrets.

### INV-4: `shared_sync_key` is for the wire, NOT the master.

`shared_sync_key` derives from the sync passphrase and
wraps records for transit.  It is **not** the key that
protects the on-disk vault.  **Do not** use `shared_sync_key`
to encrypt or decrypt anything stored in the local vault.
**Do not** publish the `vault_key` (or any device-local key)
under `shared_sync_key`.

If you find yourself wanting to "publish the vault_key to
the server" so new devices can pull it, **stop**.  The right
answer is INV-2: ship the `sealed_keys` rows.
