import { spawn } from "child_process";

export async function execute(binPath: string, args: string[]): Promise<void> {
  const cca = spawn(binPath, args);

  cca.on("error", (error) => {
    console.error("Could not execute create-comit-app:", error);
  });

  function handleSignal(code: number | NodeJS.Signals): void {
    cca.kill(code);
  }
np
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
