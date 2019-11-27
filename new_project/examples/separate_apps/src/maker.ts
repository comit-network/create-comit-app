import {
    MakerHttpApi,
    MakerNegotiator,
} from "comit-sdk/dist/src/negotiation/maker_negotiator";
import { Order } from "comit-sdk/dist/src/negotiation/order";
import { formatEther } from "ethers/utils";
import moment from "moment";
import readLineSync from "readline-sync";
import { toBitcoin } from "satoshi-bitcoin-ts";
import { startClient } from "./lib";

/**
 * Creates and Order to be published by a maker to enable a trade with a taker.
 * @returns {Order} Order defining trading pair, validity and trading amounts.
 */
function createOrder(): Order {
    return {
        id: "123",
        tradingPair: "ETH-BTC",
        valid_until: moment().unix() + 300,
        ask: {
            amount: "9000000000000000000",
            asset: "ether",
            ledger: "ethereum",
        },
        bid: {
            amount: "100000000",
            asset: "bitcoin",
            ledger: "bitcoin",
        },
    };
}

/**
 * This executable function represents the maker side during a trade.
 * A trade consists of two phases: negotiation and execution.
 *
 * During the negotiation phase the maker publishes and order that the taker can take.
 * Once the negotiation is is over (i.e. the taker has accepted the order) the execution of the swap commences.
 *
 * -- Execution details: --
 * Most of the logic of the swap execution is done by the comit-js-sdk. The example tells the ComitClient that
 * it wants to execute fund and redeem for a specific swap. The ComitClient checks for the availability of the
 * fund and redeem action in the comit node daemon.
 */
(async function main() {

    // Initialize the maker Actor
    const maker = await startClient(0);

    // print balances before swapping
    console.log(
        "[Maker] Bitcoin balance: %f. Ether balance: %f",
        parseFloat(await maker.bitcoinWallet.getBalance()).toFixed(2),
        parseFloat(
            formatEther(await maker.ethereumWallet.getBalance())
        ).toFixed(2)
    );

    // Initialize the maker negotiator that defines the negotiation phase of the trade.
    // The maker negotiator manages the maker's orders and defines when an order was taken by a taker.
    // Once an order was taken by a taker the negotiator hands over the order and execution parameters to the
    // execution phase.
    const makerNegotiator = new MakerNegotiator(
        maker.comitClient,
        {
            // Connection information for the comit network daemon.
            // The maker has to provide this to the taker for the execution phase,
            // so that the taker's comit network daemon can message the maker's comit network daemon.
            peer: {
                peer_id: maker.peerId,
                address_hint: maker.addressHint,
            },
            // The expiry time for the taker.
            alpha_expiry: moment().unix() + 7200,
            // The expiry time for the maker
            beta_expiry: moment().unix() + 3600,
            // The network the swap will be executed on.
            ledgers: {
                bitcoin: { network: "regtest" },
                ethereum: { network: "regtest" },
            },
        },
        // TODO: Difficult to explain... can we hide this in the SDK?
        // timeout and retry for auto-accept
        { timeout: 100000, tryInterval: 1000 }
    );

    // Start the HTTP service used for publishing orders.
    const makerHttpApi = new MakerHttpApi(makerNegotiator);
    // The maker's HTTP service will be served at port 2318.
    makerHttpApi.listen(2318);
    // Create an order to be published.
    const order = createOrder();
    // Publish the order so the taker can take it.
    makerNegotiator.addOrder(order);

    // Let the world know that there is an order available
    const invitationDetails = `http://localhost:2318/orders/ETH-BTC`;
    console.log(`Waiting for someone taking my order at: ${invitationDetails}`);

    // Wait for a taker to accept the order and send a swap request through the comit network daemon (cnd).
    let swapHandle;
    // This loop runs until a swap request was sent from the taker to the maker
    // and a swap is waiting to be processed on the maker's side.
    while (!swapHandle) {
        await new Promise(r => setTimeout(r, 1000));
        // Listen for incoming swaps in the comit node daemon (cnd) of the maker.
        swapHandle = await maker.comitClient.getNewSwaps().then(swaps => {
            if (swaps) {
                return swaps[0];
            } else {
                return undefined;
            }
        });
    }

    // Retrieve the details (properties) of the swap
    const swap = await swapHandle.getEntity();
    const swapParams = swap.properties!.parameters;

    // Define how often and how long the comit-js-sdk should try to execute the accept/decline, fund and redeem action.
    const actionConfig = { timeout: 100000, tryInterval: 1000 };

    console.log(
        "Swap started! Swapping %d Ether for %d %s",
        formatEther(swapParams.alpha_asset.quantity),
        toBitcoin(swapParams.beta_asset.quantity),
        swapParams.beta_asset.name
    );

    // Wait for commandline input for demo purposes
    readLineSync.question("2. Continue?");

    console.log(
        "Bitcoin HTLC funded! TXID: ",

        // -- FUND --
        // Wait for the successful execution of the funding transaction of the maker.
        //
        // -- Execution Details: --
        // The maker's fund transaction will only be executed after the maker's comit network daemon (cnd)
        // has detected the funding transaction of the taker. (The taker funds first.)
        //
        // This future will thus resolve once:
        // - The taker has sent the fund transaction,
        // - The maker's comit network daemon has retrieved the taker's fund transaction from an incoming block,
        // - The maker has sent the fund transaction.
        //
        // The transaction ID will be returned by the wallet after sending the transaction.
        await swapHandle.fund(actionConfig)
    );

    // Wait for commandline input for demo purposes
    readLineSync.question("4. Continue?");

    console.log(
        "Ether redeemed! TXID: ",

        // -- REDEEM --
        // Wait for the successful execution of the redeem transaction of the maker.
        //
        // -- Execution Details: --
        // The maker's redeem transaction will only be executed after the maker's comit network daemon (cnd)
        // has detected the redeem transaction of the taker. (The taker redeems first.)
        //
        // This future will thus resolve once:
        // - The taker has sent the fund transaction,
        // - The maker's comit network daemon has retrieved the taker's fund transaction from an incoming block,
        // - The maker has sent the fund transaction,
        // - The taker's comit network daemon has retrieved the maker's fund transaction from an incoming block,
        // - The taker has sent the redeem transaction.
        //
        // The transaction ID will be returned by the wallet after sending the transaction.
        await swapHandle.redeem(actionConfig)
    );

    console.log("Swapped!");

    // print balances after swapping
    console.log(
        "[Maker] Bitcoin balance: %f. Ether balance: %f",
        parseFloat(await maker.bitcoinWallet.getBalance()).toFixed(2),
        parseFloat(
            formatEther(await maker.ethereumWallet.getBalance())
        ).toFixed(2)
    );

    process.exit();
})();
