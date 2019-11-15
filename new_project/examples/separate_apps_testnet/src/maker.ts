import { formatEther } from "ethers/utils";
import moment from "moment";
import readLineSync from "readline-sync";
import { toBitcoin } from "satoshi-bitcoin-ts";
import { checkEnvFile, startClient } from "./lib";
import { NegotiationProtocolHandler, Order } from "./negotiation";

function createOrder(): Order {
    return {
        id: "123",
        key: "ETH-BTC",
        valid_until: moment().unix() + 300,
        ask: {
            amount: "90000000000000",
            asset: "ether",
            ledger: "ethereum",
            network: "ropsten",
        },
        bid: {
            amount: "1000",
            asset: "bitcoin",
            ledger: "bitcoin",
            network: "testnet",
        },
    };
}

(async function main() {
    checkEnvFile(process.env.DOTENV_CONFIG_PATH!);

    const maker = await startClient("MAKER");

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

    // start negotiation protocol handler so that a taker can take the order and receives the latest rate

    const negotiationProtocolHandler = new NegotiationProtocolHandler(
        {
            connection_info: {
                peer_id: peerId,
                address_hint: addressHint,
            },
            expiries: {
                ask_expiry: moment().unix() + 7200,
                bid_expiry: moment().unix() + 3600,
            },
            role: "alice",
            swap_id: "SOME_RANDOM_ID",
        },
        2318
    ); // CoBloX Founding Date 🚀

    negotiationProtocolHandler.start();
    const order = createOrder();
    negotiationProtocolHandler.addOrder(order);

    const invitationDetails = `http://localhost:2318/orders/ETH-BTC`;
    console.log(`Waiting for someone taking my order at: ${invitationDetails}`);

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

    const swap = await swapHandle.getEntity();
    const swapParams = swap.properties!.parameters;

    // only accept a request if it fits to the created order above
    if (isValid(swapParams, order)) {
        console.log("Requested order is invalid");
        await swapHandle.decline(actionConfig);
        process.exit();
    }
    console.log("Requested order is still valid");
    await swapHandle.accept(actionConfig);

    console.log(
        "Swap started! Swapping %d Ether for %d %s",
        formatEther(swapParams.alpha_asset.quantity),
        toBitcoin(swapParams.beta_asset.quantity),
        swapParams.beta_asset.name
    );

    readLineSync.question("2. Continue?");

    let btcBalance = await maker.bitcoinWallet.getBalance();
    while (btcBalance <= 0) {
        console.log("0 bitcoin balance, wallet most likely not properly initialized!");
        readLineSync.question("2. Try Again?");
        btcBalance = await maker.bitcoinWallet.getBalance();
    }

    console.log("Bitcoin balance: " + btcBalance);
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

function isValid(swapParams: any, order: Order) {
    return (
        swapParams.alpha_asset.name !== order.ask.asset ||
        swapParams.beta_asset.name !== order.bid.asset ||
        order.valid_until < moment().unix()
    );
}
