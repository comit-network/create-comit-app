use bitcoincore_rpc::RpcApi;
use envfile::EnvFile;
use futures::future::Future;
use futures::stream::Stream;
use rust_bitcoin::{self, hashes::sha256d, Address, Amount, Network};
use shiplift::{ContainerOptions, Docker, LogsOptions, PullOptions, RmContainerOptions};
use std::path::PathBuf;
use tokio;

pub const RPC_URL_KEY: &str = "BITCOIN_NODE_RPC_URL";

pub struct BitcoinNode {
    pub container_id: String,
    pub rpc_client: bitcoincore_rpc::Client,
}

// TODO: Move all envfile stuff outside
// TODO: Move free_local_port outside
impl BitcoinNode {
    const IMAGE: &'static str = "coblox/bitcoin-core:0.17.0";
    pub fn start(
        envfile_path: PathBuf,
    ) -> impl Future<Item = BitcoinNode, Error = shiplift::errors::Error> {
        let docker = Docker::new();
        docker
            .images()
            .pull(&PullOptions::builder().image(Self::IMAGE).build())
            // TODO: Pretty print progress
            .collect()
            .and_then(|_| Self::start_container(envfile_path))
    }

    fn start_container(
        envfile_path: PathBuf,
    ) -> impl Future<Item = BitcoinNode, Error = shiplift::errors::Error> {
        let username = "bitcoin";
        let password = "t68ej4UX2pB0cLlGwSwHFBLKxXYgomkXyFyxuBmm2U8=";
        let rpc_port: u32 = port_check::free_local_port().unwrap().into();
        let rpc_url = format!("http://localhost:{}", rpc_port);
        let docker = Docker::new();
        docker
            .containers()
            .create(
                &ContainerOptions::builder(Self::IMAGE)
                    .cmd(vec![
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
                    ])
                    .expose(18443, "tcp", rpc_port)
                    .expose(18444, "tcp", port_check::free_local_port().unwrap().into())
                    .build(),
            )
            .and_then({
                let docker = docker.clone();
                move |container| {
                    let id = container.id;
                    docker.containers().get(&id).start().map(|_| id)
                }
            })
            .and_then({
                let docker = docker.clone();
                move |id| {
                    docker.containers()
                        .get(&id)
                        .logs(&LogsOptions::builder().stdout(true).follow(true).build())
                        .take_while(|chunk| {
                            let log = chunk.as_string_lossy();
                            Ok(!log.contains("Flushed wallet.dat"))
                        }).collect().map(|_| id)
                }})
            .and_then({
                let rpc_url = rpc_url.clone();
                move |container_id| {
                    let rpc_client = bitcoincore_rpc::Client::new(
                        rpc_url.clone(),
                        bitcoincore_rpc::Auth::UserPass(username.to_string(), password.to_string()),
                    ).unwrap();

                    let node = BitcoinNode {
                        container_id,
                        rpc_client,
                    };

                    node.rpc_client.generate(101, None).unwrap();
                    Ok(node)
                }})
            .and_then({
                let envfile_path = envfile_path.clone();
                move |node| {
                    let mut envfile = EnvFile::new(envfile_path).unwrap();
                    envfile.update(RPC_URL_KEY, &rpc_url).write().unwrap();

                    Ok(node)
                }})
    }

    pub fn fund(&self, address: &Address, amount: Amount) -> sha256d::Hash {
        let client = &self.rpc_client;

        let transaction_id = client
            .send_to_address(&address, amount, None, None, None, None, None, None)
            .unwrap();

        client.generate(1, None).unwrap();

        transaction_id
    }
}

impl Drop for BitcoinNode {
    fn drop(&mut self) {
        let docker = Docker::new();

        let rm_fut = docker
            .containers()
            .get(&self.container_id)
            .remove(
                RmContainerOptions::builder()
                    .force(true)
                    .volumes(true)
                    .build(),
            )
            .map_err(|_| ());

        tokio::run(rm_fut);
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
            .block_on(BitcoinNode::start(file.path().to_path_buf()))
            .unwrap();

        assert!(bitcoin.rpc_client.ping().is_ok());
    }

    #[test]
    fn can_fund_bitcoin_address() {
        let mut runtime = tokio::runtime::Runtime::new().unwrap();

        let file = tempfile::Builder::new().tempfile().unwrap();

        let bitcoin = runtime
            .block_on(BitcoinNode::start(file.path().to_path_buf()))
            .unwrap();
        let client = &bitcoin.rpc_client;

        let address = client.get_new_address(None, None).unwrap();
        let value = Amount::from_sat(1_000);
        let transaction_id = bitcoin.fund(&address, value);

        assert!(find_utxo_at_transaction_for_address(client, &transaction_id, &address).is_some());
    }

    #[test]
    fn can_get_rpc_port_from_envfile() {
        let mut runtime = tokio::runtime::Runtime::new().unwrap();

        let file = tempfile::Builder::new().tempfile().unwrap();

        runtime
            .block_on(BitcoinNode::start(file.path().to_path_buf()))
            .unwrap();

        let envfile = EnvFile::new(&file.path()).unwrap();
        assert!(envfile.get(RPC_URL_KEY).is_some());
    }
}
