/* @jsxImportSource solid-js */
import { Input } from "./Input.jsx";
import { Button } from "./Button.jsx";
import { Checkbox } from "./Checkbox.jsx";
import "@atrium-ui/elements/adaptive";
import "@atrium-ui/elements/form";
import { createSignal, type ParentProps } from "solid-js";

export function Form(
  props: ParentProps<{
    onSubmit: (data: FormData) => Promise<string | undefined> | string | undefined;
  }>,
) {
  const [error, setError] = createSignal<string>();
  const [success, setSuccess] = createSignal<string>();
  const [loading, setLoading] = createSignal(false);

  // TODO: store form data in sessionStorage until its successfully submitted
  const input = (e: InputEvent) => {
    // console.log(e);
  };

  const submit = async (e: Event) => {
    const form = e.currentTarget as HTMLFormElement;

    e.preventDefault();
    e.stopPropagation();

    setLoading(true);

    try {
      const data = new FormData(form);
      const res = await props.onSubmit?.(data);
      setError(undefined);
      setSuccess(res);
    } catch (err: any) {
      setError(err);
      console.error(err);

      // backpropagate errors to inputs
      // for (const error of errors) {
      //   for (const key in error) {
      //     const e = new CustomEvent('error', { bubbles: true, detail: { name: key, message: error[key] } })
      //     form.dispatchEvent(e)
      //   }
      // }
    } finally {
      setLoading(false);
    }
  };

  return (
    <div>
      {success() ? (
        <div>
          <h2>Success</h2>
          <p>{success()}</p>
        </div>
      ) : (
        <form onSubmit={submit} onInput={input}>
          <div class="flex flex-col gap-8">{props.children}</div>

          <Button disabled={!!loading()} type="submit" class="mt-4 overflow-hidden">
            <a-adaptive>
              <div class="flex items-center gap-2">
                <span>Submit</span>
                {loading() && <span class="loading-indicator flex-none" />}
              </div>
            </a-adaptive>
          </Button>
        </form>
      )}

      {error() && (
        <div class="text-red-600">
          <p>{error()}</p>
        </div>
      )}
    </div>
  );
}

function InputField(props: { type: string }) {
  switch (props.type) {
    case "text":
    case "name":
      return <Input {...props} />;
    case "email":
      return <Input {...props} type="email" />;
    case "textarea":
      return <Input {...props} multiline />;
    case "checkbox":
      return <Checkbox {...props} checked={props.value} />;
    case "date":
      return <Input {...props} type="date" />;
    default:
      return <Input {...props} />;
  }
}

type TextFieldProps = {};
type CheckboxFieldProps = {};
type DateFieldProps = {};
type TextareaFieldProps = {};
type EmailFieldProps = {};

export function FormField(props: {
  field: {
    type: "text" | "name" | "email" | "textarea" | "checkbox" | "date";
    description?: string;
    label?: string;
    error?: string;
    placeholder: string;
    name: string;
    required?: boolean;
    value: string | boolean | undefined;
  };
}) {
  return (
    <a-form-field>
      <div class={`form-field-${props.field.type}`}>
        {props.field.type === "text" && <Input />}

        <InputField
          {...props.field}
          label={
            !props.field.description
              ? `${props.field.label} ${props.field.required ? "" : "(optional)"}`
              : undefined
          }
          id={props.field.name}
          class={`form-field-input-${props.field.type}`}
        />

        {props.field.description ? (
          <div class="form-field-description">
            <label for={props.field.name}>{props.field.description}</label>
          </div>
        ) : null}
      </div>

      <div class="text-red-400 text-xs">
        <a-form-field-error />
      </div>
    </a-form-field>
  );
}
