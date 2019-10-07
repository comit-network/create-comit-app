#!/usr/bin/env node

const fs = require("fs");
const packageJson = require("./package");
const { download } = require("./download");
const spawn = require("child_process").spawn;
const path = require("path");

async function execute(binPath, args) {
  const cca = spawn(binPath, args);

  cca.on("error", error => {
    console.error("Could not execute create-comit-app:", error);
  });

  async function handleSignal(code) {
    cca.kill(code);
  }

  process.on("beforeExit", async code => {
    await handleSignal(code);
  });

  process.on("SIGINT", async code => {
    await handleSignal(code);
  });

  process.on("SIGTERM", async code => {
    await handleSignal(code);
  });

  cca.stdout.on("data", data => {
    process.stdout.write(data.toString());
  });

  cca.stderr.on("data", data => {
    process.stderr.write(data.toString());
  });

  cca.on("close", code => {
    process.exit(code);
  });
}

(async () => {
  let args = process.argv;
  args.shift(); // node
  const dirPath = path.dirname(args[0]);
  args.shift(); // .../cca.js

  const ccaVersion = /^\d\.\d\.\d/.exec(packageJson.version)[0];
  const binPath = `${dirPath}/bin/${ccaVersion}/create-comit-app`;

  try {
    if (!fs.existsSync(binPath)) {
      process.stdout.write(
        `First time execution, downloading create-comit-app ${ccaVersion}...`
      );
      await download(ccaVersion, binPath);
      console.log("✓");
    }

    await execute(binPath, args);
  } catch (error) {
    console.error("Issue encountered:", error);
  }
})();
