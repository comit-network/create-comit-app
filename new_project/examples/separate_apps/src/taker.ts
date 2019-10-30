import { SwapRequest } from "comit-sdk";
import { formatEther } from "ethers/utils";
import moment from "moment";
import readLineSync from "readline-sync";
import { toBitcoin, toSatoshi } from "satoshi-bitcoin-ts";
import { Actor, checkEnvFile, startClient } from "./lib";

(async function main() {
    checkEnvFile(process.env.DOTENV_CONFIG_PATH!);

    const taker = await startClient(1);

    console.log(
        "[Taker] Bitcoin balance: %f. Ether balance: %f",
        parseFloat(await taker.bitcoinWallet.getBalance()).toFixed(2),
        parseFloat(
            formatEther(await taker.ethereumWallet.getBalance())
        ).toFixed(2)
    );

    const peerId = readLineSync.question("What is the Maker's peer id?");
    const addressHint = readLineSync.question(
        "What is the Maker's address hint?"
    );

    const swapMessage = createSwap(taker, peerId, addressHint);

    const swapHandle = await taker.comitClient.sendSwap(swapMessage);

    const actionConfig = { timeout: 100000, tryInterval: 1000 };

    console.log(
        "Swap started! Swapping %d Ether for %d %s",
        formatEther(swapMessage.alpha_asset.quantity),
        toBitcoin(swapMessage.beta_asset.quantity),
        swapMessage.beta_asset.name
    );

    readLineSync.question("1. Continue?");

    console.log(
        "Ethereum HTLC funded! TXID: ",
        await swapHandle.fund(actionConfig)
    );

    readLineSync.question("3. Continue?");

    console.log(
        "Bitcoin redeemed! TXID: ",
        await swapHandle.redeem(actionConfig)
    );

    console.log("Swapped!");
    console.log(
        "[Taker] Bitcoin balance: %f. Ether balance: %f",
        parseFloat(await taker.bitcoinWallet.getBalance()).toFixed(2),
        parseFloat(
            formatEther(await taker.ethereumWallet.getBalance())
        ).toFixed(2)
    );
    process.exit();
})();

function createSwap(
    actor: Actor,
    peerId: string,
    addressHint: string
): SwapRequest {
    const refundAddress = actor.ethereumWallet.getAccount();

    return {
        alpha_ledger: {
            name: "ethereum",
            network: "regtest",
        },
        beta_ledger: {
            name: "bitcoin",
            network: "regtest",
        },
        alpha_asset: {
            name: "ether",
            quantity: "9000000000000000000",
        },
        beta_asset: {
            name: "bitcoin",
            quantity: toSatoshi(1).toString(),
        },
        alpha_ledger_refund_identity: refundAddress,
        alpha_expiry: moment().unix() + 7200,
        beta_expiry: moment().unix() + 3600,
        peer: {
            peer_id: peerId,
            address_hint: addressHint,
        },
    };
}
