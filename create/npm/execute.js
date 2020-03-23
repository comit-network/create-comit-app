const spawn = require("child_process").spawn;

module.exports = async function execute(binPath, args) {
  const cca = spawn(binPath, args);

  cca.on("error", (error) => {
    console.error("Could not execute create-comit-app:", error);
  });

  function handleSignal(code) {
    cca.kill(code);
  }

  process.on("beforeExit", (code) => {
    handleSignal(code);
  });

  process.on("SIGINT", (code) => {
    handleSignal(code);
  });

  process.on("SIGTERM", (code) => {
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
};
