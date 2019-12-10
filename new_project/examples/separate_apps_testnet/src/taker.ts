import { MakerClient, Order, TakerNegotiator, TryParams } from "comit-sdk";
import { formatEther } from "ethers/utils";
import readLineSync from "readline-sync";
import { toBitcoin } from "satoshi-bitcoin-ts";
import { checkEnvFile, sleep, startClient } from "./lib";

(async function main() {
    checkEnvFile(process.env.DOTENV_CONFIG_PATH!);

    console.log("starting client...");
    const taker = await startClient("TAKER", 18332);

    console.log(
        `Fund me with BTC please: ${await taker.bitcoinWallet.getAddress()}`
    );
    console.log(
        `Fund me with ETH please: ${await taker.ethereumWallet.getAccount()}`
    );

    readLineSync.question(
        "Continue? (note, if you only funded just now, you might need to wait until the wallet has synced)"
    );

    console.log(
        "[Taker] Bitcoin balance: %f. Ether balance: %f",
        parseFloat(await taker.bitcoinWallet.getBalance()).toFixed(2),
        parseFloat(
            formatEther((await taker.ethereumWallet.getBalance()).toString())
        ).toFixed(2)
    );

    // Wait for commandline input for demo purposes
    readLineSync.question("1. Ready to accept and order from the maker?");

    // Initialize the taker negotiator that defines the negotiation phase of the trade.
    // The taker negotiator manages retrieving orders from the maker and deciding if they are acceptable for the taker.
    // Once an order was taken by a taker the negotiator hands over the order and execution parameters to the
    // execution phase.
    const takerNegotiator = new TakerNegotiator(taker.comitClient);
    const makerClient = new MakerClient("http://localhost:2318/");

    // Decide if an order is acceptable for the taker and take it.
    const isOrderAcceptable = (order: Order) => {
        // Check if the returned order matches the requested asset-pair
        if (order.ask.asset !== "ether" || order.bid.asset !== "bitcoin") {
            // These aren't the droids you're looking for
            return false;
        }

        const ether = parseFloat(order.ask.nominalAmount);
        const bitcoin = parseFloat(order.bid.nominalAmount);

        if (ether === 0 || bitcoin === 0) {
            // Let's do safe maths
            return false;
        }
        // Only accept orders that are at least 1 bitcoin for 10 Ether
        const minRate = 0.001;
        const orderRate = bitcoin / ether;
        console.log("Rate offered: ", orderRate);
        return orderRate > minRate;
    };
    const { order, swap } = await takerNegotiator.negotiateAndInitiateSwap(
        makerClient,
        // Define the trading pair to request and order for.
        "ETH-BTC",
        isOrderAcceptable
    );

    if (!swap) {
        throw new Error("Could not find an order or something else went wrong");
    }

    console.log(
        `Received latest order details: %s:%s for a rate of %d:%d`,
        order.ask.asset,
        order.bid.asset,
        order.ask.nominalAmount,
        order.bid.nominalAmount
    );

    // Retrieve the details (properties) of the swap
    const swapMessage = await swap.fetchDetails();
    const swapParameters = swapMessage.properties!.parameters;

    console.log(
        "Swap started! Swapping %d %s for %d %s",
        formatEther(swapParameters.alpha_asset.quantity),
        swapParameters.alpha_asset.name,
        toBitcoin(swapParameters.beta_asset.quantity),
        swapParameters.beta_asset.name
    );

    // Wait for commandline input for demo purposes
    readLineSync.question("2. Continue funding the Ethereum HTLC?");

    // Define how often and how long the comit-js-sdk should try to execute the fund and redeem action.
    const tryParams: TryParams = { maxTimeoutSecs: 100, tryIntervalSecs: 1 };

    console.log(
        "Ethereum HTLC funded! TXID: ",

        // -- FUND --
        // Wait for the successful execution of the funding transaction of the taker.
        //
        // -- Execution Details: --
        // The taker is the first one to fund, thus this is the first transaction sent.
        //
        // This promise will thus resolve once:
        // - The taker has sent the fund transaction.
        //
        // The transaction ID will be returned by the wallet after sending the transaction.
        await swap.fund(tryParams)
    );

    // Wait for commandline input for demo purposes
    readLineSync.question("4. Continue redeeming the Bitcoin HTLC?");

    console.log(
        "Bitcoin redeemed! TXID: ",

        // -- REDEEM --
        // Wait for the successful execution of the redeem transaction of the taker.
        //
        // -- Execution Details: --
        // The takers's redeem transaction will only be executed after the taker's comit network daemon (cnd)
        // has detected the fund transaction of the maker.
        //
        // This promise will thus resolve once:
        // - The taker has sent the fund transaction,
        // - The maker's comit network daemon has retrieved the taker's fund transaction from an incoming block,
        // - The maker has sent the fund transaction,
        // - The taker's comit network daemon has retrieved the maker's fund transaction from an incoming block,
        //
        // The transaction ID will be returned by the wallet after sending the transaction.
        await swap.redeem(tryParams)
    );

    console.log("Swapped!");

    // The comit network daemon (cnd) processes new incoming blocks faster than bcoin.
    // This results in the final balance not being printed correctly, even though the redeem transaction was already
    // noticed by cnd.
    // In order to make sure the final balance is printed correctly we thus sleep for 1 second here.
    await sleep(1000);

    // print balances after swapping
    console.log(
        "[Taker] Bitcoin balance: %f, Ether balance: %f",
        parseFloat(await taker.bitcoinWallet.getBalance()).toFixed(2),
        parseFloat(
            formatEther((await taker.ethereumWallet.getBalance()).toString())
        ).toFixed(2)
    );

    process.exit();
})();
