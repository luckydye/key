import {
	ActionPanel,
	List,
	Action,
	Form,
	Keyboard,
	getPreferenceValues,
} from "@raycast/api";
import { useExec } from "@raycast/utils";
import { useEffect, useState } from "react";

type Group = {
	type: "group";
	uuid: string;
	title: string;
	entires: Entry[];
};

type Entry = {
	type: "entry";
	uuid: string;
	title: string;
	user: string | undefined;
	password: string | undefined;
};

type Preferences = {
	database_url: string;
	keyfile_path: string;
	password: string;
	s3_access_key: string;
	s3_secret: string;
};

const Shortcuts: Record<string, Keyboard.Shortcut> = {
	Copy: {
		modifiers: ["ctrl"],
		key: "c",
	},
	Paste: {
		modifiers: ["ctrl"],
		key: "p",
	},
};

const env = () => {
	const preferences = getPreferenceValues<Preferences>();
	return {
		KEY_DATABASE_URL: preferences.database_url,
		KEY_KEYFILE: preferences.keyfile_path,
		KEY_S3_ACCESS_KEY: preferences.s3_access_key,
		KEY_S3_SECRET_KEY: preferences.s3_secret,
		KEY_PASSWORD: preferences.password,
	};
};

export default function KeyCommand() {
  const preferences = getPreferenceValues<Preferences>();
	const cmd = useExec<string>(
		preferences.bin,
		["list", "--output", "json"],
		{ env: env() },
	);

	const [data, setData] = useState<(Entry | Group)[]>();

	useEffect(() => {
		if (cmd.data) setData(JSON.parse(cmd.data));
	}, [cmd.data]);

	return (
		<List isShowingDetail={true} isLoading={cmd.isLoading}>
			{data?.map((entry, i) => {
				if (entry.type === "entry")
					return (
						<List.Item
							key={`entry_${i}_${entry.uuid}`}
							icon="list-icon.png"
							title={entry.title || "Untitled"}
							subtitle={entry.user}
							detail={
								<List.Item.Detail
									markdown={`## ${entry.title} \n### ${entry.user}`}
								/>
							}
							actions={
								<ActionPanel>
									<Action.Push
										title="Details"
										target={
											<Form>
												<Form.TextField
													id="title"
													title="Title"
													placeholder="Title"
													value={entry.title || ""}
												/>
												<Form.TextField
													id="user"
													title="User"
													placeholder="User"
													value={entry.user || ""}
												/>
												<Form.PasswordField
													id="password"
													title="Password"
													placeholder="Password"
													value={entry.password || ""}
												/>
											</Form>
										}
									/>
									{entry.password && (
										<Action.CopyToClipboard
											title="Copy Password to clipboard"
											content={entry.password}
											shortcut={Shortcuts.Copy}
										/>
									)}
									{entry.password && (
										<Action.Paste
											title="Paste Password"
											content={entry.password}
											shortcut={Shortcuts.Paste}
										/>
									)}
								</ActionPanel>
							}
						/>
					);
			})}
		</List>
	);
}
