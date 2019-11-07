import { formatEther } from "ethers/utils";
import moment from "moment";
import readLineSync from "readline-sync";
import { toBitcoin } from "satoshi-bitcoin-ts";
import { checkEnvFile, startClient } from "./lib";
import { NegotiationProtocolHandler, Order } from "./negotiation";

const defaultOrder: Order = {
    id: "123",
    key: "ETH-BTC",
    valid_until: moment().unix() + 300,
    ask: {
        amount: "9000000000000000000",
        asset: "ether",
        ledger: "ethereum",
        network: "regtest",
    },
    bid: {
        amount: "100000000",
        asset: "bitcoin",
        ledger: "bitcoin",
        network: "regtest",
    },
    execution_params: {
        connection_info: {
            peer_id: "UNDEFINED",
            address_hint: "UNDEFINED",
        },
        expiries: {
            ask_expiry: 0,
            bid_expiry: 0,
        },
        role: "",
        swap_id: "",
    },
};

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

    // start negotiation protocol handler so that a taker can take the order and receives the latest rate

    const negotiationProtocolHandler = new NegotiationProtocolHandler();
    negotiationProtocolHandler.start(2318); // CoBloX Founding Date 🚀
    const order: Order = {
        ...defaultOrder,
        execution_params: {
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
    };

    negotiationProtocolHandler.addOrder(order);
    const invitationDetails = `http://localhost:2318/ETH-BTC`;
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
