import { Show, createSignal } from "solid-js";
import { state } from "../state";
import { api } from "../api";
import { showToast } from "../state";

export function SyncPage() {
  const [url, setUrl] = createSignal(state.syncUrl());
  const [vaultId, setVaultId] = createSignal("personal");
  const [busy, setBusy] = createSignal<"" | "push" | "pull">("");
  const [status, setStatus] = createSignal<string>("");

  async function save(e: Event) {
    e.preventDefault();
    try {
      await api.configureSync(url());
      state.setSyncUrl(url());
      setStatus("configured");
      showToast("ok", "Sync URL saved");
    } catch (e) {
      showToast("err", String(e));
    }
  }

  async function pull() {
    setBusy("pull");
    try {
      const n = await api.syncPull(url(), vaultId());
      showToast("ok", `Pulled ${n} record(s)`);
      setStatus(`pulled ${n}`);
    } catch (e) {
      showToast("err", String(e));
    } finally {
      setBusy("");
    }
  }

  async function push() {
    setBusy("push");
    try {
      const n = await api.syncPush(url(), vaultId());
      showToast("ok", `Pushed ${n} record(s)`);
      setStatus(`pushed ${n}`);
    } catch (e) {
      showToast("err", String(e));
    } finally {
      setBusy("");
    }
  }

  return (
    <div class="page">
      <header class="page-header">
        <div>
          <h1 class="page-title">⇄ Sync</h1>
          <p class="page-sub">
            End-to-end-encrypted.  The server stores ciphertext;
            the shared passphrase (or your personal master) is
            the only thing protecting the data.
          </p>
        </div>
      </header>

      <div class="settings-section">
        <form onSubmit={save} class="form" style="background: transparent; border: none; padding: 0">
          <div class="form-field">
            <label>server base URL</label>
            <input
              type="url"
              placeholder="https://sync.example.com"
              value={url()}
              onInput={(e) => setUrl(e.currentTarget.value)}
            />
          </div>
          <div class="form-field">
            <label>vault id</label>
            <input
              type="text"
              placeholder="personal"
              value={vaultId()}
              onInput={(e) => setVaultId(e.currentTarget.value)}
            />
          </div>
          <div class="form-actions">
            <button type="submit" class="btn btn-primary" disabled={busy() !== ""}>
              Save
            </button>
            <button
              type="button"
              class="btn"
              onClick={push}
              disabled={busy() !== ""}
            >
              {busy() === "push" ? "Pushing…" : "Push"}
            </button>
            <button
              type="button"
              class="btn"
              onClick={pull}
              disabled={busy() !== ""}
            >
              {busy() === "pull" ? "Pulling…" : "Pull"}
            </button>
          </div>
        </form>
        <Show when={status()}>
          <p class="muted-small" style="margin-top: 0.75rem">{status()}</p>
        </Show>
      </div>
    </div>
  );
}
