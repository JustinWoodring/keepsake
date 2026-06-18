import { JSX, onMount, Show, createSignal, createResource } from "solid-js";
import { A, useLocation, useNavigate } from "@solidjs/router";
import { state, refreshStatus, refreshUsers, showToast } from "./state";
import { api, META_BY_TYPE, RECORD_TYPES } from "./api";
import { Toast } from "./components/Toast";
import { generateInsights } from "./insights";
import { installLinkClickHandler } from "./links";

export function App(props: { children?: JSX.Element }) {
  const navigate = useNavigate();
  onMount(async () => {
    // Wire static [[uuid]] link chips (rendered via innerHTML by
    // the markdown layer) into Solid Router's client-side
    // navigation.  Without this, a click on a link chip would
    // do a full page reload to the new path.
    installLinkClickHandler(navigate);
    await refreshUsers();
    await refreshStatus();
  });

  return (
    <div class="app">
      <Show when={state.status().unlocked} fallback={<Unlock />}>
        <Shell>{props.children}</Shell>
      </Show>
      <Show when={state.toast()}>
        {(t) => <Toast kind={t().kind} text={t().text} />}
      </Show>
    </div>
  );
}

function Unlock() {
  const [username, setUsername] = createSignal("");
  const [password, setPassword] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  const [importing, setImporting] = createSignal(false);
  const [importPass, setImportPass] = createSignal("");
  const [importBytes, setImportBytes] = createSignal<string>("");
  const [importError, setImportError] = createSignal<string | null>(null);

  // Reactive on the live users signal — no need for a separate
  // local signal that can drift out of sync.
  const hasVault = () => state.users().length > 0;

  // Refresh users whenever the unlock screen mounts in case the
  // signal was last populated in a different session.
  onMount(() => {
    void refreshUsers();
  });

  async function submit(e: Event) {
    e.preventDefault();
    setBusy(true);
    try {
      if (hasVault()) {
        await api.unlock(username(), password());
        showToast("ok", `Welcome back, ${username()}`);
      } else {
        await api.init(username(), password());
        showToast("ok", `Vault created for ${username()}`);
      }
      await refreshStatus();
    } catch (e) {
      showToast("err", String(e));
    } finally {
      setBusy(false);
    }
  }

  async function pickFile() {
    setImportError(null);
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({
        multiple: false,
        filters: [{ name: "Keepsake bundle", extensions: ["ksk"] }],
      });
      if (!selected) return;
      const { readTextFile } = await import("@tauri-apps/plugin-fs");
      const txt = await readTextFile(selected as string);
      setImportBytes(txt);
    } catch (e) {
      setImportError(String(e));
    }
  }

  async function doImport(e: Event) {
    e.preventDefault();
    if (!importBytes().trim()) {
      setImportError("Pick a .ksk file first");
      return;
    }
    if (!importPass()) {
      setImportError("Enter the export passphrase");
      return;
    }
    if (!username().trim()) {
      setImportError("Pick a username to associate with the import");
      return;
    }
    if (!password()) {
      setImportError("Pick a password for the new account");
      return;
    }
    let arr: number[];
    try {
      arr = JSON.parse(importBytes().trim());
      if (!Array.isArray(arr)) throw new Error("not a JSON array");
    } catch (err) {
      setImportError(`Invalid bundle format: ${err}`);
      return;
    }
    setImporting(true);
    setImportError(null);
    try {
      await api.importToNewVault({
        bytes: arr,
        passphrase: importPass(),
        username: username(),
        password: password(),
      });
      await refreshStatus();
      showToast("ok", "Bundle imported");
      setImportBytes("");
      setImportPass("");
    } catch (err) {
      setImportError(String(err));
    } finally {
      setImporting(false);
    }
  }

  return (
    <div class="unlock-shell">
      <div class="unlock-card">
        <div class="unlock-brand">
          <div class="unlock-mark">K</div>
          <span class="unlock-name">Keepsake</span>
        </div>
        <h1 class="unlock-title">
          {hasVault() ? "Unlock your vault" : importing() ? "Importing…" : "Set up your vault"}
        </h1>
        <p class="unlock-sub">
          {hasVault()
            ? "End-to-end encrypted. Local-first, sync-optional."
            : "Create a new vault, or import an existing .ksk bundle."}
        </p>

        <Show when={hasVault()}>
          <form class="unlock-form" onSubmit={submit}>
            <div class="unlock-field">
              <label>username</label>
              <input
                autocomplete="username"
                value={username()}
                onInput={(e) => setUsername(e.currentTarget.value)}
                required
                autofocus
              />
            </div>
            <div class="unlock-field">
              <label>password</label>
              <input
                type="password"
                autocomplete="current-password"
                value={password()}
                onInput={(e) => setPassword(e.currentTarget.value)}
                required
              />
            </div>
            <div class="unlock-actions">
              <button type="submit" class="btn btn-primary unlock-flex" disabled={busy()}>
                {busy() ? "Working…" : "Unlock"}
              </button>
            </div>
          </form>
        </Show>

        <Show when={!hasVault()}>
          <div class="unlock-form">
            <button
              type="button"
              class="btn btn-primary btn-block btn-lg"
              onClick={() => {
                document.getElementById("unlock-init-form")?.classList.toggle("hidden");
                document.getElementById("unlock-import-form")?.classList.add("hidden");
              }}
              disabled={importing()}
            >
              + Create new vault
            </button>

            <form id="unlock-init-form" class="unlock-init-form hidden" onSubmit={submit}>
              <div class="unlock-field">
                <label>username</label>
                <input
                  autocomplete="username"
                  value={username()}
                  onInput={(e) => setUsername(e.currentTarget.value)}
                  required
                />
              </div>
              <div class="unlock-field">
                <label>password</label>
                <input
                  type="password"
                  autocomplete="new-password"
                  value={password()}
                  onInput={(e) => setPassword(e.currentTarget.value)}
                  required
                />
              </div>
              <div class="unlock-actions">
                <button type="submit" class="btn btn-primary btn-block" disabled={busy()}>
                  {busy() ? "Creating…" : "Create vault"}
                </button>
              </div>
            </form>

            <div class="unlock-divider"><span>or</span></div>

            <button
              type="button"
              class="btn btn-block btn-lg"
              onClick={() => {
                document.getElementById("unlock-import-form")?.classList.toggle("hidden");
                document.getElementById("unlock-init-form")?.classList.add("hidden");
              }}
              disabled={importing()}
            >
              ⤓ Import .ksk bundle
            </button>

            <form id="unlock-import-form" class="unlock-init-form hidden" onSubmit={doImport}>
              <div class="unlock-field">
                <label>bundle file</label>
                <div class="row">
                  <input
                    type="text"
                    value={importBytes() ? "(loaded)" : ""}
                    placeholder="No file selected"
                    readonly
                  />
                  <button
                    type="button"
                    class="btn"
                    onClick={pickFile}
                    disabled={importing()}
                  >
                    Choose…
                  </button>
                </div>
              </div>
              <div class="unlock-field">
                <label>export passphrase</label>
                <input
                  type="password"
                  autocomplete="current-password"
                  value={importPass()}
                  onInput={(e) => setImportPass(e.currentTarget.value)}
                  required
                />
              </div>
              <Show when={importError()}>
                <div class="form-error">{importError()}</div>
              </Show>
              <div class="unlock-actions">
                <button type="submit" class="btn btn-primary btn-block" disabled={importing()}>
                  {importing() ? "Importing…" : "Import bundle"}
                </button>
              </div>
            </form>
          </div>
        </Show>

        <div class="unlock-meta">
          <Show when={hasVault() && state.users().length > 0}>
            <div>
              <span class="muted-small">users on this device:</span>{" "}
              {state.users().map((u, i) => (
                <>
                  {i > 0 ? ", " : ""}
                  <code>{u}</code>
                </>
              ))}
            </div>
          </Show>
          <div>
            <span class="muted-small">vault:</span>{" "}
            <code>{state.vaultPath() || "—"}</code>
          </div>
        </div>
      </div>
    </div>
  );
}

function Shell(props: { children?: JSX.Element }) {
  return (
    <div class="shell">
      <Sidebar />
      <main>{props.children}</main>
    </div>
  );
}

function Sidebar() {
  const nav = useNavigate();

  async function lockNow() {
    await api.lock();
    await refreshStatus();
    nav("/");
  }

  // Group record types by their sidebar group label.
  const groups: { name: string; types: typeof RECORD_TYPES }[] = [
    {
      name: "Identity & Access",
      types: RECORD_TYPES.filter((t) => t.group === "Identity & Access"),
    },
    {
      name: "Money",
      types: RECORD_TYPES.filter((t) => t.group === "Money"),
    },
    {
      name: "Property",
      types: RECORD_TYPES.filter((t) => t.group === "Property"),
    },
    {
      name: "Documents & Records",
      types: RECORD_TYPES.filter((t) => t.group === "Documents & Records"),
    },
    {
      name: "Notes & Logs",
      types: RECORD_TYPES.filter((t) => t.group === "Notes & Logs"),
    },
  ];

  return (
    <aside class="sidebar">
      <div class="sidebar-brand">
        <div class="sidebar-mark">K</div>
        <div class="sidebar-brand-text">
          <span class="sidebar-brand-name">Keepsake</span>
          <span class="sidebar-brand-user" title={state.status().username ?? ""}>
            {state.status().username ?? "—"}
          </span>
        </div>
      </div>

      <nav class="sidebar-nav">
        <SidebarLink href="/" icon="◐" label="Dashboard" end />
      </nav>

      <div class="sidebar-scroll">
        {groups.map((g) => (
          <div class="sidebar-group">
            <div class="sidebar-section">{g.name}</div>
            <nav>
              {g.types.map((t) => (
                <SidebarLink href={`/c/${t.type}`} icon={t.icon} label={t.label} />
              ))}
            </nav>
          </div>
        ))}

        <div class="sidebar-section">System</div>
        <nav>
          <SidebarLinkWithBadge href="/insights" icon="📈" label="Insights" />
          <SidebarLink href="/sync" icon="⇄" label="Sync" />
          <SidebarLink href="/audit" icon="🛡" label="Audit" />
          <SidebarLink href="/settings" icon="⚙" label="Settings" />
        </nav>
      </div>

      <div class="sidebar-footer">
        <button class="btn btn-ghost" onClick={lockNow} title="Lock vault">
          🔒 Lock
        </button>
      </div>
    </aside>
  );
}

function SidebarLink(props: { href: string; icon: string; label: string; end?: boolean }) {
  return (
    <A href={props.href} end={props.end} activeClass="active">
      <span class="ic">{props.icon}</span>
      <span class="lbl">{props.label}</span>
    </A>
  );
}

function SidebarLinkWithBadge(props: { href: string; icon: string; label: string }) {
  const [insights] = createResource(async () => {
    try { return await generateInsights(); }
    catch { return []; }
  });
  const warns = () => (insights() ?? []).filter((i) => i.severity === "warn").length;

  return (
    <A href={props.href} activeClass="active">
      <span class="ic">{props.icon}</span>
      <span class="lbl">{props.label}</span>
      <Show when={warns() > 0}>
        <span class="sidebar-badge">{warns()}</span>
      </Show>
    </A>
  );
}
