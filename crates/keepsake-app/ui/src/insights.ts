// Insight generation: scan the vault and surface
// things-the-user-should-look-at.

import { api, ListEntry } from "./api";

export type Severity = "warn" | "info" | "ok";

export interface Insight {
  id: string;
  severity: Severity;
  title: string;
  detail: string;
  /** Optional link target. */
  to?: string;
}

interface RecordPayload {
  type: string;
  [key: string]: unknown;
}

function toRecord(payload: unknown): RecordPayload {
  if (payload && typeof payload === "object" && !Array.isArray(payload)) {
    return payload as RecordPayload;
  }
  return { type: "" };
}

function daysUntil(iso: string | null | undefined): number | null {
  if (!iso) return null;
  const d = new Date(iso);
  if (isNaN(d.getTime())) return null;
  const ms = d.getTime() - Date.now();
  return Math.floor(ms / (24 * 60 * 60 * 1000));
}

function fmtDate(iso: string | null | undefined): string {
  if (!iso) return "?";
  const d = new Date(iso);
  if (isNaN(d.getTime())) return "?";
  return d.toLocaleDateString();
}

export async function generateInsights(): Promise<Insight[]> {
  const out: Insight[] = [];

  // Load all the record types we care about in parallel.
  const [
    identifications,
    documents,
    insurances,
    bankAccounts,
    creditCards,
    vehicles,
    logins,
    notes,
    health,
    runbooks,
    workLogs,
  ] = await Promise.all([
    list("identification"),
    list("document"),
    list("insurance"),
    list("bank_account"),
    list("credit_card"),
    list("vehicle"),
    list("login"),
    list("note"),
    list("health"),
    list("runbook"),
    list("work_log"),
  ]);

  // ----- Expiring soon (within 60 days) -----
  for (const e of identifications) {
    const r = await fetchRecord(e.id);
    if (!r) continue;
    const d = daysUntil(r.expires_on as string);
    if (d !== null && d <= 60) {
      out.push({
        id: `id-exp-${e.id}`,
        severity: d < 0 ? "warn" : d < 14 ? "warn" : "info",
        title: d < 0
          ? `${r.id_type} expired ${-d} day${-d === 1 ? "" : "s"} ago`
          : `${r.id_type} expires in ${d} day${d === 1 ? "" : "s"}`,
        detail: `${r.holder ?? "?"} — expires ${fmtDate(r.expires_on as string)}`,
        to: `/r/${e.id}`,
      });
    }
  }
  for (const e of documents) {
    const r = await fetchRecord(e.id);
    if (!r) continue;
    const d = daysUntil(r.expires_on as string);
    if (d !== null && d <= 60) {
      out.push({
        id: `doc-exp-${e.id}`,
        severity: d < 14 ? "warn" : "info",
        title: d < 0
          ? `${r.document_type} expired ${-d} day${-d === 1 ? "" : "s"} ago`
          : `${r.document_type} expires in ${d} day${d === 1 ? "" : "s"}`,
        detail: `${r.title} — expires ${fmtDate(r.expires_on as string)}`,
        to: `/r/${e.id}`,
      });
    }
  }
  for (const e of insurances) {
    const r = await fetchRecord(e.id);
    if (!r) continue;
    const d = daysUntil(r.renewal_on as string);
    if (d !== null && d <= 60) {
      out.push({
        id: `ins-ren-${e.id}`,
        severity: d < 14 ? "warn" : "info",
        title: d < 0
          ? `${r.policy_type} renewal overdue (${-d}d)`
          : `${r.policy_type} renews in ${d} day${d === 1 ? "" : "s"}`,
        detail: `${r.provider} — ${fmtDate(r.renewal_on as string)}`,
        to: `/r/${e.id}`,
      });
    }
  }
  for (const e of creditCards) {
    const r = await fetchRecord(e.id);
    if (!r) continue;
    const d = daysUntil(parseExpiration(r.expiration as string));
    if (d !== null && d <= 60) {
      out.push({
        id: `cc-exp-${e.id}`,
        severity: d < 14 ? "warn" : "info",
        title: d < 0
          ? `Card expired ${-d} day${-d === 1 ? "" : "s"} ago`
          : `Card expires in ${d} day${d === 1 ? "" : "s"}`,
        detail: `${r.issuer} • ${r.network} — ${r.expiration as string}`,
        to: `/r/${e.id}`,
      });
    }
  }

  // ----- Logins without TOTP (where it would matter) -----
  for (const e of logins) {
    const r = await fetchRecord(e.id);
    if (!r) continue;
    const hasTotp = !!(r.totp_secret && String(r.totp_secret).trim());
    if (!hasTotp) {
      out.push({
        id: `login-nototp-${e.id}`,
        severity: "info",
        title: `${r.service} has no 2FA configured`,
        detail: `${r.username} — add a TOTP secret to harden this account.`,
        to: `/r/${e.id}/edit`,
      });
    }
  }

  // ----- Vehicles missing VIN or plate -----
  for (const e of vehicles) {
    const r = await fetchRecord(e.id);
    if (!r) continue;
    const hasVin = !!(r.vin && String(r.vin).trim());
    const hasPlate = !!(r.license_plate && String(r.license_plate).trim());
    if (!hasVin || !hasPlate) {
      const missing = [!hasVin && "VIN", !hasPlate && "plate"].filter(Boolean).join(" & ");
      out.push({
        id: `veh-missing-${e.id}`,
        severity: "info",
        title: `${r.year} ${r.make_model} missing ${missing}`,
        detail: `Fill in the ${missing} for completeness.`,
        to: `/r/${e.id}/edit`,
      });
    }
  }

  // ----- Records with no holders set (joint = single) -----
  for (const e of [...insurances, ...bankAccounts, ...vehicles]) {
    const r = await fetchRecord(e.id);
    if (!r) continue;
    const holders = (r.holders || r.drivers) as string[] | undefined;
    if (holders && Array.isArray(holders) && holders.length === 0) {
      out.push({
        id: `no-holders-${e.id}`,
        severity: "info",
        title: `${r.type} has no holder assigned`,
        detail: `Add a holder (or "joint") so the audit log is meaningful.`,
        to: `/r/${e.id}/edit`,
      });
    }
  }

  // ----- Stale notes: not updated in 6+ months -----
  const sixMonths = 1000 * 60 * 60 * 24 * 30 * 6;
  for (const e of notes) {
    const updated = new Date(e.updated_at).getTime();
    if (Date.now() - updated > sixMonths) {
      out.push({
        id: `stale-note-${e.id}`,
        severity: "info",
        title: `Note not updated in a while`,
        detail: `Last updated ${fmtDate(e.updated_at)}. Review and refresh.`,
        to: `/r/${e.id}`,
      });
    }
  }

  // ----- Stale health records -----
  for (const e of health) {
    const updated = new Date(e.updated_at).getTime();
    if (Date.now() - updated > sixMonths) {
      out.push({
        id: `stale-health-${e.id}`,
        severity: "info",
        title: `Health record not updated in 6+ months`,
        detail: `Last updated ${fmtDate(e.updated_at)}. Verify still accurate.`,
        to: `/r/${e.id}`,
      });
    }
  }

  // ----- Work log gaps: no entries in 30 days -----
  const last30 = 1000 * 60 * 60 * 24 * 30;
  const recent = workLogs.filter((e) => Date.now() - new Date(e.updated_at).getTime() < last30);
  if (workLogs.length > 0 && recent.length === 0) {
    out.push({
      id: "wl-gap",
      severity: "info",
      title: "No work-log entries in the last 30 days",
      detail: `You have ${workLogs.length} historical entries but nothing recent. Add an entry to keep the timeline current.`,
      to: "/c/work_log/new",
    });
  }

  // ----- Runbook coverage: no runbooks at all -----
  if (runbooks.length === 0) {
    out.push({
      id: "rb-none",
      severity: "info",
      title: "No runbooks yet",
      detail: `Scenario runbooks (insurance claims, infra incidents, etc.) become invaluable during stress. Create your first one.`,
      to: "/c/runbook/new",
    });
  }

  // ----- Duplicate detection: same bank + same type -----
  const byBank = new Map<string, ListEntry[]>();
  for (const e of bankAccounts) {
    const r = await fetchRecord(e.id);
    if (!r) continue;
    const key = `${(r.bank as string)?.toLowerCase()}|${(r.account_type as string)?.toLowerCase()}`;
    const list = byBank.get(key) ?? [];
    list.push(e);
    byBank.set(key, list);
  }
  for (const [key, entries] of byBank) {
    if (entries.length > 1) {
      out.push({
        id: `dup-bank-${key}`,
        severity: "info",
        title: `Possible duplicate bank account`,
        detail: `${entries.length} records at the same bank with the same type. Check if they should be merged.`,
        to: `/c/bank_account`,
      });
    }
  }

  // ----- Welcome message for empty vaults -----
  const total = identifications.length + documents.length + insurances.length +
                creditCards.length + vehicles.length + logins.length + notes.length;
  if (total === 0) {
    out.push({
      id: "welcome",
      severity: "ok",
      title: "Welcome to Keepsake",
      detail: "Start by adding a login, a document, or a bank account. Use the sidebar to navigate categories.",
    });
  } else if (total < 5) {
    out.push({
      id: "early",
      severity: "info",
      title: "You're getting started",
      detail: `${total} record${total === 1 ? "" : "s"} so far. Consider setting up an export to back up your vault.`,
      to: "/settings#export",
    });
  }

  // Sort: warns first, then info, then ok
  const order: Record<Severity, number> = { warn: 0, info: 1, ok: 2 };
  out.sort((a, b) => order[a.severity] - order[b.severity]);
  return out;
}

async function list(type: string): Promise<ListEntry[]> {
  try {
    return await api.listRecords(type);
  } catch {
    return [];
  }
}

async function fetchRecord(id: string): Promise<RecordPayload | null> {
  try {
    const r = await api.showRecord(id, true);
    return toRecord(r);
  } catch {
    return null;
  }
}

/** "MM/YY" or "YYYY-MM-DD" or "MM/YYYY" → Date. */
function parseExpiration(s: string | null | undefined): string | null {
  if (!s) return null;
  // ISO date
  if (/^\d{4}-\d{2}-\d{2}/.test(s)) return s;
  // MM/YY
  const m = s.match(/^(\d{1,2})\/(\d{2,4})$/);
  if (m) {
    const month = parseInt(m[1], 10);
    let year = parseInt(m[2], 10);
    if (year < 100) year += 2000;
    // End of the expiry month
    const d = new Date(Date.UTC(year, month, 0));
    return d.toISOString();
  }
  return null;
}
