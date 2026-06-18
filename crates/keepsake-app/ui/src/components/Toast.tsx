import { JSX } from "solid-js";

export function Toast(props: { kind: "ok" | "err"; text: string }): JSX.Element {
  return (
    <div class={`toast ${props.kind}`}>
      {props.text}
    </div>
  );
}
