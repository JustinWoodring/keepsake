//! Typed record model.  Each record has a stable `type` tag, a
//! `schema_version`, a UUID v4 `id`, a creator and updated-by
//! username, timestamps, and a `fields` blob whose concrete shape
//! depends on the type.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod account;
pub mod contact;
pub mod document;
pub mod finance;
pub mod health;
pub mod identification;
pub mod infrastructure;
pub mod insurance;
pub mod note;
pub mod property;
pub mod runbook;
pub mod subscription;
pub mod validate;
pub mod work_log;

pub use account::Login;
pub use contact::{Address, Contact};
pub use document::Document;
pub use finance::{BankAccount, CreditCard, IncomeSource, Investment};
pub use health::HealthRecord;
pub use identification::Identification;
pub use infrastructure::{DomainRecord, InfrastructureAsset};
pub use insurance::InsurancePolicy;
pub use note::Note;
pub use property::{Phone, Residence, Vehicle};
pub use runbook::{RunbookStep, ScenarioRunbook};
pub use subscription::Subscription;
pub use work_log::WorkLogEntry;

/// All record types the vault can store.  The variant name is the
/// `type` tag in the on-disk record row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Record {
    /// A login: service + username + (optional) password + TOTP.
    /// Combines the old "Account" and "Credential" into one
    /// type so the UI doesn't have two nearly-identical lists.
    Login(Login),
    /// A free-form document or stored note reference.
    Document(Document),
    /// A government or other identification record.
    Identification(Identification),
    /// An insurance policy.
    Insurance(InsurancePolicy),
    /// A health-related record (provider, condition, allergy, etc.).
    Health(HealthRecord),
    /// A bank account.
    BankAccount(BankAccount),
    /// A credit card.
    CreditCard(CreditCard),
    /// A retirement or brokerage account.
    Investment(Investment),
    /// A recurring income source.
    IncomeSource(IncomeSource),
    /// A vehicle.
    Vehicle(Vehicle),
    /// A residence.
    Residence(Residence),
    /// A phone line / device.
    Phone(Phone),
    /// A postal address.
    Address(Address),
    /// A contact (person).
    Contact(Contact),
    /// A subscription (recurring paid service).
    Subscription(Subscription),
    /// An infrastructure asset (VPS, DNS, service).
    Infrastructure(InfrastructureAsset),
    /// A DNS domain / subdomain record.
    Domain(DomainRecord),
    /// A scenario runbook (steps for "if X, then do Y").
    Runbook(ScenarioRunbook),
    /// A dated work-log entry.
    WorkLog(WorkLogEntry),
    /// A free-form markdown note.
    Note(Note),
}

/// Every record type the vault can store.  Used by the sync
/// engine to enumerate all records.
pub const ALL_TYPES: &[&str] = &[
    "login", "document", "identification",
    "insurance", "health", "bank_account", "credit_card",
    "investment", "income_source", "vehicle", "residence",
    "phone", "address", "contact", "subscription",
    "infrastructure", "domain", "runbook", "work_log", "note",
];

impl Record {
    /// Stable type tag for this record.  Used in AEAD AAD and in the
    /// on-disk `type` column.
    pub fn type_tag(&self) -> &'static str {
        match self {
            Record::Login(_) => "login",
            Record::Document(_) => "document",
            Record::Identification(_) => "identification",
            Record::Insurance(_) => "insurance",
            Record::Health(_) => "health",
            Record::BankAccount(_) => "bank_account",
            Record::CreditCard(_) => "credit_card",
            Record::Investment(_) => "investment",
            Record::IncomeSource(_) => "income_source",
            Record::Vehicle(_) => "vehicle",
            Record::Residence(_) => "residence",
            Record::Phone(_) => "phone",
            Record::Address(_) => "address",
            Record::Contact(_) => "contact",
            Record::Subscription(_) => "subscription",
            Record::Infrastructure(_) => "infrastructure",
            Record::Domain(_) => "domain",
            Record::Runbook(_) => "runbook",
            Record::WorkLog(_) => "work_log",
            Record::Note(_) => "note",
        }
    }

    /// Schema version of the inner type.  Bump when the inner struct
    /// changes in a breaking way.
    pub fn schema_version(&self) -> u32 {
        match self {
            Record::Login(_) => 1,
            Record::Document(_) => 1,
            Record::Identification(_) => 1,
            Record::Insurance(_) => 1,
            Record::Health(_) => 1,
            Record::BankAccount(_) => 1,
            Record::CreditCard(_) => 1,
            Record::Investment(_) => 1,
            Record::IncomeSource(_) => 1,
            Record::Vehicle(_) => 1,
            Record::Residence(_) => 1,
            Record::Phone(_) => 1,
            Record::Address(_) => 1,
            Record::Contact(_) => 1,
            Record::Subscription(_) => 1,
            Record::Infrastructure(_) => 1,
            Record::Domain(_) => 1,
            Record::Runbook(_) => 1,
            Record::WorkLog(_) => 1,
            Record::Note(_) => 1,
        }
    }

    /// UUID v4 of this record.
    pub fn id(&self) -> Uuid {
        match self {
            Record::Login(r) => r.id,
            Record::Document(r) => r.id,
            Record::Identification(r) => r.id,
            Record::Insurance(r) => r.id,
            Record::Health(r) => r.id,
            Record::BankAccount(r) => r.id,
            Record::CreditCard(r) => r.id,
            Record::Investment(r) => r.id,
            Record::IncomeSource(r) => r.id,
            Record::Vehicle(r) => r.id,
            Record::Residence(r) => r.id,
            Record::Phone(r) => r.id,
            Record::Address(r) => r.id,
            Record::Contact(r) => r.id,
            Record::Subscription(r) => r.id,
            Record::Infrastructure(r) => r.id,
            Record::Domain(r) => r.id,
            Record::Runbook(r) => r.id,
            Record::WorkLog(r) => r.id,
            Record::Note(r) => r.id,
        }
    }

    /// Build a record from a JSON object whose keys are the
    /// record's field names (e.g. `service`, `username`, `holders`).
    /// `id`, `created_at`, and `updated_at` are filled in if the
    /// object doesn't already include them.  The `type` field is
    /// ignored (the caller picks the variant).
    pub fn from_json_value(
        r#type: &str,
        mut fields: serde_json::Value,
    ) -> crate::Result<Self> {
        let obj = match &mut fields {
            serde_json::Value::Object(m) => m,
            _ => return Err(crate::Error::Record(
                "fields must be a JSON object".into(),
            )),
        };
        // Tag the variant via `type` so serde picks the right
        // struct (Login vs Document vs ...).
        obj.insert("type".to_string(), serde_json::Value::String(r#type.to_string()));
        if !obj.contains_key("id") {
            obj.insert("id".to_string(), serde_json::Value::String(uuid::Uuid::new_v4().to_string()));
        }
        let now = Utc::now().to_rfc3339();
        if !obj.contains_key("created_at") {
            obj.insert("created_at".to_string(), serde_json::Value::String(now.clone()));
        }
        if !obj.contains_key("updated_at") {
            obj.insert("updated_at".to_string(), serde_json::Value::String(now));
        }
        serde_json::from_value(fields)
            .map_err(|e| crate::Error::Record(format!("{e}")))
    }

    /// Validation entry point.  Each record variant has its own
    /// type-specific validator.
    pub fn validate(&self) -> crate::Result<()> {
        match self {
            Record::Login(r) => validate::validate_login(r),
            Record::Document(r) => validate::validate_document(r),
            Record::Identification(r) => validate::validate_identification(r),
            Record::Insurance(r) => validate::validate_insurance(r),
            Record::Health(r) => validate::validate_health(r),
            Record::BankAccount(r) => validate::validate_bank_account(r),
            Record::CreditCard(r) => validate::validate_credit_card(r),
            Record::Investment(r) => validate::validate_investment(r),
            Record::IncomeSource(r) => validate::validate_income_source(r),
            Record::Vehicle(r) => validate::validate_vehicle(r),
            Record::Residence(r) => validate::validate_residence(r),
            Record::Phone(r) => validate::validate_phone(r),
            Record::Address(r) => validate::validate_address(r),
            Record::Contact(r) => validate::validate_contact(r),
            Record::Subscription(r) => validate::validate_subscription(r),
            Record::Infrastructure(r) => validate::validate_infrastructure(r),
            Record::Domain(r) => validate::validate_domain(r),
            Record::Runbook(r) => validate::validate_runbook(r),
            Record::WorkLog(r) => validate::validate_work_log(r),
            Record::Note(r) => validate::validate_note(r),
        }
    }

    /// Free-text search across all string fields.  Used by `find`.
    pub fn matches_query(&self, q: &str) -> bool {
        let q = q.to_lowercase();
        let hay = self.search_blob().to_lowercase();
        hay.contains(&q)
    }

    /// Flatten the record into a single string for search.
    pub fn search_blob(&self) -> String {
        match self {
            Record::Login(r) => format!(
                "{} {} {} {:?}",
                r.service, r.username, r.holders.join(", "), r.notes,
            ),
            Record::Document(r) => format!(
                "{} {} {:?}",
                r.title, r.document_type, r.notes,
            ),
            Record::Identification(r) => format!(
                "{} {} {} {:?}",
                r.holder, r.id_type, r.issuer.as_deref().unwrap_or(""), r.notes,
            ),
            Record::Insurance(r) => format!(
                "{} {} {} {} {:?}",
                r.provider, r.policy_type, r.policy_number,
                r.holders.join(", "), r.notes,
            ),
            Record::Health(r) => format!("{} {:?}", r.title, r.details),
            Record::BankAccount(r) => format!(
                "{} {} {} {:?}",
                r.bank, r.account_type, r.holders.join(", "), r.notes,
            ),
            Record::CreditCard(r) => format!(
                "{} {} {} {:?}",
                r.issuer, r.network, r.holders.join(", "), r.notes,
            ),
            Record::Investment(r) => format!(
                "{} {} {} {:?}",
                r.provider, r.account_type, r.holders.join(", "), r.notes,
            ),
            Record::IncomeSource(r) => format!("{} {:?}", r.source, r.notes),
            Record::Vehicle(r) => format!(
                "{} {} {} {} {:?}",
                r.year, r.make_model,
                r.nickname.as_deref().unwrap_or(""),
                r.drivers.join(", "),
                r.notes,
            ),
            Record::Residence(r) => format!(
                "{} {} {} {:?}",
                r.address,
                r.leaseholders.join(", "),
                r.occupants.join(", "),
                r.notes,
            ),
            Record::Phone(r) => format!(
                "{} {} {} {:?}",
                r.carrier, r.phone_number, r.users.join(", "), r.notes,
            ),
            Record::Address(r) => format!("{:?}", r),
            Record::Contact(r) => format!(
                "{} {} {} {}",
                r.name,
                r.relationship.as_deref().unwrap_or(""),
                r.email.as_deref().unwrap_or(""),
                r.phone.as_deref().unwrap_or(""),
            ),
            Record::Subscription(r) => format!(
                "{} {} {} {:?}",
                r.service, r.cost, r.cycle, r.notes,
            ),
            Record::Infrastructure(r) => format!(
                "{} {} {} {:?}",
                r.name, r.provider, r.asset_type, r.notes,
            ),
            Record::Domain(r) => format!(
                "{} {} {:?}",
                r.fqdn,
                r.points_to.as_deref().unwrap_or(""),
                r.notes,
            ),
            Record::Runbook(r) => format!("{} {}", r.title, r.description),
            Record::WorkLog(r) => format!("{} {} {}", r.project, r.summary, r.details),
            Record::Note(r) => format!("{} {}", r.title, r.body),
        }
    }
}

/// A record header stored alongside the encrypted fields blob in
/// the `records` table.  The fields blob itself is the AEAD
/// ciphertext of the serialized record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecordHeader {
    /// Stable type tag (see [`Record::type_tag`]).
    pub r#type: String,
    /// Schema version of the inner type.
    pub schema_version: u32,
    /// UUID v4 of the record.
    pub id: Uuid,
    /// Username that created the record.
    pub created_by: String,
    /// Username that last updated the record.
    pub updated_by: String,
    /// Creation timestamp (UTC).
    pub created_at: DateTime<Utc>,
    /// Last-update timestamp (UTC).
    pub updated_at: DateTime<Utc>,
}

impl RecordHeader {
    /// Build a header from a freshly-created record.
    pub fn new(record: &Record, username: &str) -> Self {
        let now = Utc::now();
        Self {
            r#type: record.type_tag().to_string(),
            schema_version: record.schema_version(),
            id: record.id(),
            created_by: username.to_string(),
            updated_by: username.to_string(),
            created_at: now,
            updated_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_json_value_picks_variant() {
        let v = serde_json::json!({
            "service": "ExampleMail",
            "username": "alice@example.com",
            "holders": ["Alice"],
            "password": "p@ss",
            "notes": "test",
        });
        let r = Record::from_json_value("login", v).unwrap();
        match r {
            Record::Login(l) => {
                assert_eq!(l.service, "ExampleMail");
                assert_eq!(l.holders, vec!["Alice".to_string()]);
                assert_eq!(l.password.as_deref(), Some("p@ss"));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn from_json_value_assigns_id_and_timestamps() {
        let v = serde_json::json!({ "title": "hello", "body": "world" });
        let r = Record::from_json_value("note", v).unwrap();
        match r {
            Record::Note(n) => {
                assert_eq!(n.title, "hello");
                assert_eq!(n.body, "world");
                assert!(n.id != Uuid::nil());
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn from_json_value_rejects_non_object() {
        let v = serde_json::json!([1, 2, 3]);
        assert!(Record::from_json_value("note", v).is_err());
    }
}
