import { For, Show, createResource, onMount } from "solid-js";
import { A } from "@solidjs/router";
import { api, META_BY_TYPE, RECORD_TYPES } from "../api";
import { generateInsights, Insight } from "../insights";

export function Dashboard() {
  const [counts, { refetch: refetchCounts }] = createResource(async () => {
    const out: Record<string, number> = {};
    for (const t of RECORD_TYPES) {
      try {
        const list = await api.listRecords(t.type);
        out[t.type] = list.length;
      } catch {
        out[t.type] = 0;
      }
    }
    return out;
  });

  const [activity] = createResource(async () => {
    try {
      const audit = await api.audit(false) as Array<{
        seq: number; op: string; actor: string; target_id: string | null; ts: string;
      }>;
      return audit.slice(0, 6);
    } catch {
      return [] as Array<{ seq: number; op: string; actor: string; target_id: string | null; ts: string }>;
    }
  });

  const [insights] = createResource(async () => {
    try { return await generateInsights(); }
    catch { return [] as Insight[]; }
  });
  const topInsights = () => (insights() ?? []).slice(0, 2);

  const greeting = (): string => {
    const h = new Date().getHours();
    if (h < 5) return "Working late";
    if (h < 12) return "Good morning";
    if (h < 17) return "Good afternoon";
    if (h < 21) return "Good evening";
    return "Working late";
  };

  onMount(() => {
    void refetchCounts;
  });

  // Group record types by their sidebar group.
  const groups = () => {
    const c = counts();
    if (!c) return [];
    const map = new Map<string, { name: string; total: number; types: typeof RECORD_TYPES }>();
    for (const t of RECORD_TYPES) {
      let g = map.get(t.group);
      if (!g) {
        g = { name: t.group, total: 0, types: [] };
        map.set(t.group, g);
      }
      g.types.push(t);
      g.total += c[t.type] ?? 0;
    }
    return Array.from(map.values());
  };

  const totalRecords = () => {
    const c = counts();
    if (!c) return 0;
    return Object.values(c).reduce((s, n) => s + n, 0);
  };

  return (
    <div class="page">
      <div class="dashboard-hero">
        <div>
          <h1>{greeting()}.</h1>
          <p>Your encrypted vault.</p>
        </div>
        <div class="dashboard-hero-stat">
          <span class="dashboard-hero-stat-value">{totalRecords()}</span>
          <span class="dashboard-hero-stat-label">records</span>
        </div>
      </div>

      <Show when={topInsights().length > 0}>
        <section class="dashboard-section">
          <header class="dashboard-section-header">
            <h2>Worth looking at</h2>
            <A class="muted-small" href="/insights">View all →</A>
          </header>
          <div class="insights-list">
            <For each={topInsights()}>
              {(i) => (
                <div class={`insight insight-${i.severity}`}>
                  <div class="insight-icon">
                    {i.severity === "warn" ? "⚠" : i.severity === "ok" ? "✓" : "ⓘ"}
                  </div>
                  <div class="insight-body">
                    <div class="insight-title">{i.title}</div>
                    <div class="insight-detail">{i.detail}</div>
                  </div>
                  <Show when={i.to}>
                    <A class="insight-action" href={i.to!}>Open →</A>
                  </Show>
                </div>
              )}
            </For>
          </div>
        </section>
      </Show>

      <section class="dashboard-section">
        <header class="dashboard-section-header">
          <h2>Categories</h2>
        </header>
        <div class="dashboard-groups">
          <For each={groups()}>
            {(g) => (
              <A class="dashboard-group" href={firstCategoryHref(g.name)}>
                <div class="dashboard-group-info">
                  <h3>{g.name}</h3>
                  <div class="dashboard-group-types">
                    <For each={g.types}>
                      {(t) => (
                        <span class="dashboard-group-type">
                          <span class="ic">{t.icon}</span>{t.label}
                        </span>
                      )}
                    </For>
                  </div>
                </div>
                <div class="dashboard-group-count">{g.total}</div>
              </A>
            )}
          </For>
        </div>
      </section>

      <Show when={activity() && activity()!.length > 0}>
        <section class="dashboard-section">
          <header class="dashboard-section-header">
            <h2>Recent activity</h2>
            <A class="muted-small" href="/audit">View all →</A>
          </header>
          <div class="dashboard-activity">
            <For each={activity()}>
              {(a) => (
                <div class="dashboard-activity-row">
                  <span class="dashboard-activity-time">
                    {new Date(a.ts).toLocaleString()}
                  </span>
                  <span class="dashboard-activity-op">{a.op}</span>
                  <span class="dashboard-activity-actor">{a.actor}</span>
                </div>
              )}
            </For>
          </div>
        </section>
      </Show>
    </div>
  );
}

function firstCategoryHref(groupName: string): string {
  const t = RECORD_TYPES.find((r) => r.group === groupName);
  return t ? `/c/${t.type}` : "/";
}
