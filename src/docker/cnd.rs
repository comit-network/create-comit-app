use crate::docker::{ExposedPorts, Image};
use futures::future::Future;
use futures::IntoFuture;

const HTTP_PORT_BTSIEVE: &str = "HTTP_PORT_BTSIEVE";

pub struct Cnd;

impl Image for Cnd {
    const IMAGE: &'static str = "comitnetwork/cnd:0.2.1-RC";
    const LOG_READY: &'static str = "warp drive engaged:";

    fn arguments_for_create() -> Vec<&'static str> {
        vec![]
    }

    fn expose_ports() -> Vec<ExposedPorts> {
        vec![
            ExposedPorts {
                for_client: true,
                srcport: 9939,
                env_file_key: HTTP_PORT_BTSIEVE.to_string(),
                env_file_value: Box::new(|port| format!("http://localhost:{}", port)),
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
