//! `whoami` — show the current user (if unlocked).

use keepsake_core::session::Session;

pub async fn run(session: &Option<Session>) -> anyhow::Result<()> {
    match session.as_ref() {
        Some(s) => println!("{}", s.username),
        None => println!("locked"),
    }
    Ok(())
}
