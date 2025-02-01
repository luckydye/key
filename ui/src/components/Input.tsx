import { createSignal } from "solid-js";
import { Button } from "./Button";
import { Icon } from "./Icon";

export function Input(props: {
  class?: string;
  autofocus?: boolean;
  placeholder?: string;
  label?: string;
  name?: string;
  id?: string;
  value?: string;
  error?: string;
  required?: boolean;
  readonly?: boolean;
  password?: boolean;
  multiline?: boolean;
  onInvalid?: (e: Event) => undefined | string | Error;
  onInput?: (e: Event) => void;
  onChange?: (e: Event) => void;
  onKeydown?: (e: KeyboardEvent) => void;
}) {
  const [showPassword, setShowPassword] = createSignal(false);

  return (
    <div class={props.class}>
      <div class="flex items-center gap-2 py-1">
        <div class="text-sm">
          <label>{props.label}</label>
        </div>

        {props.multiline ? (
          <textarea
            autofocus={props.autofocus}
            id={props.id}
            name={props.name}
            readonly={props.readonly}
            required={props.required || undefined}
            placeholder={props.placeholder}
            value={props.value || ""}
            class={[
              "group w-full resize-y rounded-md border border-zinc-700 bg-transparent px-3 py-1 outline-none focus:border-zinc-500 hover:border-zinc-600",
              props.error ? "border-red-600" : "border-zinc-700",
            ].join("")}
            onChange={props.onChange}
            onInput={props.onInput}
            onInvalid={(e) => {
              const err = props.onInvalid?.(e);
              e.preventDefault();
            }}
          />
        ) : (
          <div class="relative w-full">
            <input
              autofocus={props.autofocus}
              type={props.password && !showPassword() ? "password" : "text"}
              id={props.id}
              name={props.name}
              readonly={props.readonly}
              required={props.required || undefined}
              placeholder={props.placeholder}
              value={props.value || ""}
              class={[
                "group w-full min-w-0 rounded-md border border-zinc-700 bg-transparent px-3 py-1 leading-normal outline-none focus:border-zinc-500 hover:border-zinc-600",
                props.error ? "border-red-600" : "border-zinc-700",
              ].join("")}
              onChange={props.onChange}
              onKeyDown={props.onKeydown}
              onInput={props.onInput}
              onInvalid={(e) => {
                const err = props.onInvalid?.(e);
                e.preventDefault();
              }}
            />

            {props.password && (
              <Button
                variant="ghost"
                class="absolute top-0 right-0 flex h-full w-[30px] items-center justify-center"
                onClick={() => setShowPassword(!showPassword())}
              >
                <Icon name="arrow-right" />
              </Button>
            )}
          </div>
        )}
      </div>

      {props.error ? (
        <div class="mt-1 text-red-600 text-sm">
          <label>{props.error}</label>
        </div>
      ) : null}
    </div>
  );
}
