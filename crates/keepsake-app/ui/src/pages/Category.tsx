import { For, Show, createResource, createSignal } from "solid-js";
import { A, useParams } from "@solidjs/router";
import { api, META_BY_TYPE, RecordType, ListEntry } from "../api";
import { COLUMNS_BY_TYPE, renderCell } from "../columns";
import { showToast } from "../state";

export function Category() {
  const params = useParams<{ type: string }>();
  const recordType = (): RecordType => params.type as RecordType;
  const meta = () => META_BY_TYPE[recordType()];

  const [filter, setFilter] = createSignal("");
  const [entries, { refetch }] = createResource(
    () => params.type,
    async (t) => api.listRecords(t),
  );

  async function del(id: string) {
    if (!confirm("Delete this record? This cannot be undone.")) return;
    try {
      await api.deleteRecord(id);
      showToast("ok", "Record deleted");
      refetch();
    } catch (e) {
      showToast("err", String(e));
    }
  }

  // For each entry, we need the actual record fields.  Fetch
  // them lazily.  In v1 we show-record per id; for a list this
  // is N+1 but the records are small.  We can swap to a
  // list-with-fields RPC later.
  const [rows, { refetch: refetchRows }] = createResource(
    () => entries(),
    async (list) => {
      if (!list) return [];
      const out: { id: string; fields: Record<string, unknown>; updated_by: string; updated_at: string }[] = [];
      for (const e of list) {
        try {
          const rec = (await api.showRecord(e.id, false)) as Record<string, unknown>;
          out.push({
            id: e.id,
            fields: rec,
            updated_by: e.updated_by,
            updated_at: e.updated_at,
          });
        } catch {
          // skip on error
        }
      }
      return out;
    },
  );

  const filtered = () => {
    const q = filter().toLowerCase().trim();
    const all = rows() ?? [];
    if (!q) return all;
    return all.filter((r) => {
      const hay = Object.values(r.fields).join(" ").toLowerCase();
      return hay.includes(q);
    });
  };

  const columns = () => COLUMNS_BY_TYPE[recordType()] ?? [];
  const totalFlex = () => {
    let total = 0;
    for (const c of columns()) {
      if (typeof c.flex === "number") total += c.flex;
    }
    return total || 1;
  };

  return (
    <div class="page">
      <header class="page-header">
        <div>
          <h1 class="page-title">
            <span class="title-emoji">{meta()?.icon}</span>
            {meta()?.label ?? recordType()}
          </h1>
          <p class="page-sub">
            {entries()?.length ?? 0} record{(entries()?.length ?? 0) === 1 ? "" : "s"}
            <Show when={meta()?.blurb}>
              <span class="muted"> · {meta()!.blurb}</span>
            </Show>
          </p>
        </div>
        <div class="page-actions">
          <input
            class="input"
            placeholder="Filter…"
            value={filter()}
            onInput={(e) => setFilter(e.currentTarget.value)}
            style={{ width: "220px" }}
          />
          <A class="btn btn-primary" href={`/c/${params.type}/new`}>
            + New
          </A>
        </div>
      </header>

      <Show when={entries() !== undefined} fallback={<p class="muted">Loading…</p>}>
        <Show
          when={filtered().length > 0}
          fallback={
            <div class="table-wrap">
              <div class="empty-state">
                <div class="empty-state-emoji">{meta()?.icon}</div>
                <p class="empty-state-title">No {meta()?.label.toLowerCase() ?? "records"} yet</p>
                <p class="empty-state-sub">Add your first one to get started.</p>
                <A class="btn btn-primary" href={`/c/${params.type}/new`}>
                  + New {meta()?.label ?? "record"}
                </A>
              </div>
            </div>
          }
        >
          <div class="table-wrap">
            <table class="rows">
              <colgroup>
                <For each={columns()}>
                  {(c) => {
                    const w =
                      typeof c.flex === "number"
                        ? `${(c.flex / totalFlex()) * 100}%`
                        : c.flex;
                    return <col style={{ width: w }} />;
                  }}
                </For>
                <col style="width: 1px" />
              </colgroup>
              <thead>
                <tr>
                  <For each={columns()}>
                    {(c) => (
                      <th
                        class={c.align === "right" ? "col-right" : "col-left"}
                      >
                        {c.label}
                      </th>
                    )}
                  </For>
                  <th class="col-actions" style="text-align: right">actions</th>
                </tr>
              </thead>
              <tbody>
                <For each={filtered()}>
                  {(r) => (
                    <tr>
                      <For each={columns()}>
                        {(c) => (
                          <td class="col-cell">
                            <A href={`/r/${r.id}`} class="row-link">
                              {renderCell(c, r.fields) || <span class="muted">—</span>}
                            </A>
                          </td>
                        )}
                      </For>
                      <td class="actions">
                        <A href={`/r/${r.id}`}>view</A>
                        <A href={`/r/${r.id}/edit`}>edit</A>
                        <button
                          type="button"
                          class="action-danger"
                          onClick={() => del(r.id)}
                        >
                          delete
                        </button>
                      </td>
                    </tr>
                  )}
                </For>
              </tbody>
            </table>
          </div>
        </Show>
      </Show>
    </div>
  );
}
