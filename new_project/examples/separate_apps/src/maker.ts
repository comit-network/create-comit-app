import {
    MakerHttpApi,
    MakerNegotiator,
} from "comit-sdk/dist/src/negotiation/maker_negotiator";
import { Order } from "comit-sdk/dist/src/negotiation/order";
import { TryParams } from "comit-sdk/dist/src/timeout_promise";
import { formatEther } from "ethers/utils";
import moment from "moment";
import readLineSync from "readline-sync";
import { toBitcoin } from "satoshi-bitcoin-ts";
import { checkEnvFile, startClient } from "./lib";

function createOrder(): Order {
    return {
        id: "123",
        tradingPair: "ETH-BTC",
        validUntil: moment().unix() + 300,
        ask: {
            nominalAmount: "9",
            asset: "ether",
            ledger: "ethereum",
        },
        bid: {
            nominalAmount: "1",
            asset: "bitcoin",
            ledger: "bitcoin",
        },
    };
}

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

    const makerNegotiator = new MakerNegotiator(
        maker.comitClient,
        {
            peer: {
                peer_id: peerId,
                address_hint: addressHint,
            },
            alpha_expiry: moment().unix() + 7200,
            beta_expiry: moment().unix() + 3600,
            ledgers: {
                bitcoin: { network: "regtest" },
                // TODO: It should be possible to use the chain_id
                ethereum: { network: "regtest" },
            },
        },
        { maxTimeoutSecs: 1000, tryIntervalSecs: 0.1 }
    );

    const makerHttpApi = new MakerHttpApi(makerNegotiator);

    makerHttpApi.listen(2318);
    const order = createOrder();
    makerNegotiator.addOrder(order);

    const invitationDetails = `http://localhost:2318/orders/ETH-BTC`;
    console.log(`Waiting for someone taking my order at: ${invitationDetails}`);

    let swapHandle;
    while (!swapHandle) {
        await new Promise(r => setTimeout(r, 1000));
        swapHandle = await maker.comitClient.getOngoingSwaps().then(swaps => {
            if (swaps) {
                return swaps[0];
            } else {
                return undefined;
            }
        });
    }

    const swap = await swapHandle.fetchDetails();
    const swapParams = swap.properties!.parameters;

    console.log(
        "Swap started! Swapping %d Ether for %d %s",
        formatEther(swapParams.alpha_asset.quantity),
        toBitcoin(swapParams.beta_asset.quantity),
        swapParams.beta_asset.name
    );

    readLineSync.question("2. Continue?");

    const tryParams: TryParams = { maxTimeoutSecs: 100, tryIntervalSecs: 1 };

    console.log(
        "Bitcoin HTLC funded! TXID: ",
        await swapHandle.fund(tryParams)
    );

    readLineSync.question("4. Continue?");

    console.log("Ether redeemed! TXID: ", await swapHandle.redeem(tryParams));

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
