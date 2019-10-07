const fs = require("fs");
const util = require("util");
const exec = util.promisify(require("child_process").exec);
const unzipper = require("unzipper");
const axios = require("axios");
const path = require("path");

async function getSystem() {
  const { stdout } = await exec("uname -s");
  return stdout.trim();
}

async function getArch() {
  const { stdout } = await exec("uname -m");
  return stdout.trim();
}

async function unzip(zipPath, binName) {
  const directory = await unzipper.Open.file(zipPath);
  return directory.extract({ path: process.cwd() });
}

async function download(version, binPath) {
  const system = await getSystem();
  const arch = await getArch();
  if (!version || !system || !arch) {
    throw new Error("Could not retrieve needed information.");
  }
  const zipFilename = `create-comit-app_${version}_${system}_${arch}.zip`;

  if (!fs.existsSync(path.dirname(binPath))) {
    fs.mkdirSync(path.dirname(binPath));
  }

  if (fs.existsSync(zipFilename)) {
    fs.unlinkSync(zipFilename);
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

  await unzip(zipFilename, binPath);
  fs.unlinkSync(zipFilename);
  fs.renameSync("create-comit-app", binPath);
  fs.chmodSync(binPath, 755);
}

module.exports = { download };
