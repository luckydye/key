import { twMerge } from "tailwind-merge";
import type { ParentProps } from "solid-js";

export const buttonVariants = {
  base: [
    "flex cursor-pointer items-center gap-2 leading-normal",
    "rounded-lg px-3 py-1 transition-all active:transition-none",
    "outline-none focus-visible:ring focus-visible:ring-[currentColor]",
  ],
  default: [
    "bg-[var(--button-color,#bfa188)]",
    "filter active:brightness-90 hover:brightness-110 active:contrast-125",
    "border border-zinc-700",
  ],
  outline: [
    "bg-transparent hover:bg-[rgba(150,150,150,0.1)]",
    "filter active:brightness-90 hover:brightness-110 active:contrast-125",
    "border border-zinc-700",
  ],
  ghost: [
    "bg-transparent active:bg-[rgba(150,150,150,0.1)]",
    "filter hover:brightness-110",
  ],
  disabled: ["cursor-not-allowed opacity-50"],
};

export function Button(
  props: ParentProps<{
    type?: "button" | "submit" | "reset";
    inert?: boolean;
    class?: string | string[];
    slot?: string;
    disabled?: boolean;
    autofocus?: boolean;
    variant?: keyof typeof buttonVariants;
    label?: string;
    onClick?: (e: MouseEvent) => void;
  }>,
) {
  return (
    <button
      type={props.type || "button"}
      inert={props.inert || undefined}
      // @ts-ignore
      slot={props.slot || undefined}
      autofocus={props.autofocus || undefined}
      aria-disabled={props.disabled || undefined}
      class={twMerge(
        buttonVariants.base,
        buttonVariants[props.variant ?? "default"],
        props.class,
        // the disabled attribute is not used for accessibility reasons
        props.disabled && buttonVariants.disabled,
      )}
      onMouseDown={(e) => {
        if (props.disabled || props.onClick) {
          e.preventDefault();
          e.stopImmediatePropagation();
        }
        if (props.onClick && !props.disabled) {
          props.onClick(e);
        }
      }}
      title={props.label}
      aria-label={props.label}
    >
      {props.children}
    </button>
  );
}

export function Link(
  props: ParentProps<{
    variant?: keyof Omit<typeof buttonVariants, "base">;
    href: string;
    target?: string;
  }>,
) {
  return (
    <a
      class={twMerge(
        "inline text-inherit no-underline",
        "transition-all active:transition-none",
        props.variant && buttonVariants.base,
        buttonVariants[props.variant ?? "default"],
      )}
      href={props.href}
      target={props.target}
    >
      {props.children}
    </a>
  );
}
