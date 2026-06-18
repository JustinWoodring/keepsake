//! Work log entry.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A dated work-log entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkLogEntry {
    /// Stable record id.
    pub id: Uuid,
    /// Date of the work.
    pub date: NaiveDate,
    /// Project / area ("NEXO", "Slatron", ...).
    pub project: String,
    /// One-line summary.
    pub summary: String,
    /// Detailed notes.
    #[serde(default)]
    pub details: String,
    /// Free-form tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}
