//! `add-user` — add a second user to this device's vault.

use chrono::Utc;
use rand::RngCore;

use keepsake_core::audit::AuditOp;
use keepsake_core::crypto::KdfParams;
use keepsake_core::identity::{
    password_to_master_key, seal_vault_key, EnvelopeKey,
};
use keepsake_core::session::Session;
use keepsake_core::vault::SealedKeyRow;
use keepsake_core::Result;

use super::with_unlocked_mut;

pub async fn run(path: &std::path::Path, username: Option<String>, session: &mut Option<Session>) -> anyhow::Result<()> {
    let username = match username {
        Some(u) if !u.trim().is_empty() => u,
        _ => dialoguer::Input::new().with_prompt("new username").interact_text()?,
    };
    let password = rpassword::prompt_password("new password: ")?;
    let confirm  = rpassword::prompt_password("confirm:        ")?;
    if password != confirm {
        return Err(anyhow::anyhow!("passwords do not match"));
    }

    with_unlocked_mut(session, |sess| -> Result<()> {
        if sess.vault.get_sealed_key(&username)?.is_some() {
            return Err(keepsake_core::Error::AlreadyExists(username.clone()));
        }
        // Derive the new user's master key, derive their envelope
        // keypair, and seal the (existing) vault key under it.
        let params = KdfParams::default();
        let (new_master, salt) = password_to_master_key(password.as_bytes(), params)?;
        // Re-derive an existing vault key from *this* session's
        // master, then re-seal under the new master.
        let cur_row = sess.vault.get_sealed_key(&sess.username)?
            .ok_or_else(|| keepsake_core::Error::NotFound(sess.username.clone()))?;
        let vault_key = keepsake_core::identity::unseal_vault_key(
            &sess.master,
            &keepsake_core::identity::SealedVaultKey {
                nonce: cur_row.seal_nonce,
                ciphertext: cur_row.seal_ciphertext,
            },
        )?;
        let sealed = seal_vault_key(&new_master, &vault_key)?;
        let envelope = EnvelopeKey::from_master_key(&new_master)?;
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
        sess.vault.put_sealed_key(&row)?;
        sess.vault.append_audit(
            AuditOp::AddUser,
            &sess.username,
            Some(&username),
            None,
        )?;
        Ok(())
    })?;

    println!("added user {username}");
    let _ = path;
    Ok(())
}

fn random_device_id() -> [u8; 16] {
    let mut d = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut d);
    d
}
