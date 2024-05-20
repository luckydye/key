import { createSignal } from "solid-js";
import { invoke } from "@tauri-apps/api/core";

type Group = {
	type: "group";
	title: string;
	entires: Entry[];
};

type Entry = {
	type: "entry";
	title: string;
	user: string | undefined;
};

type Node = Group | Entry;

export function App() {
	const [list, setList] = createSignal<Node[]>([]);

	invoke("list").then((res) => {
		setList(JSON.parse(res as string));
	});

	return (
		<div class="container">
			<div>
				{list().map((node, i) => {
					return (
						<div key={`entry_${i}`}>
							<span>{node.title}</span>
							{node.type === "entry" && node.user ? (
								<span> ({node.user})</span>
							) : null}
						</div>
					);
				})}
			</div>
		</div>
	);
}
