// Wires the static `[[uuid]]` link chips rendered by
// `markdown.ts` into Solid Router's client-side navigation.
//
// The chips are produced as raw HTML inside `innerHTML`, so they
// can't be Solid components themselves.  This module installs a
// single document-level click handler that catches clicks on
// `a.keepsake-link`, prevents the default browser navigation, and
// delegates to a callback that calls Solid Router's `navigate`.
//
// We don't import `@solidjs/router` here directly — the actual
// `navigate` function is injected by `App.tsx` (or whoever owns
// the Router instance) via `installLinkClickHandler`.  Keeping
// the import out of this file avoids a circular dep with the
// router context and makes the module tree-shakable.

let installed = false;
type NavigateFn = (path: string) => void;
let navigateFn: NavigateFn | null = null;

/** Install the document click handler once.  `navigate` is
 * called for every `a.keepsake-link` click. */
export function installLinkClickHandler(navigate: NavigateFn): void {
  if (installed) return;
  installed = true;
  navigateFn = navigate;
  document.addEventListener("click", (ev) => {
    const target = ev.target;
    if (!(target instanceof Element)) return;
    const a = target.closest("a.keepsake-link");
    if (!a) return;
    const href = a.getAttribute("href");
    const uuid = a.getAttribute("data-uuid");
    if (!href || !uuid) return;
    ev.preventDefault();
    ev.stopPropagation();
    // Use the href so we get the right path even if data-uuid
    // is missing in some future render.  Strip any leading '#'.
    const path = href.replace(/^#/, "");
    navigateFn?.(path);
  });
}
