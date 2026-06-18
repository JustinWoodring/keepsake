import { Show, createEffect, createResource, createSignal, For } from "solid-js";
import { state, showToast } from "../state";
import { api } from "../api";

export function SyncPage() {
  const [url, setUrl] = createSignal(state.syncUrl());
  const [vaultId, setVaultIdRaw] = createSignal(state.syncVaultId());
  const [passphrase, setPassphrase] = createSignal("");
  const [busy, setBusy] = createSignal<
    "" | "push" | "pull" | "setup" | "reveal" | "delete"
  >("");
  const [revealed, setRevealed] = createSignal<[string, string] | null>(null);

  // Wrapper that mirrors vaultId to localStorage.
  const setVaultId = (v: string) => {
    setVaultIdRaw(v);
    state.setSyncVaultId(v);
  };

  // List of configured shared-sync vault ids.  Refreshed
  // after every setup/delete.
  const [syncIds, { refetch: refetchSyncIds }] = createResource(
    () => state.status().unlocked && state.status().username,
    async (unlocked) => {
      if (!unlocked) return [] as string[];
      return api.listSharedSyncs();
    },
  );

  // Reactive: if the user has not typed anything yet and we
  // have a list of known vault ids, fill the input with the
  // persisted choice if it's still valid, otherwise the
  // first one in the list.
  createEffect(() => {
    const ids = syncIds();
    if (!ids) return;
    const current = vaultId();
    if (current !== "" && ids.includes(current)) return;
    if (ids.length > 0) {
      setVaultId(ids[0]);
    }
  });

  // True iff the current vaultId has a configured shared
  // sync setup.  Drives the visibility of Push/Pull.
  const isConfigured = () => {
    const ids = syncIds() ?? [];
    const v = vaultId();
    return v !== "" && ids.includes(v);
  };

  async function save(e: Event) {
    e.preventDefault();
    state.setSyncUrl(url());
    showToast("ok", "Sync URL saved");
  }

  async function push() {
    if (!isConfigured()) {
      showToast("err", "Set up shared sync for this vault id first");
      return;
    }
    setBusy("push");
    try {
      const n = await api.syncPush(url(), vaultId());
      showToast("ok", `Pushed ${n} record(s)`);
    } catch (e) {
      showToast("err", String(e));
    } finally {
      setBusy("");
    }
  }

  async function pull() {
    if (!isConfigured()) {
      showToast("err", "Set up shared sync for this vault id first");
      return;
    }
    setBusy("pull");
    try {
      const n = await api.syncPull(url(), vaultId());
      showToast("ok", `Pulled ${n} record(s)`);
    } catch (e) {
      showToast("err", String(e));
    } finally {
      setBusy("");
    }
  }

  async function setupShared() {
    if (!vaultId() || !passphrase()) {
      showToast("err", "Both vault id and passphrase are required");
      return;
    }
    setBusy("setup");
    try {
      await api.setupSharedSync(vaultId(), passphrase());
      setPassphrase("");
      setRevealed(null);
      await refetchSyncIds();
      showToast("ok", `Shared sync set up for '${vaultId()}'`);
    } catch (e) {
      showToast("err", String(e));
    } finally {
      setBusy("");
    }
  }

  async function revealShared() {
    if (!vaultId()) {
      showToast("err", "Enter a vault id first");
      return;
    }
    setBusy("reveal");
    try {
      const r = await api.revealSharedSync(vaultId());
      setRevealed(r);
      showToast("ok", `Revealed setup for '${r[0]}'`);
    } catch (e) {
      showToast("err", String(e));
    } finally {
      setBusy("");
    }
  }

  async function deleteShared() {
    if (!vaultId()) {
      showToast("err", "Enter a vault id first");
      return;
    }
    setBusy("delete");
    try {
      await api.deleteSharedSync(vaultId());
      setRevealed(null);
      await refetchSyncIds();
      showToast("ok", `Deleted shared sync '${vaultId()}'`);
    } catch (e) {
      showToast("err", String(e));
    } finally {
      setBusy("");
    }
  }

  function copyToClipboard(s: string) {
    if (typeof navigator !== "undefined" && navigator.clipboard) {
      navigator.clipboard.writeText(s).then(
        () => showToast("ok", "Copied"),
        () => showToast("err", "Clipboard copy failed"),
      );
    } else {
      showToast("err", "Clipboard not available");
    }
  }

  return (
    <div class="page">
      <header class="page-header">
        <div>
          <h1 class="page-title">⇄ Sync</h1>
          <p class="page-sub">
            End-to-end-encrypted.  The server stores doubly-sealed
            ciphertext (vault key inner, shared sync key outer).
            Share the vault id and passphrase out-of-band to add
            another device.
          </p>
        </div>
      </header>

      <Show when={!state.status().unlocked}>
        <p class="muted">Unlock the vault to use sync.</p>
      </Show>

      <Show when={state.status().unlocked}>
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
                placeholder="family"
                list="known-vault-ids"
                value={vaultId()}
                onInput={(e) => setVaultId(e.currentTarget.value)}
              />
              <datalist id="known-vault-ids">
                <For each={syncIds() ?? []}>
                  {(id) => <option value={id} />}
                </For>
              </datalist>
            </div>
            <div class="form-field">
              <label>shared passphrase (only needed to set up or rotate)</label>
              <input
                type="password"
                placeholder="used to derive the shared sync key"
                value={passphrase()}
                onInput={(e) => setPassphrase(e.currentTarget.value)}
              />
            </div>
            <div class="form-actions">
              <button type="submit" class="btn" disabled={busy() !== ""}>
                Save URL
              </button>
              <button
                type="button"
                class="btn btn-primary"
                onClick={setupShared}
                disabled={busy() !== "" || !vaultId() || !passphrase()}
              >
                {busy() === "setup" ? "Saving…" : "Set up / rotate"}
              </button>
              <button
                type="button"
                class="btn"
                onClick={revealShared}
                disabled={busy() !== "" || !vaultId()}
              >
                {busy() === "reveal" ? "…" : "Show setup"}
              </button>
              <button
                type="button"
                class="btn"
                onClick={deleteShared}
                disabled={busy() !== "" || !vaultId()}
              >
                {busy() === "delete" ? "…" : "Delete"}
              </button>
            </div>
            <Show when={isConfigured()}>
              <div class="form-actions" style="margin-top: 0.5rem">
                <button
                  type="button"
                  class="btn"
                  onClick={push}
                  disabled={busy() !== ""}
                  title="Push every local record to the server"
                >
                  {busy() === "push" ? "Pushing…" : "Push"}
                </button>
                <button
                  type="button"
                  class="btn"
                  onClick={pull}
                  disabled={busy() !== ""}
                  title="Pull remote changes and merge them locally"
                >
                  {busy() === "pull" ? "Pulling…" : "Pull"}
                </button>
              </div>
            </Show>
          </form>

          <Show when={revealed()}>
            <div class="settings-row" style="margin-top: 1rem">
              <p class="muted-small" style="margin: 0 0 0.5rem 0">
                Share these out-of-band to set up another device:
              </p>
              <div class="form-field">
                <label>vault id</label>
                <div style="display: flex; gap: 0.5rem">
                  <input type="text" readonly value={revealed()![0]} />
                  <button
                    type="button"
                    class="btn"
                    onClick={() => copyToClipboard(revealed()![0])}
                  >
                    Copy
                  </button>
                </div>
              </div>
              <div class="form-field">
                <label>passphrase</label>
                <div style="display: flex; gap: 0.5rem">
                  <input type="text" readonly value={revealed()![1]} />
                  <button
                    type="button"
                    class="btn"
                    onClick={() => copyToClipboard(revealed()![1])}
                  >
                    Copy
                  </button>
                </div>
              </div>
              <p class="muted-small" style="margin: 0.5rem 0 0 0">
                Anyone with the vault id and passphrase can read
                and write every record in the vault.  Treat them
                like a shared key.
              </p>
            </div>
          </Show>
        </div>
      </Show>
    </div>
  );
}
