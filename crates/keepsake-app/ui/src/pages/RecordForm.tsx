import { For, Show, createResource, createSignal, onMount } from "solid-js";
import { useNavigate, useParams } from "@solidjs/router";
import { api, RECORD_TYPES, RecordType } from "../api";
import { showToast } from "../state";
import { FieldDef, SCHEMAS } from "../schemas";

export { SCHEMAS };

/** Field names that hold a comma-separated list of strings. */
const LIST_FIELDS = new Set([
  "holders", "drivers", "users", "leaseholders", "occupants",
]);

/**
 * Per-(type, field) placeholder hints.  Sample values that show
 * the expected shape without being autofill suggestions.
 * Per-type because the same field name can mean different things
 * (e.g. `issuer` is a bank for a card and an agency for an ID).
 */
const PLACEHOLDERS: Record<string, Record<string, string>> = {
  login: {
    service: "Service name",
    username: "username or email",
    holders: "John Doe, Jane Doe",
    password: "Strong password",
    totp_secret: "Base32 secret",
    recovery_codes: "One code per line",
    url: "https://service.com/login",
    notes: "Recovery questions, etc.",
  },
  document: {
    title: "Document title",
    document_type: "Lease, Contract, etc.",
    owner: "Whose document",
    number: "Document / ID number",
    issuer: "Issuing authority or agency",
    issued_on: "2020-06-15",
    expires_on: "2030-06-15",
    location: "Home safe, Bank box 47, etc.",
    notes: "Free-form notes",
  },
  identification: {
    holder: "Full legal name",
    id_type: "Driver's License, Passport, etc.",
    issuer: "Issuing state or agency",
    number: "ID number",
    country: "USA, Canada, etc.",
    class: "Class D, Real ID, etc.",
    issued_on: "2020-06-15",
    expires_on: "2030-06-15",
    notes: "Free-form notes",
  },
  insurance: {
    policy_type: "Auto, Renter's, Health, etc.",
    provider: "Carrier name",
    policy_number: "Policy number",
    group_number: "If applicable",
    member_id: "If applicable",
    holders: "John Doe, Jane Doe",
    beneficiary: "If applicable",
    insured_item: "2018 Honda Civic, 123 Main St, etc.",
    coverage: "$300,000 / $100,000",
    deductible: "$500",
    premium: "$120/mo",
    effective_on: "2025-01-01",
    renewal_on: "2026-01-01",
    agent: "Agent name & phone",
    claims_phone: "1-800-555-0100",
    notes: "Claims history, deductibles, etc.",
  },
  health: {
    subject: "Whose record (self, child, etc.)",
    title: "Title",
    details: "JSON or free-form details",
  },
  bank_account: {
    bank: "Chase, Bank of America, etc.",
    account_type: "Checking, Savings, etc.",
    holders: "John Doe, Jane Doe",
    account_number: "1234567890",
    routing_number: "021000021",
    swift: "CHASUS33",
    branch: "Branch name or address",
    online_username: "Online banking username",
    online_url: "https://bank.com/login",
    notes: "Free-form notes",
  },
  credit_card: {
    issuer: "Chase, Capital One, etc.",
    network: "Visa, Mastercard, Amex, Discover",
    holders: "John Doe, Jane Doe",
    card_number: "4242 4242 4242 4242",
    expiration: "MM/YY",
    cvv: "123",
    pin: "0000",
    billing_address: "123 Main St, City, ST 12345",
    issuer_phone: "1-800-555-0100",
    issuer_url: "https://issuer.com",
    notes: "Free-form notes",
  },
  investment: {
    provider: "Fidelity, Vanguard, etc.",
    account_type: "Brokerage, 401k, IRA, etc.",
    holders: "John Doe, Jane Doe",
    notes: "Free-form notes",
  },
  income_source: {
    source: "Employer, client, etc.",
    income_type: "Salaried, Hourly, Contract, etc.",
    rate: "$20/hr, $87k/yr, etc.",
    schedule: "Biweekly, Monthly, etc.",
    per_payment: "$2,700",
    notes: "Free-form notes",
  },
  vehicle: {
    year: "YYYY",
    make_model: "Year Make Model",
    nickname: "Friendly name",
    drivers: "John Doe, Jane Doe",
    title_holder: "Bank name, if financed",
    vin: "17-character VIN",
    license_plate: "ABC-1234",
    notes: "Free-form notes",
  },
  residence: {
    address: "Street, City, ST ZIP",
    residence_type: "Rental, Owned, Family, etc.",
    landlord: "Landlord or property manager",
    leaseholders: "John Doe, Jane Doe",
    occupants: "John Doe, Jane Doe",
    rent: "$1,500/mo",
    deposit: "$1,500",
    notes: "Free-form notes",
  },
  phone: {
    device: "Device or line name",
    model: "iPhone 15 Pro, etc.",
    phone_number: "555-555-5555",
    carrier: "Verizon, AT&T, etc.",
    plan: "Plan name",
    users: "John Doe, Jane Doe",
    account_number: "Account number",
    pin: "Account PIN",
    notes: "Free-form notes",
  },
  address: {
    label: "Home, Work, Parents, etc.",
    street: "Street address",
    city: "City",
    region: "State or region",
    postal_code: "ZIP / postal code",
    country: "Country",
    notes: "Free-form notes",
  },
  contact: {
    name: "Full name",
    relationship: "Wife, Advisor, Friend, etc.",
    email: "name@example.com",
    phone: "555-555-5555",
    notes: "Free-form notes",
  },
  subscription: {
    service: "Service name",
    cost: "$9.99",
    cycle: "Monthly, Yearly, etc.",
    holders: "John Doe, Jane Doe",
    username: "Account username or email",
    notes: "Free-form notes",
  },
  infrastructure: {
    name: "Asset name",
    provider: "Provider name",
    asset_type: "VPS, DNS, Object storage, etc.",
    holders: "John Doe, Jane Doe",
    notes: "Free-form notes",
  },
  domain: {
    fqdn: "subdomain.example.com",
    points_to: "Where it resolves to",
    holders: "John Doe, Jane Doe",
    notes: "Free-form notes",
  },
  runbook: {
    title: "Runbook title",
    description: "What scenario triggers this runbook",
    steps: "Step title | Step body | status (one per line)",
    notes: "Free-form notes",
  },
  work_log: {
    date: "YYYY-MM-DD",
    project: "Project name",
    summary: "One-line summary",
    details: "Detailed notes",
    tags: "comma, separated, tags",
  },
  note: {
    title: "Note title",
    body: "Markdown supported",
    tags: "comma, separated, tags",
  },
};


export function RecordForm() {
  const params = useParams<{ type?: string; id?: string }>();
  const nav = useNavigate();
  const isEdit = () => !!params.id;

  // When editing, the URL is /r/:id/edit and doesn't carry the
  // record type — we have to fetch the record to learn it.
  // When creating, the URL is /c/:type/new and the type is in
  // the path.
  const [editType, setEditType] = createSignal<RecordType | null>(null);
  const recordType = (): RecordType => {
    if (isEdit()) {
      const t = editType();
      if (t) return t;
    }
    return (params.type as RecordType) ?? "note";
  };
  const schema = (): FieldDef[] => SCHEMAS[recordType()] ?? [];

  const [values, setValues] = createSignal<Record<string, string>>({});
  const [busy, setBusy] = createSignal(false);

  onMount(async () => {
    if (!isEdit()) return;
    const rec = (await api.showRecord(params.id!, true)) as Record<string, unknown>;
    // The serialized Record enum has a `type` discriminator
    // injected by serde; use it to pick the right schema.
    const t = rec.type as RecordType | undefined;
    if (t) setEditType(t);
    const v: Record<string, string> = {};
    for (const f of schema()) {
      const val = rec[f.name];
      if (val == null) continue;
      if (f.name === "steps" && Array.isArray(val)) {
        v[f.name] = (val as { title: string; body: string; status?: string }[])
          .map((s) => `${s.title} | ${s.body} | ${s.status ?? ""}`)
          .join("\n");
      } else if (f.name === "details" && typeof val === "object") {
        v[f.name] = JSON.stringify(val, null, 2);
      } else if (Array.isArray(val)) {
        v[f.name] = val.join(", ");
      } else {
        v[f.name] = String(val);
      }
    }
    setValues(v);
  });

  function set(name: string, val: string) {
    setValues({ ...values(), [name]: val });
  }

  async function save(e: Event) {
    e.preventDefault();
    setBusy(true);
    try {
      const fields: Record<string, unknown> = {};
      const raw = values();
      for (const f of schema()) {
        const v = (raw[f.name] ?? "").trim();
        if (!v && !f.required) continue;
        if (f.kind === "number") {
          fields[f.name] = parseInt(v, 10);
        } else if (f.name === "steps") {
          fields[f.name] = v.split("\n").map((line) => {
            const [title, body, status] = line.split("|").map((s) => s.trim());
            return {
              order: 0,
              title: title ?? "",
              body: body ?? "",
              status: status || null,
            };
          });
        } else if (f.name === "tags") {
          fields[f.name] = v.split(",").map((s) => s.trim()).filter(Boolean);
        } else if (LIST_FIELDS.has(f.name)) {
          // Comma-separated list field (holders, drivers, users,
          // leaseholders, occupants).
          fields[f.name] = v.split(",").map((s) => s.trim()).filter(Boolean);
        } else if (f.name === "details") {
          try { fields[f.name] = JSON.parse(v); }
          catch { fields[f.name] = v; }
        } else {
          fields[f.name] = v;
        }
      }
      if (isEdit()) {
        await api.updateRecord(params.id!, fields);
        showToast("ok", "Record updated");
        nav(`/r/${params.id}`);
      } else {
        const id = await api.addRecord(recordType(), fields);
        showToast("ok", "Record created");
        nav(`/r/${id}`);
      }
    } catch (e) {
      showToast("err", String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div class="page">
      <header class="page-header">
        <div>
          <h1 class="page-title">
            {isEdit() ? "Edit" : "New"}{" "}
            {RECORD_TYPES.find((t) => t.type === recordType())?.label ?? recordType()}
          </h1>
          <p class="page-sub">
            {isEdit() ? "Update this record's fields." : "Fill in the fields below."}
          </p>
        </div>
      </header>
      <form onSubmit={save} class="form">
        <div class="form-grid">
          <For each={schema()}>
            {(f) => {
              const isList = LIST_FIELDS.has(f.name);
              const perType = PLACEHOLDERS[recordType()] ?? {};
              const placeholder =
                perType[f.name] ??
                (f.kind === "password" ? "••••••" : "");
              return (
                <div class="form-field" style={f.multiline ? "grid-column: 1 / -1" : ""}>
                  <label>
                    {f.label}{f.required ? " *" : ""}
                    {isList && <span class="form-hint">comma-separated, first is primary</span>}
                  </label>
                  <Show
                    when={f.multiline}
                    fallback={
                      <input
                        type={f.kind ?? "text"}
                        value={values()[f.name] ?? ""}
                        placeholder={placeholder}
                        onInput={(e) => set(f.name, e.currentTarget.value)}
                        required={f.required}
                      />
                    }
                  >
                    <textarea
                      rows="4"
                      value={values()[f.name] ?? ""}
                      placeholder={placeholder}
                      onInput={(e) => set(f.name, e.currentTarget.value)}
                      required={f.required}
                    />
                  </Show>
                </div>
              );
            }}
          </For>
        </div>
        <div class="form-actions">
          <button type="submit" class="btn btn-primary" disabled={busy()}>
            {busy() ? "Saving…" : isEdit() ? "Save changes" : "Create record"}
          </button>
          <button
            type="button"
            class="btn btn-ghost"
            onClick={() => {
              if (isEdit()) {
                nav(`/r/${params.id}`);
              } else if (params.type) {
                nav(`/c/${params.type}`);
              } else {
                nav("/");
              }
            }}
          >
            Cancel
          </button>
        </div>
      </form>
    </div>
  );
}
