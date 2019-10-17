use create_comit_app::create_comit_app::CreateComitApp;
use create_comit_app::new::new;
use create_comit_app::start_env::start_env;
use std::io;
use structopt::StructOpt;

const NEW_PROJECT_ARCHIVE: &'static [u8; 102689] =
    include_bytes!(concat!(env!("OUT_DIR"), "/new_project.tar.gz"));

fn main() -> io::Result<()> {
    let create_comit_app = CreateComitApp::from_args();

    match create_comit_app {
        CreateComitApp::StartEnv => start_env(),
        CreateComitApp::New { name } => new(name, NEW_PROJECT_ARCHIVE)?,
    }

    Ok(())
}
