import { MakerHttpApi, MakerNegotiator, Order, TryParams } from "comit-sdk";
import { formatEther } from "ethers/utils";
import moment from "moment";
import readLineSync from "readline-sync";
import { toBitcoin } from "satoshi-bitcoin-ts";
import { checkEnvFile, startClient } from "./lib";
import { sleep } from "./lib";

(async function main() {
    checkEnvFile(process.env.DOTENV_CONFIG_PATH!);

    const maker = await startClient("MAKER", 18333);

    console.log(
        `Fund me with BTC please: ${await maker.bitcoinWallet.getAddress()}`
    );
    console.log(
        `Fund me with ETH please: ${await maker.ethereumWallet.getAccount()}`
    );

    readLineSync.question(
        "Continue? (note, if you only funded just now, you might need to wait until the wallet has synced)"
    );

    console.log(
        "[Maker] Bitcoin balance: %f. Ether balance: %f",
        parseFloat((await maker.bitcoinWallet.getBalance()).toString()).toFixed(2),
        parseFloat(
            formatEther((await maker.ethereumWallet.getBalance()).toString())
        ).toFixed(2)
    );

    const makerNegotiator = new MakerNegotiator(
        maker.comitClient,
        {
            peer: {
                peer_id: maker.peerId,
                address_hint: maker.addressHint,
            },
            alpha_expiry: moment().unix() + 7200,
            beta_expiry: moment().unix() + 3600,
            ledgers: {
                bitcoin: { network: "testnet" },
                ethereum: { network: "ropsten" },
            },
        },
        { maxTimeoutSecs: 1000, tryIntervalSecs: 0.1 }
    );

    // Start the HTTP service used to publish orders.
    const makerHttpApi = new MakerHttpApi(makerNegotiator);
    // The maker's HTTP service will be served at port 2318.
    makerHttpApi.listen(2318);
    // Create an order to be published.
    const order: Order = {
        id: "123",
        tradingPair: "ETH-BTC",
        validUntil: moment().unix() + 300,
        ask: {
            nominalAmount: "0.02",
            asset: "ether",
            ledger: "ethereum",
        },
        bid: {
            nominalAmount: "0.0002",
            asset: "bitcoin",
            ledger: "bitcoin",
        },
    };

    // Publish the order so the taker can take it.
    makerNegotiator.addOrder(order);

    // Let the world know that there is an order available.
    // In a real-world application this information could be shared publicly, e.g. on social medias.
    const invitationDetails = `http://localhost:2318/orders/ETH-BTC`;
    console.log(`Waiting for someone taking my order at: ${invitationDetails}`);

    // Wait for a taker to accept the order and send a swap request through the comit network daemon (cnd).
    let swapHandle;
    // This loop runs until a swap request was sent from the taker to the maker
    // and a swap is waiting to be processed on the maker's side.
    while (!swapHandle) {
        await new Promise(r => setTimeout(r, 1000));
        // Check for incoming swaps in the comit node daemon (cnd) of the maker.
        swapHandle = await maker.comitClient.getOngoingSwaps().then(swaps => {
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

    const tryParams: TryParams = { maxTimeoutSecs: 40 * 60, tryIntervalSecs: 1 };

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
        parseFloat((await maker.bitcoinWallet.getBalance()).toString()).toFixed(2),
        parseFloat(
            formatEther((await maker.ethereumWallet.getBalance()).toString())
        ).toFixed(2)
    );

    process.exit();
})();
