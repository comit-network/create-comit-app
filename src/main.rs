use create_comit_app::bitcoin::{self, BitcoinNode};
use create_comit_app::ethereum::{self, EthereumNode};
use envfile::EnvFile;
use futures;
use futures::Future;
use hdwallet::traits::Serialize;
use hdwallet::{ExtendedPrivKey, KeyIndex};
use rust_bitcoin::Amount;
use std::iter::Iterator;
use std::path::PathBuf;
use web3::types::U256;

// TODO: Ensure that the .env file can only be written to by only one process at a time
// TODO: Proper error handling in particular to allow for cleanup of state after a runtime error
// TODO: Improve logs

fn main() {
    let mut runtime = tokio::runtime::Runtime::new().unwrap();

    let envfile_path = PathBuf::from(".env");
    std::fs::File::create(envfile_path.clone()).expect("Could not create .env file");

    let bitcoin_node = BitcoinNode::start(envfile_path.clone())
        .and_then({
            let mut hd_keys = Vec::new();
            move |node: BitcoinNode| {
                // Generate HD keys
                // Assume two parties for now
                for _ in 0..2 {
                    hd_keys.push(ExtendedPrivKey::random().expect("failed to generate hd key"));
                }

                // Fund addresses associated with HD keys with 1 bitcoin
                for hd_key in hd_keys.iter() {
                    let private_key = hd_key
                        .derive_private_key(KeyIndex::Normal(0))
                        .expect("failed to derive private key from hd key")
                        .private_key;
                    let address = bitcoin::derive_address(private_key);

                    node.fund(&address, Amount::from_sat(100_000_000));
                }

                Ok((node, hd_keys))
            }
        })
        .map_err(|e| {
            println!("Bitcoin node error: {}", e);
        });

    let ethereum_node = EthereumNode::start(envfile_path.clone())
        .and_then({
            let mut hd_keys = Vec::new();
            let executor = runtime.executor();
            move |node: EthereumNode| {
                // Generate HD keys
                // Assume two parties for now
                for _ in 0..2 {
                    hd_keys.push(ExtendedPrivKey::random().expect("failed to generate hd key"));
                }

                // Fund addresses associated with HD keys with 90 ether
                for hd_key in hd_keys.iter() {
                    let private_key = hd_key
                        .derive_private_key(KeyIndex::Normal(0))
                        .expect("failed to derive private key from hd key")
                        .private_key;
                    let address = ethereum::derive_address(private_key);

                    executor.spawn(
                        node.fund(address, U256::from("9000000000000000000"))
                            .and_then(|_| Ok(()))
                            .map_err(|e| println!("Could not fund ethereum addresses: {}", e)),
                    );
                }

                Ok((node, hd_keys))
            }
        })
        .map_err(|e| println!("Ethereum node error: {}", e));

    let results = runtime.block_on(bitcoin_node.join(ethereum_node)).unwrap();

    println!("Blockchain nodes up and running");

    let (bitcoin_hd_keys, ethereum_hd_keys) = ((results.0).1, (results.1).1);

    // Store HD keys in .env file
    let mut envfile = EnvFile::new(envfile_path).unwrap();

    for (i, hd_key) in bitcoin_hd_keys.iter().enumerate() {
        envfile.update(
            format!("BITCOIN_HD_KEY_{}", i).as_str(),
            hex::encode(&hd_key.serialize()).as_str(),
        );
    }

    for (i, hd_key) in ethereum_hd_keys.iter().enumerate() {
        envfile.update(
            format!("ETHEREUM_HD_KEY_{}", i).as_str(),
            hex::encode(&hd_key.serialize()).as_str(),
        );
    }
    envfile.write().unwrap();

    // TODO: Unblocking this via CTRL+C doesn't call drop on the containers afterwards
    ::std::thread::park();
}
