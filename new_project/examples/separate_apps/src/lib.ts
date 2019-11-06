import { BitcoinWallet, Cnd, ComitClient, EthereumWallet } from "comit-sdk";
import fs from "fs";

export async function startClient(index: number): Promise<Actor> {
    const bitcoinWallet = await BitcoinWallet.newInstance(
        "regtest",
        process.env.BITCOIN_P2P_URI!,
        process.env[`BITCOIN_HD_KEY_${index}`]!
    );
    // Waiting for the Bitcoin wallet to read the balance
    await new Promise(r => setTimeout(r, 1000));

    const ethereumWallet = new EthereumWallet(
        process.env.ETHEREUM_NODE_HTTP_URL!,
        process.env[`ETHEREUM_KEY_${index}`]!
    );

    const cnd = new Cnd(process.env[`HTTP_URL_CND_${index}`]!);
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
