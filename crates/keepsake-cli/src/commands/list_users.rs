//! `list-users` — list usernames on this device.  Works whether
//! or not the vault is currently unlocked.

use keepsake_core::vault::Vault;

pub async fn run(path: &std::path::Path) -> anyhow::Result<()> {
    let users: Vec<String> = if path.exists() {
        let v = Vault::open_or_create(path)?;
        v.list_users()?
    } else {
        Vec::new()
    };
    if users.is_empty() {
        println!("(no users)");
    } else {
        for u in users {
            println!("{u}");
        }
    }
    Ok(())
}
