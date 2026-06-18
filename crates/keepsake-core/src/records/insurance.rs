//! Insurance policy records.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// An insurance policy (auto, renter's, health, life, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InsurancePolicy {
    /// Stable record id.
    pub id: Uuid,
    /// Policy type ("Auto", "Renter's", "Health", "Life", ...).
    pub policy_type: String,
    /// Provider / carrier.
    pub provider: String,
    /// Policy number.
    pub policy_number: String,
    /// Group number (if any).
    #[serde(default)]
    pub group_number: Option<String>,
    /// Member ID.
    #[serde(default)]
    pub member_id: Option<String>,
    /// People covered by the policy.  First entry is typically
    /// the primary insured; additional entries are co-insureds.
    #[serde(default)]
    pub holders: Vec<String>,
    /// Beneficiary (for life insurance).
    #[serde(default)]
    pub beneficiary: Option<String>,
    /// Coverage amounts as free-form string.
    #[serde(default)]
    pub coverage: Option<String>,
    /// Deductible.
    #[serde(default)]
    pub deductible: Option<String>,
    /// Premium amount as free-form string.
    #[serde(default)]
    pub premium: Option<String>,
    /// Renewal date.
    #[serde(default)]
    pub renewal_on: Option<NaiveDate>,
    /// Effective date.
    #[serde(default)]
    pub effective_on: Option<NaiveDate>,
    /// Insured item (the car, the property, the person).
    #[serde(default)]
    pub insured_item: Option<String>,
    /// Agent name and phone.
    #[serde(default)]
    pub agent: Option<String>,
    /// Claims phone.
    #[serde(default)]
    pub claims_phone: Option<String>,
    /// Free-form notes (claims history, deductibles, etc.).
    #[serde(default)]
    pub notes: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}
