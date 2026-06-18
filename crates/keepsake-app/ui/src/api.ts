// Typed wrapper around Tauri commands.  Every function in this
// file is a 1:1 mapping to a `#[tauri::command]` in
// `crates/keepsake-app/src-tauri/src/lib.rs`.

import { invoke } from "@tauri-apps/api/core";

export type RecordType =
  | "login"
  | "document"
  | "identification"
  | "insurance"
  | "health"
  | "bank_account"
  | "credit_card"
  | "investment"
  | "income_source"
  | "vehicle"
  | "residence"
  | "phone"
  | "address"
  | "contact"
  | "subscription"
  | "infrastructure"
  | "domain"
  | "runbook"
  | "work_log"
  | "note";

export type Category =
  | "identity"   // logins, identifications, contacts
  | "money"      // bank_account, credit_card, investment, income_source, subscription, insurance
  | "property"   // vehicle, residence, phone, address
  | "documents"  // document, infrastructure, domain
  | "notes"      // runbook, work_log, note, health
  | "";

export interface RecordTypeMeta {
  type: RecordType;
  label: string;
  /** Short tagline shown under the label on cards. */
  blurb: string;
  /** Lucide-ish icon glyph */
  icon: string;
  category: Exclude<Category, "">;
  /** Group label in the sidebar */
  group: "Identity & Access" | "Money" | "Property" | "Documents & Records" | "Notes & Logs";
}

export const RECORD_TYPES: RecordTypeMeta[] = [
  { type: "login",           label: "Logins",        blurb: "Services, sites & apps",         icon: "🔑", category: "identity",   group: "Identity & Access" },
  { type: "identification",  label: "IDs",            blurb: "Government & institutional IDs", icon: "🆔", category: "identity",   group: "Identity & Access" },
  { type: "contact",         label: "Contacts",       blurb: "People",                         icon: "👤", category: "identity",   group: "Identity & Access" },

  { type: "bank_account",    label: "Bank accounts",  blurb: "Checking, savings, etc.",        icon: "🏦", category: "money",      group: "Money" },
  { type: "credit_card",     label: "Credit cards",   blurb: "Issuer / network / last 4",      icon: "💳", category: "money",      group: "Money" },
  { type: "investment",      label: "Investments",    blurb: "Brokerage, 401k, IRA",           icon: "📈", category: "money",      group: "Money" },
  { type: "income_source",   label: "Income",         blurb: "Jobs, contracts, gigs",           icon: "💼", category: "money",      group: "Money" },
  { type: "subscription",    label: "Subscriptions",  blurb: "Recurring services",             icon: "🔁", category: "money",      group: "Money" },
  { type: "insurance",       label: "Insurance",      blurb: "Policies & claims",              icon: "🛡",  category: "money",      group: "Money" },

  { type: "vehicle",         label: "Vehicles",       blurb: "Cars, plates, VINs",             icon: "🚗", category: "property",   group: "Property" },
  { type: "residence",       label: "Residences",     blurb: "Rentals, leases, addresses",      icon: "🏠", category: "property",   group: "Property" },
  { type: "phone",           label: "Phones",         blurb: "Devices & lines",                icon: "📱", category: "property",   group: "Property" },
  { type: "address",         label: "Addresses",      blurb: "Anywhere important",             icon: "📍", category: "property",   group: "Property" },

  { type: "document",        label: "Documents",      blurb: "Passports, leases, etc.",        icon: "📄", category: "documents",  group: "Documents & Records" },
  { type: "infrastructure",  label: "Infrastructure", blurb: "Servers, services, decisions",   icon: "🖥", category: "documents",  group: "Documents & Records" },
  { type: "domain",          label: "Domains",        blurb: "DNS records",                     icon: "🌐", category: "documents",  group: "Documents & Records" },

  { type: "runbook",         label: "Runbooks",       blurb: "Step-by-step procedures",        icon: "📋", category: "notes",      group: "Notes & Logs" },
  { type: "work_log",        label: "Work logs",      blurb: "Dated activity",                 icon: "🗒", category: "notes",      group: "Notes & Logs" },
  { type: "note",            label: "Notes",          blurb: "Free-form markdown",             icon: "✏",  category: "notes",      group: "Notes & Logs" },
  { type: "health",          label: "Health",         blurb: "Providers, allergies, history",  icon: "❤",  category: "notes",      group: "Notes & Logs" },
];

export const META_BY_TYPE: Record<RecordType, RecordTypeMeta> = Object.fromEntries(
  RECORD_TYPES.map((t) => [t.type, t]),
) as Record<RecordType, RecordTypeMeta>;

export interface StatusResponse {
  unlocked: boolean;
  username: string | null;
}

export interface ListEntry {
  id: string;
  type: string;
  updated_by: string;
  updated_at: string;
}

export interface SearchHit {
  id: string;
  type: string;
  snippet: string;
}

export interface AuditEntryView {
  seq: number;
  op: string;
  actor: string;
  target_id: string | null;
  details: string | null;
  ts: string;
}

export interface RecordTitle {
  id: string;
  type: string;
  title: string;
}

export const api = {
  defaultPath:   (): Promise<string> => invoke("default_path"),
  status:        (): Promise<StatusResponse> => invoke("status"),
  listUsers:     (path?: string): Promise<string[]> => invoke("list_users", { path: path ?? null }),
  init:          (username: string, password: string, path?: string): Promise<void> =>
    invoke("init", { username, password, path: path ?? null }),
  unlock:        (username: string, password: string, path?: string): Promise<void> =>
    invoke("unlock", { username, password, path: path ?? null }),
  lock:          (): Promise<void> => invoke("lock"),

  addUser:         (username: string, password: string): Promise<void> =>
    invoke("add_user", { username, password }),
  removeUser:      (username: string): Promise<void> => invoke("remove_user", { username }),
  changePassword:  (newPassword: string): Promise<void> => invoke("change_password", { newPassword }),

  listRecords:   (type: string): Promise<ListEntry[]> => invoke("list_records", { type }),
  showRecord:    (id: string, reveal = false): Promise<unknown> => invoke("show_record", { id, reveal }),
  addRecord:     (type: string, fields: Record<string, unknown>): Promise<string> =>
    invoke("add_record", { type, fields }),
  updateRecord:  (id: string, fields: Record<string, unknown>): Promise<void> =>
    invoke("update_record", { id, fields }),
  deleteRecord:  (id: string): Promise<void> => invoke("delete_record", { id }),
  find:          (query: string): Promise<SearchHit[]> => invoke("find", { query }),

  audit:         (verify: boolean): Promise<AuditEntryView[] | { ok: boolean; entries: number }> =>
    invoke("audit", { verify }),
  configureSync: (baseUrl: string): Promise<void> => invoke("configure_sync", { baseUrl }),
  recordTitles: (): Promise<RecordTitle[]> => invoke("record_titles"),
  rewriteAuditChain: (): Promise<number> => invoke("rewrite_audit_chain"),
  syncPush: (serverUrl: string, vaultId: string): Promise<number> =>
    invoke("sync_push", { serverUrl, vaultId }),
  syncPull: (serverUrl: string, vaultId: string): Promise<number> =>
    invoke("sync_pull", { serverUrl, vaultId }),

  exportBundle:  (passphrase: string): Promise<number[]> =>
    invoke("export_bundle", { passphrase }),
  importBundle:  (bytes: number[], passphrase: string): Promise<void> =>
    invoke("import_bundle", { bytes, passphrase }),
  importToNewVault: (params: {
    bytes: number[];
    passphrase: string;
    username: string;
    password: string;
    path?: string;
  }): Promise<void> =>
    invoke("import_to_new_vault", params),
};
