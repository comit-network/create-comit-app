import os from "os";

export default function makeArchiveName(version: string): string {
  const kernel = os.type();
  const arch = os.arch();

  return `create-comit-app_${version}_${kernel}_${arch}.tar.gz`;
}
