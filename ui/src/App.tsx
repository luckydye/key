import { createSignal } from "solid-js";
import { invoke } from "@tauri-apps/api/core";

export function App() {
  const [list, setList] = createSignal([]);

  return (
    <div class="container">
      <form
        class="row"
        onSubmit={async (e) => {
          e.preventDefault();
          setList(await invoke("list"));
        }}
      >
        <button type="submit">list</button>
      </form>

      <p>{list()}</p>
    </div>
  );
}
