#!/usr/bin/env node

const fs = require("fs");
const packageJson = require("./package");
const { download } = require("./download");
const util = require("util");
const spawn = require("child_process").spawn;

async function execute(binPath, args) {
  const cca = spawn(binPath, args);

  cca.on("error", function(error) {
    console.error("Could not execute create-comit-app:", error);
  });

  cca.stdout.on("data", function(data) {
    console.log(data.toString());
  });

  cca.stderr.on("data", function(data) {
    console.error(data.toString());
  });

  cca.on("exit", function(code) {
    process.exit(code);
  });
}

(async () => {
  const ccaVersion = /^\d\.\d\.\d/.exec(packageJson.version)[0];
  const binPath = `./create-comit-app_${ccaVersion}`;

  try {
    if (!fs.existsSync(binPath)) {
      await download(ccaVersion);
    }

    await execute(binPath, []);
  } catch (error) {
    console.error("Issue encountered:", error);
  }
})();
