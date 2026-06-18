// Field schemas for each record type.  These describe the full
// list of fields a record of that type can hold, with display
// labels and input kinds.  The form uses them to render inputs;
// the detail page uses them to render all fields in a
// definition-list.

import type { RecordType } from "./api";

export interface FieldDef {
  name: string;
  label: string;
  required?: boolean;
  multiline?: boolean;
  kind?: "text" | "date" | "number" | "email" | "tel" | "url" | "password";
}

export const SCHEMAS: Record<RecordType, FieldDef[]> = {
  login: [
    { name: "service", label: "Service", required: true },
    { name: "username", label: "Username", required: true },
    { name: "holders", label: "Holders" },
    { name: "password", label: "Password", kind: "password" },
    { name: "totp_secret", label: "TOTP secret", kind: "password" },
    { name: "recovery_codes", label: "Recovery codes", multiline: true },
    { name: "url", label: "URL", kind: "url" },
    { name: "notes", label: "Notes", multiline: true },
  ],
  document: [
    { name: "title", label: "Title", required: true },
    { name: "document_type", label: "Type", required: true },
    { name: "owner", label: "Owner" },
    { name: "number", label: "Document #" },
    { name: "issuer", label: "Issuer" },
    { name: "issued_on", label: "Issued", kind: "date" },
    { name: "expires_on", label: "Expires", kind: "date" },
    { name: "location", label: "Location" },
    { name: "notes", label: "Notes", multiline: true },
  ],
  identification: [
    { name: "holder", label: "Holder", required: true },
    { name: "id_type", label: "ID type", required: true },
    { name: "issuer", label: "Issuer" },
    { name: "number", label: "Number", kind: "password", required: true },
    { name: "country", label: "Country" },
    { name: "class", label: "Class" },
    { name: "issued_on", label: "Issued", kind: "date" },
    { name: "expires_on", label: "Expires", kind: "date" },
    { name: "notes", label: "Notes", multiline: true },
  ],
  insurance: [
    { name: "policy_type", label: "Type", required: true },
    { name: "provider", label: "Provider", required: true },
    { name: "policy_number", label: "Policy #", required: true },
    { name: "group_number", label: "Group #" },
    { name: "member_id", label: "Member ID" },
    { name: "holders", label: "Insured" },
    { name: "beneficiary", label: "Beneficiary" },
    { name: "insured_item", label: "Insured item" },
    { name: "coverage", label: "Coverage" },
    { name: "deductible", label: "Deductible" },
    { name: "premium", label: "Premium" },
    { name: "effective_on", label: "Effective", kind: "date" },
    { name: "renewal_on", label: "Renewal", kind: "date" },
    { name: "agent", label: "Agent" },
    { name: "claims_phone", label: "Claims phone" },
    { name: "notes", label: "Notes", multiline: true },
  ],
  health: [
    { name: "subject", label: "Subject", required: true },
    { name: "title", label: "Title", required: true },
    { name: "details", label: "Details", multiline: true },
  ],
  bank_account: [
    { name: "bank", label: "Bank", required: true },
    { name: "account_type", label: "Type", required: true },
    { name: "holders", label: "Holders" },
    { name: "account_number", label: "Account #", kind: "password" },
    { name: "routing_number", label: "Routing #", kind: "password" },
    { name: "swift", label: "SWIFT / BIC" },
    { name: "branch", label: "Branch" },
    { name: "online_username", label: "Online username" },
    { name: "online_url", label: "Online URL", kind: "url" },
    { name: "notes", label: "Notes", multiline: true },
  ],
  credit_card: [
    { name: "issuer", label: "Issuer", required: true },
    { name: "network", label: "Network", required: true },
    { name: "holders", label: "Cardholders" },
    { name: "card_number", label: "Card #", kind: "password" },
    { name: "expiration", label: "Expiration" },
    { name: "cvv", label: "CVV", kind: "password" },
    { name: "pin", label: "Card PIN", kind: "password" },
    { name: "billing_address", label: "Billing address", multiline: true },
    { name: "issuer_phone", label: "Issuer phone" },
    { name: "issuer_url", label: "Issuer URL", kind: "url" },
    { name: "notes", label: "Notes", multiline: true },
  ],
  investment: [
    { name: "provider", label: "Provider", required: true },
    { name: "account_type", label: "Type", required: true },
    { name: "holders", label: "Holders" },
    { name: "notes", label: "Notes", multiline: true },
  ],
  income_source: [
    { name: "source", label: "Source", required: true },
    { name: "income_type", label: "Type", required: true },
    { name: "rate", label: "Rate" },
    { name: "schedule", label: "Schedule" },
    { name: "per_payment", label: "Per payment" },
    { name: "notes", label: "Notes", multiline: true },
  ],
  vehicle: [
    { name: "year", label: "Year", kind: "number", required: true },
    { name: "make_model", label: "Make / model", required: true },
    { name: "nickname", label: "Nickname" },
    { name: "drivers", label: "Drivers" },
    { name: "title_holder", label: "Title holder" },
    { name: "vin", label: "VIN" },
    { name: "license_plate", label: "Plate" },
    { name: "notes", label: "Notes", multiline: true },
  ],
  residence: [
    { name: "address", label: "Address", required: true, multiline: true },
    { name: "residence_type", label: "Type" },
    { name: "landlord", label: "Landlord" },
    { name: "leaseholders", label: "Leaseholders" },
    { name: "occupants", label: "Occupants" },
    { name: "rent", label: "Rent" },
    { name: "deposit", label: "Deposit" },
    { name: "notes", label: "Notes", multiline: true },
  ],
  phone: [
    { name: "device", label: "Device", required: true },
    { name: "model", label: "Model", required: true },
    { name: "phone_number", label: "Phone #", required: true, kind: "tel" },
    { name: "carrier", label: "Carrier", required: true },
    { name: "plan", label: "Plan" },
    { name: "users", label: "Users" },
    { name: "account_number", label: "Account #" },
    { name: "pin", label: "PIN", kind: "password" },
    { name: "notes", label: "Notes", multiline: true },
  ],
  address: [
    { name: "label", label: "Label", required: true },
    { name: "street", label: "Street", required: true, multiline: true },
    { name: "city", label: "City" },
    { name: "region", label: "State / region" },
    { name: "postal_code", label: "Postal code" },
    { name: "country", label: "Country" },
    { name: "notes", label: "Notes", multiline: true },
  ],
  contact: [
    { name: "name", label: "Name", required: true },
    { name: "relationship", label: "Relationship" },
    { name: "email", label: "Email", kind: "email" },
    { name: "phone", label: "Phone", kind: "tel" },
    { name: "notes", label: "Notes", multiline: true },
  ],
  subscription: [
    { name: "service", label: "Service", required: true },
    { name: "cost", label: "Cost", required: true },
    { name: "cycle", label: "Cycle", required: true },
    { name: "holders", label: "Holders" },
    { name: "username", label: "Username" },
    { name: "notes", label: "Notes", multiline: true },
  ],
  infrastructure: [
    { name: "name", label: "Name", required: true },
    { name: "provider", label: "Provider", required: true },
    { name: "asset_type", label: "Type", required: true },
    { name: "holders", label: "Holders" },
    { name: "notes", label: "Notes", multiline: true },
  ],
  domain: [
    { name: "fqdn", label: "FQDN", required: true },
    { name: "points_to", label: "Points to" },
    { name: "holders", label: "Contacts" },
    { name: "notes", label: "Notes", multiline: true },
  ],
  runbook: [
    { name: "title", label: "Title", required: true },
    { name: "description", label: "Description", required: true, multiline: true },
    { name: "steps", label: "Steps (title | body | status, one per line)", multiline: true },
    { name: "notes", label: "Notes", multiline: true },
  ],
  work_log: [
    { name: "date", label: "Date", kind: "date", required: true },
    { name: "project", label: "Project", required: true },
    { name: "summary", label: "Summary", required: true },
    { name: "details", label: "Details", multiline: true },
    { name: "tags", label: "Tags" },
  ],
  note: [
    { name: "title", label: "Title", required: true },
    { name: "body", label: "Body (markdown)", multiline: true },
    { name: "tags", label: "Tags" },
  ],
};

/** Fields whose value the backend marks as sensitive (e.g.
 * passwords, card numbers).  Mirrors the `sensitive` list in
 * `crates/keepsake-app/src-tauri/src/session.rs` and the
 * `kind: "password"` schema entries.  The detail page masks
 * these by default and reveals them when the operator toggles
 * "Reveal sensitive". */
export const SENSITIVE_FIELDS = new Set<string>([
  "password",
  "totp_secret",
  "recovery_codes",
  "number",
  "account_number",
  "routing_number",
  "online_username",
  "card_number",
  "expiration",
  "cvv",
  "pin",
  "issuer_phone",
  "claims_phone",
]);

/** Whether a field should be hidden when the user has not
 * toggled "Reveal sensitive".  Returns true for fields that
 * the backend masks (covered by `SENSITIVE_FIELDS`) or fields
 * whose `kind: "password"` in the schema. */
export function isSensitive(fieldName: string, schema?: FieldDef): boolean {
  if (SENSITIVE_FIELDS.has(fieldName)) return true;
  if (schema?.kind === "password") return true;
  return false;
}
