//! `init` — create a new vault on this device.

use chrono::Utc;
use rand::RngCore;

use keepsake_core::crypto::KdfParams;
use keepsake_core::identity::{password_to_master_key, seal_vault_key, EnvelopeKey, VaultKey};
use keepsake_core::vault::{SealedKeyRow, Vault};
use keepsake_core::session::Session;

pub async fn run(path: &std::path::Path, username: Option<String>, session: &mut Option<Session>) -> anyhow::Result<()> {
    if path.exists() {
        return Err(anyhow::anyhow!("vault already exists at {}", path.display()));
    }

    let username = match username {
        Some(u) if !u.trim().is_empty() => u,
        _ => dialoguer::Input::new().with_prompt("username").interact_text()?,
    };
    let password = rpassword::prompt_password("password: ")?;
    let confirm  = rpassword::prompt_password("confirm:  ")?;
    if password != confirm {
        return Err(anyhow::anyhow!("passwords do not match"));
    }

    let params = KdfParams::default();
    let (master, salt) = password_to_master_key(password.as_bytes(), params)?;
    let vault_key = VaultKey::from_bytes(random_key_bytes());
    let sealed = seal_vault_key(&master, &vault_key)?;
    let envelope = EnvelopeKey::from_master_key(&master)?;

    let vault = Vault::open_or_create(path)?;
    let row = SealedKeyRow {
        username: username.clone(),
        device_id: random_device_id(),
        kdf_salt: salt.0,
        kdf_params: params.encode(),
        seal_nonce: sealed.nonce,
        seal_ciphertext: sealed.ciphertext,
        envelope_pk: envelope.public_key().to_bytes(),
        created_at: Utc::now(),
    };
    vault.put_sealed_key(&row)?;
    drop(vault);

    // Open the vault, install the in-memory key, and write a
    // genesis audit entry.
    let mut vault = Vault::open_or_create(path)?;
    vault.unlock(&vault_key)?;
    vault.append_audit(
        keepsake_core::audit::AuditOp::AddUser,
        &username,
        Some(&username),
        Some("vault initialized"),
    )?;
    vault.append_audit(
        keepsake_core::audit::AuditOp::Unlock,
        &username,
        None,
        None,
    )?;
    let _ = vault;

    *session = Some(keepsake_core::session::Session {
        path: path.to_path_buf(),
        vault,
        master,
        username,
    });

    println!("initialized vault at {}", path.display());
    Ok(())
}

fn random_key_bytes() -> [u8; 32] {
    let mut k = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut k);
    k
}

fn random_device_id() -> [u8; 16] {
    let mut d = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut d);
    d
}
