import { MakerHttpApi, MakerNegotiator, Order, TryParams } from "comit-sdk";
import { formatEther } from "ethers/utils";
import moment from "moment";
import * as readline from "readline";
import { toBitcoin } from "satoshi-bitcoin-ts";
import { Actor, checkEnvFile, printBalance, startClient } from "./lib";
import { sleep } from "./lib";

(async function main() {
    checkEnvFile(process.env.DOTENV_CONFIG_PATH!);

    const maker = await startClient("MAKER", 0);

    console.log(
        `Fund me with BTC please: ${await maker.bitcoinWallet.getAddress()}`
    );
    console.log(
        `Fund me with ETH please: ${await maker.ethereumWallet.getAccount()}`
    );

    const rl = readline.createInterface({
        input: process.stdin,
        output: process.stdout,
    });
    await printBalance(maker, "maker");
    rl.question(
        "Continue? (note, if you only funded just now, you might need to wait until the wallet has synced)",
        async () => {
            await executeWorkflow(maker);
            rl.close();
        }
    );
})();

async function executeWorkflow(maker: Actor) {
    await printBalance(maker, "maker");

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

    const makerHttpApi = new MakerHttpApi(makerNegotiator);
    makerHttpApi.listen(2318);
    const order: Order = {
        id: "123",
        tradingPair: "ETH-BTC",
        validUntil: moment().unix() + 300,
        ask: {
            nominalAmount: "0.2",
            asset: "ether",
            ledger: "ethereum",
        },
        bid: {
            nominalAmount: "0.002",
            asset: "bitcoin",
            ledger: "bitcoin",
        },
    };

    // Publish the order so the maker can take it.
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

    // Retrieve the details (properties) of the swap
    const swap = await swapHandle.fetchDetails();
    const swapParams = swap.properties!.parameters;

    console.log(
        "Swap started! Swapping %d Ether for %d %s",
        formatEther(swapParams.alpha_asset.quantity),
        toBitcoin(swapParams.beta_asset.quantity),
        swapParams.beta_asset.name
    );

    console.log("3. Continuing funding the Bitcoin HTLC");

    const tryParams: TryParams = {
        maxTimeoutSecs: 40 * 60,
        tryIntervalSecs: 1,
    };

    console.log(
        "Bitcoin HTLC funded! TXID: ",
        await swapHandle.fund(tryParams)
    );

    console.log("5. Continuing redeeming the Ethereum HTLC");

    console.log(
        "Ether redeemed! TXID: ",

        await swapHandle.redeem(tryParams)
    );

    console.log("Swapped!");

    await sleep(3 * 60 * 1000);

    await printBalance(maker, "maker");

    process.exit();
}
