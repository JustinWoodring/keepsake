# Sync protocol

The sync server stores opaque ciphertext and forwards
signed CRDT updates.  The server cannot read or forge
vault contents; it authenticates users by verifying
Ed25519 signatures against a `envelope_pk` registered at
signup time.

## Transport

HTTP/1.1 over TLS (terminated by the reverse proxy in
front of the server).  JSON request and response bodies.
Long-poll on `GET /v1/changes` returns within 30 seconds
or when there are new changes.

## Authentication

Every request carries:

* `Authorization: Bearer <bearer>` — short-lived bearer
  derived from the user's master key via HKDF.
* `X-Envelope-Pk: <hex>` — the user's envelope public key.
* `X-Signature: <hex>` — Ed25519 signature over the
  canonical serialization of the request body.

The server verifies the signature with the public key,
then validates the bearer against the username, then
routes the request.

## Endpoints

```
POST /v1/auth/register        { username, envelope_pk }                 → 201 | 409
POST /v1/auth/login           { username, challenge, signature }         → { bearer, expires_at }
GET  /v1/changes?since=vc     long-poll ≤30s, returns { updates: [..], vc }
POST /v1/push                 { updates: [..], new_vc }                 → 204
GET  /v1/blobs/{hash}         → ciphertext bytes
PUT  /v1/blobs/{hash}         body=ciphertext
GET  /v1/audit?since=seq      opaque audit tail
```

## Request and response types

Defined in `crates/keepsake-core/src/sync/protocol.rs`.
The wire types are versioned; the first version is
implicit and matches the file as written.

## CRDT model (planned)

The Y-CRDT engine will run on top of the existing
`Engine`.  Each vault has one YDoc per scope
(`"notes"`, `"records-index"`).  Updates are
`yrs::Update = Vec<u8>` wrapped in an AEAD envelope
keyed by `HKDF(vault_key, "crdt/v1")`.

The vector clock is per-actor; the server stores updates
keyed by the latest vector clock it has seen from each
actor.  Pulls are `since = client_clock`; pushes are
`new_clock` + the batch of changes.

## What the server sees

```json
{
  "username": "justin",
  "envelope_pk": "9Kp2...rT4v",
  "body": {
    "kind": "push",
    "new_clock": { "counters": { "justin": 17 } },
    "changes": [
      {
        "id": "...",
        "lamport": 17,
        "ts": "2026-06-17T16:00:00Z",
        "author": "justin",
        "record_id": "...",
        "payload": "<opaque ciphertext>"
      }
    ]
  }
}
```

The server can:

* Verify the signature.
* Increment counters.
* Store and forward the opaque payload.
* Tell the client which actors have advanced since the
  client's last clock.

The server cannot:

* Decrypt `payload`.
* Forge updates for a user (no signing key).
* Replay a delete as a non-delete (signature is over the
  body).
* Distinguish one record type from another (record types
  are inside the ciphertext).
