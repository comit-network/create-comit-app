import { CoinType, Offer, OrderBook } from "../src/orderBook";

describe("OrderBook tests", () => {
    it("an added offer should be retrievable in the correct form", () => {
        const orderBook = new OrderBook();
        const offer: Offer = {
            sellCoin: {
                coin: CoinType.Ether,
                amount: 10,
            },
            buyCoin: {
                coin: CoinType.Bitcoin,
                amount: 1,
            },
            makerPeerId: "peerID",
            makerPeerAddress: "/ip4/127.0.0.1/tcp/1337",
        };

        orderBook.addOffer(offer);

        const foundOffers = orderBook.findOffers({
            buyCoin: CoinType.Ether,
            sellCoin: CoinType.Bitcoin,
            buyAmount: 5,
        });

        expect(foundOffers).toHaveLength(1);
        const foundOffer = foundOffers[0]!;
        expect(foundOffer).toEqual({
            buyCoin: {
                coin: offer.sellCoin.coin,
                amount: 5,
            },
            sellCoin: {
                coin: offer.buyCoin.coin,
                amount: 0.5,
            },
            makerPeerId: offer.makerPeerId,
            makerPeerAddress: offer.makerPeerAddress,
        });
    });
});
