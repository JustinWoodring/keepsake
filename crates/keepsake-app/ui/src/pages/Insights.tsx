import { For, Show, createResource, onMount } from "solid-js";
import { A } from "@solidjs/router";
import { generateInsights, Insight } from "../insights";

export function Insights() {
  const [insights, { refetch }] = createResource(async () => {
    return await generateInsights();
  });

  onMount(() => {
    refetch();
  });

  function refetchOnFocus() {
    refetch();
  }
  if (typeof window !== "undefined") {
    window.addEventListener("focus", refetchOnFocus);
  }

  return (
    <div class="page">
      <header class="page-header">
        <div>
          <h1 class="page-title">📈 Insights</h1>
          <p class="page-sub">
            Things worth looking at — expiring items, missing
            fields, gaps in your records.
          </p>
        </div>
        <div class="page-actions">
          <button class="btn" onClick={() => refetch()}>↻ Refresh</button>
        </div>
      </header>

      <Show when={insights()} fallback={<p class="muted">Scanning vault…</p>}>
        <Show
          when={insights()!.length > 0}
          fallback={
            <div class="table-wrap">
              <div class="empty-state">
                <div class="empty-state-emoji">✨</div>
                <p class="empty-state-title">Nothing to flag right now</p>
                <p class="empty-state-sub">Your vault is in good shape.</p>
              </div>
            </div>
          }
        >
          <div class="insights-list">
            <For each={insights()}>
              {(i) => <InsightCard insight={i} />}
            </For>
          </div>
        </Show>
      </Show>
    </div>
  );
}

function InsightCard(props: { insight: Insight }) {
  const i = () => props.insight;
  return (
    <div class={`insight insight-${i().severity}`}>
      <div class="insight-icon">{iconFor(i().severity)}</div>
      <div class="insight-body">
        <div class="insight-title">{i().title}</div>
        <div class="insight-detail">{i().detail}</div>
      </div>
      <Show when={i().to}>
        <A class="insight-action" href={i().to!}>
          Open →
        </A>
      </Show>
    </div>
  );
}

function iconFor(s: Insight["severity"]): string {
  if (s === "warn") return "⚠";
  if (s === "ok") return "✓";
  return "ⓘ";
}
