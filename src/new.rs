use flate2::read::GzDecoder;
use std::io;
use std::path::Path;
use tar::Archive;

pub fn new(name: String, examples_archive: &[u8]) -> io::Result<()> {
    let tar = GzDecoder::new(examples_archive);
    let mut archive = Archive::new(tar);

    let path_to_write = Path::new(&name);
    archive.unpack(path_to_write)?;

    println!("Your project `{}` has been created!\nNow, start your development environment by running `create-comit-app start-env`", name);

    Ok(())
}
