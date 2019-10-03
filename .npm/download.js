const fs = require('fs');
const packageJson = require("./package");
const util = require('util');
const exec = util.promisify(require('child_process').exec);
const unzipper = require("unzipper");
const axios = require('axios');

async function getSystem() {
  const {stdout} = await exec("uname -s");
  return stdout.trim();
}

async function getArch() {
  const {stdout} = await exec("uname -m");
  return stdout.trim();
}

function unzip(filepath) {
  fs.createReadStream(filepath)
    .pipe(unzipper.Extract({path: './'}));
}

(async () => {
  const version = packageJson.version;
  const system = await getSystem();
  const arch = await getArch();
  if (!version || !system || !arch) {
    throw new Error("Could not retrieve needed information.");
  }
  const filename = `create-comit-app_${version}_${system}_${arch}.zip`;

  if (fs.existsSync(filename)) {
    fs.unlinkSync(filename);
  }

  const url = `https://github.com/comit-network/create-comit-app/releases/download/${version}/${filename}`;
  const file = fs.createWriteStream(filename);

  let response = await axios({
    url,
    method: 'GET',
    responseType: 'stream'
  });

  if (response.status === 302) {
    console.log(response.headers.location);
    response = await axios({
      url: response.headers.location,
      method: 'GET',
      responseType: 'stream'
    });
  }

  response.data.pipe(file);

  // unzip(filename);
})();

