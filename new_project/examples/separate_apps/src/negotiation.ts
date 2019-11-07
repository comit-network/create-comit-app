import axios from "axios";
import express from "express";

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
    execution_params: {
        swap_id: string;
        connection_info: {
            peer_id: string;
            address_hint: string;
        };
        role: string;
        expiries: { bid_expiry: number; ask_expiry: number };
    };
}

export class NegotiationProtocolHandler {
    private orders: { [key: string]: Order } = {};

    public start(port: number) {
        const app = express();

        app.get("/", (_, res) =>
            res.send("Negotiation Protocol up and running!")
        );
        app.get("/:params", (req, res) => {
            const order = this.orders[req.params.params];
            if (!order) {
                res.status(451).send("Unavailable For Legal Reasons");
            } else {
                res.send(JSON.stringify(this.orders[req.params.params]));
            }
        });

        app.listen(port, () =>
            console.log(`Negotiation Protocol listening on port ${port}!`)
        );
    }

    public addOrder(order: Order) {
        this.orders[order.key] = order;
    }
}

export class NegotiationProtocolClient {
    public async getOrder(inviteDetails: string) {
        const response = await axios.get(inviteDetails);
        return response.data;
    }
}
