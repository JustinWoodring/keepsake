import { For, Show, createResource, createSignal, Match, Switch } from "solid-js";
import { api, AuditEntryView } from "../api";
import { showToast, state } from "../state";

type VerifyResult = { ok: boolean; entries: number; first_broken?: number };

export function AuditPage() {
  // The data resource always fetches the entry list — the
  // "Verify chain" button is a separate one-shot action that
  // doesn't change which data the page shows.
  const [refreshTick, setRefreshTick] = createSignal(0);
  const [entries] = createResource<AuditEntryView[], number>(
    () => refreshTick(),
    async () => {
      try {
        return (await api.audit(false)) as AuditEntryView[];
      } catch (e) {
        throw e;
      }
    },
  );

  const [lastVerify, setLastVerify] = createSignal<VerifyResult | null>(null);

  async function doVerify() {
    try {
      const r = (await api.audit(true)) as VerifyResult;
      setLastVerify(r);
      if (r.ok) {
        showToast("ok", `Audit chain verified (${r.entries} entries)`);
      } else {
        showToast(
          "err",
          `Chain verification failed at entry ${r.first_broken}`,
        );
      }
    } catch (e) {
      showToast("err", String(e));
    }
  }

  function refresh() {
    setRefreshTick((n) => n + 1);
  }

  async function resetChain() {
    const ok = confirm(
      "Reset audit chain?\n\n" +
      "This drops every entry before the first one whose hash doesn't match the current chain format, then re-chains the survivors starting from a new genesis. Destructive; cannot be undone.",
    );
    if (!ok) return;
    try {
      const dropped = await api.rewriteAuditChain();
      showToast("ok", dropped === 0
        ? "Audit chain was already valid"
        : `Dropped ${dropped} legacy entries and re-chained the rest`,
      );
      // Clear the verify banner and refetch the list.
      setLastVerify(null);
      refresh();
    } catch (e) {
      showToast("err", String(e));
    }
  }

  const list = (): AuditEntryView[] => entries() ?? [];

  return (
    <div class="page">
      <header class="page-header">
        <div>
          <h1 class="page-title">🛡 Audit</h1>
          <p class="page-sub">
            Append-only, hash-chained record of every change.
          </p>
        </div>
        <div class="page-actions">
          <button class="btn" onClick={refresh}>Refresh</button>
          <button class="btn" onClick={doVerify}>Verify chain</button>
          <button class="btn btn-danger" onClick={resetChain}>Reset chain</button>
        </div>
      </header>

      <Show when={state.status().unlocked} fallback={
        <div class="empty-state">
          <div class="empty-state-emoji">🔒</div>
          <p class="empty-state-title">Vault is locked</p>
          <p class="empty-state-sub">
            Unlock the vault from the sidebar to view the audit log.
          </p>
        </div>
      }>
        <Show when={lastVerify() && !lastVerify()!.ok}>
          <div class="banner banner-warn">
            ⚠️ Chain verification failed at entry {lastVerify()!.first_broken}.
            The entries below are still displayed, but the chain is broken —
            usually because an older entry was written by an earlier
            version of Keepsake with a different hash function.
          </div>
        </Show>
        <Switch>
          <Match when={entries.loading}>
            <p class="muted">Loading…</p>
          </Match>
          <Match when={entries.error}>
            <div class="empty-state">
              <div class="empty-state-emoji">⚠️</div>
              <p class="empty-state-title">Failed to read audit log</p>
              <p class="empty-state-sub">{String(entries.error)}</p>
            </div>
          </Match>
          <Match when={list().length === 0}>
            <div class="empty-state">
              <div class="empty-state-emoji">🛡</div>
              <p class="empty-state-title">No audit entries yet</p>
              <p class="empty-state-sub">
                Actions like unlocking the vault, adding records, and changing your password are recorded here.
              </p>
            </div>
          </Match>
          <Match when={list().length > 0}>
            <div class="table-wrap">
              <table class="rows">
                <thead>
                  <tr>
                    <th>seq</th>
                    <th>op</th>
                    <th>actor</th>
                    <th>target</th>
                    <th>details</th>
                    <th>ts</th>
                  </tr>
                </thead>
                <tbody>
                  <For each={list()}>
                    {(e) => (
                      <tr class="audit-row">
                        <td>{e.seq}</td>
                        <td><span class="op">{e.op}</span></td>
                        <td>{e.actor}</td>
                        <td><code>{e.target_id ?? ""}</code></td>
                        <td>{e.details ?? ""}</td>
                        <td>{new Date(e.ts).toLocaleString()}</td>
                      </tr>
                    )}
                  </For>
                </tbody>
              </table>
            </div>
          </Match>
        </Switch>
      </Show>
    </div>
  );
}
