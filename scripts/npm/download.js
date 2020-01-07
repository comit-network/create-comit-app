const fs = require("fs");
const util = require("util");
const extract = util.promisify(require("targz").decompress);
const axios = require("axios");
const path = require("path");
const makeArchiveName = require("./makeArchiveName");

module.exports = async function download(version, binTarget) {
  const targetDir = path.dirname(binTarget);

  const archiveName = makeArchiveName(version);
  const archivePath = targetDir + "/" + archiveName;

  if (!fs.existsSync(targetDir)) {
    fs.mkdirSync(targetDir, { recursive: true });
  }

  if (fs.existsSync(archivePath)) {
    fs.unlinkSync(archivePath);
  }

  const url = `https://github.com/comit-network/create-comit-app/releases/download/${version}/${archiveName}`;

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

  const file = fs.createWriteStream(archivePath);

  response.data.pipe(file);
  await new Promise((resolve, reject) => {
    response.data.on("end", () => {
      resolve();
    });

    response.data.on("error", err => {
      reject(err);
    });
  }).catch();

  await extract({
    src: archivePath,
    dest: targetDir
  });
  fs.unlinkSync(archivePath);
  fs.renameSync(targetDir + "/create-comit-app", binTarget);
  fs.chmodSync(binTarget, 755);
};
