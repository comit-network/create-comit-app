use bitcoincore_rpc::RpcApi;
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

        let client = bitcoincore_rpc::Client::new(
            endpoint,
            bitcoincore_rpc::Auth::UserPass(auth.clone().username, auth.clone().password),
        )
        .unwrap();

        client.generate(101, None).unwrap();

        node
    }
}

pub struct Auth {
    username: String,
    password: String,
}

impl From<Auth> for bitcoincore_rpc::Auth {
    fn from(source: Auth) -> bitcoincore_rpc::Auth {
        bitcoincore_rpc::Auth::UserPass(source.username, source.password)
    }
}

// pub fn start_bitcoin_node() {
//     // Fund maker address

//     // FIXME: Derive address from seed
//     let maker_address =
//         rust_bitcoin::Address::from_str("bcrt1qmmpq3reyxf7866xk57lvqemguhsjwq06h6x9pg").unwrap();
//     let amount = Amount::from_sat(100_000_000);

//     let _ = client.send_to_address(&maker_address, amount, None, None, None, None, None, None);

//     client.generate(1, None).unwrap();
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_ping_bitcoin_node() {
        let bitcoin = BitcoinNode::start();

        let endpoint = format!("http://localhost:{}", bitcoin.port);
        let client = bitcoincore_rpc::Client::new(endpoint, bitcoin.auth.into()).unwrap();

        assert!(client.ping().is_ok())
    }
}
