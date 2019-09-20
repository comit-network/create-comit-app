use create_comit_app::create_comit_app::CreateComitApp;
use create_comit_app::new::new;
use create_comit_app::start_env::start_env;
use structopt::StructOpt;

fn main() {
    let create_comit_app = CreateComitApp::from_args();

    match create_comit_app {
        CreateComitApp::StartEnv => start_env(),
        CreateComitApp::New => new(),
    }
}
