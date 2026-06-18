import { For, Show, createResource, createSignal } from "solid-js";
import { A, useParams } from "@solidjs/router";
import { api, META_BY_TYPE, RecordType } from "../api";
import { showToast } from "../state";
import { FieldDef, SCHEMAS, isSensitive } from "../schemas";
import { Markdown } from "../components/Markdown";

interface RecordFields {
  type?: string;
  notes?: string;
  body?: string;
  steps?: Array<{ title: string; body: string; status?: string }>;
  details?: unknown;
  [key: string]: unknown;
}

type Step = { title: string; body: string; status?: string };

function asSteps(v: unknown): Step[] | null {
  if (!Array.isArray(v)) return null;
  return v.filter(
    (s): s is Step =>
      typeof s === "object" && s !== null &&
      typeof (s as Step).title === "string" &&
      typeof (s as Step).body === "string",
  );
}

/** Mask a value when the field is sensitive and the operator
 * has not toggled "Reveal sensitive".  Strings are shown as
 * `••••••`; arrays (e.g. `holders`) are shown as `•••`.
 * `null` and `""` always render as the muted em-dash. */
function maskIfSensitive(
  fieldName: string,
  schema: FieldDef | undefined,
  value: unknown,
  reveal: boolean,
): { display: string; hidden: boolean } {
  if (value == null || value === "") {
    return { display: "—", hidden: false };
  }
  if (!isSensitive(fieldName, schema) || reveal) {
    if (Array.isArray(value)) {
      return { display: value.join(", "), hidden: false };
    }
    if (typeof value === "object") {
      return {
        display: JSON.stringify(value, null, 2),
        hidden: false,
      };
    }
    return { display: String(value), hidden: false };
  }
  if (Array.isArray(value)) {
    return { display: "•••", hidden: true };
  }
  if (typeof value === "object") {
    return { display: "•••", hidden: true };
  }
  // Mask: keep at most 4 trailing characters so the user can
  // still verify the *last four* of a card or account.
  const s = String(value);
  if (s.length <= 4) {
    return { display: "•".repeat(s.length), hidden: true };
  }
  return { display: "•".repeat(s.length - 4) + s.slice(-4), hidden: true };
}

export function RecordDetail() {
  const params = useParams<{ id: string }>();
  const [reveal, setReveal] = createSignal(false);
  const [data, { refetch }] = createResource(
    () => ({ id: params.id, reveal: reveal() }),
    (k) => api.showRecord(k.id, k.reveal),
  );
  // Title table for resolving `[[uuid]]` link markers in
  // markdown fields.  Refreshed when the record changes.
  const [titles] = createResource(
    () => params.id,
    () => api.recordTitles().then((rows) => {
      const m: Record<string, string> = {};
      for (const r of rows) m[r.id] = r.title;
      return m;
    }),
  );

  async function del() {
    if (!confirm("Delete this record? This cannot be undone.")) return;
    try {
      await api.deleteRecord(params.id);
      showToast("ok", "Record deleted");
      history.back();
    } catch (e) {
      showToast("err", String(e));
    }
  }

  const recordType = (): RecordType | null => {
    const d = data() as RecordFields | undefined;
    return (d?.type as RecordType | undefined) ?? null;
  };

  /** Iterate the schema for this record type, returning each
   * field with its label, value, and whether it should be
   * rendered as a long/markdown block.  Skips fields with no
   * value. */
  const fieldRows = (): Array<{
    name: string;
    label: string;
    value: string;
    hidden: boolean;
    multiline: boolean;
  }> => {
    const d = data() as RecordFields | undefined;
    if (!d || !d.type) return [];
    const schema = SCHEMAS[d.type as RecordType] ?? [];
    const out: Array<{
      name: string;
      label: string;
      value: string;
      hidden: boolean;
      multiline: boolean;
    }> = [];
    for (const f of schema) {
      // Skip the implicit `type` discriminator and the auto-set
      // `id`/`created_at`/`updated_at` — those are shown in
      // the header already.
      if (
        f.name === "type" ||
        f.name === "id" ||
        f.name === "created_at" ||
        f.name === "updated_at" ||
        f.name === "created_by" ||
        f.name === "updated_by" ||
        f.name === "schema_version" ||
        f.name === "steps"
      ) {
        continue;
      }
      if (!(f.name in d)) continue;
      const raw = d[f.name];
      // Skip empty values entirely so the dl doesn't fill
      // with em-dashes for fields the operator never filled
      // in.
      if (raw == null || raw === "" ||
          (Array.isArray(raw) && raw.length === 0)) {
        continue;
      }
      const { display, hidden } = maskIfSensitive(
        f.name, f, raw, reveal(),
      );
      // For long blocks (notes, body, billing_address,
      // description, etc.) we want the value rendered as a
      // paragraph, not a one-liner.
      const multiline =
        f.multiline === true ||
        f.name === "body" ||
        f.name === "notes" ||
        f.name === "description" ||
        f.name === "billing_address" ||
        f.name === "address" ||
        f.name === "details";
      out.push({
        name: f.name,
        label: f.label,
        value: display,
        hidden,
        multiline,
      });
    }
    return out;
  };

  return (
    <div class="page">
      <header class="page-header">
        <div>
          <h1 class="page-title">
            <Show when={recordType() && META_BY_TYPE[recordType()!]}>
              <span class="title-emoji">{META_BY_TYPE[recordType()!].icon}</span>
              {META_BY_TYPE[recordType()!].label}
            </Show>
          </h1>
          <p class="page-sub"><code class="id-mono">{params.id}</code></p>
        </div>
        <div class="page-actions">
          <button
            class="btn"
            onClick={() => {
              setReveal(!reveal());
              refetch();
            }}
          >
            {reveal() ? "🙈 Hide sensitive" : "👁 Reveal sensitive"}
          </button>
          <A class="btn" href={`/r/${params.id}/edit`}>✎ Edit</A>
          <button class="btn btn-danger" onClick={del}>🗑 Delete</button>
        </div>
      </header>

      <Show when={data()} fallback={<p class="muted">Loading…</p>}>
        <Show
          when={fieldRows().length > 0}
          fallback={<p class="muted">This record has no fields yet.</p>}
        >
          <dl class="detail-fields">
            <For each={fieldRows()}>
              {(row) => (
                <>
                  <dt class="detail-label">{row.label}</dt>
                  <dd
                    class={
                      "detail-value" +
                      (row.multiline ? " detail-multiline" : "") +
                      (row.hidden ? " detail-masked" : "")
                    }
                  >
                    <Show
                      when={row.multiline}
                      fallback={<span>{row.value}</span>}
                    >
                      <Markdown source={row.value} titles={titles() ?? {}} />
                    </Show>
                  </dd>
                </>
              )}
            </For>
          </dl>
        </Show>

        <Show when={recordType() === "runbook"}>
          {(() => {
            const steps = asSteps((data() as RecordFields).steps);
            return (
              <Show when={steps && steps.length > 0}>
                <section class="runbook-section">
                  <h3>Steps</h3>
                  <ol class="runbook-steps">
                    <For each={steps ?? []}>
                      {(s) => (
                        <li>
                          <div class="step-title">{s.title}</div>
                          <Markdown source={s.body} titles={titles() ?? {}} />
                          <Show when={s.status}>
                            <div class="step-status">{s.status}</div>
                          </Show>
                        </li>
                      )}
                    </For>
                  </ol>
                </section>
              </Show>
            );
          })()}
        </Show>

        <details class="raw-details">
          <summary>Raw JSON</summary>
          <pre class="json-view">{JSON.stringify(data(), null, 2)}</pre>
        </details>
      </Show>
    </div>
  );
}
