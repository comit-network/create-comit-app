use std::path::PathBuf;

use anyhow::Context;

pub const DIR_NAME: &str = ".create-comit-app";
const ENV_FILE_NAME: &str = "env";

fn home() -> anyhow::Result<PathBuf> {
    dirs::home_dir().ok_or_else(|| anyhow::anyhow!("unable to determine home directory"))
}

pub fn dir_path() -> anyhow::Result<PathBuf> {
    Ok(home()?.join(DIR_NAME))
}

pub fn env_file_path() -> anyhow::Result<PathBuf> {
    Ok(dir_path()?.join(ENV_FILE_NAME))
}

pub async fn create_env_file() -> anyhow::Result<String> {
    let _ = ensure_cca_directory().await?;

    let env_file_path = env_file_path()?;
    tokio::fs::File::create(&env_file_path)
        .await
        .with_context(|| format!("failed to create file {}", env_file_path.display()))?;
    Ok(format!(
        "{}/{}/{}",
        home()?.display(),
        DIR_NAME,
        ENV_FILE_NAME
    ))
}

pub async fn dir_exist() -> bool {
    if let Ok(dir_path) = dir_path() {
        tokio::fs::read_dir(dir_path).await.is_ok()
    } else {
        false
    }
}

pub async fn temp_folder() -> anyhow::Result<PathBuf> {
    let path = ensure_cca_directory().await?;

    let path = tempfile::tempdir_in(&path)
        .with_context(|| format!("failed to create temporary directory in {}", path.display()))?
        .into_path();

    Ok(path)
}

async fn ensure_cca_directory() -> anyhow::Result<PathBuf> {
    let cca_path = dir_path()?;
    tokio::fs::create_dir_all(&cca_path)
        .await
        .with_context(|| format!("failed to create directory: {}", cca_path.display()))?;

    Ok(cca_path)
}
