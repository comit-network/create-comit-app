import {
    Actor,
    BigNumber,
    createActor as createActorSdk,
    EthereumWallet,
    InMemoryBitcoinWallet,
    SwapRequest,
} from "comit-sdk";
import dotenv from "dotenv";
import fs from "fs";
import moment from "moment";
import * as os from "os";
import * as path from "path";
import { toBitcoin, toSatoshi } from "satoshi-bitcoin-ts";

(async function main() {
    loadEnvironment();

    const maker = await createActor(0, "Maker");
    const taker = await createActor(1, "Taker");

    console.log("Maker Ethereum address: ", maker.ethereumWallet.getAccount());

    console.log("Taker Ethereum address: ", taker.ethereumWallet.getAccount());

    await printBalances(maker);
    await printBalances(taker);

    const swapMessage = createSwap(maker, taker);

    const takerSwapHandle = await taker.comitClient.sendSwap(swapMessage);
    await new Promise((r) => setTimeout(r, 1000));
    const makerSwapHandle = await maker.comitClient
        .getNewSwaps()
        .then((swaps) => swaps[0]);

    const tryParams = { maxTimeoutSecs: 10, tryIntervalSecs: 1 };
    await makerSwapHandle.accept(tryParams);

    console.log(
        "Swap started! Swapping %d %s for %d %s",
        toBitcoin(swapMessage.alpha_asset.quantity),
        swapMessage.alpha_asset.name,
        toNominal(swapMessage.beta_asset.quantity, 18),
        swapMessage.beta_asset.name
    );

    console.log(
        "Ethereum HTLC funded! TXID: ",
        await takerSwapHandle.fund(tryParams)
    );

    console.log(
        "Bitcoin HTLC funded! TXID: ",
        await makerSwapHandle.fund(tryParams)
    );

    console.log(
        "Bitcoin HTLC redeemed! TXID: ",
        await takerSwapHandle.redeem(tryParams)
    );

    console.log(
        "Ethereum HTLC redeemed! TXID: ",
        await makerSwapHandle.redeem(tryParams)
    );

    console.log("Swapped!");
    await printBalances(maker);
    await printBalances(taker);
    process.exit();
})();

async function createActor(index: number, name: string): Promise<Actor> {
    const bitcoinWallet = await InMemoryBitcoinWallet.newInstance(
        "regtest",
        process.env.BITCOIN_P2P_URI!,
        process.env[`BITCOIN_HD_KEY_${index}`]!
    );
    await new Promise((r) => setTimeout(r, 1000));

    const ethereumWallet = new EthereumWallet(
        process.env.ETHEREUM_NODE_HTTP_URL!,
        process.env[`ETHEREUM_KEY_${index}`]!
    );

    return createActorSdk(
        bitcoinWallet,
        ethereumWallet,
        process.env[`HTTP_URL_CND_${index}`]!,
        name
    );
}

function createSwap(maker: Actor, taker: Actor): SwapRequest {
    const to = maker.peerId;
    const redeemAddress = taker.ethereumWallet.getAccount();

    return {
        alpha_ledger: {
            name: "bitcoin",
            network: "regtest",
        },
        beta_ledger: {
            name: "ethereum",
            chain_id: 17,
        },
        alpha_asset: {
            name: "bitcoin",
            quantity: toSatoshi(0.1).toString(),
        },
        beta_asset: {
            name: "ether",
            quantity: "5000000000000000000",
        },
        beta_ledger_redeem_identity: redeemAddress,
        alpha_expiry: moment().unix() + 7200,
        beta_expiry: moment().unix() + 3600,
        peer: {
            peer_id: to,
            address_hint: maker.addressHint,
        },
    };
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

async function printBalances(actor: Actor) {
    const tokenWei = await actor.ethereumWallet.getBalance();
    const bitcoinBalance = await actor.bitcoinWallet.getBalance();
    console.log(
        "%s Bitcoin balance: %d. Ether balance: %d",
        actor.name,
        bitcoinBalance.toFixed(2),
        toNominal(tokenWei.toString(), 18)
    );
}

function toNominal(tokenWei: string, decimals: number) {
    return new BigNumber(tokenWei).div(new BigNumber(10).pow(decimals));
}
