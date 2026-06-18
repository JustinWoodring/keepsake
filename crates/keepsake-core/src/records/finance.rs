//! Financial records: bank accounts, credit cards, investments,
//! income sources.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A list of people who hold or use this account/record.  An
/// empty list means "no holders recorded" — a single-element
/// list means one owner.
pub type Holders = Vec<String>;

/// A bank account.  Full account number and routing number are
/// stored (encrypted at rest), so the user doesn't have to go
/// fetch them when paying a bill.  `holders` is a list of
/// account-holder names.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BankAccount {
    /// Stable record id.
    pub id: Uuid,
    /// Bank name.
    pub bank: String,
    /// Account type ("Checking", "Savings", "Hi-Yield Savings", ...).
    pub account_type: String,
    /// Full account number.  Kept encrypted at rest by the vault.
    #[serde(default)]
    pub account_number: Option<String>,
    /// Full ABA routing number.
    #[serde(default)]
    pub routing_number: Option<String>,
    /// Account holders (names on the account).  May be one or
    /// many (joint account).
    #[serde(default)]
    pub holders: Holders,
    /// SWIFT / BIC code for international wires.
    #[serde(default)]
    pub swift: Option<String>,
    /// Branch / bank address.
    #[serde(default)]
    pub branch: Option<String>,
    /// Online banking username.
    #[serde(default)]
    pub online_username: Option<String>,
    /// Online banking URL.
    #[serde(default)]
    pub online_url: Option<String>,
    /// Free-form notes.
    #[serde(default)]
    pub notes: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// A credit card.  Full PAN and CVV are stored (encrypted at
/// rest); the CVV is shown only when the user explicitly
/// reveals it.  `holders` lists the cardholder name(s) — the
/// first entry is the primary account holder, additional
/// entries are authorized users.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreditCard {
    /// Stable record id.
    pub id: Uuid,
    /// Card issuer ("Discover", "Barclays", ...).
    pub issuer: String,
    /// Network ("Visa", "Mastercard", "Amex", "Discover").
    pub network: String,
    /// Full primary account number (PAN).  Encrypted at rest.
    #[serde(default)]
    pub card_number: Option<String>,
    /// Expiration "MM/YY" (or full date as a free-form string).
    #[serde(default)]
    pub expiration: Option<String>,
    /// CVV / CVC / CVV2.  Encrypted at rest.
    #[serde(default)]
    pub cvv: Option<String>,
    /// Cardholder name(s).  First entry is the primary
    /// account holder; additional entries are authorized
    /// users on the account.
    #[serde(default)]
    pub holders: Holders,
    /// Billing address (free-form).
    #[serde(default)]
    pub billing_address: Option<String>,
    /// Issuer customer-service phone number.
    #[serde(default)]
    pub issuer_phone: Option<String>,
    /// Issuer customer-service URL.
    #[serde(default)]
    pub issuer_url: Option<String>,
    /// Card-level PIN if set (separate from online banking PIN).
    #[serde(default)]
    pub pin: Option<String>,
    /// Free-form notes.
    #[serde(default)]
    pub notes: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// An investment / retirement account.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Investment {
    /// Stable record id.
    pub id: Uuid,
    /// Provider ("Fidelity", "Robinhood", ...).
    pub provider: String,
    /// Account type ("Brokerage", "401k", "Roth IRA", ...).
    pub account_type: String,
    /// Account holder(s).
    #[serde(default)]
    pub holders: Holders,
    /// Free-form notes.
    #[serde(default)]
    pub notes: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// A recurring income source.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IncomeSource {
    /// Stable record id.
    pub id: Uuid,
    /// Source name / employer.
    pub source: String,
    /// Income type ("Salaried", "Hourly", "Contract", "Per course", ...).
    pub income_type: String,
    /// Rate or salary as a free-form string.
    #[serde(default)]
    pub rate: Option<String>,
    /// Pay schedule ("Biweekly", "Monthly", ...).
    #[serde(default)]
    pub schedule: Option<String>,
    /// Per-payment amount as a free-form string.
    #[serde(default)]
    pub per_payment: Option<String>,
    /// Free-form notes.
    #[serde(default)]
    pub notes: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}
