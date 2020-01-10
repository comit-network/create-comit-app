use structopt::StructOpt;

use create_comit_app::new::new;

fn main() -> std::io::Result<()> {
    let mut runtime = tokio_compat::runtime::Runtime::new()?;

    let args = CreateComitApp::from_args();

    runtime.block_on_std(run(args))?;

    Ok(())
}

async fn run(args: CreateComitApp) -> std::io::Result<()> {
    new(args.name).await?;
    Ok(())
}

#[derive(StructOpt, Debug)]
#[structopt(name = "create-comit-app")]
pub struct CreateComitApp {
    #[structopt(name = "name")]
    name: String,
}
