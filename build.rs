use flate2::{write::GzEncoder, Compression};
use ignore::WalkBuilder;
use snafu::ResultExt;
use std::path::PathBuf;
use std::{env, fs::File, io, path::Path};

#[derive(Debug, snafu::Snafu)]
enum Error {
    #[snafu(display("Unable to read OUT_DIR environment variable"))]
    ResolveOutDir { source: env::VarError },
    #[snafu(display("Unable to create temporary archive at {}", path.display()))]
    CreateArchive { source: io::Error, path: PathBuf },
    #[snafu(display("Unable to switch current working directory to {}", path.display()))]
    SwitchWorkingDirectory { source: io::Error, path: PathBuf },
    #[snafu(display("Unable to append {} to archive", path.display()))]
    AppendToArchive { source: io::Error, path: PathBuf },
    #[snafu(display("Failure while walking directory tree"))]
    WalkDir { source: ignore::Error },
}

fn main() -> Result<(), Error> {
    let out_dir = env::var("OUT_DIR").context(ResolveOutDir)?;
    let out_dir = Path::new(&out_dir);
    let archive = out_dir.join("new_project.tar.gz");

    let archive = File::create(archive.clone()).context(CreateArchive { path: archive })?;
    let mut archive = tar::Builder::new(GzEncoder::new(archive, Compression::default()));

    let new_project_folder = Path::new("./new_project");
    env::set_current_dir(&new_project_folder).context(SwitchWorkingDirectory {
        path: new_project_folder,
    })?;

    // use the ignore library to skip all files specified in .gitignore
    for result in WalkBuilder::new("./").hidden(false).build() {
        let entry = result.context(WalkDir)?;
        let path = entry.path();

        archive
            .append_path(path)
            .context(AppendToArchive { path })?;
        // prevent rerun if files did not change
        println!("cargo:rerun-if-changed={}", path.display());
    }
    Ok(())
}
