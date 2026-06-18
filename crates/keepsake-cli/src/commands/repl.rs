//! `repl` — unlock the vault and then process further
//! subcommands read from stdin, one per line.  Useful for
//! scripting bulk imports without re-prompting the password
//! for every record.
//!
//! Format on each line: a `keepsake` subcommand, with arguments
//! exactly as you'd pass them on the command line.  The vault
//! path and global flags are inherited from this invocation
//! and cannot be overridden by input lines.
//!
//! The first line of stdin is treated as the user's password
//! (the script provides it; `rpassword`'s /dev/tty prompt is
//! bypassed so non-interactive scripts work).  Use a heredoc or
//! printf to feed it; for interactive use, the operator can
//! paste the password then press Return.
//!
//! Example:
//!   printf 'hunter2\nadd --type login --from-json /tmp/01.json\nexit\n' \
//!     | keepsake repl --username alice
//!
//! `exit` (or EOF) terminates the REPL.

use std::io::Write;

use clap::Parser;
use keepsake_core::session::Session;

use crate::cli::{dispatch, Cli};

pub async fn run(
    path: &std::path::Path,
    username: Option<String>,
    session: &mut Option<Session>,
) -> anyhow::Result<()> {
    // First line of stdin is the password (or an env-var
    // override).  After the password is consumed, the
    // remaining stdin is the command stream.
    let password = read_password(&username)?;
    unlock_with(path, &username, &password, session)?;

    // Now process commands from stdin.
    let stdin = std::io::stdin();
    let mut line = String::new();
    loop {
        line.clear();
        let n = stdin.read_line(&mut line)?;
        if n == 0 {
            break; // EOF
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let argv: Vec<&str> = trimmed.split_whitespace().collect();
        if argv.is_empty() {
            continue;
        }
        if matches!(argv[0], "exit" | "quit") {
            break;
        }
        // Synthesize a `keepsake` argv (clap expects argv[0] =
        // program name).
        let mut full = vec!["keepsake"];
        full.extend(argv.iter().copied());
        let parsed = match Cli::try_parse_from(&full) {
            Ok(cli) => cli,
            Err(e) => {
                eprintln!("{e}");
                continue;
            }
        };
        if parsed.dev_server {
            eprintln!("`--dev-server` is not valid inside a REPL session");
            continue;
        }
        // Ignore the vault path in the input line; the REPL's
        // own path is authoritative.
        let _ = parsed.vault;
        let fut = dispatch(path, parsed.command, session);
        if let Err(e) = Box::pin(fut).await {
            eprintln!("{e:?}");
        }
    }
    // Lock the vault on the way out so the session state is
    // zeroized from memory.
    if let Some(sess) = session.as_mut() {
        sess.vault.lock();
    }
    *session = None;
    Ok(())
}

fn read_password(_username: &Option<String>) -> anyhow::Result<String> {
    // The first line of stdin is the password.  We don't try to
    // be clever about /dev/tty: rpassword's tty probe fails in
    // some non-interactive environments (e.g. CI, our own
    // import script), so a plain stdin read is more portable.
    //
    // If the user wants to avoid piping the password at all,
    // they can `KEEPSAKE_PASSWORD=...` set the env var; we
    // *also* consume the matching first line from stdin in
    // that case so the rest of the input stream lines up.
    let mut s = String::new();
    std::io::stdin().read_line(&mut s)?;
    let p = s.trim_end_matches(['\n', '\r']).to_string();
    if let Ok(env) = std::env::var("KEEPSAKE_PASSWORD") {
        if !env.is_empty() {
            return Ok(env);
        }
    }
    // Echo a newline so the operator's terminal cursor moves
    // to a fresh line after the password.
    let _ = std::io::stdout().flush();
    Ok(p)
}

fn unlock_with(
    path: &std::path::Path,
    username: &Option<String>,
    password: &str,
    session: &mut Option<Session>,
) -> anyhow::Result<()> {
    use keepsake_core::crypto::KdfParams;
    use keepsake_core::identity::{unseal_vault_key, SealedVaultKey};
    use keepsake_core::vault::Vault;
    use keepsake_core::Error;

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
        Some(u) => users.iter().find(|x| x == &u).cloned()
            .ok_or_else(|| anyhow::anyhow!("no such user: {u}"))?,
        None => {
            return Err(anyhow::anyhow!(
                "no username provided; pass `--username` to the REPL"
            ));
        }
    };
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
    eprintln!("unlocked as {username}");
    Ok(())
}
