//! A login record: service + username + (optional) password +
//! optional TOTP secret.  Combines the old Account + Credential
//! into a single type so the UI doesn't have two near-identical
//! lists.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A login record.  Service is the website or app; username is
/// the login id; password and TOTP are optional because some
/// services are SSO-only.  `holders` lists the usernames on this
/// account (most logins have one, but a joint family streaming
/// account has several).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Login {
    /// Stable record id.
    pub id: Uuid,
    /// Service name (e.g. "GitHub", "Bank of America").
    pub service: String,
    /// Primary username / login id for the primary holder.
    pub username: String,
    /// Account holders (primary + any additional members on the
    /// account).  A single-element list is the common case.
    #[serde(default)]
    pub holders: Vec<String>,
    /// Password.  None for SSO-only services.
    #[serde(default)]
    pub password: Option<String>,
    /// TOTP secret (base32).  None if 2FA isn't set up.
    #[serde(default)]
    pub totp_secret: Option<String>,
    /// Recovery codes (free-form list).
    #[serde(default)]
    pub recovery_codes: Option<String>,
    /// URL of the login page.
    #[serde(default)]
    pub url: Option<String>,
    /// Free-form notes (security questions, etc.).
    #[serde(default)]
    pub notes: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}
