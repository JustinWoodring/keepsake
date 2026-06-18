//! Infrastructure and domain records.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A piece of infrastructure (VPS, DNS provider, service).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InfrastructureAsset {
    /// Stable record id.
    pub id: Uuid,
    /// Asset name.
    pub name: String,
    /// Provider ("Linode", "AWS", "Cloudflare", ...).
    pub provider: String,
    /// Asset type ("VPS", "Object storage", "Email relay", ...).
    pub asset_type: String,
    /// Account holders (people responsible for this asset).
    #[serde(default)]
    pub holders: Vec<String>,
    /// Free-form notes — keep architectural decisions here.
    #[serde(default)]
    pub notes: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// A DNS domain or subdomain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DomainRecord {
    /// Stable record id.
    pub id: Uuid,
    /// Fully-qualified domain name ("slatron.justinwoodring.com").
    pub fqdn: String,
    /// Where it points to (a free-form description).
    #[serde(default)]
    pub points_to: Option<String>,
    /// Account holders (registrant contacts).
    #[serde(default)]
    pub holders: Vec<String>,
    /// Notes.
    #[serde(default)]
    pub notes: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}
