//! `import` — read a `.ksk` bundle and write its contents into a
//! new vault on this device.  Refuses to clobber an existing
//! vault file.  Prompts for a username + password to associate
//! with the import.

use std::io::Read;

use keepsake_core::crypto::KdfParams;
use keepsake_core::export::{decrypt_bundle, read_bundle};
use keepsake_core::identity::{password_to_master_key, seal_vault_key, EnvelopeKey, SealedVaultKey};
use keepsake_core::session::Session;
use keepsake_core::vault::SealedKeyRow;
use rand::RngCore;

pub async fn run(
    path: &std::path::Path,
    input: std::path::PathBuf,
    _session: &mut Option<Session>,
) -> anyhow::Result<()> {
    if path.exists() {
        return Err(anyhow::anyhow!(
            "vault already exists at {}; refusing to clobber",
            path.display()
        ));
    }

    // Read the bundle.
    let bytes = {
        let mut f = std::fs::File::open(&input)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        buf
    };
    let passphrase = rpassword::prompt_password("export passphrase: ")?;
    let bundle = read_bundle(&input)?;
    let bundle = if bundle.payload.is_empty() {
        keepsake_core::export::parse_bundle(&bytes)?
    } else {
        bundle
    };
    drop(bytes);

    let (_export_master, vault_key, payload) =
        decrypt_bundle(&bundle, passphrase.as_bytes())?;

    // Prompt for a username + password to own this vault.
    let username: String = dialoguer::Input::new()
        .with_prompt("username for the new vault")
        .interact_text()?;
    if username.trim().is_empty() {
        return Err(anyhow::anyhow!("username must not be empty"));
    }
    let password = rpassword::prompt_password("password: ")?;
    let confirm = rpassword::prompt_password("confirm: ")?;
    if password != confirm {
        return Err(anyhow::anyhow!("passwords do not match"));
    }

    // Build a fresh master key, seal the vault key under it.
    let params = KdfParams::default();
    let (master, salt) = password_to_master_key(password.as_bytes(), params)?;
    let _ = _export_master;
    let new_sealed: SealedVaultKey = seal_vault_key(&master, &vault_key)?;
    let envelope = EnvelopeKey::from_master_key(&master)?;

    // Create the vault file and write the sealed-keys row.
    let v = keepsake_core::vault::Vault::open_or_create(path)?;
    let row = SealedKeyRow {
        username: username.clone(),
        device_id: random_device_id(),
        kdf_salt: salt.0,
        kdf_params: params.encode(),
        seal_nonce: new_sealed.nonce,
        seal_ciphertext: new_sealed.ciphertext,
        envelope_pk: envelope.public_key().to_bytes(),
        created_at: chrono::Utc::now(),
    };
    v.put_sealed_key(&row)?;
    drop(v);

    // Reopen, unlock with the bundle's vault key, write the
    // records.
    let mut v = keepsake_core::vault::Vault::open_or_create(path)?;
    v.unlock(&vault_key)?;
    let n = payload.records.len();
    for entry in &payload.records {
        v.put_record(&entry.header, &entry.record)?;
    }
    for a in &payload.attachments {
        v.put_attachment(&a.clone().into_row())?;
    }
    v.append_audit(
        keepsake_core::audit::AuditOp::Import,
        &username,
        Some("(import)"),
        Some(&format!("{n} records")),
    )?;

    println!(
        "imported {n} record{} from {}",
        if n == 1 { "" } else { "s" },
        input.display()
    );
    Ok(())
}

fn random_device_id() -> [u8; 16] {
    let mut d = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut d);
    d
}
