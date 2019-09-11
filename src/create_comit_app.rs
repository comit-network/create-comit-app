use std::process::Command;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "create-comit-app")]
pub enum CreateComitApp {
    StartEnv {},
}

pub fn start_cnd() {
    Command::new("cnd").spawn().expect("cnd not found in path");
}

pub fn start_btsieve() {
    Command::new("btsieve")
        .spawn()
        .expect("btsieve not found in path");
}
