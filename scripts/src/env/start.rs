use anyhow::Context;
use envfile::EnvFile;

use crate::{
    config::{self, Config},
    docker::{
        self,
        bitcoin::{self, BitcoindInstance, PASSWORD, USERNAME},
        cnd::{self, CndInstance},
        ethereum::{self, GethInstance},
    },
    print_progress, temp_fs,
};
use std::path::Path;

pub struct Environment {
    pub docker_network_id: String,
    pub bitcoind: BitcoindInstance,
    pub geth: GethInstance,
    pub cnd_0: CndInstance,
    pub cnd_1: CndInstance,
}

pub async fn execute() -> anyhow::Result<Environment> {
    print_progress!("Creating Docker network (create-comit-app)");

    let docker_network_id = docker::create_network().await?;

    println!("✓");

    print_progress!("Reading config file");

    let path = std::env::current_dir()?.join(Path::new(config::FILE_NAME));
    let (bitcoin_config, ethereum_config) = match Config::from_file(&path) {
        Ok(config) => {
            println!("✓");
            (config.ethereum, config.bitcoin)
        }
        Err(_e) => {
            println!("No config file found, only funding generated default accounts.");
            (None, None)
        }
    };

    print_progress!("Starting Ethereum node");

    let geth = ethereum::new_geth_instance(bitcoin_config).await?;

    println!("✓");

    print_progress!("Starting Bitcoin node");

    let bitcoind = bitcoin::new_bitcoind_instance(ethereum_config).await?;

    println!("✓");

    print_progress!("Starting two cnds");
    let cnd_0 = cnd::new_instance(0)
        .await
        .context("failed to start first cnd")?;

    let cnd_1 = cnd::new_instance(1)
        .await
        .context("failed to start second cnd")?;

    println!("✓");

    let env_file_str = temp_fs::create_env_file().await?;
    print_progress!("Writing configuration to {}", env_file_str);

    let mut envfile = EnvFile::new(env_file_str)?;
    envfile.update("ETHEREUM_KEY_0", &format!("{}", geth.account_0.private_key));
    envfile.update("ETHEREUM_KEY_1", &format!("{}", geth.account_1.private_key));
    envfile.update(
        "ERC20_CONTRACT_ADDRESS",
        &format!("{:#x}", geth.erc20_contract_address),
    );
    envfile.update("ETHEREUM_NODE_HTTP_URL", &geth.http_endpoint.to_string());

    envfile.update("BITCOIN_WALLET_0", &bitcoind.account_0.to_string());
    envfile.update("BITCOIN_WALLET_1", &bitcoind.account_1.to_string());
    envfile.update("BITCOIN_P2P_URI", &bitcoind.p2p_uri.to_string());
    envfile.update("BITCOIN_HTTP_URI", &bitcoind.http_endpoint.to_string());
    envfile.update("BITCOIN_USERNAME", USERNAME);
    envfile.update("BITCOIN_PASSWORD", PASSWORD);

    envfile.update("HTTP_URL_CND_0", &cnd_0.http_endpoint.to_string());
    envfile.update("HTTP_URL_CND_1", &cnd_1.http_endpoint.to_string());

    envfile.write()?;

    println!("✓");

    println!("🎉 Environment is ready, time to create a COMIT app!");
    Ok(Environment {
        docker_network_id,
        geth,
        bitcoind,
        cnd_0,
        cnd_1,
    })
}
