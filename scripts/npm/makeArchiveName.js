const os = require("os");

module.exports = function makeArchiveName(version) {
  const kernel = os.type();
  const arch = os.arch();

  return `comit-scripts_${version}_${kernel}_${arch}.tar.gz`;
};
