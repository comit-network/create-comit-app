import { MakerNegotiator, TryParams } from "comit-sdk";
import { formatEther } from "ethers/utils";
import moment from "moment";
import readLineSync from "readline-sync";
import { toBitcoin } from "satoshi-bitcoin-ts";
import { createActor, sleep } from "./lib";

/**
 * This executable function represents the maker side during a trade.
 * A trade consists of two phases: negotiation and execution.
 *
 * During the negotiation phase the maker publishes an order that the taker can take.
 * Once the negotiation is over (i.e. the taker has accepted the order) the execution of the swap commences.
 *
 * -- Execution details: --
 * Most of the logic of the swap execution is done by COMIT SDK. The example tells the ComitClient that
 * it wants to execute fund and redeem for a specific swap. The ComitClient checks for the availability of the
 * fund and redeem action in the comit node daemon.
 */
(async function main() {
    // Initialize the maker Actor
    const maker = await createActor(0);

    // print balances before swapping
    console.log(
        "[Maker] Bitcoin balance: %f, Ether balance: %f",
        (await maker.bitcoinWallet.getBalance()).toFixed(2),
        parseFloat(
            formatEther(await maker.ethereumWallet.getBalance())
        ).toFixed(2)
    );

    // Initialize the maker negotiator that defines the negotiation phase of the trade.
    // The maker negotiator manages the maker's orders, makes them available to potential takers
    // and defines when an order was taken by a taker.
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
                ethereum: { chain_id: 17 },
            },
        },
        { maxTimeoutSecs: 1000, tryIntervalSecs: 0.1 }
    );

    // Start the HTTP service used to publish orders.
    // The maker's HTTP service will be served at http://localhost:2318/
    await makerNegotiator.listen(2318, "localhost");
    // Create an order to be published.
    const order = {
        id: "123",
        validUntil: moment().unix() + 300,
        ask: {
            nominalAmount: "50",
            asset: "ether",
            ledger: "ethereum",
        },
        bid: {
            nominalAmount: "1",
            asset: "bitcoin",
            ledger: "bitcoin",
        },
    };

    // Publish the order so the taker can take it.
    makerNegotiator.addOrder(order);

    // Let the world know that you are a maker.
    // Your app could publish this link on a forum or social media so takers can connect to you.
    const link = makerNegotiator.getUrl();
    console.log(`Waiting for someone to take my order at: ${link}`);

    // Wait for a taker to accept the order and send a swap request through the comit network daemon (cnd).
    let swapHandle;
    // This loop runs until a swap request was sent from the taker to the maker
    // and a swap is waiting to be processed on the maker's side.
    while (!swapHandle) {
        await new Promise((r) => setTimeout(r, 1000));
        // Check for incoming swaps in the comit node daemon (cnd) of the maker.
        swapHandle = await maker.comitClient.getOngoingSwaps().then((swaps) => {
            if (swaps) {
                return swaps[0];
            } else {
                return undefined;
            }
        });
    }

    // Retrieve the details (properties) of the swap
    const swap = await swapHandle.fetchDetails();
    const swapParams = swap.properties!.parameters;

    console.log(
        "Swap started! Swapping %d Ether for %d %s",
        formatEther(swapParams.alpha_asset.quantity),
        toBitcoin(swapParams.beta_asset.quantity),
        swapParams.beta_asset.name
    );

    // Wait for commandline input for demo purposes
    readLineSync.question("3. Continue funding the Bitcoin HTLC?");

    const tryParams: TryParams = { maxTimeoutSecs: 100, tryIntervalSecs: 1 };

    console.log(
        "Bitcoin HTLC funded! TXID: ",

        // -- FUND --
        // Wait for the successful execution of the funding transaction of the maker.
        //
        // -- Execution Details: --
        // The maker's fund transaction will only be executed after the maker's comit network daemon (cnd)
        // has detected the funding transaction of the taker. (The taker funds first.)
        //
        // This promise will thus resolve once:
        // - The taker has sent the fund transaction,
        // - The maker's comit network daemon has retrieved the taker's fund transaction from an incoming block,
        // - The maker has sent the fund transaction.
        //
        // The transaction ID will be returned by the wallet after sending the transaction.
        await swapHandle.fund(tryParams)
    );

    // Wait for commandline input for demo purposes
    readLineSync.question("5. Continue redeeming the Ethereum HTLC?");

    console.log(
        "Ether redeemed! TXID: ",

        // -- REDEEM --
        // Wait for the successful execution of the redeem transaction of the maker.
        //
        // -- Execution Details: --
        // The maker's redeem transaction will only be executed after the maker's comit network daemon (cnd)
        // has detected the redeem transaction of the taker. (The taker redeems first.)
        //
        // This promise will thus resolve once:
        // - The taker has sent the fund transaction,
        // - The maker's comit network daemon has retrieved the taker's fund transaction from an incoming block,
        // - The maker has sent the fund transaction,
        // - The taker's comit network daemon has retrieved the maker's fund transaction from an incoming block,
        // - The taker has sent the redeem transaction.
        //
        // The transaction ID will be returned by the wallet after sending the transaction.
        await swapHandle.redeem(tryParams)
    );

    console.log("Swapped!");

    // The comit network daemon (cnd) processes new incoming blocks faster than etherjs.
    // This results in the final balance not being printed correctly, even though the redeem transaction was already
    // noticed by cnd.
    // In order to make sure the final balance is printed correctly we thus sleep for 1 second here.
    await sleep(1000);

    // print balances after swapping
    console.log(
        "[Maker] Bitcoin balance: %f, Ether balance: %f",
        (await maker.bitcoinWallet.getBalance()).toFixed(2),
        parseFloat(
            formatEther(await maker.ethereumWallet.getBalance())
        ).toFixed(2)
    );

    process.exit();
})();
