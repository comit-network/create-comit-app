import os from "os";

export function makeArchiveName(bin: string, version: string): string {
  const kernel = os.type();
  const arch = os.arch();

  return `${bin}_${version}_${kernel}_${arch}.tar.gz`;
}
