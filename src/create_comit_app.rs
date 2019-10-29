use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "create-comit-app")]
pub enum CreateComitApp {
    StartEnv,
    New {
        #[structopt(name = "name")]
        name: String,
    },
    ForceCleanEnv,
}
