use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "create-comit-app")]
pub enum CreateComitApp {
    StartEnv {},
}

// pub fn start_btsieve() {
//     Command::new("btsieve")
//         .spawn()
//         .expect("btsieve not found in path");
// }
