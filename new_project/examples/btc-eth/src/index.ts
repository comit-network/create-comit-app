import { BitcoinWallet, EthereumWallet } from "comit-sdk";
import { formatEther } from "ethers/utils";
import fs from "fs";
import { CoinType, HelloSwap, WhoAmI } from "./helloSwap";
import { createLogger } from "./logger";
import { OrderBook } from "./orderBook";

(async function main() {
    checkEnvFile(process.env.DOTENV_CONFIG_PATH!);

    const orderBook = new OrderBook();

    const maker = await startApp("maker", 0);
    const taker = await startApp("taker", 1);

    // Maker creates and publishes offer
    const makerOffer = await maker.createOffer(
        {
            coin: CoinType.Ether,
            amount: 10,
        },
        {
            coin: CoinType.Bitcoin,
            amount: 1,
        }
    );
    orderBook.addOffer(makerOffer);

    // Taker finds and takes offer
    const foundOffers = orderBook.findOffers({
        buyCoin: CoinType.Ether,
        sellCoin: CoinType.Bitcoin,
        buyAmount: 5,
    });
    await taker.takeOffer(foundOffers[0]);

    process.stdin.resume(); // so the program will not close instantly

    async function exitHandler() {
        maker.stop();
        taker.stop();

        await logBalances(maker).then(() => logBalances(taker));
        process.exit();
    }

    process.on("SIGINT", exitHandler);
    process.on("SIGUSR1", exitHandler);
    process.on("SIGUSR2", exitHandler);
})();

async function startApp(whoAmI: WhoAmI, index: number) {
    const bitcoinWallet = await BitcoinWallet.newInstance(
        "regtest",
        process.env.BITCOIN_P2P_URI!,
        process.env[`BITCOIN_HD_KEY_${index}`]!
    );
    await new Promise(r => setTimeout(r, 1000));

    const ethereumWallet = new EthereumWallet(
        process.env[`ETHEREUM_KEY_${index}`]!,
        process.env.ETHEREUM_NODE_HTTP_URL!
    );

    const app = new HelloSwap(
        process.env[`HTTP_URL_CND_${index}`]!,
        whoAmI,
        bitcoinWallet,
        ethereumWallet
    );

    await logBalances(app);

    return app;
}

async function logBalances(app: HelloSwap) {
    const logger = createLogger();
    logger[app.whoAmI](
        "Bitcoin balance: %f. Ether balance: %f",
        parseFloat(await app.getBitcoinBalance()).toFixed(2),
        parseFloat(formatEther(await app.getEtherBalance())).toFixed(2)
    );
}

function checkEnvFile(path: string) {
    if (!fs.existsSync(path)) {
        const logger = createLogger();
        logger.error(
            "Could not find %s file. Did you run \\`create-comit-app start-env\\`?",
            path
        );
        process.exit(1);
    }
}
