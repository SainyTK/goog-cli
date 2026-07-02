use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

static CURRENT_DIR_LOCK: Mutex<()> = Mutex::new(());

pub(crate) struct CurrentDirGuard {
    original: PathBuf,
    _current_dir_lock: MutexGuard<'static, ()>,
}

impl CurrentDirGuard {
    pub(crate) fn enter(path: impl AsRef<Path>) -> Self {
        let current_dir_lock = CURRENT_DIR_LOCK.lock().unwrap();
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(path).unwrap();
        Self {
            original,
            _current_dir_lock: current_dir_lock,
        }
    }
}

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.original).unwrap();
    }
}
