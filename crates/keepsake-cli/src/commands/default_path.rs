//! `default-path` — print the default vault path for this OS.

use keepsake_core::config::default_vault_path;

pub async fn run(_path: &std::path::Path) -> anyhow::Result<()> {
    println!("{}", default_vault_path().display());
    Ok(())
}
