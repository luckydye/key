import * as key from "key";
import { createSignal, createEffect } from "solid-js";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { render } from "solid-js/web";
import "./App.css";
import { Filter, FilterItem } from "./components/Filter";
import { Input } from "./components/Input";
import { Form } from "./components/Form";
import { error, info } from "./logger";

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
  password: string | undefined;
};

function ListView() {
  const [list, setList] = createSignal<(Group | Entry)[]>([]);
  const [selected, setSelected] = createSignal<string>();
  const [detail, setDetail] = createSignal<Entry | Group>();
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
      <EntryView title={selected()} data={detail()} />
    </div>
  );
}

function EntryView(props: { title?: string; data?: Entry | Group }) {
  return (
    <div>
      <div class="p-2 px-4 text-lg">{props.title}</div>

      {props.data && (
        <div class="p-2 px-4 pb-4">
          <Input label="User" placeholder="User" value={props.data.user} />
          <Input
            label="Password"
            password
            placeholder="Password"
            value={props.data?.password}
          />
          <Input label="Website" placeholder="Website" value={props.data.website} />
          <Input multiline label="Notes" placeholder="Notes" value={props.data.notes} />
        </div>
      )}
    </div>
  );
}

function App() {
  const [unlocked, setUnlocked] = createSignal<boolean>(false);

  return (
    <div>
      {unlocked() ? (
        <ListView />
      ) : (
        <div class="flex h-screen items-center justify-center">
          <Form
            onSubmit={async (data) => {
              const pw = data.get("password");
              info("pw", pw);

              if (pw) {
                await invoke("unlock", { password: pw }).then((res) => {
                  info("res", res);
                });
                setUnlocked(true);
                return "Unlocked";
              }

              throw "failed to unlock";
            }}
          >
            <Input
              password
              autofocus
              name="password"
              label="Password"
              placeholder="Password"
              value=""
            />
          </Form>
        </div>
      )}
    </div>
  );
}

render(() => <App />, document.getElementById("root") as HTMLElement);
