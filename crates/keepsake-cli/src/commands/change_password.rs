//! `change-password` — rotate the current user's password.

use keepsake_core::audit::AuditOp;
use keepsake_core::crypto::KdfParams;
use keepsake_core::identity::{password_to_master_key, seal_vault_key};
use keepsake_core::session::Session;
use keepsake_core::Result;

use super::with_unlocked_mut;

pub async fn run(path: &std::path::Path, session: &mut Option<Session>) -> anyhow::Result<()> {
    let new_pw = rpassword::prompt_password("new password: ")?;
    let confirm = rpassword::prompt_password("confirm:      ")?;
    if new_pw != confirm {
        return Err(anyhow::anyhow!("passwords do not match"));
    }
    let params = KdfParams::default();
    let (new_master, salt) = password_to_master_key(new_pw.as_bytes(), params)?;

    with_unlocked_mut(session, |sess| -> Result<()> {
        // Re-derive the vault key from the current master.
        let cur = sess.vault.get_sealed_key(&sess.username)?
            .ok_or_else(|| keepsake_core::Error::NotFound(sess.username.clone()))?;
        let vault_key = keepsake_core::identity::unseal_vault_key(
            &sess.master,
            &keepsake_core::identity::SealedVaultKey {
                nonce: cur.seal_nonce,
                ciphertext: cur.seal_ciphertext,
            },
        )?;
        let sealed = seal_vault_key(&new_master, &vault_key)?;
        let new_row = keepsake_core::vault::SealedKeyRow {
            username: sess.username.clone(),
            device_id: cur.device_id,
            kdf_salt: salt.0,
            kdf_params: params.encode(),
            seal_nonce: sealed.nonce,
            seal_ciphertext: sealed.ciphertext,
            envelope_pk: cur.envelope_pk,
            created_at: cur.created_at,
        };
        sess.vault.put_sealed_key(&new_row)?;
        sess.vault.append_audit(
            AuditOp::RotatePassword,
            &sess.username,
            None,
            None,
        )?;
        sess.master = new_master;
        Ok(())
    })?;

    println!("password rotated for this device");
    let _ = path;
    Ok(())
}
