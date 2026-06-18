//! Government / institutional identification.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// An identification record (driver's license, SSN card, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Identification {
    /// Stable record id.
    pub id: Uuid,
    /// Whose ID this is.
    pub holder: String,
    /// Type of ID ("Driver's License", "Passport", "SSN", etc.).
    pub id_type: String,
    /// Issuing authority / agency.
    #[serde(default)]
    pub issuer: Option<String>,
    /// The number itself (kept encrypted at the field level too).
    pub number: String,
    /// Issue date.
    #[serde(default)]
    pub issued_on: Option<NaiveDate>,
    /// Expiry date.
    #[serde(default)]
    pub expires_on: Option<NaiveDate>,
    /// Issuing country / state.
    #[serde(default)]
    pub country: Option<String>,
    /// Class / type detail ("Class D", "Real ID", etc.).
    #[serde(default)]
    pub class: Option<String>,
    /// Free-form notes.
    #[serde(default)]
    pub notes: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}
