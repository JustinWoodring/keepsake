// Markdown rendering with HTML sanitization.  Notes are
// rendered as markdown; everything else is plain text.
//
// Cross-record links via `[[uuid]]` are converted to clickable
// `<a class="keepsake-link" href="#/r/{uuid}">{title}</a>`
// before being handed to marked.  Resolution uses the title
// table passed in by the caller; missing/unknown ids are
// rendered as plain text so nothing is lost.

import DOMPurify from "dompurify";
import { marked } from "marked";

marked.setOptions({
  gfm: true,
  breaks: true,
});

const UUID_RE = /^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$/;

/** Replace `[[uuid]]` markers in `src` with anchor tags. */
export function expandLinks(
  src: string,
  titles: Record<string, string>,
): string {
  return src.replace(/\[\[([^\]]+)\]\]/g, (whole, inner: string) => {
    const trimmed = inner.trim();
    if (!UUID_RE.test(trimmed)) return whole;
    const title = titles[trimmed];
    if (title) {
      const safeTitle = escapeAttr(title);
      return `<a class="keepsake-link" href="/r/${trimmed}" data-uuid="${trimmed}">[[${safeTitle}]]</a>`;
    }
    return `<a class="keepsake-link keepsake-link--missing" href="/r/${trimmed}" data-uuid="${trimmed}">[[${trimmed} ??]]</a>`;
  });
}

function escapeAttr(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/"/g, "&quot;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

/** Render a markdown string to sanitized HTML with `[[id]]`
 * markers resolved against the given title table. */
export function renderMarkdown(
  src: string,
  titles: Record<string, string> = {},
): string {
  if (!src) return "";
  const expanded = expandLinks(src, titles);
  const html = marked.parse(expanded, { async: false }) as string;
  return DOMPurify.sanitize(html, {
    USE_PROFILES: { html: true },
    ADD_ATTR: ["data-uuid", "class"],
  });
}
