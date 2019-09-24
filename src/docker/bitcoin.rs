use crate::docker::{BlockchainImage, ExposedPorts, Image};
use bitcoincore_rpc::RpcApi;
use futures::future::Future;
use futures::IntoFuture;
use rust_bitcoin::{self, hashes::sha256d, Address, Amount, Network};

pub const P2P_URI_KEY: &str = "BITCOIN_P2P_URI";
pub const HTTP_URL_KEY: &str = "BITCOIN_NODE_RPC_URL";

pub struct BitcoinNode {
    pub rpc_client: bitcoincore_rpc::Client,
}

impl BitcoinNode {
    // TODO: use cookie file instead
    const USERNAME: &'static str = "bitcoin";
    const PASSWORD: &'static str = "t68ej4UX2pB0cLlGwSwHFBLKxXYgomkXyFyxuBmm2U8=";
}

impl Image for BitcoinNode {
    const IMAGE: &'static str = "coblox/bitcoin-core:0.17.0";
    const NAME: &'static str = "bitcoin";
    const LOG_READY: &'static str = "Flushed wallet.dat";

    fn arguments_for_create() -> Vec<&'static str> {
        vec![
        "-regtest",
        "-server",
        "-rest",
        "-printtoconsole",
        "-bind=0.0.0.0:18444",
        "-rpcbind=0.0.0.0:18443",
        "-rpcauth=bitcoin:1c0e8f3de84926c04115e7da7e501346$a48f42ad32741dd1755649c8b98663b3ccbebeb75f196389f9a5c8a96b72edb3",
        "-rpcallowip=0.0.0.0/0",
        "-debug=1",
        "-acceptnonstdtxn=0",
        "-txindex",
    ]
    }

    fn expose_ports() -> Vec<ExposedPorts> {
        vec![
            ExposedPorts {
                for_client: true,
                srcport: 18443,
                env_file_key: HTTP_URL_KEY.to_string(),
                env_file_value: Box::new(|port| format!("http://localhost:{}", port)),
            },
            ExposedPorts {
                for_client: false,
                srcport: 18444,
                env_file_key: P2P_URI_KEY.to_string(),
                env_file_value: Box::new(|port| format!("127.0.0.1:{}", port)),
            },
        ]
    }

    fn new(endpoint: String) -> Self {
        let rpc_client = bitcoincore_rpc::Client::new(
            endpoint.clone(),
            bitcoincore_rpc::Auth::UserPass(Self::USERNAME.to_string(), Self::PASSWORD.to_string()),
        )
        .expect("Could not create client");

        Self { rpc_client }
    }
    fn post_start_actions(&self) {
        // TODO: Properly handle failure
        self.rpc_client.generate(101, None).unwrap();
    }
}

impl BlockchainImage for BitcoinNode {
    type Address = Address;
    type Amount = Amount;
    type TxId = sha256d::Hash;
    type ClientError = bitcoincore_rpc::Error;

    fn fund(
        &self,
        address: Self::Address,
        value: Self::Amount,
    ) -> Box<dyn Future<Item = Self::TxId, Error = Self::ClientError> + Send + Sync> {
        let client = &self.rpc_client;

        let response = client
            .send_to_address(&address, value, None, None, None, None, None, None)
            .and_then(|txid| client.generate(1, None).map(|_| txid));

        Box::new(response.into_future())
    }
}

pub fn derive_address(secret_key: secp256k1::SecretKey) -> Address {
    let public_key =
        secp256k1::PublicKey::from_secret_key(&secp256k1::Secp256k1::new(), &secret_key);
    derive_p2wpkh_regtest_address(public_key)
}

fn derive_p2wpkh_regtest_address(public_key: secp256k1::PublicKey) -> Address {
    Address::p2wpkh(
        &rust_bitcoin::PublicKey {
            compressed: true, // Only used for serialization
            key: public_key,
        },
        Network::Regtest,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docker::BlockchainImage;
    use crate::docker::Node;
    use envfile::EnvFile;
    use rust_bitcoin::{Address, TxOut};
    use std::convert::TryFrom;

    fn find_utxo_at_transaction_for_address(
        rpc_client: &bitcoincore_rpc::Client,
        transaction_id: &sha256d::Hash,
        address: &Address,
    ) -> Option<TxOut> {
        let address = address.clone();
        let unspent = rpc_client
            .list_unspent(Some(1), None, Some(&[address]), None, None)
            .unwrap();

        #[allow(clippy::cast_sign_loss)] // it is just for the tests
        unspent
            .into_iter()
            .find(|utxo| utxo.txid == *transaction_id)
            .map(|result| {
                let value = u64::try_from(result.amount.as_sat()).unwrap();
                TxOut {
                    value,
                    script_pubkey: result.script_pub_key,
                }
            })
    }

    #[test]
    fn can_ping_bitcoin_node() {
        let mut runtime = tokio::runtime::Runtime::new().unwrap();

        let file = tempfile::Builder::new().tempfile().unwrap();

        let bitcoin = runtime
            .block_on(Node::<BitcoinNode>::start(file.path().to_path_buf()))
            .unwrap();

        assert!(bitcoin.node_image.rpc_client.ping().is_ok());
    }

    #[test]
    fn can_fund_bitcoin_address() {
        let mut runtime = tokio::runtime::Runtime::new().unwrap();

        let file = tempfile::Builder::new().tempfile().unwrap();

        let bitcoin = runtime
            .block_on(Node::<BitcoinNode>::start(file.path().to_path_buf()))
            .unwrap();
        let client = &bitcoin.node_image.rpc_client;

        let address = client.get_new_address(None, None).unwrap();
        let value = Amount::from_sat(1_000);
        let transaction_id = bitcoin
            .node_image
            .fund(address.clone(), value)
            .wait()
            .unwrap();

        assert!(find_utxo_at_transaction_for_address(client, &transaction_id, &address).is_some());
    }

    #[test]
    fn can_get_rpc_port_from_envfile() {
        let mut runtime = tokio::runtime::Runtime::new().unwrap();

        let file = tempfile::Builder::new().tempfile().unwrap();

        runtime
            .block_on(Node::<BitcoinNode>::start(file.path().to_path_buf()))
            .unwrap();

        let envfile = EnvFile::new(&file.path()).unwrap();
        assert!(envfile.get(HTTP_URL_KEY).is_some());
    }

    #[test]
    fn can_get_p2p_port_from_envfile() {
        let mut runtime = tokio::runtime::Runtime::new().unwrap();

        let file = tempfile::Builder::new().tempfile().unwrap();

        runtime
            .block_on(Node::<BitcoinNode>::start(file.path().to_path_buf()))
            .unwrap();

        let envfile = EnvFile::new(&file.path()).unwrap();
        assert!(envfile.get(P2P_URI_KEY).is_some());
    }
}
