//! Property records: vehicles, residences, phones.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A vehicle record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Vehicle {
    /// Stable record id.
    pub id: Uuid,
    /// Model year.
    pub year: u16,
    /// Make and model ("Hyundai Sonata").
    pub make_model: String,
    /// VIN.
    #[serde(default)]
    pub vin: Option<String>,
    /// License plate.
    #[serde(default)]
    pub license_plate: Option<String>,
    /// Friendly name ("Sonata").
    #[serde(default)]
    pub nickname: Option<String>,
    /// Authorized drivers (first entry is the primary driver /
    /// title holder).  Other entries are additional drivers.
    #[serde(default)]
    pub drivers: Vec<String>,
    /// Title holder (often a bank for a leased/financed vehicle).
    /// Distinct from `drivers` because the title holder may not
    /// be a person who drives the car.
    #[serde(default)]
    pub title_holder: Option<String>,
    /// Free-form notes.
    #[serde(default)]
    pub notes: String,
    /// Registration expiry.
    #[serde(default)]
    pub registration_expires: Option<NaiveDate>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// A residence (current or past).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Residence {
    /// Stable record id.
    pub id: Uuid,
    /// Full street address.
    pub address: String,
    /// Type ("Rental", "Owned", "Family", ...).
    #[serde(default)]
    pub residence_type: Option<String>,
    /// Landlord / property manager.
    #[serde(default)]
    pub landlord: Option<String>,
    /// Who is on the lease / owns the residence.  Separate from
    /// `occupants` because the leaseholder isn't always the
    /// person living there (and vice versa).
    #[serde(default)]
    pub leaseholders: Vec<String>,
    /// People who actually live in the residence.
    #[serde(default)]
    pub occupants: Vec<String>,
    /// Lease start date.
    #[serde(default)]
    pub lease_start: Option<NaiveDate>,
    /// Lease end date.
    #[serde(default)]
    pub lease_end: Option<NaiveDate>,
    /// Monthly rent as a free-form string.
    #[serde(default)]
    pub rent: Option<String>,
    /// Security deposit.
    #[serde(default)]
    pub deposit: Option<String>,
    /// Free-form notes.
    #[serde(default)]
    pub notes: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// A phone line / device.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Phone {
    /// Stable record id.
    pub id: Uuid,
    /// Friendly device name ("Outer Space" or "Dahlia's iPhone").
    pub device: String,
    /// Device model ("iPhone 15").
    pub model: String,
    /// Phone number ("985-634-5293" or E.164).
    pub phone_number: String,
    /// Carrier ("Verizon", ...).
    pub carrier: String,
    /// Plan name.
    #[serde(default)]
    pub plan: Option<String>,
    /// Users on the line.  First entry is the primary
    /// account holder; additional entries are authorized
    /// users on the same plan.
    #[serde(default)]
    pub users: Vec<String>,
    /// Account number with the carrier.
    #[serde(default)]
    pub account_number: Option<String>,
    /// PIN for the carrier account.
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
