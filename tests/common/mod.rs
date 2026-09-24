use std::sync::{Mutex, MutexGuard, PoisonError};

static RECOGNITION: Mutex<()> = Mutex::new(());

pub fn recognition_lock() -> MutexGuard<'static, ()> {
    RECOGNITION.lock().unwrap_or_else(PoisonError::into_inner)
}
