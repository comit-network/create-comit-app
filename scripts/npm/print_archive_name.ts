import makeArchiveName from "./make_archive_name";

const version = process.argv[2];

if (!version) {
  console.error("Please pass the version as the first argument.");
  process.exit(1);
}

const archiveName = makeArchiveName(version);

console.log(archiveName);
