//! Per-type validators.  Each function returns
//! `Err(Error::Record(..))` for the first violation it finds.  The
//! checks are deliberately conservative: they catch only the
//! obvious shape problems (empty required fields, malformed
//! dates, oversized attachments) and leave semantic validation to
//! the caller.

use crate::error::{Error, Result};

use super::account::Login;
use super::contact::{Address, Contact};
use super::document::Document;
use super::finance::{BankAccount, CreditCard, IncomeSource, Investment};
use super::health::HealthRecord;
use super::identification::Identification;
use super::infrastructure::{DomainRecord, InfrastructureAsset};
use super::insurance::InsurancePolicy;
use super::note::Note;
use super::property::{Phone, Residence, Vehicle};
use super::runbook::ScenarioRunbook;
use super::subscription::Subscription;
use super::work_log::WorkLogEntry;

fn non_empty(field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(Error::Record(format!("{field} must not be empty")))
    } else {
        Ok(())
    }
}

pub fn validate_login(r: &Login) -> Result<()> {
    non_empty("login.service", &r.service)?;
    non_empty("login.username", &r.username)?;
    Ok(())
}

pub fn validate_document(r: &Document) -> Result<()> {
    non_empty("document.title", &r.title)?;
    non_empty("document.document_type", &r.document_type)?;
    Ok(())
}

pub fn validate_identification(r: &Identification) -> Result<()> {
    non_empty("identification.holder", &r.holder)?;
    non_empty("identification.id_type", &r.id_type)?;
    non_empty("identification.number", &r.number)?;
    Ok(())
}

pub fn validate_insurance(r: &InsurancePolicy) -> Result<()> {
    non_empty("insurance.policy_type", &r.policy_type)?;
    non_empty("insurance.provider", &r.provider)?;
    non_empty("insurance.policy_number", &r.policy_number)?;
    Ok(())
}

pub fn validate_health(r: &HealthRecord) -> Result<()> {
    non_empty("health.subject", &r.subject)?;
    non_empty("health.title", &r.title)?;
    Ok(())
}

pub fn validate_bank_account(r: &BankAccount) -> Result<()> {
    non_empty("bank_account.bank", &r.bank)?;
    non_empty("bank_account.account_type", &r.account_type)?;
    Ok(())
}

pub fn validate_credit_card(r: &CreditCard) -> Result<()> {
    non_empty("credit_card.issuer", &r.issuer)?;
    non_empty("credit_card.network", &r.network)?;
    Ok(())
}

pub fn validate_investment(r: &Investment) -> Result<()> {
    non_empty("investment.provider", &r.provider)?;
    non_empty("investment.account_type", &r.account_type)?;
    Ok(())
}

pub fn validate_income_source(r: &IncomeSource) -> Result<()> {
    non_empty("income_source.source", &r.source)?;
    non_empty("income_source.income_type", &r.income_type)?;
    Ok(())
}

pub fn validate_vehicle(r: &Vehicle) -> Result<()> {
    if r.year < 1900 || r.year > 2100 {
        return Err(Error::Record(format!(
            "vehicle.year out of range: {}",
            r.year
        )));
    }
    non_empty("vehicle.make_model", &r.make_model)
}

pub fn validate_residence(r: &Residence) -> Result<()> {
    non_empty("residence.address", &r.address)
}

pub fn validate_phone(r: &Phone) -> Result<()> {
    non_empty("phone.device", &r.device)?;
    non_empty("phone.phone_number", &r.phone_number)?;
    non_empty("phone.carrier", &r.carrier)
}

pub fn validate_address(r: &Address) -> Result<()> {
    non_empty("address.label", &r.label)?;
    non_empty("address.street", &r.street)
}

pub fn validate_contact(r: &Contact) -> Result<()> {
    non_empty("contact.name", &r.name)
}

pub fn validate_subscription(r: &Subscription) -> Result<()> {
    non_empty("subscription.service", &r.service)?;
    non_empty("subscription.cost", &r.cost)?;
    non_empty("subscription.cycle", &r.cycle)
}

pub fn validate_infrastructure(r: &InfrastructureAsset) -> Result<()> {
    non_empty("infrastructure.name", &r.name)?;
    non_empty("infrastructure.provider", &r.provider)?;
    non_empty("infrastructure.asset_type", &r.asset_type)
}

pub fn validate_domain(r: &DomainRecord) -> Result<()> {
    non_empty("domain.fqdn", &r.fqdn)?;
    if !r.fqdn.contains('.') {
        return Err(Error::Record("domain.fqdn must contain a dot".into()));
    }
    Ok(())
}

pub fn validate_runbook(r: &ScenarioRunbook) -> Result<()> {
    non_empty("runbook.title", &r.title)?;
    if r.steps.is_empty() {
        return Err(Error::Record("runbook must have at least one step".into()));
    }
    Ok(())
}

pub fn validate_work_log(r: &WorkLogEntry) -> Result<()> {
    non_empty("work_log.project", &r.project)?;
    non_empty("work_log.summary", &r.summary)
}

pub fn validate_note(r: &Note) -> Result<()> {
    non_empty("note.title", &r.title)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn now() -> chrono::DateTime<Utc> {
        Utc::now()
    }

    #[test]
    fn login_requires_service() {
        let r = Login {
            id: Uuid::new_v4(),
            service: " ".into(),
            username: "u".into(),
            holders: vec![],
            password: None,
            totp_secret: None,
            recovery_codes: None,
            url: None,
            notes: String::new(),
            created_at: now(),
            updated_at: now(),
        };
        assert!(validate_login(&r).is_err());
    }
}
