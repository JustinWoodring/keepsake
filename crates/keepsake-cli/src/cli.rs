//! CLI plumbing shared between `main.rs` and the `repl`
//! subcommand.

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use keepsake_core::config::default_vault_path;
use keepsake_core::session::Session;

/// Top-level CLI.
#[derive(Debug, Parser)]
#[command(
    name = "keepsake",
    about = "End-to-end-encrypted life organizer",
    version
)]
pub struct Cli {
    /// Path to the vault file.  Defaults to the per-OS standard location.
    #[arg(long, global = true, value_name = "PATH")]
    pub vault: Option<PathBuf>,

    /// Run the in-process dev server (Phase 2 feature; listens on 127.0.0.1:8484).
    #[arg(long, global = true)]
    pub dev_server: bool,

    /// Subcommand.
    #[command(subcommand)]
    pub command: Command,
}

/// Subcommands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Initialize a new vault on this device.
    Init {
        #[arg(long)]
        username: Option<String>,
    },
    /// Unlock the vault for this session.
    Unlock {
        #[arg(long)]
        username: Option<String>,
    },
    /// Lock the vault, zeroizing the in-memory key.
    Lock,
    /// Show the current user (if unlocked).
    Whoami,
    /// Print { unlocked, username } status.
    Status,
    /// Print the default vault path.
    DefaultPath,
    /// List usernames on this device.
    ListUsers,
    /// Add a new user to this device's vault.
    AddUser {
        #[arg(long)]
        username: Option<String>,
    },
    /// Remove a user from this device.
    RemoveUser {
        #[arg(long)]
        username: Option<String>,
    },
    /// Change the current user's password.
    ChangePassword,
    /// Add a record.
    Add {
        #[arg(long)]
        r#type: String,
        /// Read field values from a JSON file instead of
        /// prompting interactively.  Useful for scripting
        /// bulk imports.
        #[arg(long, value_name = "PATH")]
        from_json: Option<PathBuf>,
    },
    /// Update a record by id (re-prompt for all fields).
    Update { id: String },
    /// Edit a record by id (alias for update).
    Edit { id: String },
    /// Delete a record by id.
    Delete { id: String },
    /// List records of a given type.
    List { r#type: String },
    /// Show a record by id.
    Show {
        id: String,
        #[arg(long)]
        reveal: bool,
    },
    /// Free-text search.
    Find { query: String },
    /// Show the cross-record links attached to a record
    /// (forward + reverse).  `[[uuid]]` markers in any text
    /// field are detected.
    Links {
        id: String,
        /// Direction: "out", "in", or "both" (default).
        #[arg(long, default_value = "both")]
        direction: String,
    },
    /// Show a record with all `[[uuid]]` link markers rendered
    /// as the target record's title.
    Resolve {
        id: String,
        #[arg(long)]
        reveal: bool,
    },
    /// Sync with a self-hosted server.  Subcommand is one of
    /// `push` or `pull`.  `--vault` selects the vault id.
    Sync {
        #[arg(long)]
        server: Option<String>,
        #[arg(long)]
        vault: Option<String>,
        /// Subcommand: push | pull
        #[arg(default_value = "push")]
        sub: String,
    },
    /// Export the vault to a `.ksk` file.
    Export { out: PathBuf },
    /// Import a `.ksk` file into a new vault.
    Import { input: PathBuf },
    /// Read or verify the audit chain.
    Audit { #[arg(long)] verify: bool },
    /// Drop legacy audit entries and re-chain the rest.  Used
    /// to recover a vault whose genesis was written by an
    /// older version of the code.
    AuditReset { #[arg(long)] yes: bool },
    /// Open an interactive REPL that holds the vault open
    /// across multiple commands read from stdin.
    Repl {
        #[arg(long)]
        username: Option<String>,
    },
}

pub fn default_path() -> PathBuf {
    default_vault_path()
}

pub async fn dispatch(
    path: &std::path::Path,
    cmd: Command,
    session: &mut Option<Session>,
) -> anyhow::Result<()> {
    use crate::commands;
    match cmd {
        Command::Init { username }           => commands::init::run(path, username, session).await,
        Command::Unlock { username }         => commands::unlock::run(path, username, session).await,
        Command::Lock                        => commands::lock::run(session).await,
        Command::Whoami                      => commands::whoami::run(session).await,
        Command::Status                      => commands::status::run(session).await,
        Command::DefaultPath                 => commands::default_path::run(path).await,
        Command::ListUsers                   => commands::list_users::run(path).await,
        Command::AddUser { username }        => commands::add_user::run(path, username, session).await,
        Command::RemoveUser { username }     => commands::remove_user::run(path, username, session).await,
        Command::ChangePassword              => commands::change_password::run(path, session).await,
        Command::Add { r#type, from_json }   => commands::add::run(path, r#type, from_json, session).await,
        Command::Update { id }               => commands::edit::run(path, id, session).await,
        Command::Edit { id }                 => commands::edit::run(path, id, session).await,
        Command::Delete { id }               => commands::delete::run(path, id, session).await,
        Command::List { r#type }             => commands::list::run(path, r#type, session).await,
        Command::Show { id, reveal }         => commands::show::run(path, id, reveal, session).await,
        Command::Find { query }              => commands::find::run(path, query, session).await,
        Command::Links { id, direction }     => commands::links::run(path, id, direction, session).await,
        Command::Resolve { id, reveal }      => commands::resolve::run(path, id, reveal, session).await,
        Command::Sync { server, vault, sub } => commands::sync::run(path, server, vault, sub, session).await,
        Command::Export { out }              => commands::export::run(path, out, session).await,
        Command::Import { input }            => commands::import::run(path, input, session).await,
        Command::Audit { verify }            => commands::audit::run(path, verify, session).await,
        Command::AuditReset { yes }          => commands::audit_reset::run(path, yes, session).await,
        Command::Repl { username }           => commands::repl::run(path, username, session).await,
    }
}
