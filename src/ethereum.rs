use testcontainers::{self, Docker};
use web3::{
    api::Web3,
    futures::Future,
    transports::Http,
    types::{Address, TransactionRequest, U256},
};

pub struct EthereumNode {
    pub port: u32,
}

impl EthereumNode {
    pub fn start() -> Self {
        let docker = testcontainers::clients::Cli::default();
        let container =
            docker.run(testcontainers::images::parity_parity::ParityEthereum::default());

        EthereumNode {
            port: container.get_host_port(8545).unwrap(),
        }
    }

    pub fn fund(&self, address: Address, value: U256) {
        let endpoint = format!("http://localhost:{}", &self.port);

        let (_event_loop, transport) = Http::new(&endpoint).unwrap();
        let client = Web3::new(transport);

        let parity_dev_account: web3::types::Address =
            "00a329c0648769a73afac7f9381e08fb43dbea72".parse().unwrap();

        // FIXME: Derive address from seed
        // let taker_address: web3::types::Address =
        //     "458968726a444a90fda1edc082129c661d39c7ff".parse().unwrap();

        // U256::from_dec_str("200000000000000000000").unwrap();

        client
            .personal()
            .send_transaction(
                TransactionRequest {
                    from: parity_dev_account,
                    to: Some(address),
                    gas: None,
                    gas_price: None,
                    value: Some(value),
                    data: None,
                    nonce: None,
                    condition: None,
                },
                "",
            )
            .wait()
            .unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::property_based::Quickcheck;
    use quickcheck;
    use web3::types::{Address, BlockId, BlockNumber, U128};

    #[test]
    fn got_port() {
        let ethereum = EthereumNode::start();

        let endpoint = format!("http://localhost:{}", ethereum.port);
        let (_event_loop, transport) = Http::new(&endpoint).unwrap();
        let client = Web3::new(transport);

        let _ = client
            .eth()
            .block(BlockId::Number(BlockNumber::from(0)))
            .map(|block| assert_eq!(block.unwrap().number, Some(U128::from(0))))
            .map_err(|_| panic!());
    }

    #[test]
    fn can_fund() {
        fn prop(address: Quickcheck<Address>, value: Quickcheck<U256>) -> bool {
            let ethereum = EthereumNode::start();

            ethereum.fund(address.clone().into(), value.clone().into());

            let endpoint = format!("http://localhost:{}", ethereum.port);
            let (_event_loop, transport) = Http::new(&endpoint).unwrap();
            let client = Web3::new(transport);

            client
                .eth()
                .balance(address.into(), None)
                .map(|balance| balance == value.into())
                .wait()
                .unwrap()
        }

        quickcheck::QuickCheck::new()
            .max_tests(1)
            .quickcheck(prop as fn(Quickcheck<Address>, Quickcheck<U256>) -> bool)
    }
}
