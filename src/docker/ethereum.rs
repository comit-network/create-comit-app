use crate::docker::NodeImage;
use tiny_keccak;
use web3::transports::EventLoopHandle;
use web3::{
    api::Web3,
    futures::Future,
    transports::Http,
    types::{Address, TransactionRequest, H256, U256},
};

pub struct EthereumNode {
    pub http_client: Web3<Http>,
    _event_loop: EventLoopHandle,
}

impl NodeImage for EthereumNode {
    const IMAGE: &'static str = "parity/parity:v2.5.0";
    const HTTP_URL_KEY: &'static str = "ETHEREUM_NODE_HTTP_URL";
    type Address = Address;
    type Amount = U256;
    type TxId = H256;
    type Error = web3::error::Error;

    fn arguments_for_create() -> Vec<&'static str> {
        vec![
            "--config=dev",
            "--jsonrpc-apis=all",
            "--unsafe-expose",
            "--tracing=on",
            "--jsonrpc-cors=all",
        ]
    }

    fn client_port() -> u32 {
        8545
    }

    fn new(endpoint: String) -> Self {
        let (_event_loop, transport) = Http::new(&endpoint).unwrap();
        let http_client = Web3::new(transport);
        Self {
            http_client,
            _event_loop,
        }
    }

    fn fund(
        &self,
        address: Self::Address,
        value: Self::Amount,
    ) -> Box<dyn Future<Item = Self::TxId, Error = Self::Error> + Send + Sync> {
        let parity_dev_account: web3::types::Address =
            "00a329c0648769a73afac7f9381e08fb43dbea72".parse().unwrap();

        let future = self.http_client.personal().send_transaction(
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
        );
        Box::new(future)
    }
}

pub fn derive_address(secret_key: secp256k1::SecretKey) -> Address {
    let public_key =
        secp256k1::PublicKey::from_secret_key(&secp256k1::Secp256k1::new(), &secret_key);

    let serialized_public_key = public_key.serialize_uncompressed();
    // Remove the silly openssl 0x04 byte from the front of the
    // serialized public key. This is a bitcoin thing that
    // ethereum doesn't want. Eth pubkey should be 32 + 32 = 64 bytes.
    let actual_public_key = &serialized_public_key[1..];
    let hash = tiny_keccak::keccak256(actual_public_key);
    // Ethereum address is the last twenty bytes of the keccak256 hash
    let ethereum_address_bytes = &hash[12..];
    let mut address = Address::default();
    address.assign_from_slice(ethereum_address_bytes);
    address
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docker::Node;
    use envfile::EnvFile;
    use std::str::FromStr;
    use web3::types::{Address, BlockId, BlockNumber, U128};

    #[test]
    fn can_ping_ethereum_node() {
        let mut runtime = tokio::runtime::Runtime::new().unwrap();

        let file = tempfile::Builder::new().tempfile().unwrap();

        let ethereum = runtime
            .block_on(Node::<EthereumNode>::start(file.path().to_path_buf()))
            .unwrap();

        ethereum
            .node_image
            .http_client
            .eth()
            .block(BlockId::Number(BlockNumber::from(0)))
            .map(|block| assert_eq!(block.unwrap().number, Some(U128::from(0))))
            .wait()
            .unwrap();
    }

    #[test]
    fn can_fund_ethereum_address() {
        let mut runtime = tokio::runtime::Runtime::new().unwrap();

        let file = tempfile::Builder::new().tempfile().unwrap();

        let ethereum = runtime
            .block_on(Node::<EthereumNode>::start(file.path().to_path_buf()))
            .unwrap();

        let address = Address::from_str("98e8183a8bf0b7805ed7eb1044ba3e9eb2ed6c1d").unwrap();
        let value = U256::from(1_000);

        let _ = runtime
            .block_on(
                ethereum
                    .node_image
                    .fund(address.clone().into(), value.clone().into()),
            )
            .unwrap();

        let balance = runtime.block_on(
            ethereum
                .node_image
                .http_client
                .eth()
                .balance(address.into(), None),
        );
        assert_eq!(balance, Ok(value.into()));
    }

    #[test]
    fn can_get_http_port_from_envfile() {
        let mut runtime = tokio::runtime::Runtime::new().unwrap();

        let file = tempfile::Builder::new().tempfile().unwrap();

        runtime
            .block_on(Node::<EthereumNode>::start(file.path().to_path_buf()))
            .unwrap();

        let envfile = EnvFile::new(&file.path()).unwrap();
        assert!(envfile.get(&EthereumNode::HTTP_URL_KEY).is_some());
    }
}
