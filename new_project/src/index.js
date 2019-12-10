const fs = require("fs");
const path = require("path");
const os = require("os");
const dotenv = require("dotenv");

const configPath = path.join(os.homedir(), ".create-comit-app", "env");
dotenv.config({path: configPath});

if (fs.existsSync(configPath)) {
  console.log("Environment configuration:");

  // Bitcoin HD keys with address funded during environment set-up.
  // Used to initialize Bitcoin wallets with funds.
  console.log("Bitcoin HD keys:");
  console.log("1. ", process.env.BITCOIN_HD_KEY_0);
  console.log("2. ", process.env.BITCOIN_HD_KEY_1);

  // The URI of the regtest Bitcoin node created during environment set-up.
  // Used to initialize Bitcoin wallets connected to the correct local network.
  console.log("Bitcoin node P2P URI: ", process.env.BITCOIN_P2P_URI);

  // Ethereum private keys with address funded during environment set-up.
  // Used to initialize Ethereum wallets with funds.
  console.log("Ethereum private keys:");
  console.log("1. ", process.env.ETHEREUM_KEY_0);
  console.log("2. ", process.env.ETHEREUM_KEY_1);

  // The URL for the HTTP API of the regtest Ethereum node created during environment set-up.
  // Used to initialize Ethereum wallets connected to the correct local network.
  console.log("Ethereum node HTTP URL: ", process.env.ETHEREUM_NODE_HTTP_URL);

  // The contract address of the ERC20 token deployed on the regtest network during environment set-up.
  // Used to identify the ERC20 token when performing swaps.
  console.log(
    "ERC20 token contract address: ",
    process.env.ERC20_CONTRACT_ADDRESS
  );

  // HTTP API URLs for the instances of cnd created during environment set-up.
  // Used to interact with said instances of cnd. In particular, it allows for
  // instantiating `ComitClient`s when using `comit-sdk`.
  console.log("cnd HTTP API URLs:");
  console.log("1. ", process.env.HTTP_URL_CND_0);
  console.log("2. ", process.env.HTTP_URL_CND_1);
} else {
  console.log(
    "Could not find file %s. Did you run `yarn start-env`?",
    configPath
  );
}
