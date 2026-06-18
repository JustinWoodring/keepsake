//! `unlock` — unlock the vault as the given user.

use keepsake_core::crypto::KdfParams;
use keepsake_core::identity::{unseal_vault_key, SealedVaultKey};
use keepsake_core::session::Session;
use keepsake_core::vault::Vault;
use keepsake_core::Error;

pub async fn run(path: &std::path::Path, username: Option<String>, session: &mut Option<Session>) -> anyhow::Result<()> {
    if session.is_some() {
        return Err(anyhow::anyhow!("vault is already unlocked"));
    }
    if !path.exists() {
        return Err(anyhow::anyhow!(
            "vault not found at {}; run `keepsake init` first",
            path.display()
        ));
    }

    let vault = Vault::open_or_create(path)?;
    let users = vault.list_users()?;
    if users.is_empty() {
        return Err(anyhow::anyhow!(
            "vault has no users; run `keepsake init` to create one"
        ));
    }
    let username = match username {
        Some(u) => u,
        None => dialoguer::Select::new()
            .with_prompt("username")
            .items(&users)
            .default(0)
            .interact()?
            .pipe(|i| users[i].clone()),
    };
    if !users.contains(&username) {
        return Err(anyhow::anyhow!("no such user: {username}"));
    }
    let password = rpassword::prompt_password("password: ")?;
    let row = vault
        .get_sealed_key(&username)?
        .ok_or_else(|| Error::NotFound(username.clone()))?;
    let params = KdfParams::decode(&row.kdf_params)?;
    let master = keepsake_core::crypto::derive_master_key(
        password.as_bytes(),
        &row.kdf_salt,
        params,
    )?;
    let vault_key = unseal_vault_key(
        &master,
        &SealedVaultKey {
            nonce: row.seal_nonce,
            ciphertext: row.seal_ciphertext,
        },
    )?;

    let mut vault = vault;
    vault.unlock(&vault_key)?;
    vault.append_audit(
        keepsake_core::audit::AuditOp::Unlock,
        &username,
        None,
        None,
    )?;

    *session = Some(keepsake_core::session::Session::new(
        path.to_path_buf(),
        vault,
        master,
        username.clone(),
    )?);

    println!("unlocked as {username}");
    Ok(())
}

trait Pipe: Sized {
    fn pipe<R>(self, f: impl FnOnce(Self) -> R) -> R { f(self) }
}
impl<T> Pipe for T {}
