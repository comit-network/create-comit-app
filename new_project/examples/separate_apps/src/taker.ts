import {
    MakerClient,
    TakerNegotiator,
} from "comit-sdk/dist/src/negotiation/taker_negotiator";
import { formatEther } from "ethers/utils";
import readLineSync from "readline-sync";
import { toBitcoin } from "satoshi-bitcoin-ts";
import { checkEnvFile, startClient } from "./lib";

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

    readLineSync.question("0. Ready?");

    const takerNegotiator = new TakerNegotiator();
    const makerClient = new MakerClient("http://localhost:2318/");

    // take an order from a maker
    // Accept any order
    const isOrderAcceptable = () => true;
    const { order, swap } = await takerNegotiator.negotiateAndSendRequest(
        taker.comitClient,
        makerClient,
        "ETH-BTC",
        isOrderAcceptable
    );

    if (!swap) {
        throw new Error("Could not find an order or something else went wrong");
    }

    const swapMessage = await swap.getEntity();
    const swapParameters = swapMessage.properties!.parameters;
    const ether = formatEther(order.ask.amount);
    const bitcoin = toBitcoin(order.bid.amount);
    console.log(
        `Received latest order details: %s:%s for a rate of %d:%d`,
        order.ask.asset,
        order.bid.asset,
        ether,
        bitcoin
    );

    const actionConfig = { timeout: 100000, tryInterval: 1000 };

    console.log(
        "Swap started! Swapping %d %s for %d %s",
        formatEther(swapParameters.alpha_asset.quantity),
        swapParameters.alpha_asset.name,
        toBitcoin(swapParameters.beta_asset.quantity),
        swapParameters.beta_asset.name
    );

    readLineSync.question("1. Continue?");

    console.log("Ethereum HTLC funded! TXID: ", await swap.fund(actionConfig));

    readLineSync.question("3. Continue?");

    console.log("Bitcoin redeemed! TXID: ", await swap.redeem(actionConfig));

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
