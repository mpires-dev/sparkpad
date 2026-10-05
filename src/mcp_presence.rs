//! Process-held file locks expose initialized stdio sessions to the desktop app.
//! The OS releases a lock on exit/crash; no heartbeat writes or idle timeout are needed.
use anyhow::Result;
use std::{fs::{self, File, TryLockError}, path::{Path, PathBuf}};

pub struct Session {
    path: PathBuf,
    _lock: File,
}

impl Session {
    pub fn start(directory: &Path) -> Result<Self> {
        fs::create_dir_all(directory)?;
        let id = uuid::Uuid::new_v4();
        let pending = directory.join(format!("{id}.pending"));
        let path = directory.join(format!("{id}.lock"));
        let file = File::create_new(&pending)?;
        // Publish only after locking, so the app cannot mistake a new session for
        // an abandoned file in the interval between creation and lock acquisition.
        if let Err(error) = file.lock().and_then(|()| fs::rename(&pending, &path)) {
            let _ = fs::remove_file(&pending);
            return Err(error.into());
        }
        Ok(Self { path, _lock: file })
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub fn connected(directory: &Path) -> Result<bool> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    let mut connected = false;
    for entry in entries {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("lock") {
            continue;
        }
        let file = match File::options().read(true).write(true).open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        match file.try_lock() {
            Err(TryLockError::WouldBlock) => connected = true,
            Ok(()) => { let _ = fs::remove_file(&path); }
            Err(TryLockError::Error(error)) => return Err(error.into()),
        }
    }
    Ok(connected)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn concurrent_sessions_and_abandoned_locks() -> Result<()> {
        let directory = std::env::temp_dir().join(format!("sparkpad-presence-{}", uuid::Uuid::new_v4()));
        assert!(!connected(&directory)?);
        let first = Session::start(&directory)?;
        let second = Session::start(&directory)?;
        assert!(connected(&directory)?);
        drop(first);
        assert!(connected(&directory)?);
        drop(second);
        assert!(!connected(&directory)?);
        File::create_new(directory.join("abandoned.lock"))?;
        File::create_new(directory.join("unpublished.pending"))?;
        assert!(!connected(&directory)?);
        assert!(!directory.join("abandoned.lock").exists());
        fs::remove_dir_all(directory)?;
        Ok(())
    }
}
