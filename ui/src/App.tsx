import * as key from "key";
import { createSignal, createEffect } from "solid-js";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { render } from "solid-js/web";
import "./app.css";
import { Filter, FilterItem } from "./components/Filter";
import { Input } from "./components/Input";
import { Form } from "./components/Form";

type Group = {
  type: "group";
  uuid: string;
  title: string;
  entires: Entry[];
};

type Entry = {
  type: "entry";
  uuid: string;
  title: string | undefined;
  user: string | undefined;
};

export function App() {
  const [list, setList] = createSignal<(Group | Entry)[]>([]);
  const [selected, setSelected] = createSignal<string>();
  const [detail, setDetail] = createSignal<Entry | Group>();
  const [password, setPassword] = createSignal<string>("x");
  const [filterValue, setFilterValue] = createSignal<string>("");

  createEffect(() => {
    const id = selected();
    if (!id) return;

    if (isTauri()) {
      invoke("entry", { name: id }).then((res) => {
        setDetail(res as Entry | Group);
      });
    }
  });

  if (isTauri()) {
    invoke("list").then((res) => {
      const list = JSON.parse(res as string);
      setList(list);

      if (!selected()) {
        setSelected(list[1].title);
      }
    });
  } else {
    key.greet();
  }

  return (
    <div>
      {password() ? (
        <div class="grid h-screen w-screen grid-cols-1 grid-rows-[1fr_auto] overflow-hidden">
          <Filter
            value={filterValue()}
            onInput={(str) => {
              setFilterValue(str);
            }}
            onSelect={(items) => {
              setSelected(items[0]);
            }}
          >
            {list()
              .filter((item) => {
                return item.title?.toLowerCase()?.match(filterValue().toLowerCase());
              })
              .map((node) => {
                return (
                  <FilterItem key={node.uuid} value={node.title}>
                    <span>{node.title}</span>
                  </FilterItem>
                );
              })}
          </Filter>

          <div>
            <div class="p-2">{selected()}</div>
            <pre>{JSON.stringify(detail(), null, "  ")}</pre>
          </div>
        </div>
      ) : (
        <div class="flex h-screen items-center justify-center">
          <Form
            onSubmit={async (data) => {
              const pw = data.get("password");
              console.log(pw);
              // setPassword(?.value);
            }}
          >
            <Input
              password
              autofocus
              name="password"
              label="Password"
              value={password()}
              placeholder="Password"
            />
          </Form>
        </div>
      )}
    </div>
  );
}

render(() => <App />, document.getElementById("root") as HTMLElement);
