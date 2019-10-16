extern crate flate2;
extern crate tar;

use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::File;

use ignore::{Walk, WalkBuilder};
use std::path::Path;
use std::{env, io};

fn main() -> io::Result<()> {
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir);
    let archive = out_dir.join("new_project.tar.gz");

    let tar_gz = File::create(archive)?;
    let enc = GzEncoder::new(tar_gz, Compression::default());
    let mut tar = tar::Builder::new(enc);

    let root = Path::new("./new_project");
    env::set_current_dir(&root)?;

    // use the ignore library to skip all files specified in .gitignore
    for result in WalkBuilder::new("./").hidden(false).build() {
        match result {
            Ok(entry) => {
                tar.append_path(entry.path())?;
                // prevent rerun if files did not change
                println!("cargo:rerun-if-changed={}", entry.path().display());
            }
            Err(err) => {}
        }
    }
    Ok(())
}
