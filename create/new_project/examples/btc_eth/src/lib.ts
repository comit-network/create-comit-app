import {
    Actor,
    createActor as createActorSdk,
    EthereumWallet,
    InMemoryBitcoinWallet,
} from "comit-sdk";
import dotenv from "dotenv";
import fs from "fs";
import * as os from "os";
import * as path from "path";

export async function createActor(index: number): Promise<Actor> {
    loadEnvironment();

    const bitcoinWallet = await InMemoryBitcoinWallet.newInstance(
        "regtest",
        process.env.BITCOIN_P2P_URI!,
        process.env[`BITCOIN_HD_KEY_${index}`]!
    );
    // Waiting for the Bitcoin wallet to read the balance
    await new Promise((r) => setTimeout(r, 1000));

    const ethereumWallet = new EthereumWallet(
        process.env.ETHEREUM_NODE_HTTP_URL!,
        process.env[`ETHEREUM_KEY_${index}`]!
    );

    return await createActorSdk(
        bitcoinWallet,
        ethereumWallet,
        process.env[`HTTP_URL_CND_${index}`]!
    );
}

function loadEnvironment() {
    const envFilePath = path.join(os.homedir(), ".create-comit-app", "env");

    if (!fs.existsSync(envFilePath)) {
        console.log(
            "Could not find file %s. Did you run `yarn start-env`?",
            envFilePath
        );
        process.exit(1);
    }

    dotenv.config({ path: envFilePath });
}

export async function sleep(ms: number) {
    return new Promise((resolve) => setTimeout(resolve, ms));
}
