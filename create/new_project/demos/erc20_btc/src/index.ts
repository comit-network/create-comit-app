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
import readLineSync from "readline-sync";
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
        "Swap started! Swapping %d tokens @ contract address %s for %d %s",
        toNominal(swapMessage.alpha_asset.quantity, 18),
        swapMessage.alpha_asset.token_contract,
        toBitcoin(swapMessage.beta_asset.quantity),
        swapMessage.beta_asset.name
    );

    wait_for_confirmation("Continue?");

    console.log(
        "Ethereum HTLC deployed! TXID: ",
        await takerSwapHandle.deploy(tryParams)
    );

    wait_for_confirmation("Continue?");

    console.log(
        "Ethereum HTLC funded! TXID: ",
        await takerSwapHandle.fund(tryParams)
    );

    wait_for_confirmation("Continue?");

    console.log(
        "Bitcoin HTLC funded! TXID: ",
        await makerSwapHandle.fund(tryParams)
    );

    wait_for_confirmation("Continue?");

    console.log(
        "Bitcoin HTLC redeemed! TXID: ",
        await takerSwapHandle.redeem(tryParams)
    );

    wait_for_confirmation("Continue?");

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
    const refundAddress = taker.ethereumWallet.getAccount();

    return {
        alpha_ledger: {
            name: "ethereum",
            chain_id: 17,
        },
        beta_ledger: {
            name: "bitcoin",
            network: "regtest",
        },
        alpha_asset: {
            name: "erc20",
            token_contract: process.env.ERC20_CONTRACT_ADDRESS,
            quantity: "10000000000000000000",
        },
        beta_asset: {
            name: "bitcoin",
            quantity: toSatoshi(1).toString(),
        },
        alpha_ledger_refund_identity: refundAddress,
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
    // Wait a second to let the Ethereum wallet catch up
    await new Promise((r) => setTimeout(r, 1000));

    const bitcoinBalance = await actor.bitcoinWallet.getBalance();
    const tokenAmount = await actor.ethereumWallet.getErc20Balance(
        process.env.ERC20_CONTRACT_ADDRESS!
    );
    console.log(
        "%s Bitcoin balance: %d. Erc20 Token balance: %d",
        actor.name,
        bitcoinBalance.toFixed(2),
        tokenAmount
    );
}

function toNominal(tokenWei: string, decimals: number) {
    return new BigNumber(tokenWei).div(new BigNumber(10).pow(decimals));
}

function wait_for_confirmation(text: string) {
    if (process.env.NON_INTERACTIVE && process.env.NON_INTERACTIVE === "true") {
        return;
    }
    readLineSync.question(text);
}
