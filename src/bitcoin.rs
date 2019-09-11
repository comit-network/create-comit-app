use bitcoincore_rpc::RpcApi;
use rust_bitcoin::{self, Amount};
use std::str::FromStr;
use testcontainers::{self, Docker};

pub fn start_bitcoin_node() {
    let docker = testcontainers::clients::Cli::default();
    let container = docker.run(testcontainers::images::coblox_bitcoincore::BitcoinCore::default());

    let port = container.get_host_port(18443).unwrap();
    let auth = container.image().auth();

    let endpoint = format!("http://localhost:{}", port);

    let client = bitcoincore_rpc::Client::new(
        endpoint,
        bitcoincore_rpc::Auth::UserPass(auth.username().to_owned(), auth.password().to_owned()),
    )
    .unwrap();

    client.generate(101, None).unwrap();

    // Fund maker address

    // FIXME: Derive address from seed
    let maker_address =
        rust_bitcoin::Address::from_str("bcrt1qmmpq3reyxf7866xk57lvqemguhsjwq06h6x9pg").unwrap();
    let amount = Amount::from_sat(100_000_000);

    let _ = client.send_to_address(&maker_address, amount, None, None, None, None, None, None);

    client.generate(1, None).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_ping_bitcoin_node() {
        let bitcoin = BitcoinNode::start();

        let endpoint = format!("http://localhost:{}", bitcoin.port);
        let client = bitcoincore_rpc::Client::new(endpoint, bitcoin.auth).unwrap();

        assert!(client.ping().is_ok())
    }
}
