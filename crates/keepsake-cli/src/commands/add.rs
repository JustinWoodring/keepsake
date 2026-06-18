//! `add` — add a new record.
//!
//! The record-from-type construction lives in this module
//! because it's also used by `edit` to re-prompt with current
//! values as defaults.

use std::path::PathBuf;

use chrono::Utc;
use uuid::Uuid;

use keepsake_core::audit::AuditOp;
use keepsake_core::records::{Record, RecordHeader};
use keepsake_core::session::Session;
use keepsake_core::Result;

use super::with_unlocked_mut;

pub async fn run(
    path: &std::path::Path,
    type_arg: String,
    from_json: Option<PathBuf>,
    session: &mut Option<Session>,
) -> anyhow::Result<()> {
    let r#type = normalize_type(&type_arg);
    let record = match from_json {
        Some(p) => read_from_json(&r#type, &p)?,
        None    => prompt_for(&r#type)?,
    };
    record.validate().map_err(|e| anyhow::anyhow!("{e}"))?;

    with_unlocked_mut(session, |sess| -> Result<()> {
        let header = RecordHeader::new(&record, &sess.username);
        sess.vault.put_record(&header, &record)?;
        sess.vault.append_audit(
            AuditOp::Create,
            &sess.username,
            Some(&record.id().to_string()),
            Some(&r#type),
        )?;
        Ok(())
    })?;

    println!("created {} ({})", record.id(), r#type);
    let _ = path;
    Ok(())
}

fn read_from_json(r#type: &str, path: &std::path::Path) -> anyhow::Result<Record> {
    let bytes = std::fs::read(path)
        .map_err(|e| anyhow::anyhow!("read {}: {e}", path.display()))?;
    let v: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|e| anyhow::anyhow!("parse {}: {e}", path.display()))?;
    keepsake_core::records::Record::from_json_value(r#type, v)
        .map_err(|e| anyhow::anyhow!("{e}"))
}

fn normalize_type(t: &str) -> String {
    t.trim().to_lowercase().replace('-', "_")
}

/// Prompt for a brand-new record of `r#type`.
pub fn prompt_for(r#type: &str) -> anyhow::Result<Record> {
    let placeholder = Record::Note(keepsake_core::records::Note {
        id: Uuid::nil(),
        title: String::new(),
        body: String::new(),
        tags: vec![],
        created_at: Utc::now(),
        updated_at: Utc::now(),
    });
    prompt_for_with_defaults(r#type, &placeholder)
}

/// Re-prompt for every field of `r#type`, with the values in
/// `defaults` shown as dialoguer defaults.
pub fn prompt_for_with_defaults(r#type: &str, defaults: &Record) -> anyhow::Result<Record> {
    let now = Utc::now();
    let id = defaults.id();

    let r = match (r#type, defaults) {
        ("login", Record::Login(d)) => Record::Login(keepsake_core::records::Login {
            id,
            service:      edit_field("service",      &d.service),
            username:     edit_field("username",     &d.username),
            holders:      edit_list("holders (comma-separated)", &d.holders),
            password:     edit_opt("password",       d.password.as_deref()),
            totp_secret:  edit_opt("totp_secret",    d.totp_secret.as_deref()),
            recovery_codes: edit_opt("recovery_codes", d.recovery_codes.as_deref()),
            url:          edit_opt("url",            d.url.as_deref()),
            notes:        edit_field("notes",        &d.notes),
            created_at:   d.created_at,
            updated_at:   now,
        }),
        ("document", Record::Document(d)) => Record::Document(keepsake_core::records::Document {
            id,
            title:         edit_field("title",         &d.title),
            document_type: edit_field("document_type", &d.document_type),
            owner:         edit_opt("owner",          d.owner.as_deref()),
            number:        edit_opt("number",         d.number.as_deref()),
            issuer:        edit_opt("issuer",         d.issuer.as_deref()),
            issued_on:     d.issued_on,
            expires_on:    d.expires_on,
            location:      edit_opt("location",       d.location.as_deref()),
            attachment_id: d.attachment_id,
            notes:         edit_field("notes",         &d.notes),
            created_at:    d.created_at,
            updated_at:    now,
        }),
        ("identification", Record::Identification(d)) => Record::Identification(keepsake_core::records::Identification {
            id,
            holder:      edit_field("holder",   &d.holder),
            id_type:     edit_field("id_type",  &d.id_type),
            issuer:      edit_opt("issuer",     d.issuer.as_deref()),
            number:      edit_field("number",   &d.number),
            country:     edit_opt("country",    d.country.as_deref()),
            class:       edit_opt("class",      d.class.as_deref()),
            issued_on:   d.issued_on,
            expires_on:  d.expires_on,
            notes:       edit_field("notes",    &d.notes),
            created_at:  d.created_at,
            updated_at:  now,
        }),
        ("insurance", Record::Insurance(d)) => Record::Insurance(keepsake_core::records::InsurancePolicy {
            id,
            policy_type:   edit_field("policy_type",   &d.policy_type),
            provider:      edit_field("provider",      &d.provider),
            policy_number: edit_field("policy_number", &d.policy_number),
            group_number:  edit_opt("group_number",    d.group_number.as_deref()),
            member_id:     edit_opt("member_id",       d.member_id.as_deref()),
            holders:       edit_list("holders (comma-separated)", &d.holders),
            beneficiary:   edit_opt("beneficiary",     d.beneficiary.as_deref()),
            insured_item:  edit_opt("insured_item",    d.insured_item.as_deref()),
            coverage:      edit_opt("coverage",        d.coverage.as_deref()),
            deductible:    edit_opt("deductible",      d.deductible.as_deref()),
            premium:       edit_opt("premium",         d.premium.as_deref()),
            effective_on:  d.effective_on,
            renewal_on:    d.renewal_on,
            agent:         edit_opt("agent",           d.agent.as_deref()),
            claims_phone:  edit_opt("claims_phone",    d.claims_phone.as_deref()),
            notes:         edit_field("notes",         &d.notes),
            created_at:    d.created_at,
            updated_at:    now,
        }),
        ("health", Record::Health(d)) => Record::Health(keepsake_core::records::HealthRecord {
            id,
            subject:    edit_field("subject", &d.subject),
            title:      edit_field("title",   &d.title),
            details:    d.details.clone(),
            created_at: d.created_at,
            updated_at: now,
        }),
        ("bank_account", Record::BankAccount(d)) => Record::BankAccount(keepsake_core::records::BankAccount {
            id,
            bank:            edit_field("bank",            &d.bank),
            account_type:    edit_field("account_type",    &d.account_type),
            account_number:  edit_opt("account_number",    d.account_number.as_deref()),
            routing_number:  edit_opt("routing_number",    d.routing_number.as_deref()),
            holders:         edit_list("holders (comma-separated)", &d.holders),
            swift:           edit_opt("swift",             d.swift.as_deref()),
            branch:          edit_opt("branch",            d.branch.as_deref()),
            online_username: edit_opt("online_username",   d.online_username.as_deref()),
            online_url:      edit_opt("online_url",        d.online_url.as_deref()),
            notes:           edit_field("notes",           &d.notes),
            created_at:      d.created_at,
            updated_at:      now,
        }),
        ("credit_card", Record::CreditCard(d)) => Record::CreditCard(keepsake_core::records::CreditCard {
            id,
            issuer:          edit_field("issuer",            &d.issuer),
            network:         edit_field("network",           &d.network),
            card_number:     edit_opt("card_number",          d.card_number.as_deref()),
            expiration:      edit_opt("expiration (MM/YY)",   d.expiration.as_deref()),
            cvv:             edit_opt("cvv",                  d.cvv.as_deref()),
            holders:         edit_list("cardholders (comma-separated; first is primary)", &d.holders),
            billing_address: edit_opt("billing_address",      d.billing_address.as_deref()),
            issuer_phone:    edit_opt("issuer_phone",         d.issuer_phone.as_deref()),
            issuer_url:      edit_opt("issuer_url",           d.issuer_url.as_deref()),
            pin:             edit_opt("pin",                  d.pin.as_deref()),
            notes:           edit_field("notes",              &d.notes),
            created_at:      d.created_at,
            updated_at:      now,
        }),
        ("investment", Record::Investment(d)) => Record::Investment(keepsake_core::records::Investment {
            id,
            provider:    edit_field("provider",    &d.provider),
            account_type: edit_field("account_type", &d.account_type),
            holders:     edit_list("holders (comma-separated)", &d.holders),
            notes:       edit_field("notes",       &d.notes),
            created_at:  d.created_at,
            updated_at:  now,
        }),
        ("income_source", Record::IncomeSource(d)) => Record::IncomeSource(keepsake_core::records::IncomeSource {
            id,
            source:      edit_field("source",      &d.source),
            income_type: edit_field("income_type", &d.income_type),
            rate:        edit_opt("rate",          d.rate.as_deref()),
            schedule:    edit_opt("schedule",      d.schedule.as_deref()),
            per_payment: edit_opt("per_payment",   d.per_payment.as_deref()),
            notes:       edit_field("notes",       &d.notes),
            created_at:  d.created_at,
            updated_at:  now,
        }),
        ("vehicle", Record::Vehicle(d)) => Record::Vehicle(keepsake_core::records::Vehicle {
            id,
            year: edit_field("year", &d.year.to_string()).parse().map_err(|_| anyhow::anyhow!("year: not a number"))?,
            make_model:         edit_field("make_model",           &d.make_model),
            vin:                edit_opt("vin",                    d.vin.as_deref()),
            license_plate:      edit_opt("license_plate",          d.license_plate.as_deref()),
            nickname:           edit_opt("nickname",               d.nickname.as_deref()),
            drivers:            edit_list("drivers (comma-separated; first is primary)", &d.drivers),
            title_holder:       edit_opt("title_holder",           d.title_holder.as_deref()),
            notes:              edit_field("notes",                &d.notes),
            registration_expires: d.registration_expires,
            created_at:         d.created_at,
            updated_at:         now,
        }),
        ("residence", Record::Residence(d)) => Record::Residence(keepsake_core::records::Residence {
            id,
            address:        edit_field("address",         &d.address),
            residence_type: edit_opt("residence_type",    d.residence_type.as_deref()),
            landlord:       edit_opt("landlord",          d.landlord.as_deref()),
            leaseholders:   edit_list("leaseholders (comma-separated)", &d.leaseholders),
            occupants:      edit_list("occupants (comma-separated)",    &d.occupants),
            lease_start:    d.lease_start,
            lease_end:      d.lease_end,
            rent:           edit_opt("rent",              d.rent.as_deref()),
            deposit:        edit_opt("deposit",           d.deposit.as_deref()),
            notes:          edit_field("notes",           &d.notes),
            created_at:     d.created_at,
            updated_at:     now,
        }),
        ("phone", Record::Phone(d)) => Record::Phone(keepsake_core::records::Phone {
            id,
            device:         edit_field("device",         &d.device),
            model:          edit_field("model",          &d.model),
            phone_number:   edit_field("phone_number",   &d.phone_number),
            carrier:        edit_field("carrier",        &d.carrier),
            plan:           edit_opt("plan",             d.plan.as_deref()),
            users:          edit_list("users (comma-separated; first is primary)", &d.users),
            account_number: edit_opt("account_number",   d.account_number.as_deref()),
            pin:            edit_opt("pin",              d.pin.as_deref()),
            notes:          edit_field("notes",          &d.notes),
            created_at:     d.created_at,
            updated_at:     now,
        }),
        ("address", Record::Address(d)) => Record::Address(keepsake_core::records::Address {
            id,
            label:       edit_field("label",        &d.label),
            street:      edit_field("street",       &d.street),
            city:        edit_opt("city",            d.city.as_deref()),
            region:      edit_opt("region",          d.region.as_deref()),
            postal_code: edit_opt("postal_code",     d.postal_code.as_deref()),
            country:     edit_opt("country",         d.country.as_deref()),
            notes:       edit_field("notes",         &d.notes),
            created_at:  d.created_at,
            updated_at:  now,
        }),
        ("contact", Record::Contact(d)) => Record::Contact(keepsake_core::records::Contact {
            id,
            name:         edit_field("name",         &d.name),
            relationship: edit_opt("relationship",   d.relationship.as_deref()),
            email:        edit_opt("email",          d.email.as_deref()),
            phone:        edit_opt("phone",          d.phone.as_deref()),
            notes:        edit_field("notes",        &d.notes),
            created_at:   d.created_at,
            updated_at:   now,
        }),
        ("subscription", Record::Subscription(d)) => Record::Subscription(keepsake_core::records::Subscription {
            id,
            service:      edit_field("service",      &d.service),
            cost:         edit_field("cost",         &d.cost),
            cycle:        edit_field("cycle",        &d.cycle),
            next_billing: d.next_billing,
            holders:      edit_list("holders (comma-separated)", &d.holders),
            username:     edit_opt("username",        d.username.as_deref()),
            notes:        edit_field("notes",        &d.notes),
            created_at:   d.created_at,
            updated_at:   now,
        }),
        ("infrastructure", Record::Infrastructure(d)) => Record::Infrastructure(keepsake_core::records::InfrastructureAsset {
            id,
            name:       edit_field("name",       &d.name),
            provider:   edit_field("provider",   &d.provider),
            asset_type: edit_field("asset_type", &d.asset_type),
            holders:    edit_list("holders (comma-separated)", &d.holders),
            notes:      edit_field("notes",      &d.notes),
            created_at: d.created_at,
            updated_at: now,
        }),
        ("domain", Record::Domain(d)) => Record::Domain(keepsake_core::records::DomainRecord {
            id,
            fqdn:      edit_field("fqdn",                  &d.fqdn),
            points_to: edit_opt("points_to",               d.points_to.as_deref()),
            holders:   edit_list("registrant contacts (comma-separated)", &d.holders),
            notes:     edit_field("notes",                 &d.notes),
            created_at: d.created_at,
            updated_at: now,
        }),
        ("runbook", Record::Runbook(d)) => Record::Runbook(keepsake_core::records::ScenarioRunbook {
            id,
            title:       edit_field("title",       &d.title),
            description: edit_field("description", &d.description),
            steps: d.steps.iter().enumerate().map(|(i, s)| {
                keepsake_core::records::RunbookStep {
                    order: i as u32,
                    title: edit_field(&format!("step_{}_title", i + 1), &s.title),
                    body:  edit_field(&format!("step_{}_body",  i + 1), &s.body),
                    status: s.status.clone(),
                }
            }).collect(),
            notes:      edit_field("notes",      &d.notes),
            created_at: d.created_at,
            updated_at: now,
        }),
        ("work_log", Record::WorkLog(d)) => Record::WorkLog(keepsake_core::records::WorkLogEntry {
            id,
            date: d.date,
            project:  edit_field("project",  &d.project),
            summary:  edit_field("summary",  &d.summary),
            details:  edit_field("details",  &d.details),
            tags:     d.tags.clone(),
            created_at: d.created_at,
            updated_at: now,
        }),
        ("note", Record::Note(d)) => Record::Note(keepsake_core::records::Note {
            id,
            title:  edit_field("title",  &d.title),
            body:   edit_field("body",   &d.body),
            tags:   d.tags.clone(),
            created_at: d.created_at,
            updated_at: now,
        }),
        _ => return Err(anyhow::anyhow!("unknown record type: {}", r#type)),
    };
    Ok(r)
}

fn edit_field(field: &str, current: &str) -> String {
    dialoguer::Input::<String>::new()
        .with_prompt(field)
        .default(current.to_string())
        .interact_text()
        .unwrap_or_else(|_| current.to_string())
}

fn edit_opt(field: &str, current: Option<&str>) -> Option<String> {
    let s = dialoguer::Input::<String>::new()
        .with_prompt(field)
        .default(current.unwrap_or_default().to_string())
        .allow_empty(true)
        .interact_text()
        .unwrap_or_default();
    if s.is_empty() { None } else { Some(s) }
}

fn edit_list(field: &str, current: &[String]) -> Vec<String> {
    let joined = current.join(", ");
    let s = dialoguer::Input::<String>::new()
        .with_prompt(field)
        .default(joined)
        .allow_empty(true)
        .interact_text()
        .unwrap_or_default();
    s.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect()
}
