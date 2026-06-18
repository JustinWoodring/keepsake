import { For, Show, createSignal, onMount } from "solid-js";
import { state, refreshUsers, showToast } from "../state";
import { api } from "../api";

export function Settings() {
  onMount(async () => {
    await refreshUsers();
  });

  return (
    <div class="page">
      <header class="page-header">
        <div>
          <h1 class="page-title">Settings</h1>
          <p class="page-sub">Vault, security, and backup.</p>
        </div>
      </header>

      <VaultInfo />
      <Backup />
      <Users />
      <ChangePassword />
      <Security />
    </div>
  );
}

function VaultInfo() {
  return (
    <div class="settings-section">
      <h2>Vault</h2>
      <dl class="dl">
        <dt>Path</dt>
        <dd><code>{state.vaultPath() || "—"}</code></dd>
        <dt>Current user</dt>
        <dd>
          <Show when={state.status().username} fallback="—">
            <span class="badge accent">{state.status().username}</span>
          </Show>
        </dd>
      </dl>
    </div>
  );
}

function Backup() {
  const [mode, setMode] = createSignal<"idle" | "export" | "import">("idle");
  const [pass, setPass] = createSignal("");
  const [confirm, setConfirm] = createSignal("");
  const [bytes, setBytes] = createSignal("");
  const [busy, setBusy] = createSignal(false);

  async function doExport(e: Event) {
    e.preventDefault();
    if (pass() !== confirm()) {
      showToast("err", "Passphrases do not match");
      return;
    }
    setBusy(true);
    try {
      const data = await api.exportBundle(pass());
      setBytes(JSON.stringify(data));
      showToast("ok", `Bundle generated (${data.length} bytes). Copy and save to a safe location.`);
    } catch (err) {
      showToast("err", String(err));
    } finally {
      setBusy(false);
    }
  }

  async function doImport(e: Event) {
    e.preventDefault();
    if (!bytes().trim()) {
      showToast("err", "Paste bundle bytes first");
      return;
    }
    let arr: number[];
    try {
      arr = JSON.parse(bytes().trim());
      if (!Array.isArray(arr)) throw new Error("not an array");
    } catch (err) {
      showToast("err", `Invalid bundle: ${err}`);
      return;
    }
    setBusy(true);
    try {
      await api.importBundle(arr, pass());
      showToast("ok", "Bundle imported");
      setMode("idle");
    } catch (err) {
      showToast("err", String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div class="settings-section" id="export">
      <h2>Backup</h2>
      <p class="muted-small" style="margin: 0 0 0.75rem 0">
        Export the entire vault to an encrypted <code>.ksk</code> bundle.
        The bundle is sealed under a passphrase you choose — it can be
        different from your daily password. Save the bundle to offline
        media for recovery.
      </p>

      <Show when={mode() === "idle"}>
        <div class="form-actions" style="border-top: none; padding-top: 0">
          <button class="btn btn-primary" onClick={() => setMode("export")}>
            ⤓ Export vault
          </button>
          <button class="btn" onClick={() => setMode("import")}>
            ⤒ Import bundle
          </button>
        </div>
      </Show>

      <Show when={mode() === "export"}>
        <form onSubmit={doExport} class="backup-form">
          <div class="form-field">
            <label>export passphrase</label>
            <input
              type="password"
              value={pass()}
              onInput={(e) => setPass(e.currentTarget.value)}
              placeholder="A long, written-down string"
              required
            />
          </div>
          <div class="form-field">
            <label>confirm</label>
            <input
              type="password"
              value={confirm()}
              onInput={(e) => setConfirm(e.currentTarget.value)}
              required
            />
          </div>
          <Show when={bytes()}>
            <div class="form-field">
              <label>bundle (copy this and save somewhere safe)</label>
              <textarea
                rows="6"
                readonly
                value={bytes()}
                onClick={(e) => e.currentTarget.select()}
              />
            </div>
          </Show>
          <div class="form-actions">
            <button type="submit" class="btn btn-primary" disabled={busy()}>
              {busy() ? "Generating…" : "Generate bundle"}
            </button>
            <button type="button" class="btn btn-ghost" onClick={() => { setMode("idle"); setPass(""); setConfirm(""); setBytes(""); }}>
              Cancel
            </button>
          </div>
        </form>
      </Show>

      <Show when={mode() === "import"}>
        <form onSubmit={doImport} class="backup-form">
          <div class="form-field">
            <label>bundle passphrase</label>
            <input
              type="password"
              value={pass()}
              onInput={(e) => setPass(e.currentTarget.value)}
              required
            />
          </div>
          <div class="form-field">
            <label>bundle bytes (JSON array)</label>
            <textarea
              rows="6"
              value={bytes()}
              onInput={(e) => setBytes(e.currentTarget.value)}
              placeholder="Paste the [12, 34, 56, ...] array from your export"
              required
            />
          </div>
          <div class="form-actions">
            <button type="submit" class="btn btn-primary" disabled={busy()}>
              {busy() ? "Importing…" : "Import bundle"}
            </button>
            <button type="button" class="btn btn-ghost" onClick={() => { setMode("idle"); setPass(""); setBytes(""); }}>
              Cancel
            </button>
          </div>
        </form>
      </Show>
    </div>
  );
}

function Users() {
  const [showForm, setShowForm] = createSignal(false);
  const [newName, setNewName] = createSignal("");
  const [newPw, setNewPw] = createSignal("");
  const [busy, setBusy] = createSignal(false);

  async function addUser(e: Event) {
    e.preventDefault();
    setBusy(true);
    try {
      await api.addUser(newName(), newPw());
      setNewName("");
      setNewPw("");
      setShowForm(false);
      await refreshUsers();
      showToast("ok", `Added user ${newName()}`);
    } catch (err) {
      showToast("err", String(err));
    } finally {
      setBusy(false);
    }
  }

  async function removeUser(username: string) {
    if (!confirm(`Remove user "${username}" from this device? They can be re-added later with their password.`)) {
      return;
    }
    try {
      await api.removeUser(username);
      await refreshUsers();
      showToast("ok", `Removed user ${username}`);
    } catch (err) {
      showToast("err", String(err));
    }
  }

  return (
    <div class="settings-section">
      <div class="settings-section-header">
        <h2>Users on this device</h2>
        <Show
          when={!showForm()}
          fallback={
            <button class="btn btn-ghost" onClick={() => setShowForm(false)}>
              Cancel
            </button>
          }
        >
          <button class="btn" onClick={() => setShowForm(true)}>
            + Add user
          </button>
        </Show>
      </div>

      <Show when={showForm()}>
        <form onSubmit={addUser} class="add-user-form">
          <div class="form-field">
            <label>username</label>
            <input
              value={newName()}
              onInput={(e) => setNewName(e.currentTarget.value)}
              placeholder="e.g. dahlia"
              required
            />
          </div>
          <div class="form-field">
            <label>password</label>
            <input
              type="password"
              value={newPw()}
              onInput={(e) => setNewPw(e.currentTarget.value)}
              placeholder="strong password"
              required
            />
          </div>
          <div class="form-actions">
            <button type="submit" class="btn btn-primary" disabled={busy()}>
              {busy() ? "Adding…" : "Add user"}
            </button>
            <button type="button" class="btn btn-ghost" onClick={() => setShowForm(false)}>
              Cancel
            </button>
          </div>
        </form>
      </Show>

      <Show
        when={state.users().length > 0}
        fallback={
          <Show when={!showForm()}>
            <p class="muted">No users on this device.</p>
          </Show>
        }
      >
        <ul class="user-list">
          <For each={state.users()}>
            {(u) => (
              <li class="user-row">
                <span class="user-name">
                  <Show
                    when={u === state.status().username}
                    fallback={<span class="badge">{u}</span>}
                  >
                    <span class="badge accent">{u}</span>
                    <span class="muted-small" style="margin-left: 0.5rem">current</span>
                  </Show>
                </span>
                <Show when={u !== state.status().username}>
                  <button
                    class="btn btn-ghost"
                    onClick={() => removeUser(u)}
                    title="Remove from this device"
                  >
                    Remove
                  </button>
                </Show>
              </li>
            )}
          </For>
        </ul>
      </Show>
    </div>
  );
}

function ChangePassword() {
  const [next, setNext] = createSignal("");
  const [confirm, setConfirm] = createSignal("");
  const [busy, setBusy] = createSignal(false);

  async function submit(e: Event) {
    e.preventDefault();
    if (next() !== confirm()) {
      showToast("err", "New passwords do not match");
      return;
    }
    setBusy(true);
    try {
      await api.changePassword(next());
      setNext("");
      setConfirm("");
      showToast("ok", "Password changed");
    } catch (err) {
      showToast("err", String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div class="settings-section">
      <h2>Change password</h2>
      <p class="muted-small" style="margin: 0 0 1rem 0">
        Re-seals the vault key under a new password. The vault key itself stays the same.
      </p>
      <form onSubmit={submit} class="change-pw-form">
        <div class="form-field">
          <label>new password</label>
          <input
            type="password"
            value={next()}
            onInput={(e) => setNext(e.currentTarget.value)}
            required
          />
        </div>
        <div class="form-field">
          <label>confirm</label>
          <input
            type="password"
            value={confirm()}
            onInput={(e) => setConfirm(e.currentTarget.value)}
            required
          />
        </div>
        <div class="form-actions">
          <button type="submit" class="btn btn-primary" disabled={busy()}>
            {busy() ? "Changing…" : "Change password"}
          </button>
        </div>
      </form>
    </div>
  );
}

function Security() {
  return (
    <div class="settings-section">
      <h2>Security</h2>
      <p style="margin: 0">
        All data is end-to-end encrypted. The master password is the only
        thing protecting the vault; if you lose it, recovery requires a
        <code> .ksk</code> export.
      </p>
      <p class="muted-small" style="margin: 0.5rem 0 0 0">
        See <code>docs/threat-model.md</code> in the source tree for the full
        threat model.
      </p>
    </div>
  );
}
