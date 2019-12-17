#!/usr/bin/env node

const main = require("./main");
const path = require("path");

(async () => {
  let args = process.argv;
  args.shift(); // Remove arg "node"
  const dirPath = path.dirname(args[0]);
  args[0] = "new"; // Replace ".../cca.js" with "new"

  await main(dirPath, args);
})();
