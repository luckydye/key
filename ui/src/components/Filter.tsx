/* @jsxImportSource solid-js */
import "@atrium-ui/elements/toggle";
import type { Toggle } from "@atrium-ui/elements/toggle";
import { createSignal, type ParentProps } from "solid-js";
import { Input } from "./Input";

export function Filter(
  props: ParentProps<{
    value: string;
    onSelect(item: string[]): void;
    onInput(str: string): void;
  }>,
) {
  const [toggle, setToggle] = createSignal<Toggle>();

  return (
    <div class="grid h-full grid-rows-[auto_1fr] overflow-hidden rounded-lg border border-zinc-700 bg-zinc-800 p-1">
      <Input
        onInput={(e) => props.onInput(e.currentTarget?.value)}
        class="w-full"
        placeholder="Type to filter..."
        value={props.value}
        onKeydown={(e: KeyboardEvent) => {
          switch (e.key) {
            case "ArrowUp":
              toggle()?.selectPrev();
              document.activeElement?.click();
              toggle()
                ?.querySelector("[data-selected]")
                ?.scrollIntoView({ block: "nearest" });
              e.preventDefault();
              e.stopPropagation();
              e.stopImmediatePropagation();
              e.currentTarget?.focus();
              break;
            case "ArrowDown":
              toggle()?.selectNext();
              document.activeElement?.click();
              toggle()
                ?.querySelector("[data-selected]")
                ?.scrollIntoView({ block: "nearest" });
              e.stopPropagation();
              e.stopImmediatePropagation();
              e.preventDefault();
              e.currentTarget?.focus();
              break;
          }
        }}
      />

      <a-toggle
        ref={setToggle}
        class="block h-full overflow-auto"
        active-attribute="data-selected"
        value={["0"]}
        oninput={(e: Event) => {
          const toggle = e.currentTarget as Toggle;
          props.onSelect(toggle.value);
        }}
      >
        {props.children}
      </a-toggle>
    </div>
  );
}

export function FilterItem(
  props: ParentProps<{ value?: string; onClick(): void }>,
) {
  return (
    <button
      type="button"
      value={props.value}
      class="flex w-full cursor-pointer items-center justify-start rounded-md p-1 active:bg-zinc-700 data-[selected]:bg-zinc-700 hover:bg-zinc-600 focus:ring"
      onClick={(e) => {
        e.currentTarget?.focus();
      }}
    >
      <div>{props.children}</div>
    </button>
  );
}
