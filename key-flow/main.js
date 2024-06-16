const { exec } = require("child_process");
const { promisify } = require("util");
const cp = require("copy-paste");

const { method, parameters } = JSON.parse(process.argv[2]);

const preferences = {};

function send(data) {
  console.log(JSON.stringify(data));
}

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

(async () => {
  switch (method) {
    case "query": {
      const data = await execkey("list", ["--output", "json"]);
      let list = JSON.parse(data);

      if (parameters.length === 0) {
        list = list.filter((entry) => {
          return entry.title.match(parameters[0]);
        });
      }

      return send({
        result: list.map((entry) => {
          return {
            Title: entry.title,
            Subtitle: entry.user,
            JsonRPCAction: {
              method: "copy_to_clipboard",
              parameters: [entry.title],
            },
            IcoPath: "Images\\key.png",
          };
        }),
      });
    }

    case "copy_to_clipboard": {
      const title = parameters[0];
      const data = await execkey("get", [title]);
      cp.copy(data);
      return;
    }
  }
})();
