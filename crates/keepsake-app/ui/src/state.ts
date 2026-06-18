// Global UI state.  In v1 this is a simple signal-based store
// kept in module scope; it can be lifted into a context later
// if we add per-page state that needs to react to global
// changes.

import { createSignal } from "solid-js";
import type { StatusResponse } from "./api";
import { api } from "./api";

const [status, setStatus] = createSignal<StatusResponse>({
  unlocked: false,
  username: null,
});

const [vaultPath, setVaultPath] = createSignal<string>("");

const [users, setUsers] = createSignal<string[]>([]);

function persistedSignal<T extends string>(
  key: string,
  initial: T,
): [() => T, (v: T) => void] {
  let starting: T = initial;
  try {
    if (typeof localStorage !== "undefined") {
      const stored = localStorage.getItem(key);
      if (stored !== null) starting = stored as T;
    }
  } catch {
    // localStorage may be disabled (private mode, etc.) —
    // fall back to the initial value.
  }
  const [get, set] = createSignal<T>(starting, { equals: false });
  return [
    get,
    (v: T) => {
      set(() => v);
      try {
        if (typeof localStorage !== "undefined") {
          localStorage.setItem(key, v);
        }
      } catch {
        // ignore: best effort
      }
    },
  ];
}

const [syncUrl, setSyncUrl] = persistedSignal<string>("keepsake.syncUrl", "");
const [syncVaultId, setSyncVaultId] = persistedSignal<string>(
  "keepsake.syncVaultId",
  "",
);

const [toast, setToast] = createSignal<{ kind: "ok" | "err"; text: string } | null>(null);

export function showToast(kind: "ok" | "err", text: string) {
  setToast({ kind, text });
  setTimeout(() => setToast(null), 3500);
}

export async function refreshStatus() {
  try {
    setStatus(await api.status());
  } catch {
    setStatus({ unlocked: false, username: null });
  }
}

export async function refreshUsers() {
  try {
    const p = await api.defaultPath();
    setVaultPath(p);
    const u = await api.listUsers(p);
    setUsers(u);
  } catch {
    setUsers([]);
  }
}

export const state = {
  status,
  setStatus,
  vaultPath,
  setVaultPath,
  users,
  setUsers,
  syncUrl,
  setSyncUrl,
  syncVaultId,
  setSyncVaultId,
  toast,
  setToast,
};
