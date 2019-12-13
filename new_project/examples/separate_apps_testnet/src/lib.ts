import { BitcoinWallet, Cnd, ComitClient, EthereumWallet } from "comit-sdk";
import { formatEther } from "ethers/utils";
import fs from "fs";
import { TestnetBitcoinWallet } from "./bcoinWallet";

export async function startClient(
    actor: string,
    portInc: number
): Promise<Actor> {
    const bitcoinWallet = await TestnetBitcoinWallet.newInstance(
        "testnet",
        process.env[`BITCOIN_HD_KEY_${actor}`]!,
        actor,
        portInc
    );

    // Waiting for the Bitcoin wallet to read the balance
    await new Promise(r => setTimeout(r, 1000));

    const ethereumWallet = new EthereumWallet(
        process.env.ETHEREUM_NODE_HTTP_URL!,
        process.env[`ETHEREUM_KEY_${actor}`]!
    );

    const cnd = new Cnd(process.env[`HTTP_URL_CND_${actor}`]!);
    const peerId = await cnd.getPeerId();
    const addressHint = await cnd
        .getPeerListenAddresses()
        .then(addresses => addresses[0]);

    const comitClient = new ComitClient(bitcoinWallet, ethereumWallet, cnd);

    return {
        comitClient,
        peerId,
        addressHint,
        bitcoinWallet,
        ethereumWallet,
    };
}

export interface Actor {
    comitClient: ComitClient;
    peerId: string;
    addressHint: string;
    bitcoinWallet: BitcoinWallet;
    ethereumWallet: EthereumWallet;
}

export function checkEnvFile(path: string) {
    if (!fs.existsSync(path)) {
        console.log(
            "Could not find %s file. Did you run \\`create-comit-app start-env\\`?",
            path
        );
        process.exit(1);
    }
}

export async function sleep(ms: number) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

export async function printBalance(actor: Actor, name: string) {
    console.log(
        `[${name}] Bitcoin balance: %f, Ether balance: %f`,
        await actor.bitcoinWallet.getBalance(),
        parseFloat(
            formatEther((await actor.ethereumWallet.getBalance()).toString())
        ).toFixed(2)
    );
}
