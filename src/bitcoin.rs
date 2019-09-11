use bitcoincore_rpc::RpcApi;
use rust_bitcoin::{hashes::sha256d, Address, Amount};
use testcontainers::{self, Docker};

pub struct BitcoinNode {
    pub port: u32,
    pub auth: Auth,
}

impl BitcoinNode {
    pub fn start() -> BitcoinNode {
        let docker = testcontainers::clients::Cli::default();
        let container =
            docker.run(testcontainers::images::coblox_bitcoincore::BitcoinCore::default());
        let auth = container.image().auth();

        let node = BitcoinNode {
            port: container.get_host_port(18443).unwrap(),
            auth: Auth {
                username: auth.username().to_string(),
                password: auth.password().to_string(),
            },
        };

        let endpoint = format!("http://localhost:{}", node.port);
        let client = bitcoincore_rpc::Client::new(endpoint, node.auth.clone().into()).unwrap();

        client.generate(101, None).unwrap();

        node
    }

    pub fn fund(&self, address: &Address, amount: Amount) -> sha256d::Hash {
        let endpoint = format!("http://localhost:{}", self.port);
        let client = bitcoincore_rpc::Client::new(endpoint, self.auth.clone().into()).unwrap();

        let transaction_id = client
            .send_to_address(&address, amount, None, None, None, None, None, None)
            .unwrap();

        client.generate(1, None).unwrap();

        transaction_id
    }
}

#[derive(Clone)]
pub struct Auth {
    username: String,
    password: String,
}

impl From<Auth> for bitcoincore_rpc::Auth {
    fn from(source: Auth) -> bitcoincore_rpc::Auth {
        bitcoincore_rpc::Auth::UserPass(source.username, source.password)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_bitcoin::{Address, TxOut};
    use std::convert::TryFrom;

    trait FindUtxo {
        fn find_utxo_at_transaction_for_address(
            &self,
            transaction_id: &sha256d::Hash,
            address: &Address,
        ) -> Option<TxOut>;
    }

    // Copied from blockchain_contracts tests
    impl<Rpc: bitcoincore_rpc::RpcApi> FindUtxo for Rpc {
        fn find_utxo_at_transaction_for_address(
            &self,
            transaction_id: &sha256d::Hash,
            address: &Address,
        ) -> Option<TxOut> {
            let address = address.clone();
            let unspent = self
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
    }

    #[test]
    fn can_ping_bitcoin_node() {
        let bitcoin = BitcoinNode::start();

        let endpoint = format!("http://localhost:{}", bitcoin.port);
        let client = bitcoincore_rpc::Client::new(endpoint, bitcoin.auth.into()).unwrap();

        assert!(client.ping().is_ok())
    }

    #[test]
    fn can_fund_bitcoin_address() {
        let bitcoin = BitcoinNode::start();

        let endpoint = format!("http://localhost:{}", bitcoin.port);
        let client = bitcoincore_rpc::Client::new(endpoint, bitcoin.auth.clone().into()).unwrap();

        let address = client.get_new_address(None, None).unwrap();

        let value = Amount::from_sat(1_000);

        let transaction_id = bitcoin.fund(&address, value);

        assert!(client
            .find_utxo_at_transaction_for_address(&transaction_id, &address)
            .is_some());
    }
}
