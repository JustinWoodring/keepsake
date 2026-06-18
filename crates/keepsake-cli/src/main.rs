//! `keepsake` — command-line interface to the encrypted vault.
//!
//! See `README.md` for usage and `docs/threat-model.md` for the
//! security model.

use std::process::ExitCode;

use clap::Parser;
use keepsake_core::session::Session;

mod cli;
mod commands;
mod dev_server;

use cli::{default_path, Cli};

fn main() -> ExitCode {
    let cli = Cli::parse();
    let vault_path = cli.vault.clone().unwrap_or_else(default_path);

    if cli.dev_server {
        return match dev_server::run(&vault_path) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("dev server: {e}");
                ExitCode::FAILURE
            }
        };
    }

    let rt = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("tokio: {e}");
            return ExitCode::FAILURE;
        }
    };

    let result = rt.block_on(async {
        // Session is held on the main thread.  Commands borrow
        // it via this owned value.  `Vault` (and therefore
        // `Session`) is not `Sync`, so we run everything on the
        // current thread.
        let mut session: Option<Session> = None;
        cli::dispatch(&vault_path, cli.command, &mut session).await
    });
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e:?}");
            ExitCode::FAILURE
        }
    }
}
