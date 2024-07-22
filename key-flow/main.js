import { Flow } from 'flow-launcher-helper';
import { exec } from "child_process";
import { promisify } from "util";
import cp from "copy-paste";

const { showResult, on, run } = new Flow('app.png');

const preferences = {

};

async function execp(command, options) {
  const execp = promisify(exec);
  const output = await execp(command, {
    env: options?.env,
  });
  return output.stdout.trim();
}

async function execkey(command, args) {
  return await execp(
    `key.exe ${command} ${args.map((str) => `"${str}"`).join(" ")}`,
    { env: env() },
  );
}

function env() {
  return {
    KEY_DATABASE_URL: preferences.database_url,
    KEY_KEYFILE: preferences.keyfile_path,
    KEY_S3_ACCESS_KEY: preferences.s3_access_key,
    KEY_S3_SECRET_KEY: preferences.s3_secret,
    KEY_PASSWORD: preferences.password,
  };
}

on('query', async (params) => {
  const data = await execkey("list", ["--output", "json"]);
  let list = JSON.parse(data);

  showResult(...list.filter((entry) => {
    return entry.title.toLocaleLowerCase().includes(params[0].toLocaleLowerCase());
  }).map((entry) => {
    return {
      title: entry.title,
      subtitle: entry.user,
      method: "copy_to_clipboard",
      params: [entry.title],
      iconPath: "Images\\key.png",
    };
  }))
});

on('copy_to_clipboard', async (title) => {
  const data = await execkey("get", [title]);
  cp.copy(data);
  return;
});

run();
