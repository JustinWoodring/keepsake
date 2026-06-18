import { Show, createSignal } from "solid-js";
import { state } from "../state";
import { api } from "../api";
import { showToast } from "../state";

export function SyncPage() {
  const [url, setUrl] = createSignal(state.syncUrl());
  const [busy, setBusy] = createSignal<"" | "register" | "push" | "pull">("");
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

  async function register() {
    setBusy("register");
    try {
      await api.syncRegister(url());
      showToast("ok", "Registered with server");
      setStatus("registered");
    } catch (e) {
      showToast("err", String(e));
    } finally {
      setBusy("");
    }
  }

  async function pull() {
    setBusy("pull");
    try {
      const n = await api.syncPull(url());
      showToast("ok", `Pulled ${n} change(s)`);
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
      const n = await api.syncPush(url());
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
            Configure the self-hosted sync server.  All data
            remains end-to-end-encrypted; the server only
            stores ciphertext.
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
          <div class="form-actions">
            <button type="submit" class="btn btn-primary" disabled={busy() !== ""}>
              Save
            </button>
            <button
              type="button"
              class="btn"
              onClick={register}
              disabled={busy() !== ""}
            >
              {busy() === "register" ? "Registering…" : "Register"}
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
