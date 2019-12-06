const os = require("os");

exports.default = function makeArchiveName(version) {
  const kernel = os.type();
  const arch = os.arch();

  return `create-comit-app_${version}_${kernel}_${arch}.tar.gz`;
};
