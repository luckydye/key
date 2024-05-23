import * as key from "key";
import { createSignal, createEffect } from "solid-js";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { render } from "solid-js/web";
import "./app.css";

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
		<div class="grid grid-cols-2 h-screen w-screen overflow-hidden">
			<div class="p-2 h-full overflow-auto">
				{list().map((node, i) => {
					return (
						<div key={`entry_${i}`}>
							<button
								type="button"
								onClick={() => {
									setSelected(node.title);
								}}
							>
								<span>{node.title}</span>
							</button>
						</div>
					);
				})}
			</div>

			<div>
				<div class="p-2">{selected()}</div>

				<pre>{JSON.stringify(detail(), null, "  ")}</pre>
			</div>
		</div>
	);
}

render(() => <App />, document.getElementById("root") as HTMLElement);
