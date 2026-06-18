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

const [syncUrl, setSyncUrl] = createSignal<string>("");

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
  toast,
  setToast,
};
