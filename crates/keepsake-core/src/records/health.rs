//! Health records (providers, allergies, conditions, etc.).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A health record.  The `details` field is free-form JSON to keep
/// flexibility for any of: provider, condition, allergy, medication,
/// immunization, etc.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthRecord {
    /// Stable record id.
    pub id: Uuid,
    /// Subject ("Justin", "Dahlia", "Shared").
    pub subject: String,
    /// Record title (e.g. "PCP", "Penicillin allergy").
    pub title: String,
    /// Free-form structured details.
    pub details: serde_json::Value,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}
