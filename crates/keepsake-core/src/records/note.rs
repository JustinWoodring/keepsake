//! Free-form markdown note (CRDT-tracked body).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A free-form note.  The `body` is markdown.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Note {
    /// Stable record id.
    pub id: Uuid,
    /// Note title.
    pub title: String,
    /// Markdown body.
    pub body: String,
    /// Free-form tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}
