import axios from "axios";
import express from "express";

export interface ExecutionParams {
    swap_id: string;
    connection_info: {
        peer_id: string;
        address_hint: string;
    };
    role: string;
    expiries: { bid_expiry: number; ask_expiry: number };
}

export interface Order {
    key: string;
    id: string;
    valid_until: number;
    bid: {
        ledger: string;
        asset: string;
        amount: string;
        network: string;
    };
    ask: {
        ledger: string;
        asset: string;
        amount: string;
        network: string;
    };
}

export class NegotiationProtocolHandler {
    private orders: { [key: string]: Order } = {};
    private executionParams: ExecutionParams;
    private port: number;

    constructor(executionParams: ExecutionParams, port: number) {
        this.executionParams = executionParams;
        this.port = port;
    }

    public start() {
        const app = express();

        app.get("/", (_, res) =>
            res.send("Negotiation Protocol up and running!")
        );
        app.get("/orders/:keypair", (req, res) => {
            const order = this.orders[req.params.keypair];
            if (!order) {
                res.status(451).send("Unavailable For Legal Reasons");
            } else {
                res.send(order);
            }
        });
        app.get("/orders/:keypair/:orderid/accept", (req, res) => {
            const order = this.orders[req.params.keypair];
            if (!order) {
                res.status(451).send("Unavailable For Legal Reasons");
            } else {
                res.send({ order, execution_params: this.executionParams });
            }
        });

        app.listen(this.port, () =>
            console.log(`Negotiation Protocol listening on port ${this.port}!`)
        );
    }

    public addOrder(order: Order) {
        this.orders[order.key] = order;
    }
}

export class NegotiationProtocolClient {
    private static async getOrder(connectionInfo: string, tradingPair: string) {
        const response = await axios.get(`${connectionInfo}/${tradingPair}`);
        return response.data;
    }

    private static async acceptOrder(connectionInfo: string, order: Order) {
        const response = await axios.get(
            `${connectionInfo}/${order.key}/${order.id}/accept`
        );
        return response.data;
    }

    public async startNegotiation(connectionInfo: string, tradingPair: string) {
        const order: Order = await NegotiationProtocolClient.getOrder(
            connectionInfo,
            tradingPair
        );
        return NegotiationProtocolClient.acceptOrder(connectionInfo, order);
    }
}
