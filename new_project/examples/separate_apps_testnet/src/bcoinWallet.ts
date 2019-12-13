import { Amount, Network, Pool, SPVNode, TX } from "bcoin";
import * as bcoin from "bcoin";
import Logger from "blgr";
import { BitcoinWallet } from "comit-sdk";

export class TestnetBitcoinWallet implements BitcoinWallet {
    public static async newInstance(
        networkString: string,
        hdKey: string,
        location: string,
        portInc: number = 0
    ): Promise<TestnetBitcoinWallet> {
        const parsedNetwork = Network.get(networkString);

        const logger = new Logger({
            level: "info",
        });

        parsedNetwork.rpcPort += portInc;
        parsedNetwork.port += portInc;
        parsedNetwork.walletPort += portInc;

        const walletPlugin = bcoin.wallet.plugin;
        bcoin.set(networkString);

        const node = new SPVNode({
            logger,
            network: networkString,
            file: true,
            argv: true,
            env: true,
            logFile: true,
            logConsole: true,
            db: "leveldb",
            memory: false,
            persistent: true,
            workers: true,
            listen: true,
            loader: require,
            prefix: `${location}/.bcoin/`,
            httpPort: parsedNetwork.port,
        });

        node.network.walletPort = parsedNetwork.walletPort;
        node.network.rpcPort = parsedNetwork.rpcPort;

        // We do not need the RPC interface
        node.rpc = null;

        node.pool = new Pool({
            chain: node.chain,
            spv: true,
            maxPeers: 8,
        });

        const walletdb = node.use(walletPlugin).wdb;

        // Validate the prefix directory (probably ~/.bcoin)
        await node.ensure();
        await node.open();
        await node.connect();

        const wallet = await walletdb.ensure({
            debug_logger: logger,
            network: networkString,
            master: hdKey,
            witness: true,
            accountDepth: 0,
            id: location,
        });

        const account = await wallet.getAccount(0);

        for (let i = 0; i < 10; i++) {
            node.pool.watchAddress(await account.deriveReceive(i).getAddress());
            node.pool.watchAddress(await account.deriveChange(i).getAddress());
        }

        node.pool.startSync();

        node.pool.on("tx", (tx: any) => {
            walletdb.addTX(tx);
        });

        node.pool.on("block", (block: any) => {
            walletdb.addBlock(block);
            if (block.txs.length > 0) {
                block.txs.forEach((tx: any) => {
                    walletdb.addTX(tx);
                });
            }
        });

        wallet.on("balance", (balance: any) => {
            console.log("Balance updated:\n", balance.toJSON());
        });

        walletdb.on("confirmed", (details: any) => {
            console.log(" -- wallet confirmed -- \n", details);
        });

        walletdb.on("address", (details: any) => {
            console.log(" -- wallet address -- \n", details);
        });

        node.startSync();
        await wallet.open();
        await walletdb.watch();

        const balance = await wallet.getBalance();
        console.log("Balance: ", balance);
        console.log("Wallet State: ", await walletdb.getState());
        // console.log("Wallet Tip: ", await wdb.getTip());
        console.log("Wallets: ", await walletdb.getWallets());

        return new TestnetBitcoinWallet(parsedNetwork, walletdb, node, wallet);
    }

    private constructor(
        public readonly network: any,

        // @ts-ignore
        private readonly walletdb: any,
        private readonly node: any,

        private readonly wallet: any
    ) {}

    public async getBalance(): Promise<number> {
        const balance = await this.wallet.getBalance();
        // TODO: Balances stay unconfirmed, try to use bcoin.SPVNode (and set node.http to undefined) see if it catches the confirmations
        const amount = new Amount(balance.toJSON().confirmed, "sat");
        return amount.toBTC(true);
    }

    public async getAddress() {
        const receiveAddress = await this.wallet.receiveAddress(0);
        return receiveAddress.toString(this.network);
    }

    public async sendToAddress(
        address: string,
        satoshis: number,
        network: string
    ): Promise<string> {
        this.assertNetwork(network);

        const transaction = await this.wallet.send({
            witness: true,
            outputs: [
                {
                    address,
                    value: satoshis,
                },
            ],
        });
        await this.node.pool.broadcast(transaction);

        return transaction.txid();
    }

    public async broadcastTransaction(
        transactionHex: string,
        network: string
    ): Promise<string> {
        this.assertNetwork(network);

        const transaction = TX.fromRaw(transactionHex, "hex");

        await this.node.pool.broadcast(transaction);

        return transaction.txid();
    }

    public getFee() {
        // should be dynamic in a real application
        return "150";
    }

    private assertNetwork(network: string) {
        if (network !== this.network.type) {
            throw new Error(
                `This wallet is only connected to the ${this.network.type} network and cannot perform actions on the ${network} network`
            );
        }
    }
}
