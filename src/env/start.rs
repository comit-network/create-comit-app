use crate::cnd_settings;
use crate::docker::bitcoin::{self, BitcoinNode};
use crate::docker::ethereum::{self, EthereumNode};
use crate::docker::Cnd;
use crate::docker::{create_network, Node};
use crate::env::temp_fs;
use crate::print_progress;
use anyhow::Context;
use envfile::EnvFile;
use futures::{FutureExt, TryFutureExt};
use rand::{thread_rng, Rng};
use rust_bitcoin::util::bip32::ChildNumber;
use rust_bitcoin::util::bip32::ExtendedPrivKey;
use rust_bitcoin::Amount;
use rust_bitcoin::PrivateKey;
use secp256k1::{Secp256k1, SecretKey};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::prelude::stream;
use tokio::prelude::{Future, Stream};
use tokio::runtime::Runtime;
use web3::types::{Address, U256};

pub struct Services {
    pub docker_network_id: String,
    pub bitcoin_node: Arc<Node<BitcoinNode>>,
    pub ethereum_node: Arc<Node<EthereumNode>>,
    pub cnds: Arc<Vec<Node<Cnd>>>,
}

#[derive(Clone)]
pub struct BitcoinAccount {
    master: ExtendedPrivKey,
    first_account: PrivateKey,
}

impl BitcoinAccount {
    fn new_random() -> anyhow::Result<Self> {
        let mut seed = [0u8; 32];
        thread_rng().fill_bytes(&mut seed);

        let master = ExtendedPrivKey::new_master(rust_bitcoin::Network::Regtest, &seed)
            .context("failed to generate new random extended private key from seed")?;

        // define derivation path to derive private keys from the master key
        let derivation_path = vec![
            ChildNumber::from_hardened_idx(44)?,
            ChildNumber::from_hardened_idx(1)?,
            ChildNumber::from_hardened_idx(0)?,
            ChildNumber::from_normal_idx(0)?,
            ChildNumber::from_normal_idx(0)?,
        ];

        // derive a private key from the master key
        let priv_key = master
            .derive_priv(&Secp256k1::new(), &derivation_path)
            .map(|secret_key| secret_key.private_key)?;

        // it is not great to store derived data in here but since the derivation can fail, it is better to fail early instead of later
        Ok(Self {
            master,
            first_account: priv_key,
        })
    }

    fn first_account(&self) -> (PrivateKey, rust_bitcoin::Address) {
        // derive an address from the private key
        let address = bitcoin::derive_address(self.first_account.key);

        (self.first_account, address)
    }
}

#[derive(Clone)]
pub struct EthereumAccount {
    private_key: SecretKey,
}

impl EthereumAccount {
    fn new_random() -> Self {
        Self {
            private_key: SecretKey::new(&mut thread_rng()),
        }
    }
}

fn build_futures() -> anyhow::Result<(
    Vec<BitcoinAccount>,
    Vec<EthereumAccount>,
    PathBuf,
    impl Future<Item = String, Error = anyhow::Error>,
    impl Future<Item = Node<BitcoinNode>, Error = anyhow::Error>,
    impl Future<Item = (Address, Node<EthereumNode>), Error = anyhow::Error>,
    impl Future<Item = Vec<Node<Cnd>>, Error = anyhow::Error>,
)> {
    let bitcoin_accounts = vec![BitcoinAccount::new_random()?, BitcoinAccount::new_random()?];

    let ethereum_accounts = vec![EthereumAccount::new_random(), EthereumAccount::new_random()];

    let env_file_path = temp_fs::env_file_path()?;
    let docker_network_create =
        create_network().map_err(|e| e.context("failed to create docker network"));
    let bitcoin_node = start_bitcoin_node(&env_file_path, bitcoin_accounts.clone())
        .map_err(|e| e.context("failed to start bitcoin node"));
    let ethereum_node = start_ethereum_node(&env_file_path, ethereum_accounts.clone())
        .map_err(|e| e.context("failed to start ethereum node"));
    let cnds = {
        let path = temp_fs::temp_folder()?;
        start_cnds(&env_file_path, path).map_err(|e| e.context("failed to start cnds"))
    };
    Ok((
        bitcoin_accounts,
        ethereum_accounts,
        env_file_path,
        docker_network_create,
        bitcoin_node,
        ethereum_node,
        cnds,
    ))
}

pub fn execute(runtime: &mut Runtime, terminate: &Arc<AtomicBool>) -> anyhow::Result<Services> {
    let (
        bitcoin_accounts,
        ethereum_accounts,
        env_file_path,
        docker_network_create,
        bitcoin_node,
        ethereum_node,
        cnds,
    ) = build_futures()?;

    let env_file_str = temp_fs::create_env_file()?;

    print_progress!("Creating Docker network (create-comit-app)");
    let docker_network_id = runtime
        .block_on(docker_network_create)
        .context("Could not create docker network, aborting...")?;
    println!("✓");
    check_signal(terminate)?;

    print_progress!("Starting Ethereum node");
    let (contract_address, ethereum_node) = runtime
        .block_on(ethereum_node)
        .context("Could not start Ethereum node, aborting...")
        .map(|(contract_address, node)| (contract_address, Arc::new(node)))?;
    println!("✓");
    check_signal(terminate)?;

    print_progress!("Starting Bitcoin node");
    let bitcoin_node = runtime
        .block_on(bitcoin_node)
        .context("Could not start bitcoin node, aborting...")
        .map(Arc::new)?;
    println!("✓");
    check_signal(terminate)?;

    print_progress!("Writing configuration in env file");
    let mut envfile = EnvFile::new(env_file_path.clone())
        .with_context(|| format!("Could not read {} file, aborting...", env_file_str))?;

    for (i, account) in bitcoin_accounts.iter().enumerate() {
        envfile.update(
            format!("BITCOIN_HD_KEY_{}", i).as_str(),
            format!("{}", account.master).as_str(),
        );
    }

    for (i, account) in ethereum_accounts.iter().enumerate() {
        envfile.update(
            format!("ETHEREUM_KEY_{}", i).as_str(),
            format!("{}", account.private_key).as_str(),
        );
    }

    envfile.update(
        "ERC20_CONTRACT_ADDRESS",
        format!("0x{:x}", contract_address).as_str(),
    );

    envfile
        .write()
        .with_context(|| format!("Could not write {} file, aborting...", env_file_str))?;
    println!("✓");
    check_signal(terminate)?;

    print_progress!("Starting two cnds");
    let cnds = runtime
        .block_on(cnds)
        .context("Could not start cnds, cleaning up...")
        .map(Arc::new)?;
    println!("✓");
    check_signal(terminate)?;

    println!("🎉 Environment is ready, time to create a COMIT app!");
    Ok(Services {
        docker_network_id,
        bitcoin_node,
        ethereum_node,
        cnds,
    })
}

fn start_bitcoin_node(
    envfile_path: &PathBuf,
    bitcoin_accounts: Vec<BitcoinAccount>,
) -> impl Future<Item = Node<BitcoinNode>, Error = anyhow::Error> {
    Node::<BitcoinNode>::start(envfile_path.clone(), "bitcoin")
        .from_err()
        .and_then(move |node| {
            stream::iter_ok(bitcoin_accounts).fold(node, |node, account| {
                let (_, address) = account.first_account();
                bitcoin::fund(
                    node.node_image.username.clone(),
                    node.node_image.password.clone(),
                    node.node_image.endpoint.clone(),
                    address,
                    Amount::from_sat(1_000_000_000),
                )
                .map(|_| node)
            })
        })
}

fn start_ethereum_node(
    envfile_path: &PathBuf,
    ethereum_accounts: Vec<EthereumAccount>,
) -> impl Future<Item = (Address, Node<EthereumNode>), Error = anyhow::Error> {
    Node::<EthereumNode>::start(envfile_path.clone(), "ethereum")
        .from_err()
        .and_then({
            let secret_keys = ethereum_accounts
                .clone()
                .into_iter()
                .map(|account| account.private_key)
                .collect();
            |node| {
                let web3 = node.node_image.http_client.clone();
                ethereum::fund_ether(web3, secret_keys, U256::from(100u128 * 10u128.pow(18)))
                    .boxed()
                    .compat()
                    .from_err()
                    .map(|_| node)
            }
        })
        .and_then({
            let secret_keys = ethereum_accounts
                .into_iter()
                .map(|account| account.private_key)
                .collect();
            |node| {
                let web3 = node.node_image.http_client.clone();
                ethereum::fund_erc20(web3, secret_keys, U256::from(100u128 * 10u128.pow(18)))
                    .boxed()
                    .compat()
                    .from_err()
                    .map(|contract_address| (contract_address, node))
            }
        })
}

fn start_cnds(
    envfile_path: &PathBuf,
    config_folder: PathBuf,
) -> impl Future<Item = Vec<Node<Cnd>>, Error = anyhow::Error> {
    stream::iter_ok(vec![0, 1])
        .and_then({
            let envfile_path = envfile_path.clone();

            move |i| {
                tokio::fs::create_dir_all(config_folder.clone())
                    .from_err()
                    .and_then({
                        let config_folder = config_folder.clone();

                        move |_| {
                            let settings = cnd_settings::Settings {
                                bitcoin: cnd_settings::Bitcoin {
                                    network: String::from("regtest"),
                                    node_url: "http://bitcoin:18443".to_string(),
                                },
                                ethereum: cnd_settings::Ethereum {
                                    network: String::from("regtest"),
                                    node_url: "http://ethereum:8545".to_string(),
                                },
                                ..Default::default()
                            };

                            let config_file = config_folder.join("cnd.toml");
                            let settings = toml::to_string(&settings)
                                .expect("could not serialize hardcoded settings");

                            tokio::fs::write(config_file, settings).from_err()
                        }
                    })
                    .and_then({
                        let envfile_path = envfile_path.clone();
                        let config_folder = config_folder.clone();

                        move |_| {
                            let volume = format!("{}:/config", config_folder.display());

                            Node::<Cnd>::start_with_volume(
                                envfile_path.to_path_buf(),
                                format!("cnd_{}", i).as_str(),
                                &volume,
                            )
                            .from_err()
                        }
                    })
            }
        })
        .collect()
}

#[derive(Debug, thiserror::Error)]
#[error("received termination signal")]
pub struct SignalReceived;

fn check_signal(terminate: &Arc<AtomicBool>) -> Result<(), SignalReceived> {
    if terminate.load(Ordering::Relaxed) {
        Err(SignalReceived)
    } else {
        Ok(())
    }
}
