//! Address and Contact records.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A postal address (home, work, important location).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Address {
    /// Stable record id.
    pub id: Uuid,
    /// Address label ("Home", "Work", "Parent's house", ...).
    pub label: String,
    /// Full street address (single line or multi-line).
    pub street: String,
    /// City.
    #[serde(default)]
    pub city: Option<String>,
    /// State / region.
    #[serde(default)]
    pub region: Option<String>,
    /// Postal code.
    #[serde(default)]
    pub postal_code: Option<String>,
    /// Country.
    #[serde(default)]
    pub country: Option<String>,
    /// Free-form notes.
    #[serde(default)]
    pub notes: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// A person contact.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Contact {
    /// Stable record id.
    pub id: Uuid,
    /// Contact name.
    pub name: String,
    /// Relationship ("Wife", "Advisor", "Friend", ...).
    #[serde(default)]
    pub relationship: Option<String>,
    /// Email.
    #[serde(default)]
    pub email: Option<String>,
    /// Phone.
    #[serde(default)]
    pub phone: Option<String>,
    /// Free-form notes.
    #[serde(default)]
    pub notes: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}
