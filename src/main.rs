#![type_length_limit = "8760995"]

use create_comit_app::{create_comit_app::CreateComitApp, env, new::new};
use structopt::StructOpt;

fn main() -> std::io::Result<()> {
    let mut runtime = tokio_compat::runtime::Runtime::new()?;

    let command = CreateComitApp::from_args();

    runtime.block_on_std(run_command(command))?;

    Ok(())
}

async fn run_command(command: CreateComitApp) -> std::io::Result<()> {
    match command {
        CreateComitApp::StartEnv => env::start().await,
        CreateComitApp::New { name } => new(name).await?,
        CreateComitApp::ForceCleanEnv => env::clean_up().await,
    }

    Ok(())
}
