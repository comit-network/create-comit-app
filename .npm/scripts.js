#!/usr/bin/env node

const main = require("./main");
const path = require("path");

(async () => {
  let args = process.argv;
  args.shift(); // Remove arg "node"
  const dirPath = path.dirname(args[0]);
  args.shift(); // Remove arg ".../cca.js"

  await main(dirPath, args);
})();
