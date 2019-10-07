const fs = require("fs");
const packageJson = require("./package");
const util = require("util");
const exec = util.promisify(require("child_process").exec);
const unzipper = require("unzipper");
const axios = require("axios");

async function getSystem() {
  const { stdout } = await exec("uname -s");
  return stdout.trim();
}

async function getArch() {
  const { stdout } = await exec("uname -m");
  return stdout.trim();
}

async function unzip(filepath) {
  const directory = await unzipper.Open.file(filepath);
  return directory.extract({ path: process.cwd() });
}

(async () => {
  const version = /^\d\.\d\.\d/.exec(packageJson.version)[0];
  const system = await getSystem();
  const arch = await getArch();
  if (!version || !system || !arch) {
    throw new Error("Could not retrieve needed information.");
  }
  const binName = "create-comit-app";
  const zipFilename = `${binName}_${version}_${system}_${arch}.zip`;

  if (fs.existsSync(zipFilename)) {
    fs.unlinkSync(zipFilename);
  }

  if (fs.existsSync(binName)) {
    fs.unlinkSync(binName);
  }

  const url = `https://github.com/comit-network/create-comit-app/releases/download/${version}/${zipFilename}`;

  let response = await axios({
    url,
    method: "GET",
    responseType: "stream"
  });

  if (response.status === 302) {
    response = await axios({
      url: response.headers.location,
      method: "GET",
      responseType: "stream"
    });
  }

  const file = fs.createWriteStream(zipFilename);

  response.data.pipe(file);
  await new Promise((resolve, reject) => {
    response.data.on("end", () => {
      resolve();
    });

    response.data.on("error", err => {
      reject(err);
    });
  }).catch();

  await unzip(zipFilename);
  fs.unlinkSync(zipFilename);
})();

