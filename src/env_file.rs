use envfile::EnvFile;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::Path;

const LOCK_FILE: &str = ".env.lock";
const LOCK_FILE_CONTENT: &[u8] =
    b"# This lock file is to ensure that no two create-comit-app updates the .env file.";

pub enum Error {
    IsLocked,
    CannotWriteLock(io::Error),
    CannotRemoveLock(io::Error),
    CannotUpdateEnvFile(io::Error),
}

pub trait LockUpdateWrite {
    fn lock_update_write(&mut self, key: &str, value: &str) -> Result<(), Error>;
}

fn lock() -> Result<(), Error> {
    let lock_path = Path::new(LOCK_FILE);
    if lock_path.exists() {
        Err(Error::IsLocked)
    } else {
        let mut file = File::create(LOCK_FILE)?;
        file.write_all(LOCK_FILE_CONTENT)?;
        Ok(())
    }
}

fn unlock() -> Result<(), Error> {
    std::fs::remove_file(LOCK_FILE).map_err(|e| Error::CannotRemoveLock(e))
}

impl LockUpdateWrite for EnvFile {
    fn lock_update_write(&mut self, key: &str, value: &str) -> Result<(), Error> {
        lock()
            .and(
                self.update(key, value)
                    .write()
                    .map_err(Error::CannotUpdateEnvFile),
            )
            .and(unlock())
    }
}

impl From<io::Error> for Error {
    fn from(io_error: io::Error) -> Self {
        Error::CannotWriteLock(io_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_lock_and_unlock() {
        let path = tempfile::Builder::new().tempfile().unwrap();
        let mut env_file = EnvFile::new(path.path()).unwrap();

        let res = env_file.lock_update_write("key", "value");

        assert!(res.is_ok())
    }
}
