use crate::{
    docker::{
        self,
        bitcoin::{self, BitcoindInstance},
        cnd::{self, CndInstance},
        ethereum::{self, ParityInstance},
    },
    print_progress, temp_fs,
};
use anyhow::Context;
use envfile::EnvFile;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub struct Environment {
    pub docker_network_id: String,
    pub bitcoind: BitcoindInstance,
    pub parity: ParityInstance,
    pub cnd_0: CndInstance,
    pub cnd_1: CndInstance,
}

pub async fn execute(terminate: Arc<AtomicBool>) -> anyhow::Result<Environment> {
    print_progress!("Creating Docker network (create-comit-app)");

    let docker_network_id = docker::create_network().await?;

    println!("✓");
    check_signal(terminate.as_ref())?;

    print_progress!("Starting Ethereum node");

    let parity = ethereum::new_parity_instance().await?;

    println!("✓");
    check_signal(terminate.as_ref())?;

    print_progress!("Starting Bitcoin node");

    let bitcoind = bitcoin::new_bitcoind_instance().await?;

    println!("✓");
    check_signal(terminate.as_ref())?;

    print_progress!("Starting two cnds");
    let cnd_0 = cnd::new_instance(0)
        .await
        .context("failed to start first cnd")?;

    let cnd_1 = cnd::new_instance(1)
        .await
        .context("failed to start second cnd")?;

    println!("✓");
    check_signal(terminate.as_ref())?;

    print_progress!("Writing configuration in env file");

    let env_file_str = temp_fs::create_env_file()?;
    let mut envfile = EnvFile::new(env_file_str)?;

    envfile.update(
        "ETHEREUM_KEY_0",
        &format!("{}", parity.account_0.private_key),
    );
    envfile.update(
        "ETHEREUM_KEY_1",
        &format!("{}", parity.account_1.private_key),
    );
    envfile.update(
        "ERC20_CONTRACT_ADDRESS",
        &format!("{:#x}", parity.erc20_contract_address),
    );
    envfile.update("ETHEREUM_NODE_HTTP_URL", &parity.http_endpoint.to_string());

    envfile.update(
        "BITCOIN_HD_KEY_0",
        &format!("{}", bitcoind.account_0.master),
    );
    envfile.update(
        "BITCOIN_HD_KEY_1",
        &format!("{}", bitcoind.account_1.master),
    );
    envfile.update("BITCOIN_NODE_RPC_URL", &bitcoind.http_endpoint.to_string());
    envfile.update("BITCOIN_P2P_URI", &bitcoind.p2p_uri.to_string());

    envfile.update("HTTP_URL_CND_0", &cnd_0.http_endpoint.to_string());
    envfile.update("HTTP_URL_CND_1", &cnd_1.http_endpoint.to_string());

    envfile.write()?;

    println!("✓");
    check_signal(terminate.as_ref())?;

    println!("🎉 Environment is ready, time to create a COMIT app!");
    Ok(Environment {
        docker_network_id,
        parity,
        bitcoind,
        cnd_0,
        cnd_1,
    })
}

#[derive(Debug, thiserror::Error)]
#[error("received termination signal")]
pub struct SignalReceived;

fn check_signal(terminate: &AtomicBool) -> Result<(), SignalReceived> {
    if terminate.load(Ordering::Relaxed) {
        Err(SignalReceived)
    } else {
        Ok(())
    }
}
