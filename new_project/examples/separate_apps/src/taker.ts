import { SwapRequest } from "comit-sdk";
import { formatEther } from "ethers/utils";
import readLineSync from "readline-sync";
import { toBitcoin } from "satoshi-bitcoin-ts";
import { Actor, checkEnvFile, startClient } from "./lib";
import {
    ExecutionParams,
    NegotiationProtocolClient,
    Order,
} from "./negotiation";

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

    // take an order from a maker
    const negotiationProtocolClient = new NegotiationProtocolClient();
    const {
        order,
        execution_params,
    } = await negotiationProtocolClient.startNegotiation(
        "http://localhost:2318/orders",
        "ETH-BTC"
    );

    const ether = formatEther(order.ask.amount);
    const bitcoin = toBitcoin(order.bid.amount);
    console.log(
        `Received latest order details: %s:%s for a rate of %d:%d`,
        order.ask.asset,
        order.bid.asset,
        ether,
        bitcoin
    );

    const swapMessage = createSwap(taker, order, execution_params);

    const swapHandle = await taker.comitClient.sendSwap(swapMessage);

    const actionConfig = { timeout: 100000, tryInterval: 1000 };

    console.log(
        "Swap started! Swapping %d %s for %d %s",
        formatEther(swapMessage.alpha_asset.quantity),
        swapMessage.alpha_asset.name,
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
    order: Order,
    executionParams: ExecutionParams
): SwapRequest {
    const refundAddress = actor.ethereumWallet.getAccount();

    return {
        alpha_ledger: {
            name: order.ask.ledger,
            network: order.ask.network,
        },
        beta_ledger: {
            name: order.bid.ledger,
            network: order.bid.network,
        },
        alpha_asset: {
            name: order.ask.asset,
            quantity: order.ask.amount,
        },
        beta_asset: {
            name: order.bid.asset,
            quantity: order.bid.amount,
        },
        alpha_ledger_refund_identity: refundAddress,
        alpha_expiry: executionParams.expiries.ask_expiry,
        beta_expiry: executionParams.expiries.bid_expiry,
        peer: {
            peer_id: executionParams.connection_info.peer_id,
            address_hint: executionParams.connection_info.address_hint,
        },
    };
}
