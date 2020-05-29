import { makeArchiveName } from "common";

const version = process.argv[2];

if (!version) {
  console.error("Please pass the version as the first argument.");
  process.exit(1);
}

const archiveName = makeArchiveName("comit-scripts", version);

console.log(archiveName);
