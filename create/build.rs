use std::{env, fs::File, path::Path};

use anyhow::Context;
use flate2::{write::GzEncoder, Compression};
use ignore::WalkBuilder;

fn main() -> anyhow::Result<()> {
    let out_dir = env::var("OUT_DIR").context("unable to read OUT_DIR variable")?;
    let out_dir = Path::new(&out_dir);
    let archive = out_dir.join("new_project.tar.gz");

    let archive = File::create(archive.clone())
        .with_context(|| format!("unable to create archive at {}", archive.display()))?;
    let mut archive = tar::Builder::new(GzEncoder::new(archive, Compression::default()));

    let new_project_folder = Path::new(".").canonicalize()?.join("new_project");

    // we set the working directory to the `new_project` folder to avoid it being contained in the archive
    env::set_current_dir(&new_project_folder).context("unable to enter new_project directory")?;

    // use the ignore library to skip all files specified in .gitignore
    for result in WalkBuilder::new(".").hidden(false).build() {
        let entry = result.context("unable to walk directory")?;
        let path = entry.path();

        archive
            .append_path(path)
            .with_context(|| format!("unable to add {} to the archive", path.display()))?;
        // prevent rerun if files did not change
        println!(
            "cargo:rerun-if-changed={}",
            new_project_folder.join(path).display()
        );
    }
    Ok(())
}
