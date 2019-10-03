const https = require('https');
const fs = require('fs');
const packageJson = require("./package");

const util = require('util');
const exec = util.promisify(require('child_process').exec);

async function getSystem() {
  const {stdout} = await exec("uname -s");
  return stdout.trim();
}

async function getArch() {
  const {stdout} = await exec("uname -m");
  return stdout.trim();
}


(async () => {
  const version = packageJson.version;
  const system = await getSystem();
  const arch = await getArch();
  if (!version || !system || !arch) {
    throw new Error("Could not retrieve needed information.");
  }
  const filename = `create-comit-app_${version}_${system}_${arch}.zip`;
  const url = `https://github.com/comit-network/create-comit-app/releases/download/${version}/${filename}`;
  console.log(url);
  const file = fs.createWriteStream(filename);

  https.get(url, (response) => {
    const {statusCode} = response;

    if (statusCode === 404) {
      throw new Error("404 error received, your system may not be supported or GitHub is down.")
    } else if (statusCode !== 200 && statusCode !== 302) {
      throw new Error('Download Failed.\n' +
        `Status Code: ${statusCode}`);
    }

    response.pipe(file);
  });
})();

