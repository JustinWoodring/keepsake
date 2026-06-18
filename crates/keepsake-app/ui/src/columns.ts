// Per-type column schemas for the list view.  Each record type
// gets a small set of meaningful columns instead of the
// generic "id / updated by / updated at" table that worked
// for nothing in particular.

import { RecordType } from "./api";

export type CellValue = string | number | null | undefined | string[];

export interface ColumnDef {
  label: string;
  flex: number | string;
  field?: string;
  /** Custom rendering for the cell. */
  format?: (record: Record<string, unknown>) => CellValue;
  align?: "left" | "right";
}

function joinList(record: Record<string, unknown>, field: string): string {
  const v = record[field];
  if (Array.isArray(v)) return v.join(", ");
  if (v == null) return "";
  return String(v);
}

function stripScheme(url: string | undefined): string {
  if (!url) return "";
  return url.replace(/^https?:\/\//, "").replace(/\/$/, "");
}

function preview(body: string | undefined): string {
  if (!body) return "";
  const oneLine = body.replace(/\s+/g, " ").trim();
  return oneLine.length > 80 ? oneLine.slice(0, 80) + "…" : oneLine;
}

export const COLUMNS_BY_TYPE: Record<RecordType, ColumnDef[]> = {
  login: [
    { label: "Service",   flex: 2, field: "service" },
    { label: "Username",  flex: 2, field: "username" },
    { label: "Holders",   flex: 2, format: (r) => joinList(r, "holders") },
    { label: "URL",       flex: 2, format: (r) => stripScheme(r.url as string | undefined) },
  ],
  document: [
    { label: "Title",     flex: 3, field: "title" },
    { label: "Type",      flex: 2, field: "document_type" },
    { label: "Owner",     flex: 1, field: "owner" },
    { label: "Expires",   flex: 1, field: "expires_on" },
  ],
  identification: [
    { label: "Holder",    flex: 2, field: "holder" },
    { label: "Type",      flex: 2, field: "id_type" },
    { label: "Issuer",    flex: 2, field: "issuer" },
    { label: "Expires",   flex: 1, field: "expires_on" },
  ],
  insurance: [
    { label: "Type",      flex: 2, field: "policy_type" },
    { label: "Provider",  flex: 2, field: "provider" },
    { label: "Insured",   flex: 2, format: (r) => joinList(r, "holders") },
    { label: "Renewal",   flex: 1, field: "renewal_on" },
  ],
  health: [
    { label: "Subject",   flex: 2, field: "subject" },
    { label: "Title",     flex: 3, field: "title" },
  ],
  bank_account: [
    { label: "Bank",      flex: 3, field: "bank" },
    { label: "Type",      flex: 2, field: "account_type" },
    { label: "Holders",   flex: 3, format: (r) => joinList(r, "holders") },
  ],
  credit_card: [
    { label: "Issuer",    flex: 2, field: "issuer" },
    { label: "Network",   flex: 1, field: "network" },
    { label: "Cardholders", flex: 3, format: (r) => joinList(r, "holders") },
    { label: "Expires",   flex: 1, field: "expiration" },
  ],
  investment: [
    { label: "Provider",  flex: 2, field: "provider" },
    { label: "Type",      flex: 2, field: "account_type" },
    { label: "Holders",   flex: 2, format: (r) => joinList(r, "holders") },
  ],
  income_source: [
    { label: "Source",    flex: 3, field: "source" },
    { label: "Type",      flex: 2, field: "income_type" },
    { label: "Schedule",  flex: 2, field: "schedule" },
    { label: "Rate",      flex: 1, field: "rate" },
  ],
  vehicle: [
    { label: "Year",      flex: 1, field: "year" },
    { label: "Make/Model",flex: 3, field: "make_model" },
    { label: "Drivers",   flex: 3, format: (r) => joinList(r, "drivers") },
    { label: "Plate",     flex: 1, field: "license_plate" },
  ],
  residence: [
    { label: "Address",   flex: 4, field: "address" },
    { label: "Leaseholders", flex: 2, format: (r) => joinList(r, "leaseholders") },
    { label: "Type",      flex: 1, field: "residence_type" },
    { label: "Rent",      flex: 1, field: "rent" },
  ],
  phone: [
    { label: "Device",    flex: 2, field: "device" },
    { label: "Number",    flex: 2, field: "phone_number" },
    { label: "Carrier",   flex: 2, field: "carrier" },
    { label: "Users",     flex: 2, format: (r) => joinList(r, "users") },
  ],
  address: [
    { label: "Label",     flex: 2, field: "label" },
    { label: "Street",    flex: 3, field: "street" },
    { label: "City",      flex: 2, field: "city" },
  ],
  contact: [
    { label: "Name",      flex: 3, field: "name" },
    { label: "Relationship", flex: 2, field: "relationship" },
    { label: "Email",     flex: 2, field: "email" },
    { label: "Phone",     flex: 2, field: "phone" },
  ],
  subscription: [
    { label: "Service",   flex: 3, field: "service" },
    { label: "Cost",      flex: 1, field: "cost" },
    { label: "Cycle",     flex: 1, field: "cycle" },
    { label: "Holders",   flex: 1, format: (r) => joinList(r, "holders") },
  ],
  infrastructure: [
    { label: "Name",      flex: 2, field: "name" },
    { label: "Provider",  flex: 2, field: "provider" },
    { label: "Type",      flex: 2, field: "asset_type" },
    { label: "Holders",   flex: 1, format: (r) => joinList(r, "holders") },
  ],
  domain: [
    { label: "FQDN",      flex: 3, field: "fqdn" },
    { label: "Points to", flex: 2, field: "points_to" },
    { label: "Holders",   flex: 1, format: (r) => joinList(r, "holders") },
  ],
  runbook: [
    { label: "Title",     flex: 3, field: "title" },
    { label: "Description", flex: 4, field: "description" },
  ],
  work_log: [
    { label: "Date",      flex: 1, field: "date" },
    { label: "Project",   flex: 2, field: "project" },
    { label: "Summary",   flex: 3, field: "summary" },
  ],
  note: [
    { label: "Title",     flex: 3, field: "title" },
    { label: "Preview",   flex: 4, format: (r) => preview(r.body as string) },
  ],
};

export function renderCell(def: ColumnDef, fields: Record<string, unknown>): string {
  let raw: CellValue;
  if (def.format) {
    raw = def.format(fields);
  } else if (def.field) {
    raw = fields[def.field] as CellValue;
  } else {
    raw = "";
  }
  if (raw == null) return "";
  if (Array.isArray(raw)) return raw.join(", ");
  if (typeof raw === "string") return raw;
  if (typeof raw === "number") return String(raw);
  return String(raw);
}
