export interface Offer {
    sellCoin: Coin;
    buyCoin: Coin;
    makerPeerId: string;
    makerPeerAddress: string;
}

export enum CoinType {
    Bitcoin = "bitcoin",
    Ether = "ether",
}

export interface Coin {
    coin: CoinType;
    amount: number;
}

export interface FindOfferQuery {
    buyCoin: CoinType;
    sellCoin: CoinType;
    buyAmount: number;
}

export class OrderBook {
    private readonly offers: Offer[];

    public constructor() {
        this.offers = [];
    }

    public addOffer(offer: Offer) {
        const offerToPublish = {
            ...offer,
            sellCoin: {
                ...offer.buyCoin,
            },
            buyCoin: {
                ...offer.sellCoin,
            },
        };

        this.offers.push(offerToPublish);
    }

    /**
     * This function represents the online order book. It looks up offers by coin types and wanted buyAmount.
     * The offers returned are mirrored in order to make the offers suitable for the taker, i.e. while the maker
     * added her offers in `addOffer(..)`, this function returns the same offers with sellCoin = buyCoin and
     * buyCoin = sellCoin
     * @param buyCoin: the coin requester wants to buy
     * @param sellCoin: the coin requester wants to sell
     * @param buyAmount: the amount requester wants to buy
     */
    public findOffers({
        buyCoin,
        sellCoin,
        buyAmount,
    }: FindOfferQuery): Offer[] {
        return this.offers
            .filter(
                offer =>
                    offer.buyCoin.coin === buyCoin &&
                    offer.sellCoin.coin === sellCoin &&
                    offer.buyCoin.amount >= buyAmount
            )
            .map((offer: Offer) => {
                return {
                    ...offer,
                    sellCoin: {
                        ...offer.sellCoin,
                        amount:
                            (buyAmount * offer.sellCoin.amount) /
                            offer.buyCoin.amount,
                    },
                    buyCoin: {
                        ...offer.buyCoin,
                        amount: buyAmount,
                    },
                };
            });
    }
}
