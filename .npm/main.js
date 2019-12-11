const packageJson = require("./package");
const fs = require("fs");
const { download } = require("./download");
const { execute } = require("./execute");

async function main(dirPath, args) {
  const ccaVersion = /^\d\.\d\.\d/.exec(packageJson.version)[0];
  const binPath = `${dirPath}/create-comit-app_${ccaVersion}/create-comit-app`;

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
}

module.exports = main;
