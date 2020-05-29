#!/usr/bin/env node

import { execute } from "common";
import { download } from "common";
import fs from "fs";
import path from "path";
import packageJson from "./package.json";

async function main(dirPath: string, args: string[]): Promise<void> {
  // Sometimes the only way to test is to publish on npmjs.com, In this case,
  // it is easiest to change the version in package.json to X.Y.ZrcN,
  // with X.Y.Z the version of create-comit-app. The regex below removes
  // the "rcN" suffix to download the binary.
  const ccaVersion = /^\d\.\d\.\d/.exec(packageJson.version)![0];
  const binPath = `${dirPath}/create-comit-app_${ccaVersion}/create-comit-app`;

  try {
    if (!fs.existsSync(binPath)) {
      process.stdout.write(
        `First time execution, downloading create-comit-app ${ccaVersion}...`
      );
      await download("create-comit-app", ccaVersion, binPath);
      console.log("✓");
    }

    await execute(binPath, args);
  } catch (error) {
    console.error("Issue encountered:", error);
  }
}

(async () => {
  const args = process.argv;
  args.shift(); // Remove arg "node"
  const dirPath = path.dirname(args[0]);
  args.shift(); // Remove arg ".../main.js"

  await main(dirPath, args);
})();
