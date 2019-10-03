use crate::docker::{blockchain::BlockchainImage, ExposedPorts, Image};
use lazy_static::lazy_static;
use tiny_keccak;
use web3::transports::EventLoopHandle;
use web3::{
    api::Web3,
    futures::Future,
    transports::Http,
    types::{Address, TransactionRequest, H256, U256},
};

lazy_static! {
// expect: Should always be able to parse
        static ref PARITY_DEV_ACCOUNT: web3::types::Address = "00a329c0648769a73afac7f9381e08fb43dbea72"
            .parse()
            .expect("Could not parse DEV account address");
}

pub const HTTP_URL_KEY: &str = "ETHEREUM_NODE_HTTP_URL";

pub struct EthereumNode {
    pub http_client: Web3<Http>,
    _event_loop: EventLoopHandle,
}

impl Image for EthereumNode {
    const IMAGE: &'static str = "parity/parity:v2.5.0";
    const LOG_READY: &'static str = "Public node URL:";

    fn arguments_for_create() -> Vec<&'static str> {
        vec![
            "--config=dev",
            "--jsonrpc-apis=all",
            "--unsafe-expose",
            "--tracing=on",
            "--jsonrpc-cors=all",
        ]
    }

    // TODO: Probably actually use the name instead of HTTP_URL_KEY
    fn expose_ports(_: &str) -> Vec<ExposedPorts> {
        vec![ExposedPorts {
            for_client: true,
            srcport: 8545,
            env_file_key: HTTP_URL_KEY.to_string(),
            env_file_value: Box::new(|port| format!("http://localhost:{}", port)),
        }]
    }

    fn new(endpoint: Option<String>) -> Self {
        let endpoint: String = endpoint.unwrap_or_else(|| {
            panic!("Internal Error: Url for web3 client should have been set.");
        });
        let (_event_loop, transport) = Http::new(&endpoint).unwrap();
        let http_client = Web3::new(transport);
        Self {
            http_client,
            _event_loop,
        }
    }
}

impl BlockchainImage for EthereumNode {
    type Address = Address;
    type Amount = U256;
    type TxId = H256;
    type ClientError = web3::error::Error;

    fn fund(
        &self,
        address: Self::Address,
        value: Self::Amount,
    ) -> Box<dyn Future<Item = Self::TxId, Error = Self::ClientError> + Send + Sync> {
        let future = self.http_client.personal().send_transaction(
            TransactionRequest {
                from: *PARITY_DEV_ACCOUNT,
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
