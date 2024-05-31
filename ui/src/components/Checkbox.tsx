/* @jsxImportSource solid-js */
import { Icon } from "./Icon";
import { type ParentProps, createEffect, createSignal } from "solid-js";
import { twMerge } from "tailwind-merge";

export type CheckboxProps = {
  id: string;
  checked?: boolean;
  onChange?: (event: Event) => void;
};

export function Checkbox(props: ParentProps<CheckboxProps>) {
  const [checked, setChecked] = createSignal(props.checked);
  const [input, setInput] = createSignal<HTMLInputElement>();

  createEffect(() => {
    setChecked(props.checked);
  });

  const handleChange = (value: boolean) => {
    setChecked(value);

    const inputElement = input();
    if (inputElement) {
      inputElement.checked = value;
      inputElement.dispatchEvent(new Event("change", { bubbles: true }));
    }
  };

  return (
    <div class="flex items-start gap-3">
      <button
        role="checkbox"
        aria-checked={checked()}
        type="button"
        aria-labelledby={`label_${props.id}`}
        onClick={() => handleChange(!checked())}
        class="mt-[2px] h-6 w-6 cursor-pointer rounded-md border border-zinc-700 bg-transparent p-0 align-bottom hover:border-zinc-600"
      >
        <div
          aria-hidden="true"
          class={twMerge("flex items-center justify-center", !checked() && "hidden")}
        >
          <Icon name="check" />
        </div>
      </button>

      <input
        ref={setInput}
        type="checkbox"
        class="hidden"
        id={`input_${props.id}`}
        name={props.id}
        checked={checked() || undefined}
        onInput={(e: Event) => handleChange((e.target as HTMLInputElement).checked)}
      />

      <label
        id={`label_${props.id}`}
        for={`input_${props.id}`}
        class="cursor-pointer text-lg"
      >
        {props.children}
      </label>
    </div>
  );
}
