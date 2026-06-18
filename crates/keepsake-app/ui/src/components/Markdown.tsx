import { createMemo } from "solid-js";
import { renderMarkdown } from "../markdown";

/** Render a sanitized markdown block.
 * `titles` is an optional `{ uuid: display_title }` map used to
 * resolve `[[uuid]]` link markers into clickable chips. */
export function Markdown(props: {
  source: string;
  titles?: Record<string, string>;
}) {
  const html = createMemo(() => renderMarkdown(props.source, props.titles ?? {}));
  return <div class="markdown" innerHTML={html()} />;
}
