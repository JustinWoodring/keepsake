# Threat model

This document is the authoritative description of what
`keepsake` is and isn't designed to defend against.  It also
documents the trust relationships between the client, the
self-hosted server, and the storage layer.

## Goal

A `keepsake` vault contains the kind of information you would
keep in a physical family safe: online account credentials,
government identification, insurance and policy numbers,
health information, financial accounts, infrastructure
decisions, scenario runbooks, and dated work logs.  The goal
is to make the digital version of this collection as
resistant as the physical one — including against the
operator of the server.

## Trust model at a glance

| Component        | Sees plaintext? | Can impersonate users? | Can deny service? |
|------------------|-----------------|------------------------|--------------------|
| Local device     | ✅ yes (after unlock) | ✅ that user | ✅ |
| Sync server      | ❌ no            | ❌ no (Ed25519 check)   | ✅ |
| Backup of `vault.db` (no password) | ❌ no | ❌ no | n/a |
| Backup of `vault.db` + password   | ✅ yes | ✅ that user | n/a |

The single point of total compromise is **`vault.db` plus
any one user's password**.  The sync server never appears in
that table because it never has access to either ingredient.

## Cryptographic keys

* **Master key** — derived from the user's password using
  Argon2id with `m = 64 MiB`, `t = 3`, `p = 4`.  Lives only
  in process memory while the vault is unlocked; zeroized on
  drop and on lock.
* **Vault key** — random 32 bytes generated at vault
  initialization, sealed under the master key with
  XChaCha20-Poly1305 and stored in the `sealed_keys` table.
  Decrypts every record and attachment at rest.
* **AEAD key** — `HKDF-SHA-512(vault_key, "aead/v1")`; used
  to encrypt each record's fields blob.  The nonce is fresh
  per record; the AAD is
  `record_type || schema_version || record_id`.
* **Envelope signing key** — `HKDF-SHA-512(master_key,
  "keepsake/envelope/v1")` → Ed25519 seed.  Used to sign
  every sync request to the server.
* **Bearer secret** — `HKDF-SHA-512(master_key, "keepsake/bearer/v1")`.
  Short-lived authentication for the server.

The envelope signing key and the vault key are derived from
disjoint key material: the server can authenticate a user
without ever being able to derive the vault key.

## Adversaries we defend against

* **Curious server operator.**  Sees only ciphertext, public
  keys, and timestamps.  Cannot read, forge, or selectively
  tamper with vault contents.
* **Network observer.**  Same as above for transport; TLS
  terminates at the server's reverse proxy.
* **Backup leak.**  An attacker who exfiltrates `vault.db`
  but not the user's password cannot read it in plausible
  time.  Argon2id is the wall; bumping the parameters makes
  this even more expensive.
* **Lost or stolen device, locked.**  Same as backup leak:
  without the user's password the file is opaque.  Without
  the file at all, no attack is possible.

## Adversaries we don't defend against

* **Compromised client binary.**  If the running `keepsake`
  is a tampered version (modified installer, malicious
  dependency, etc.), it can read plaintext and exfiltrate it
  freely.  Mitigations live outside the application: signed
  releases, reproducible builds, and supply-chain hygiene.
* **Coerced disclosure.**  If an attacker can force the user
  to type their password (duress, "evil maid" with rubber
  hose), the vault is theirs.  `.ksk` exports to offline
  media are a partial mitigation: a long, written-down
  export passphrase can be different from the daily
  password, and the daily password can be rotated.
* **Compromised OS / keylogger / screen scraper.**  Anything
  that captures the master password at the OS layer
  compromises the vault.
* **Side-channel attacks on the unlocked process.**  Memory
  disclosure, Spectre/Meltdown, or rowhammer on the host.
  Out of scope for v1; the `zeroize` crate gets us most of
  the way for cold-boot attacks.

## What lives where

| Data                    | At rest on device | On the server |
|-------------------------|-------------------|----------------|
| Master key              | Process memory only | Never |
| Vault key               | Sealed in `sealed_keys` row | Never |
| Envelope signing key    | Process memory only | Never |
| Bearer secret           | Process memory only | Never |
| Record fields           | Encrypted in `records` table | Encrypted in sync blob |
| Note body               | Encrypted in `records` table | Encrypted in sync blob |
| Attachment plaintext    | Encrypted in `attachments` table | Encrypted in sync blob |
| Audit chain             | `audit` table with hash chain | Opaque ciphertext (server stores it but cannot verify) |
| Audit actor pubkey      | In the entry itself | Same as on device |

The server stores audit entries as opaque ciphertext, sealed
under the vault key.  It can store them and serve them
back, but it cannot verify the chain or read the contents.

## Recovery

* **Per-user `.ksk` export.**  Encrypted to a
  user-supplied passphrase (which can differ from the
  daily password).  Contains the sealed vault key plus all
  records and attachments.  Recommended storage: encrypted
  USB drive in a fireproof location.
* **Password rotation.**  Per-user.  Re-seals the vault
  key under the new password; the vault key itself is
  stable.  Old `.ksk` exports still work because they
  carry their own sealed vault key.
* **Adding a new device.**  Either import a `.ksk` on the
  new device, or sign in with the same username + password
  on the new device and let sync pull the rest from the
  server.

## Why we don't have "password reset"

There is no server-side password reset because the server
has no key material to reset.  Password recovery is by
export only.  This is intentional and is the cost of true
E2E encryption.
