const spawn = require("child_process").spawn;

async function execute(binPath, args) {
  const cca = spawn(binPath, args);

  cca.on("error", error => {
    console.error("Could not execute create-comit-app:", error);
  });

  async function handleSignal(code) {
    cca.kill(code);
  }

  process.on("beforeExit", async code => {
    await handleSignal(code);
  });

  process.on("SIGINT", async code => {
    await handleSignal(code);
  });

  process.on("SIGTERM", async code => {
    await handleSignal(code);
  });

  cca.stdout.on("data", data => {
    process.stdout.write(data.toString());
  });

  cca.stderr.on("data", data => {
    process.stderr.write(data.toString());
  });

  cca.on("close", code => {
    process.exit(code);
  });
}

module.exports = { execute };
