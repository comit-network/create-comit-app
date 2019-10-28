use create_comit_app::create_comit_app::CreateComitApp;
use create_comit_app::env::clean_up;
use create_comit_app::env::start;
use create_comit_app::new::new;
use std::io;
use structopt::StructOpt;

const NEW_PROJECT_ARCHIVE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/new_project.tar.gz"));

fn main() -> io::Result<()> {
    let create_comit_app = CreateComitApp::from_args();

    match create_comit_app {
        CreateComitApp::StartEnv => start(),
        CreateComitApp::New { name } => new(name, NEW_PROJECT_ARCHIVE)?,
        CreateComitApp::ForceCleanEnv => clean_up(),
    }

    Ok(())
}
