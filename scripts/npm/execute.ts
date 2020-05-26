import { spawn } from "child_process";

export default async function execute(
  binPath: string,
  args: string[]
): Promise<void> {
  const cca = spawn(binPath, args);

  cca.on("error", (error) => {
    console.error("Could not execute create-comit-app:", error);
  });

  function handleSignal(code: NodeJS.Signals | number): void {
    cca.kill(code);
  }

  process.on("beforeExit", (code: number) => {
    handleSignal(code);
  });

  process.on("SIGINT", (code: NodeJS.Signals) => {
    handleSignal(code);
  });

  process.on("SIGTERM", (code: NodeJS.Signals) => {
    handleSignal(code);
  });

  cca.stdout.on("data", (data) => {
    process.stdout.write(data.toString());
  });

  cca.stderr.on("data", (data) => {
    process.stderr.write(data.toString());
  });

  cca.on("close", (code) => {
    process.exit(code);
  });
}
