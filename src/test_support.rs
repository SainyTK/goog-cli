use std::sync::{Mutex, MutexGuard};

static CURRENT_DIR_LOCK: Mutex<()> = Mutex::new(());

pub(crate) struct CurrentDirGuard {
    original: std::path::PathBuf,
    _lock: MutexGuard<'static, ()>,
}

impl CurrentDirGuard {
    pub(crate) fn enter(path: impl AsRef<std::path::Path>) -> Self {
        let lock = CURRENT_DIR_LOCK.lock().unwrap();
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(path).unwrap();
        Self {
            original,
            _lock: lock,
        }
    }
}

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.original).unwrap();
    }
}
