import { formatEther } from "ethers/utils";
import readLineSync from "readline-sync";
import { toBitcoin } from "satoshi-bitcoin-ts";
import { checkEnvFile, startClient } from "./lib";

(async function main() {
    checkEnvFile(process.env.DOTENV_CONFIG_PATH!);

    const maker = await startClient(0);

    console.log(
        "[Maker] Bitcoin balance: %f. Ether balance: %f",
        parseFloat(await maker.bitcoinWallet.getBalance()).toFixed(2),
        parseFloat(
            formatEther(await maker.ethereumWallet.getBalance())
        ).toFixed(2)
    );

    const peerId = await maker.comitClient.getPeerId();
    const addressHint = await maker.comitClient
        .getPeerListenAddresses()
        .then(addresses => addresses[0]);

    console.log("[Maker] peer id:", peerId);
    console.log("[Maker] address hint:", addressHint);

    // Note that we assume the maker published an offer
    // somehow somewhere and that someone (a taker)
    // will take such offer by send a SWAP request to the
    // maker's COMIT node. Publishing and Finding offers
    // is not part of this example.

    console.log("Waiting for someone to take my offer...");

    let swapHandle;
    while (!swapHandle) {
        await new Promise(r => setTimeout(r, 1000));
        swapHandle = await maker.comitClient.getNewSwaps().then(swaps => {
            if (swaps) {
                return swaps[0];
            } else {
                return undefined;
            }
        });
    }

    const actionConfig = { timeout: 100000, tryInterval: 1000 };
    await swapHandle.accept(actionConfig);

    const swap = await swapHandle.getEntity();
    const swapParams = swap.properties!.parameters;
    console.log(
        "Swap started! Swapping %d Ether for %d %s",
        formatEther(swapParams.alpha_asset.quantity),
        toBitcoin(swapParams.beta_asset.quantity),
        swapParams.beta_asset.name
    );

    readLineSync.question("2. Continue?");

    console.log(
        "Bitcoin HTLC funded! TXID: ",
        await swapHandle.fund(actionConfig)
    );

    readLineSync.question("4. Continue?");

    console.log(
        "Ether redeemed! TXID: ",
        await swapHandle.redeem(actionConfig)
    );

    console.log("Swapped!");
    console.log(
        "[Maker] Bitcoin balance: %f. Ether balance: %f",
        parseFloat(await maker.bitcoinWallet.getBalance()).toFixed(2),
        parseFloat(
            formatEther(await maker.ethereumWallet.getBalance())
        ).toFixed(2)
    );
    process.exit();
})();
