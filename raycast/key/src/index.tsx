import {
	ActionPanel,
	List,
	Action,
	Form,
	Keyboard,
	getPreferenceValues,
	showHUD,
	PopToRootType,
	Clipboard,
} from "@raycast/api";
import { exec } from "child_process";
import { promisify } from "util";
import { useExec, getFavicon } from "@raycast/utils";
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
	website: string | undefined;
	has_otp: boolean;
};

type Preferences = {
	bin: string;
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
	OTP: {
		modifiers: ["ctrl"],
		key: "o",
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

const execp = async (
	command: string,
	options?: { env: NodeJS.ProcessEnv | undefined },
): Promise<string> => {
	const execp = promisify(exec);
	const output = await execp(command, {
		env: options?.env,
	});
	return output.stdout.trim();
};

const execkey = async (command: string, args: string[]) => {
	const preferences = getPreferenceValues<Preferences>();
	return await execp(
		`${preferences.bin} ${command} ${args.map((str) => `"${str}"`).join(" ")}`,
		{ env: env() },
	);
};

const copyOtpToClipboard = async (entry: Entry) => {
	const otp = await execkey("otp", [entry.title]);
	await Clipboard.copy(otp);
	await showHUD("Copied to clipboard", {
		clearRootSearch: true,
		popToRootType: PopToRootType.Immediate,
	});
};

const copyPasswordToClipboard = async (entry: Entry) => {
	const pw = await execkey("get", [entry.title]);
	await Clipboard.copy(pw);
	await showHUD("Copied to clipboard", {
		clearRootSearch: true,
		popToRootType: PopToRootType.Immediate,
	});
};

const pastePassword = async (entry: Entry) => {
	const pw = await execkey("get", [entry.title]);
	await Clipboard.paste(pw);
	await showHUD("Pasted Password", {
		clearRootSearch: true,
		popToRootType: PopToRootType.Immediate,
	});
};

export default function KeyCommand() {
	const preferences = getPreferenceValues<Preferences>();
	const cmd = useExec<string>(preferences.bin, ["list", "--output", "json"], {
		env: env(),
	});

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
							icon={entry.website ? getFavicon(entry.website) : "key.png"}
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
										target={<EditForm entry={entry} />}
									/>
									<Action
										icon="clipboard.svg"
										title="Copy Password to clipboard"
										onAction={() => copyPasswordToClipboard(entry)}
										shortcut={Shortcuts.Copy}
									/>
									<Action
										icon="clipboard.svg"
										title="Paste Password"
										onAction={() => pastePassword(entry)}
										shortcut={Shortcuts.Paste}
									/>
									{entry.has_otp && (
										<Action
											icon="otp.png"
											title="Copy One Time Password"
											onAction={() => copyOtpToClipboard(entry)}
											shortcut={Shortcuts.OTP}
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

function EditForm({ entry }: { entry: Entry }) {
  return (
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
			<Form.TextField
				id="website"
				title="Website"
				placeholder="Website"
				value={entry.website}
			/>
			<Form.TextArea
				id="notes"
				title="Notes"
				placeholder="Notes"
				value={entry.notes}
			/>
			<Form.PasswordField
				id="password"
				title="Password"
				placeholder="Password"
				value={entry.password || ""}
			/>
		</Form>
  )
}
