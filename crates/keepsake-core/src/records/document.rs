//! Document record — title, type, optional location.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A document record (passport, birth certificate, lease, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Document {
    /// Stable record id.
    pub id: Uuid,
    /// Document title.
    pub title: String,
    /// Document type (e.g. "Passport", "Lease").
    pub document_type: String,
    /// Whose document.
    #[serde(default)]
    pub owner: Option<String>,
    /// Document number.
    #[serde(default)]
    pub number: Option<String>,
    /// Issuer.
    #[serde(default)]
    pub issuer: Option<String>,
    /// Issue date.
    #[serde(default)]
    pub issued_on: Option<NaiveDate>,
    /// Expiry date.
    #[serde(default)]
    pub expires_on: Option<NaiveDate>,
    /// Free-form notes.
    #[serde(default)]
    pub notes: String,
    /// Physical location ("Home safe", "Bank box 47").
    #[serde(default)]
    pub location: Option<String>,
    /// Reference to an attachment blob id.
    #[serde(default)]
    pub attachment_id: Option<Uuid>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}
