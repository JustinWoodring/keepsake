# Cryptographic design

## Key hierarchy

```
password ──Argon2id──► master_key ──┬──► unseal vault_key (per-user sealed_keys row)
                                   │
                                   ├──► envelope_sk   = HKDF(master_key, "keepsake/envelope/v1") → Ed25519 seed
                                   │
                                   └──► bearer_secret = HKDF(master_key, "keepsake/bearer/v1")

vault_key ──HKDF──► aead_key = HKDF(vault_key, "aead/v1")          # 32 bytes
                    per-record nonce = random 24 bytes
                    per-record AAD   = "keepsake/record/v1\n" || type || 0x00 || schema_version || 0x00 || uuid
                    ciphertext       = XChaCha20-Poly1305(key=aead_key, nonce, plaintext, AAD)
```

The same key hierarchy is used for `.ksk` exports, with a
per-export `export_key` derived from the user-supplied
export passphrase.

## Primitives

| Purpose                  | Primitive                | Notes |
|--------------------------|--------------------------|-------|
| Password KDF             | Argon2id (m=64 MiB, t=3, p=4) | ~200 ms on a modern laptop |
| Authenticated encryption | XChaCha20-Poly1305       | 24-byte nonce, 16-byte tag |
| Key derivation           | HKDF-SHA-512             | All subkeys derived via `HKDF(ikm, salt, info, len)` |
| Server authentication    | Ed25519                  | Deterministic 32-byte seed from HKDF |
| Audit chain              | blake3 keyed             | `hash_n = blake3(seq, op, actor, target_id, details, ts, prev_hash_n-1)` |

All primitives come from well-audited Rust crates:
`argon2`, `chacha20poly1305`, `hkdf`+`sha2`, `ed25519-dalek`,
and `blake3`.  None are home-rolled.

## Why XChaCha20-Poly1305

* 24-byte nonce allows safe random nonces without realistic
  collision risk (`2^192` space).
* AEAD construction gives confidentiality + integrity in one
  primitive.
* Faster than AES-GCM on platforms without AES-NI.

## Why per-record AAD

The AAD binds the ciphertext to its `record_type` and
`record_id`.  This means an attacker who can swap rows in
the database cannot move a credential's ciphertext into a
note's slot — decryption fails.

## Why per-record HKDF was dropped

The earlier design used `HKDF(vault_key, record_type ||
record_id)` to derive a per-record AEAD key.  The
simplified design uses a single vault-wide AEAD key and
relies on per-record (nonce, AAD) to provide isolation.
The two designs are functionally equivalent given that the
AAD includes both `record_type` and `record_id`; the
simpler design has fewer moving parts.

## Audit chain

Every entry `E_n` is `blake3(seq, op, actor, target_id,
details, ts, prev_hash_n-1)`.  The genesis entry has
`prev_hash = 0x00 * 32`.  Any tamper with a single entry
breaks every subsequent hash; verification walks the chain
and reports the first broken entry.

## Zeroize

All key material is wrapped in `Zeroizing<_>` and
implementations of `Drop` zero their contents.  The
`master_key`, `vault_key`, and envelope signing key are
all zeroized on drop, on lock, and on process exit.

## What's not here yet

* **SQLCipher at-rest encryption.**  v1 uses plain SQLite.
  Production deployments should turn on the SQLCipher
  feature in `crates/keepsake-core/Cargo.toml`.  This is a
  one-line change.
* **Large attachments.**  v1 inlines everything up to 256
  KiB.  Larger files require a separate blob chunking path.
* **Y-CRDT integration.**  v1 ships the `doc` module as a
  thin opaque-blob wrapper.  The wire format is stable; the
  CRDT merge can be added without breaking changes.
