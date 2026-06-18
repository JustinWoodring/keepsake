//! Subscription record (recurring paid service).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A recurring paid subscription.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Subscription {
    /// Stable record id.
    pub id: Uuid,
    /// Service name ("Claude Pro", "Amazon Prime", ...).
    pub service: String,
    /// Cost as a free-form string ("$20.00").
    pub cost: String,
    /// Billing cycle ("Monthly", "Yearly").
    pub cycle: String,
    /// Next billing date.
    #[serde(default)]
    pub next_billing: Option<NaiveDate>,
    /// Account holders (usually one).
    #[serde(default)]
    pub holders: Vec<String>,
    /// Username / email on the account.
    #[serde(default)]
    pub username: Option<String>,
    /// Free-form notes.
    #[serde(default)]
    pub notes: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}
